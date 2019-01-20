use std::collections::{VecDeque, HashMap, HashSet, BTreeSet};
use std::rc::Rc;
use std::cell::RefCell;

use super::{strings::*, ast::*, parser::*, bytecode::*, compiler::*};

mod bindings;
mod internal;
mod simulation;
mod manipulation;
mod jumping;
mod types;
mod variableaccess;
mod control;

pub use self::types::*;

/// Returned by the step() method of an interpreter.
pub type StepResult = Result<(), Option<String>>;
type OpResult = Result<(), String>;
/// Type signature of functions to be registered as bindings.
pub type Binding = FnMut(&mut Interpreter, Vec<Value>) -> Result<Value, String>;
pub type SimpleBinding = FnMut(Vec<Value>) -> Result<Value, String>;

fn minierr(mystr : &'static str) -> String
{
    mystr.to_string()
}
fn plainerr<T>(mystr : &'static str) -> Result<T, String>
{
    Err(minierr(mystr))
}

// global interpreter data
struct GlobalState {
    instance_id: usize,
    object_id: usize,
    instances: HashMap<usize, Instance>,
    instances_by_type: HashMap<usize, BTreeSet<usize>>,
    objectnames: HashMap<String, usize>,
    objects: HashMap<usize, ObjSpec>,
    parser: Option<Parser>,
    variables: HashMap<String, Value>, // accessed as global.varname
    functions: HashMap<String, Value>, // accessed as funcname
}

impl GlobalState {
    fn new(parser : Option<Parser>) -> GlobalState
    {
        GlobalState {
            instance_id : 1,
            object_id : 1,
            instances : HashMap::new(),
            instances_by_type : HashMap::new(),
            objectnames : HashMap::new(),
            objects : HashMap::new(),
            parser,
            variables : HashMap::new(),
            functions : HashMap::new(),
        }
    }
}

// interpreter state
/// Interprets compiled bytecode.
pub struct Interpreter {
    top_frame: Frame,
    frames: Vec<Frame>,
    doexit: bool,
    bindings: HashMap<String, Rc<RefCell<Binding>>>,
    simple_bindings: HashMap<String, Rc<RefCell<SimpleBinding>>>,
    global: GlobalState,
    /// Last error returned by step(). Gets cleared (reset to None) when step() runs without returning an error.
    pub last_error: Option<String>
}

impl Interpreter {
    /// Creates a new interpreter 
    pub fn new(code : &Rc<Vec<u8>>, parser : Option<Parser>) -> Interpreter
    {
        Interpreter {
            top_frame : Frame::new_root(Rc::clone(code)),
            frames : vec!(),
            doexit : false,
            bindings : HashMap::new(),
            simple_bindings : HashMap::new(),
            global : GlobalState::new(parser),
            last_error : None,
        }
    }
    /// Loads new code into the interpreter.
    /// 
    /// Unloads the old bytecode and all interpreter state, no matter what state the interpreter was in.
    /// 
    /// Does not unload the parser that was loaded into the interpreter upon creation.
    /// 
    /// Does not unload internal function bindings.
    /// 
    /// Does not reset global state (objects/instances).
    pub fn restart(&mut self, code: &Rc<Vec<u8>>) -> StepResult
    {
        self.top_frame = Frame::new_root(Rc::clone(code));
        self.frames = vec!();
        self.doexit = false;
        self.last_error = None;
        Ok(())
    }
    /// Clears global state (objects/instances).
    /// 
    /// This GRACELESSLY deletes all objects and instances, even if they contained code that has not yet finished running or needs special destruction.
    /// 
    /// Does not unload the parser that was loaded into the interpreter upon creation.
    /// 
    /// Does not unload internal function bindings.
    /// 
    /// Does not reset global state (objects/instances).
    pub fn clear_global_state(&mut self) -> StepResult
    {
        let mut parser : Option<Parser> = None;
        std::mem::swap(&mut parser, &mut self.global.parser);
        self.global = GlobalState::new(parser);
        Ok(())
    }
    fn step_internal(&mut self) -> StepResult
    {
        if self.get_pc() < self.top_frame.startpc || self.get_pc() > self.top_frame.endpc
        {
            return Err(Some(minierr("internal error: simulation stepped while outside of the range of the frame it was in")));
        }
        let op = self.pull_single_from_code()?;
        
        let opfunc = match_or_err!(self.get_opfunc(op), Some(opfunc) => opfunc, Some(format!("internal error: unknown operation 0x{:02X}", op)))?;
        
        opfunc(self).map_err(Some)?;
        self.handle_flow_control()?;
        
        if self.doexit
        {
            Err(None)
        }
        else
        {
            Ok(())
        }
    }
    /// Steps the interpreter by a single operation.
    ///
    /// Handles flow control after stepping, not before.
    ///
    /// If execution can continue, Ok(()) is returned.
    ///
    /// If execution cannot continue, Err(Option<String>) is returned. This includes graceful exits (end of code).
    ///
    /// If there is an error string (i.e. Err(Some(string))), exit was non-graceful (i.e. there was an error). Otherwise (i.e. Err(None)), it was graceful.
    pub fn step(&mut self) -> StepResult
    {
        self.last_error = None;
        let ret = self.step_internal();
        self.last_error = ret.clone().err().unwrap_or(None).map(|x| format!("{}\nline:{}", x, self.top_frame.currline));
        ret
    }
}
