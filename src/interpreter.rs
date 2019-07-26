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
pub type StepResult = Result<bool, String>;
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
    instances: BTreeMap<usize, Instance>,
    instances_by_type: BTreeMap<usize, BTreeSet<usize>>,
    objectnames: BTreeMap<usize, usize>,
    objects: BTreeMap<usize, ObjSpec>,
    parser: Option<Parser>,
    variables: BTreeMap<usize, ValRef>, // accessed as global.varname
    functions: BTreeMap<usize, Value>, // accessed as funcname
}

impl GlobalState {
    fn new(parser : Option<Parser>) -> GlobalState
    {
        GlobalState {
            instance_id : 1,
            object_id : 1,
            instances : BTreeMap::new(),
            instances_by_type : BTreeMap::new(),
            objectnames : BTreeMap::new(),
            objects : BTreeMap::new(),
            parser,
            variables : BTreeMap::new(),
            functions : BTreeMap::new(),
        }
    }
}

type OpFunc = fn(&mut Interpreter) -> OpResult;

// interpreter state
/// Interprets compiled bytecode.
pub struct Interpreter {
    top_frame: Frame,
    frames: Vec<Frame>,
    pub (crate) bindings: BTreeMap<usize, Rc<RefCell<Binding>>>,
    pub (crate) simple_bindings: BTreeMap<usize, Rc<RefCell<SimpleBinding>>>,
    pub (crate) arrow_bindings: BTreeMap<usize, Rc<RefCell<ArrowBinding>>>,
    global: GlobalState,
    /// Last error returned by step(). Gets cleared (reset to None) when step() runs without returning an error.
    pub last_error: Option<String>,
    pub (crate) opfunc_map: Box<[OpFunc; 256]>,
    pub (crate) op_map_hits: BTreeMap<u8, u128>,
    pub (crate) op_map: BTreeMap<u8, u128>,
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
            bindings : BTreeMap::new(),
            simple_bindings : BTreeMap::new(),
            arrow_bindings : BTreeMap::new(),
            global : GlobalState::new(parser),
            last_error : None,
            opfunc_map : Interpreter::build_opfunc_table(),
            op_map_hits : BTreeMap::new(),
            op_map : BTreeMap::new(),
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
    pub fn restart(&mut self, code: &Code)
    {
        self.top_frame = Frame::new_root(code);
        self.frames = vec!();
        self.doexit = false;
        self.last_error = None;
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
    pub fn clear_global_state(&mut self)
    {
        let mut parser : Option<Parser> = None;
        std::mem::swap(&mut parser, &mut self.global.parser);
        self.global = GlobalState::new(parser);
    }
    #[inline]
    fn step_internal(&mut self) -> OpResult
    {
        use std::time::Instant;
        if self.get_pc() < self.top_frame.startpc || self.get_pc() >= self.top_frame.endpc
        {
            return Err(minierr("internal error: simulation stepped while outside of the range of the frame it was in"));
        }
        
        if !self.track_op_performance
        {
            let op = self.pull_single_from_code().unwrap();
            self.run_opfunc(op)?;
        }
        else
        {
            let start_time = Instant::now();
            let op = self.pull_single_from_code().unwrap();
            self.run_opfunc(op)?;
            *self.op_map_hits.entry(op).or_insert(0) += 1;
            *self.op_map.entry(op).or_insert(0) += Instant::now().duration_since(start_time).as_nanos();
        }
        Ok(())
    }
    /// Steps the interpreter by a single operation.
    ///
    /// Handles flow control after stepping, not before.
    ///
    /// If execution can continue, Ok(true) is returned. Stepping the interpreter past this point will trigger an error.
    ///
    /// If execution has exited normally, Ok(false) is returned.
    ///
    /// If an error occurs, Err(String) is returned. This includes graceful exits (end of code).
    pub fn step(&mut self) -> StepResult
    {
        let start_pc = self.get_pc();
        let ret = self.step_internal();
        if self.doexit
        {
            Ok(false)
        }
        else if let Err(err) = &ret
        {
            if let Some(info) = self.top_frame.code.get_debug_info(start_pc)
            {
                self.last_error = Some(format!("{}\nline: {}\ncolumn: {}\npc: 0x{:X}", err, info.last_line, info.last_index, start_pc))
            }
            else
            {
                self.last_error = Some(format!("{}\n(unknown or missing context - code probably desynced - location {} - map {:?})", err, start_pc, self.top_frame.code.debug))
            }
            Err(err.to_string())
        }
        else
        {
            Ok(true)
        }
    }
    pub fn step_until_error_or_exit(&mut self) -> Result<u64, String>
    {
        let mut steps = 0;
        let mut start_pc = self.get_pc();
        loop
        {
            let ret = self.step_internal();
            steps += 1;
            if self.doexit
            {
                return Ok(steps);
            }
            else if let Ok(_) = ret
            {
                start_pc = self.get_pc();
                continue;
            }
            else if let Err(err) = ret
            {
                if let Some(info) = self.top_frame.code.get_debug_info(start_pc)
                {
                    self.last_error = Some(format!("{}\nline: {}\ncolumn: {}\npc: 0x{:X}", err, info.last_line, info.last_index, start_pc))
                }
                else
                {
                    self.last_error = Some(format!("{}\n(unknown or missing context - code probably desynced - location {} - map {:?})", err, start_pc, self.top_frame.code.debug))
                }
                return Err(err);
            }
        }
    }
}
