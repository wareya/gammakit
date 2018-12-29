#![allow(clippy::cast_lossless)]
#![allow(clippy::float_cmp)]
#![allow(clippy::type_complexity)]

use crate::interpreter::*;

pub (crate) mod ops;

pub (crate) use self::ops::*;

// internal types

#[derive(Debug)]
#[derive(Clone)]
pub (crate) struct ControlData {
    pub (super) controltype: u8,
    pub (super) controlpoints: Vec<usize>,
    pub (super) scopes: u16,
    pub (super) other: Option<VecDeque<usize>> // in with(), a list of instance IDs
}
pub (crate) struct Frame {
    pub (super) code: Rc<Vec<u8>>,
    pub (super) startpc: usize,
    pub (super) pc: usize,
    pub (super) endpc: usize,
    pub (super) scopes: Vec<HashMap<String, Value>>,
    pub (super) scopestarts: Vec<usize>,
    pub (super) instancestack: Vec<usize>,
    pub (super) controlstack: Vec<ControlData>,
    pub (super) stack: Vec<StackValue>,
    pub (super) isexpr: bool,
    pub (super) currline: usize,
    pub (super) impassable: bool,
}

#[derive(Clone)]
#[derive(Debug)]
pub (crate) struct FrameIdentity {
    pub (super) code: Weak<Vec<u8>>,
    pub (super) startpc: usize,
    pub (super) endpc: usize,
    pub (super) scopestarts: Vec<usize>,
}

#[derive(Clone)]
#[derive(Debug)]
pub (crate) struct FuncSpecLocation {
    pub (super) outer_frames : Vec<FrameIdentity>,
    pub (super) top_frame : FrameIdentity,
}

// inaccessible types

#[derive(Debug)]
#[derive(Clone)]
pub (crate) struct FuncSpec {
    pub (super) varnames: Vec<String>,
    pub (super) code: Rc<Vec<u8>>,
    pub (super) startaddr: usize,
    pub (super) endaddr: usize,
    pub (super) fromobj: bool,
    pub (super) parentobj: usize,
    pub (super) forcecontext: usize,
    pub (super) location: FuncSpecLocation,
    pub (super) impassable: bool,
}
pub (crate) struct ObjSpec {
    #[allow(unused)]
    pub (super) ident: usize,
    pub (super) name: String,
    pub (super) functions: HashMap<String, FuncSpec>
}
pub (crate) struct Instance {
    pub (super) objtype: usize,
    pub (super) ident: usize,
    pub (super) variables: HashMap<String, Value>
}

// variable types (i.e. how to access a variable as an lvalue)

// internal to ArrayVar
#[derive(Debug)]
#[derive(Clone)]
pub (super) enum NonArrayVariable {
    Indirect(IndirectVar), // x.y.z evaluates x.y before storing it as the instance identity under which to find y
    Direct(DirectVar),
    ActualArray(VecDeque<Value>) // for situations where the compiler doesn't know that EVALUATE is unnecessary, like func()[0]
}

#[derive(Debug)]
#[derive(Clone)]
pub (crate) struct ArrayVar { // for x[y]
    pub (super) location: NonArrayVariable,
    pub (super) indexes: Vec<Value>
}
#[derive(Debug)]
#[derive(Clone)]
pub (crate) struct IndirectVar { // for x.y
    pub (super) ident: usize, // id of an instance
    pub (super) name: String
}
#[derive(Debug)]
#[derive(Clone)]
pub (crate) struct DirectVar { // for x
    pub (super) name: String
}
#[derive(Debug)]
#[derive(Clone)]
pub (crate) enum Variable {
    Array(ArrayVar),
    Indirect(IndirectVar),
    Direct(DirectVar)
}

// value types
#[derive(Debug)]
#[derive(Clone)]
pub struct FuncVal {
    pub (super) internal: bool,
    pub (super) name: Option<String>,
    pub (super) predefined: Option<HashMap<String, Value>>,
    pub (super) userdefdata: Option<FuncSpec>
}

