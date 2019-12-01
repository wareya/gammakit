use std::collections::{HashMap, HashSet, BTreeMap, BTreeSet};
use std::rc::Rc;
use std::cell::RefCell;

use super::{strings::*, ast::*, parser::*, bytecode::*, compiler::*};

mod bindings;
mod internal;
mod simulation;
mod manipulation;
mod jumping;
pub (crate) mod types;
mod variableaccess;

pub use self::types::*;
use variableaccess::ValueLoc;

/// Returned by the step() method of an interpreter.
pub type StepResult = Result<bool, String>;
type OpResult = Result<(), String>;
/// Type signature of functions to be registered as bindings.
pub type Binding = dyn FnMut(&mut Interpreter, Vec<Value>) -> Result<Value, String>;
/// For trivial bindings.
pub type TrivialBinding = fn(&mut Interpreter, Vec<Value>) -> Result<Value, String>;
/// For simple bindings.
pub type SimpleBinding = dyn FnMut(Vec<Value>) -> Result<Value, String>;
/// For trivial simple bindings.
pub type TrivialSimpleBinding = fn(Vec<Value>) -> Result<Value, String>;
/// For arrow bindings.
pub type ArrowBinding = dyn FnMut(ValueLoc, Vec<Value>) -> Result<Value, String>;
/// For trivial arrow bindings.
pub type TrivialArrowBinding = fn(ValueLoc, Vec<Value>) -> Result<Value, String>;

