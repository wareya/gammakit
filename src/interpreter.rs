use std::collections::VecDeque;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;
use std::rc::Weak;

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
type InternalFunction = Fn(&mut Interpreter, Vec<Value>, bool) -> Result<(Value, bool), Option<String>>;

fn plainerr<T>(mystr : &'static str) -> Result<T, Option<String>>
{
    Err(Some(mystr.to_string()))
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
    suppress_for_expr_end: bool,
    internal_functions: HashMap<String, Rc<InternalFunction>>,
    internal_functions_noreturn: HashSet<String>,
    global: GlobalState,
}

impl Interpreter {
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
        let op = self.pull_single_from_code()?;
        
        if let Some(opfunc) = self.get_opfunc(op)
        {
            opfunc(self)?;
            self.handle_flow_control();
            if self.doexit
            {
                Err(None)
            }
            else
            {
                Ok(())
            }
        }
        else
        {
            Err(Some(format!("internal error: unknown operation 0x{:02X}\nline: {}", op, self.top_frame.currline)))
        }
    }
}
