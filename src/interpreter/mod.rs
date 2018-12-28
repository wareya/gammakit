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

// global interpreter data
pub struct GlobalState {
    instance_id: usize,// init 100000000
    object_id: usize,  // init 300000000
    instances: HashMap<usize, Instance>,
    instances_by_type: HashMap<usize, Vec<usize>>,
    objectnames: HashMap<String, usize>,
    objects: HashMap<usize, ObjSpec>,
    regex_holder: RegexHolder,
    parser: Parser
}

impl GlobalState {
    pub (crate) fn new(parser : Parser) -> GlobalState
    {
        GlobalState { instance_id : 1_0000_0000, object_id : 3_0000_0000, instances : HashMap::new(), instances_by_type : HashMap::new(), objectnames : HashMap::new(), objects : HashMap::new() , regex_holder : RegexHolder::new(), parser }
    }
}

pub type InternalFunction = Fn(&mut Interpreter, &mut GlobalState, Vec<Value>, bool) -> (Value, bool);

// interpreter state
pub struct Interpreter {
    top_frame: Frame,
    frames: Vec<Frame>,
    doexit: bool,
    suppress_for_expr_end: bool,
    internal_functions: HashMap<String, Rc<InternalFunction>>,
    internal_functions_noreturn: HashSet<String>
}

impl Interpreter {
    pub fn new(code : Vec<u8>) -> Interpreter
    {
        Interpreter {
            top_frame : Frame::new_root(Rc::new(code)),
            frames : vec!(),
            doexit : false,
            suppress_for_expr_end : false,
            internal_functions : HashMap::new(),
            internal_functions_noreturn : HashSet::new()
        }
    }
    pub fn step(&mut self, global : &mut GlobalState) -> bool // TODO: return whether there was an error or not
    {
        let code = self.get_code();
        
        if self.top_frame.pc >= code.len()
        {
            println!("internal error: ran past end of code");
            return false;
        }
        let op = self.pull_single_from_code();
        
        if let Some(opfunc) = self.get_opfunc(op)
        {
            opfunc(self, global);
            self.handle_flow_control();
            return !self.doexit;
        }
        else
        {
            println!("internal error: unknown operation 0x{:02X}", op);
            println!("line: {}", self.top_frame.currline);
            return false;
        }
    }
}