fn minierr(mystr : &'static str) -> String
{
    mystr.to_string()
}
fn plainerr<T>(mystr : &'static str) -> Result<T, String>
{
    Err(minierr(mystr))
}
fn fat_vec<T>() -> Vec<T>
{
    Vec::with_capacity(4)
}

// global interpreter data
pub struct GlobalState {
    string_index: usize,
    string_table : HashMap<String, usize>,
    string_table_reverse : BTreeMap<usize, String>,
    
    instance_id: usize,
    instances: BTreeMap<usize, Instance>,
    pub (crate) instances_by_type: BTreeMap<usize, BTreeSet<usize>>,
    
    pub (crate) objects: BTreeMap<usize, ObjSpec>,
    pub (crate) variables: BTreeMap<usize, Value>, // accessed as global.varname
    pub (crate) barevariables: BTreeMap<usize, Value>, // accessed as varname
    pub (crate) functions: BTreeMap<usize, Value>, // accessed as funcname
    
    pub (crate) bindings: BTreeMap<usize, Rc<RefCell<Binding>>>,
    pub (crate) trivial_bindings: BTreeMap<usize, TrivialBinding>,
    pub (crate) simple_bindings: BTreeMap<usize, Rc<RefCell<SimpleBinding>>>,
    pub (crate) trivial_simple_bindings: BTreeMap<usize, TrivialSimpleBinding>,
    pub (crate) arrow_bindings: BTreeMap<usize, Rc<RefCell<ArrowBinding>>>,
    pub (crate) trivial_arrow_bindings: BTreeMap<usize, TrivialArrowBinding>,
    
    parser: Parser,
}

impl GlobalState {
    fn new(parser : Parser) -> GlobalState
    {
        GlobalState {
            string_index : 1,
            string_table : HashMap::new(),
            string_table_reverse : BTreeMap::new(),
            
            instance_id : 1,
            instances : BTreeMap::new(),
            instances_by_type : BTreeMap::new(),
            
            objects : BTreeMap::new(),
            variables : BTreeMap::new(),
            barevariables : BTreeMap::new(),
            functions : BTreeMap::new(),
            
            bindings : BTreeMap::new(),
            trivial_bindings : BTreeMap::new(),
            simple_bindings : BTreeMap::new(),
            trivial_simple_bindings : BTreeMap::new(),
            arrow_bindings : BTreeMap::new(),
            trivial_arrow_bindings : BTreeMap::new(),
            
            parser,
        }
    }
    #[allow(clippy::ptr_arg)]
    pub (crate) fn get_string_index(&mut self, string : &String) -> usize
    {
        if let Some(index) = self.string_table.get(string)
        {
            *index
        }
        else
        {
            let index = self.string_index;
            self.string_index += 1;
            self.string_table.insert(string.clone(), index);
            self.string_table_reverse.insert(index, string.clone());
            index
        }
    }
    pub (crate) fn get_string(&self, index : usize) -> String
    {
        if let Some(string) = self.string_table_reverse.get(&index)
        {
            return string.clone();
        }
        format!("<index {} with no associated string>", index)
    }
    pub (crate) fn insert_bare_global(&mut self, index : usize)
    {
        self.barevariables.insert(index, Value::default());
    }
    pub (crate) fn insert_global(&mut self, index : usize)
    {
        self.variables.insert(index, Value::default());
    }
    pub (crate) fn insert_globalfunc(&mut self, index : usize, func : FuncSpec)
    {
        self.functions.insert(index, Value::new_funcval(None, func));
    }
}

type OpFunc = fn(&mut Interpreter) -> OpResult;

const TRACK_OP_PERFORMANCE : bool = false;

// interpreter state
/// Interprets compiled bytecode.
pub struct Interpreter {
    top_frame: Frame,
    frames: Vec<Frame>,
    global: GlobalState,
    /// Last error returned by step(). Gets cleared (reset to None) when step() runs without returning an error.
    pub last_error: Option<String>,
    pub (crate) op_map_hits: BTreeMap<u8, u128>,
    pub (crate) op_map: BTreeMap<u8, u128>,
    doexit: bool,
}


impl Interpreter {
    /// Creates a new interpreter 
    pub fn new(parser : Parser) -> Interpreter
    {
        simulation::build_opfunc_table();
        Interpreter {
            top_frame : Frame::new_root(&Code::new()),
            frames : fat_vec(),
            doexit : false,
            global : GlobalState::new(parser),
            last_error : None,
            op_map_hits : BTreeMap::new(),
            op_map : BTreeMap::new(),
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
        self.frames = fat_vec();
        self.doexit = false;
        self.last_error = None;
    }
    
    pub fn restart_in_place(&mut self)
    {
        self.restart(&self.top_frame.code.clone());
    }
    
    pub fn restart_into_string(&mut self, text: &str) -> Result<Code, String>
    {
        let program_lines : Vec<String> = text.lines().map(|x| x.to_string()).collect();
        
        let tokens = self.global.parser.tokenize(&program_lines, false)?;
        
        let ast = self.global.parser.parse_program(&tokens, &program_lines, false)?.ok_or_else(|| "failed to parse program".to_string())?;
        
        let code = compile_bytecode(&ast, &mut self.global)?;
        self.restart(&code);
        Ok(code)
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
        let mut parser = Parser::default();
        std::mem::swap(&mut parser, &mut self.global.parser);
        self.global = GlobalState::new(parser);
    }
    #[inline]
    fn step_internal(&mut self) -> OpResult
    {
        if !TRACK_OP_PERFORMANCE
        {
            self.run_next_op()?;
        }
        else
        {
            use std::time::Instant;
            let start_time = Instant::now();
            let op = self.run_next_op()?;
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
            else if ret.is_ok()
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
    pub fn dump_code(&self) -> Vec<u8>
    {
        let mut out = Vec::new();
        for word in self.top_frame.code.get(..).unwrap()
        {
            out.extend(&word.to_ne_bytes());
        }
        out
    }
    pub fn print_op_perf_log(&self)
    {
        //let mut op_map = interpreter.op_map.iter().map(|(k, v)| (*k, *v as f64 / 1_000_000.0 / (*interpreter.op_map_hits.get(k).unwrap() as f64).sqrt())).collect::<Vec<_>>();
        //let mut op_map = interpreter.op_map.iter().map(|(k, v)| (*k, *v as f64 / *interpreter.op_map_hits.get(k).unwrap() as f64)).collect::<Vec<_>>();
        let mut op_map = self.op_map.iter().map(|(k, v)| (*k, *v as f64 / 1_000_000_000.0)).collect::<Vec<_>>();
        op_map.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        for (op, time) in op_map
        {
            println!("{:02X}:{}", op, time);
        }
    }
}