#[derive(Debug)]
#[derive(Clone)]
pub enum Value {
    Number(f64),
    Text(String),
    Array(VecDeque<Value>),
    Dict(HashMap<HashableValue, Value>),
    Func(Box<FuncVal>),
}
#[derive(Debug)]
#[derive(Clone)]
pub (super) enum StackValue {
    Val(Value),
    Var(Variable),
}

#[derive(Debug)]
#[derive(Clone)]
pub enum HashableValue {
    Number(f64),
    Text(String),
}

// implementations

impl Frame {
    pub (super) fn new_root(code : Rc<Vec<u8>>) -> Frame
    {
        let codelen = code.len();
        Frame { code, startpc : 0, pc : 0, endpc : codelen, scopes : vec!(HashMap::<String, Value>::new()), scopestarts : Vec::new(), instancestack : Vec::new(), controlstack : Vec::new(), stack : Vec::new(), isexpr : false, currline : 0, impassable: true }
    }
    pub (super) fn new_from_call(code : Rc<Vec<u8>>, startpc : usize, endpc : usize, isexpr : bool, impassable : bool) -> Frame
    {
        Frame { code, startpc, pc : startpc, endpc, scopes : vec!(HashMap::<String, Value>::new()), scopestarts : Vec::new(), instancestack : Vec::new(), controlstack : Vec::new(), stack : Vec::new(), isexpr, currline : 0, impassable }
    }
    pub (super) fn len(&mut self) -> usize
    {
        self.stack.len()
    }
    pub (super) fn pop_val(&mut self) -> Option<Value>
    {
        if let Some(StackValue::Val(r)) = self.stack.pop()
        {
            return Some(r);
        }
        None
    }
    pub (super) fn pop_var(&mut self) -> Option<Variable>
    {
        if let Some(StackValue::Var(r)) = self.stack.pop()
        {
            return Some(r);
        }
        None
    }
    pub (super) fn pop(&mut self) -> Option<StackValue>
    {
        self.stack.pop()
    }
    pub (super) fn push_val(&mut self, value : Value)
    {
        self.stack.push(StackValue::Val(value))
    }
    pub (super) fn push_var(&mut self, variable : Variable)
    {
        self.stack.push(StackValue::Var(variable))
    }
    pub (super) fn push(&mut self, stackvalue : StackValue)
    {
        self.stack.push(stackvalue)
    }
}

impl FrameIdentity {
    pub (crate) fn new(frame : &Frame) -> FrameIdentity
    {
        FrameIdentity { code : Rc::downgrade(&frame.code), startpc : frame.startpc, endpc : frame.endpc, scopestarts : frame.scopestarts.clone() }
    }
}

impl Value
{
    pub (crate) fn new_funcval(internal : bool, name : Option<String>, predefined : Option<HashMap<String, Value>>, userdefdata : Option<FuncSpec>) -> Value
    {
        Value::Func(Box::new(FuncVal{internal, name, predefined, userdefdata}))
    }
}

pub (crate) fn hashval_to_val(hashval : &HashableValue) -> Value
{
    match hashval
    {
        HashableValue::Number(val) => Value::Number(*val),
        HashableValue::Text(val) => Value::Text(val.clone()),
    }
}

impl std::hash::Hash for HashableValue {
    fn hash<H: std::hash::Hasher>(&self, state : &mut H)
    {
        match self
        {
            HashableValue::Number(num) =>
            {
                pun_f64_as_u64(*num).hash(state);
            }
            HashableValue::Text(text) =>
            {
                text.hash(state);
            }
        }
    }
}
impl std::cmp::PartialEq for HashableValue {
    fn eq(&self, other : &HashableValue) -> bool
    {
        match (self, other)
        {
            (HashableValue::Number(left), HashableValue::Number(right)) =>
            {
                pun_f64_as_u64(*left) == pun_f64_as_u64(*right)
            }
            (HashableValue::Text(left), HashableValue::Text(right)) =>
            {
                left == right
            }
            _ => { false }
        }
    }
}

impl std::cmp::Eq for HashableValue { }
