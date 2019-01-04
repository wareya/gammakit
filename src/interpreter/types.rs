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
#[derive(Debug)]
#[derive(Clone)]
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
    pub (super) generator: bool,
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
    pub (super) impassable: bool,
    pub (super) generator: bool,
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
pub (crate) enum IndirectSource {
    Ident(usize), // id of an instance
    Global,
}
#[derive(Debug)]
#[derive(Clone)]
pub (crate) struct IndirectVar { // for x.y
    pub (super) source: IndirectSource,
    pub (super) name: String
}

impl IndirectVar {
    pub (crate) fn from_ident(ident : usize, name : String) -> Variable
    {
        Variable::Indirect(IndirectVar{source: IndirectSource::Ident(ident), name})
    }
    pub (crate) fn from_global(name : String) -> Variable
    {
        Variable::Indirect(IndirectVar{source: IndirectSource::Global, name})
    }
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

// value types
#[derive(Debug)]
#[derive(Clone)]
pub struct GeneratorState {
    pub (super) frame: Option<Frame>, // stores code, pc, and stacks; becomes None after the generator returns/finalizes or exits through its bottom
}

#[derive(Debug)]
#[derive(Clone)]
pub enum Special {
    Global
}
#[derive(Debug)]
#[derive(Clone)]
pub enum Value {
    Number(f64),
    Text(String),
    Array(VecDeque<Value>),
    Dict(HashMap<HashableValue, Value>),
    Func(Box<FuncVal>),
    Generator(GeneratorState),
    Special(Special),
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
        Frame { code, startpc : 0, pc : 0, endpc : codelen, scopes : vec!(HashMap::<String, Value>::new()), scopestarts : Vec::new(), instancestack : Vec::new(), controlstack : Vec::new(), stack : Vec::new(), isexpr : false, currline : 0, impassable: true, generator: false }
    }
    pub (super) fn new_from_call(code : Rc<Vec<u8>>, startpc : usize, endpc : usize, isexpr : bool, impassable : bool, generator : bool) -> Frame
    {
        Frame { code, startpc, pc : startpc, endpc, scopes : vec!(HashMap::<String, Value>::new()), scopestarts : Vec::new(), instancestack : Vec::new(), controlstack : Vec::new(), stack : Vec::new(), isexpr, currline : 0, impassable, generator }
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
            HashableValue::Number(num) => pun_f64_as_u64(*num).hash(state),
            HashableValue::Text(text) => text.hash(state)
        }
    }
}
impl std::cmp::PartialEq for HashableValue {
    fn eq(&self, other : &HashableValue) -> bool
    {
        match (self, other)
        {
            (HashableValue::Number(left), HashableValue::Number(right)) => pun_f64_as_u64(*left) == pun_f64_as_u64(*right),
            (HashableValue::Text(left), HashableValue::Text(right)) => left == right,
            _ => false
        }
    }
}

impl std::cmp::Eq for HashableValue { }
