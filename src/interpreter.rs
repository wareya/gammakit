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
pub fn default_step_result() -> StepResult
{
    Ok(false)
}
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
    instance_id: usize,
    pub (crate) instances: BTreeMap<usize, Instance>,
    pub (crate) instances_by_type: Box<BTreeMap<usize, BTreeSet<usize>>>,
    
    pub (crate) objects: Box<BTreeMap<usize, ObjSpec>>,
    pub (crate) variables: BTreeMap<usize, Value>, // accessed as global.varname
    pub (crate) barevariables: BTreeMap<usize, Value>, // accessed as varname
    pub (crate) functions: BTreeMap<usize, Value>, // accessed as funcname
    
    // TODO: same map
    pub (crate) bindings: Box<BTreeMap<usize, Rc<RefCell<Binding>>>>,
    pub (crate) trivial_bindings: Box<BTreeMap<usize, TrivialBinding>>,
    pub (crate) simple_bindings: Box<BTreeMap<usize, Rc<RefCell<SimpleBinding>>>>,
    pub (crate) trivial_simple_bindings: Box<BTreeMap<usize, TrivialSimpleBinding>>,
    
    // TODO: same map
    pub (crate) arrow_bindings: Box<BTreeMap<usize, Rc<RefCell<ArrowBinding>>>>,
    pub (crate) trivial_arrow_bindings: Box<BTreeMap<usize, TrivialArrowBinding>>,
    
    string_index: usize,
    string_table : Box<HashMap<String, usize>>,
    string_table_reverse : Box<BTreeMap<usize, String>>,
    
    parser: Box<Parser>,
}

