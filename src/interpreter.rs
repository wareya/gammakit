use std::collections::{HashMap, HashSet, BTreeMap, BTreeSet};
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

pub use self::types::*;

/// Returned by the step() method of an interpreter.
pub type StepResult = Result<(), Option<String>>;
type OpResult = Result<(), String>;
/// Type signature of functions to be registered as bindings.
pub type Binding = FnMut(&mut Interpreter, Vec<Value>) -> Result<Value, String>;
/// Type signature of functions to be registered as simple bindings.
pub type SimpleBinding = FnMut(Vec<Value>) -> Result<Value, String>;
/// Type signature of functions to be registered as arrow function bindings.
pub type ArrowBinding = FnMut(ValRef, Vec<Value>) -> Result<Value, String>;

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
    objectnames: HashMap<usize, usize>,
    objects: HashMap<usize, ObjSpec>,
    parser: Option<Parser>,
    variables: HashMap<usize, ValRef>, // accessed as global.varname
    functions: HashMap<usize, Value>, // accessed as funcname
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

type OpFunc = fn(&mut Interpreter) -> OpResult;

// interpreter state
/// Interprets compiled bytecode.
pub struct Interpreter {
    top_frame: Frame,
    frames: Vec<Frame>,
    pub (crate) bindings: HashMap<usize, Rc<RefCell<Binding>>>,
    pub (crate) simple_bindings: HashMap<usize, Rc<RefCell<SimpleBinding>>>,
    pub (crate) arrow_bindings: HashMap<usize, Rc<RefCell<ArrowBinding>>>,
    global: GlobalState,
    /// Last error returned by step(). Gets cleared (reset to None) when step() runs without returning an error.
    pub last_error: Option<String>,
    pub (crate) opfunc_map: Box<[OpFunc; 256]>,
    pub (crate) op_map: HashMap<u8, u128>,
    doexit: bool,
    pub (crate) track_op_performance: bool,
}

impl Interpreter {
    /// Creates a new interpreter 
    pub fn new(code : &Code, parser : Option<Parser>) -> Interpreter
    {
        Interpreter {
            top_frame : Frame::new_root(code),
            frames : vec!(),
            doexit : false,
            bindings : HashMap::new(),
            simple_bindings : HashMap::new(),
            arrow_bindings : HashMap::new(),
            global : GlobalState::new(parser),
            last_error : None,
            opfunc_map : Interpreter::build_opfunc_table(),
            op_map : HashMap::new(),
            track_op_performance : true
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
    pub fn restart(&mut self, code: &Code) -> StepResult
    {
        self.top_frame = Frame::new_root(code);
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
        use std::time::Instant;
        
        if self.get_pc() < self.top_frame.startpc || self.get_pc() > self.top_frame.endpc
        {
            return Err(Some(minierr("internal error: simulation stepped while outside of the range of the frame it was in")));
        }
        
        let op = self.pull_single_from_code()?;
        
        if !self.track_op_performance
        {
            self.run_opfunc(op)?;
        }
        else
        {
            let start_time = Instant::now();
            self.run_opfunc(op)?;
            *self.op_map.entry(op).or_insert(0) += Instant::now().duration_since(start_time).as_nanos();
        }
        
        return if self.doexit { Err(None) } else { Ok(()) }
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
        let start_pc = self.get_pc();
        let ret = self.step_internal();
        if let Err(Some(err)) = &ret
        {
            if let Some(info) = self.get_code().get_debug_info(start_pc)
            {
                self.last_error = Some(format!("{}\nline: {}\ncolumn: {}", err, info.last_line, info.last_index))
            }
            else
            {
                self.last_error = Some(format!("{}\n(unknown or missing context - code probably desynced - location {} - map {:?})", err, start_pc, self.get_code().debug))
            }
        }
        ret
    }
}
