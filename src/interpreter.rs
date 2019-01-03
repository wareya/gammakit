use std::collections::VecDeque;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

use super::strings::*;
use super::ast::*;
use super::parser::*;
use super::bytecode::*;
use super::compiler::*;
use super::regexholder::RegexHolder;

mod bindings;
mod internal;
mod simulation;
mod manipulation;
mod jumping;
mod types;
mod variableaccess;
mod control;

use self::types::*;

pub type StepResult = Result<(), Option<String>>;
type OpResult = Result<(), String>;
type InternalFunction = Fn(&mut Interpreter, Vec<Value>, bool) -> Result<(Value, bool), String>;

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
    instance_id: usize,// init 100000000
    object_id: usize,  // init 300000000
    instances: HashMap<usize, Instance>,
    instances_by_type: HashMap<usize, Vec<usize>>,
    objectnames: HashMap<String, usize>,
    objects: HashMap<usize, ObjSpec>,
    regex_holder: RegexHolder,
    parser: Option<Parser>
}

impl GlobalState {
    fn new(parser : Option<Parser>) -> GlobalState
    {
        GlobalState {
            instance_id : 1_0000_0000,
            object_id : 3_0000_0000,
            instances : HashMap::new(),
            instances_by_type : HashMap::new(),
            objectnames : HashMap::new(),
            objects : HashMap::new(),
            regex_holder : RegexHolder::new(),
            parser
        }
    }
}

// interpreter state
pub struct Interpreter {
    top_frame: Frame,
    frames: Vec<Frame>,
    doexit: bool,
    // TODO: look into how to avoid this and why I don't need it for while loops
    suppress_for_expr_end: bool,
    internal_functions: HashMap<String, Rc<InternalFunction>>,
    internal_functions_noreturn: HashSet<String>,
    global: GlobalState,
    /// Last error returned by step(). Gets cleared (reset to None) when step() runs without returning an error.
    pub last_error: Option<String>
}

impl Interpreter {
    /// Creates a new interpreter 
    pub fn new(code : Vec<u8>, parser : Option<Parser>) -> Interpreter
    {
        Interpreter {
            top_frame : Frame::new_root(Rc::new(code)),
            frames : vec!(),
            doexit : false,
            suppress_for_expr_end : false,
            internal_functions : HashMap::new(),
            internal_functions_noreturn : HashSet::new(),
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
    pub fn restart(&mut self, code: Vec<u8>) -> StepResult
    {
        self.top_frame = Frame::new_root(Rc::new(code));
        self.frames = vec!();
        self.doexit = false;
        self.suppress_for_expr_end = false;
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
        
        let opfunc = match_or_err!(self.get_opfunc(op), Some(opfunc) => opfunc, Some(format!("internal error: unknown operation 0x{:02X}\nline: {}", op, self.top_frame.currline)))?;
        
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
    /// If execution can control, Ok(()) is returned.
    ///
    /// If execution cannot return, Err(Option<String>) is returned. This includes graceful exits.
    ///
    /// If there is an error string (Err(Some(string))), exit was non-graceful (i.e. there was an error). Otherwise (Err(None)), it was graceful.
    pub fn step(&mut self) -> StepResult
    {
        self.last_error = None;
        let ret = self.step_internal();
        self.last_error = ret.clone().err().unwrap_or(None);
        ret
    }
}