impl GlobalState {
    fn new(parser : Parser) -> GlobalState
    {
        GlobalState {
            instance_id : 1,
            instances : BTreeMap::new(),
            instances_by_type : Box::new(BTreeMap::new()),
            
            objects : Box::new(BTreeMap::new()),
            variables : BTreeMap::new(),
            barevariables : BTreeMap::new(),
            functions : BTreeMap::new(),
            
            bindings : Box::new(BTreeMap::new()),
            trivial_bindings : Box::new(BTreeMap::new()),
            simple_bindings : Box::new(BTreeMap::new()),
            trivial_simple_bindings : Box::new(BTreeMap::new()),
            arrow_bindings : Box::new(BTreeMap::new()),
            trivial_arrow_bindings : Box::new(BTreeMap::new()),
            
            parser : Box::new(parser),
            
            string_index : 1,
            string_table : Box::new(HashMap::new()),
            string_table_reverse : Box::new(BTreeMap::new()),
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

type OpFunc = fn(&mut Interpreter) -> StepResult;


// interpreter state
/// Interprets compiled bytecode.
pub struct Interpreter {
    top_frame: Frame,
    frames: Vec<Frame>,
    global: GlobalState,
    /// Last error returned by step(). Gets cleared (reset to None) when step() runs without returning an error.
    pub last_error: Option<String>,
}

#[cfg(feature = "track_op_performance")]
static mut OP_MAP_HITS : [u128; 256] = [0; 256];
#[cfg(feature = "track_op_performance")]
static mut OP_MAP : [u128; 256] = [0; 256];

impl Interpreter {
    /// Creates a new interpreter 
    pub fn new(parser : Parser) -> Interpreter
    {
        println!("- sizeof Value {}", std::mem::size_of::<Value>());
        println!("- sizeof Variable {}", std::mem::size_of::<Variable>());
        println!("- sizeof FuncSpec {}", std::mem::size_of::<FuncSpec>());
        println!("- sizeof Frame {}", std::mem::size_of::<Frame>());
        println!("- sizeof Vec<StackValue> {}", std::mem::size_of::<Vec<StackValue>>());
        println!("- sizeof HashMap<HashableValue, Value> {}", std::mem::size_of::<HashMap<HashableValue, Value>>());
        println!("- sizeof NonArrayVariable {}", std::mem::size_of::<NonArrayVariable>());
        println!("- sizeof Interpreter {}", std::mem::size_of::<Interpreter>());
        println!("- sizeof GlobalState {}", std::mem::size_of::<GlobalState>());
        simulation::build_opfunc_table();
        Interpreter {
            top_frame : Frame::new_root(&Code::new()),
            frames : fat_vec(),
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
    pub fn restart(&mut self, code: &Code)
    {
        self.top_frame = Frame::new_root(code);
        self.frames = fat_vec();
        self.last_error = None;
    }
    pub fn restart_full_of_nops(&mut self, count : usize)
    {
        let mut code = Code::new();
        for _ in 0..count
        {
            code.push_for_nop_thing_only(NOP);
        }
        code.push_for_nop_thing_only(EXIT);
        self.restart(&code);
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
    fn step_internal(&mut self) -> StepResult
    {
        #[cfg(not(feature = "track_op_performance"))]
        {
            unsafe { simulation::OPTABLE[self.pull_single_from_code() as usize](self) }
        }
        #[cfg(feature = "track_op_performance")]
        {
            let op = self.pull_single_from_code();
            
            use std::time::Instant;
            
            let test_time = Instant::now();
            let start_time = Instant::now();
            let ret = unsafe { simulation::OPTABLE[op as usize](self) };
            let end_time = Instant::now();
            
            let reference_time = start_time.duration_since(test_time).as_nanos();
            let real_time = end_time.duration_since(start_time).as_nanos();
            let adjusted_time = real_time - reference_time;
            //let adjusted_time = real_time;
            
            unsafe { OP_MAP_HITS[op as usize] += 1 };
            unsafe { OP_MAP[op as usize] += adjusted_time };
            ret
        }
    }
    /// Steps the interpreter by a single operation.
    ///
    /// Handles flow control after stepping, not before.
    ///
    /// If execution can continue, Ok(false) is returned. Stepping the interpreter past this point will trigger an error.
    ///
    /// If execution has exited normally, Ok(true) is returned.
    ///
    /// If an error occurs, Err(String) is returned. This includes graceful exits (end of code).
    pub fn step(&mut self) -> StepResult
    {
        let ret = self.step_internal();
        match ret
        {
            Ok(r) => Ok(r),
            Err(err) =>
            {
                let pc = self.get_pc();
                if let Some(info) = self.top_frame.code.get_debug_info(pc)
                {
                    self.last_error = Some(format!("{}\nline: {}\ncolumn: {}\npc: 0x{:X}", err, info.last_line, info.last_index, pc));
                }
                else
                {
                    self.last_error = Some(format!("{}\n(unknown or missing context - code probably desynced - location {} - map {:?})", err, pc, self.top_frame.code.debug));
                }
                Err(err.to_string())
            }
        }
    }
    pub fn step_until_error_or_exit(&mut self) -> Result<u64, String>
    {
        let mut steps = 1;
        
        let mut ret = self.step_internal();
        while matches!(ret, Ok(false))
        {
            steps += 1;
            ret = self.step_internal();
        }
        if let Err(err) = ret
        {
            let pc = self.get_pc();
            if let Some(info) = self.top_frame.code.get_debug_info(pc)
            {
                self.last_error = Some(format!("{}\nline: {}\ncolumn: {}\npc: 0x{:X} (off by one instruction)", err, info.last_line, info.last_index, pc));
            }
            else
            {
                self.last_error = Some(format!("{}\n(unknown or missing context - code probably desynced - location {} - map {:?})", err, pc, self.top_frame.code.debug));
            }
            return Err(err);
        }
        else
        {
            return Ok(steps);
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
    
    #[cfg(feature = "track_op_performance")]
    pub fn print_op_perf_log(&self)
    {
        let op_map = unsafe { OP_MAP.iter().enumerate().filter(|(k, _v)| OP_MAP_HITS[*k] != 0).map(|(k, v)| (k, if *v > u128::MAX >> 2 { 0 } else { *v } )) };
        // messy per hit
        //let op_map = op_map.map(|(k, v)| (k, *v as f64 / 1_000_000.0 / (self.op_map_hits[k] as f64).sqrt())).collect::<Vec<_>>();
        // per hit
        let op_map = unsafe { op_map.map(|(k, v)| (k, v as f64 / OP_MAP_HITS[k] as f64)) };
        // raw
        //let op_map = op_map.map(|(k, v)| (k, v as f64 / 1_000_000.0));
        // mod (hacked together)
        // let op_map = op_map.map(|(k, v)| (k, (*v as f64 / *self.op_map_hits.get(k).unwrap() as f64 - 80.0) *  *self.op_map_hits.get(k).unwrap() as f64 / 1_000_000.0)).collect::<Vec<_>>();
        //let op_map = op_map.map(|(k, v)| (k, (*v as f64 / *self.op_map_hits.get(k).unwrap() as f64 - 80.0))).collect::<Vec<_>>();
        let mut op_map = op_map.collect::<Vec<_>>();
        op_map.retain(|x| !x.1.is_nan());
        op_map.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        for (op, time) in op_map
        {
            println!("{:05.05}:\t{}", time, crate::bytecode::op_to_name(op as u8));
        }
    }
}
