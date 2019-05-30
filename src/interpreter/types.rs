#![allow(clippy::cast_lossless)]
#![allow(clippy::float_cmp)]
#![allow(clippy::type_complexity)]

use crate::interpreter::*;

pub (crate) mod ops;

pub (crate) use self::ops::*;

// note: for loops are controlled the same way as while loops

#[derive(Debug, Clone)]
pub (crate) struct WhileData {
    pub (super) scopes: u16,
    pub (super) expr_start: usize, // continue destination
    pub (super) loop_start: usize,
    pub (super) loop_end: usize, // continue from here
}

#[derive(Debug, Clone)]
pub (crate) struct WithData {
    pub (super) scopes: u16,
    pub (super) loop_start: usize,
    pub (super) loop_end: usize,
    pub (super) instances: VecDeque<Value>,
}

#[derive(Debug, Clone)]
pub (crate) enum ForEachValues {
    List(VecDeque<Value>),
    Gen(GeneratorState),
}

#[derive(Debug, Clone)]
pub (crate) struct ForEachData {
    pub (super) scopes: u16,
    pub (super) loop_start: usize,
    pub (super) loop_end: usize,
    pub (super) name: String,
    pub (super) values: ForEachValues,
}

#[derive(Debug, Clone)]
pub (crate) struct SwitchData {
    pub (super) scopes: u16,
    pub (super) blocks: Vec<usize>,
    pub (super) exit: usize,
    pub (super) value: Value,
}

#[derive(Debug, Clone)]
pub (crate) enum Controller {
    While(WhileData),
    With(WithData),
    ForEach(ForEachData),
    Switch(SwitchData),
}

#[derive(Debug, Clone)]
pub (crate) struct Frame {
    pub (super) code: Rc<Vec<u8>>,
    pub (super) startpc: usize,
    pub (super) pc: usize,
    pub (super) endpc: usize,
    pub (super) scopes: Vec<HashMap<String, Value>>,
    pub (super) scopestarts: Vec<usize>,
    pub (super) instancestack: Vec<usize>,
    pub (super) controlstack: Vec<Controller>,
    pub (super) stack: Vec<StackValue>,
    pub (super) isexpr: bool,
    pub (super) currline: usize,
    pub (super) impassable: bool,
    pub (super) generator: bool,
}

// inaccessible types

#[derive(PartialEq, Eq, Debug, Clone)]
pub (crate) struct FuncSpec {
    pub (super) varnames: Vec<String>,
    pub (super) code: Rc<Vec<u8>>,
    pub (super) startaddr: usize,
    pub (super) endaddr: usize,
    pub (super) fromobj: bool, // function is associated with an object type and must be placed in the context of an instance to be used
    pub (super) parentobj: usize,
    pub (super) forcecontext: usize, // the instance to use as context when executing an object function
    pub (super) impassable: bool, // blocks visibility of scopes outside the called function
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
#[derive(Debug, Clone)]
pub (super) enum NonArrayVariable {
    Indirect(IndirectVar), // x.y.z evaluates x.y before storing it as the instance identity under which to find y
    Direct(DirectVar),
    ActualArray(Vec<Value>),
    ActualDict(HashMap<HashableValue, Value>),
    ActualText(String)
}

#[derive(Debug, Clone)]
pub (crate) struct ArrayVar { // for x[y]
    pub (super) location: NonArrayVariable,
    pub (super) indexes: Vec<Value>
}

#[derive(Debug, Clone)]
pub (crate) enum IndirectSource {
    Ident(usize), // id of an instance
    Global,
}
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub (crate) struct DirectVar { // for x
    pub (super) name: String
}
#[derive(Debug, Clone)]
pub (crate) enum Variable {
    Array(ArrayVar),
    Indirect(IndirectVar),
    Direct(DirectVar)
}

// value types
#[derive(Debug, Clone)]
// TODO split into InternalFuncVal and FuncVal
/// Intentionally opaque. Wrapped by Value.
pub struct FuncVal {
    pub (super) internal: bool,
    pub (super) name: Option<String>,
    pub (super) predefined: Option<HashMap<String, Value>>,
    pub (super) userdefdata: Option<FuncSpec>
}

#[derive(Debug, Clone)]
/// Intentionally opaque. Wrapped by Value.
pub struct GeneratorState {
    pub (super) frame: Option<Frame>, // stores code, pc, and stacks; becomes None after the generator returns/finalizes or exits through its bottom
}
/// Used internally for expressions like "global",
#[derive(Debug, Clone)]
pub enum Special {
    Global
}
/// For custom bindings dealing with manually-managed data that belongs to the application.
#[derive(Debug, Clone)]
pub struct Custom {
    /// Typically for determining what kind of data is stored in this value.
    pub discrim: u64,
    /// Typically for determining what identity of the given kind is stored in this value.
    pub storage: u64,
}

#[derive(Debug, Clone)]
/// Intentionally opaque. Wrapped by Value.
pub struct SubFuncVal {
    pub (super) source: StackValue,
    pub (super) name: String
}

/// Stores typed values (e.g. variables after evaluation, raw literal values).
#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    Text(String),
    Array(Vec<Value>),
    Dict(HashMap<HashableValue, Value>),
    Set(HashSet<HashableValue>),
    Func(Box<FuncVal>),
    Generator(GeneratorState),
    Instance(usize),
    Object(usize),
    Custom(Custom),
    // cannot be assigned
    Special(Special),
    SubFunc(Box<SubFuncVal>),
}

type VarRef = Rc<RefCell<Value>>;

#[derive(Debug, Clone)]
pub (super) enum StackValue {
    Val(Value),
    Var(Variable),
}

#[derive(Debug, Clone)]
/// Enum of only the kinds of Value that can be used as dict keys or set entries.
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
    pub (super) fn new_from_call(code : Rc<Vec<u8>>, startpc : usize, endpc : usize, isexpr : bool, outer : Option<&Frame>, generator : bool) -> Frame
    {
        let instancestack = if let Some(outer) = outer { outer.instancestack.clone() } else { Vec::new() };
        Frame { code, startpc, pc : startpc, endpc, scopes : vec!(HashMap::<String, Value>::new()), scopestarts : Vec::new(), instancestack, controlstack : Vec::new(), stack : Vec::new(), isexpr, currline : 0, impassable : outer.is_none(), generator }
    }
    pub (super) fn len(&mut self) -> usize
    {
        self.stack.len()
    }
    pub (super) fn pop_val(&mut self) -> Option<Value>
    {
        match_or_none!(self.stack.pop(), Some(StackValue::Val(r)) => r)
    }
    pub (super) fn pop_var(&mut self) -> Option<Variable>
    {
        match_or_none!(self.stack.pop(), Some(StackValue::Var(r)) => r)
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

pub (crate) fn hashval_to_val(hashval : HashableValue) -> Value
{
    match hashval
    {
        HashableValue::Number(val) => Value::Number(val),
        HashableValue::Text(val) => Value::Text(val),
    }
}
pub (crate) fn val_to_hashval(val : Value) -> Result<HashableValue, String>
{
    match val
    {
        Value::Number(number) => Ok(HashableValue::Number(number)),
        Value::Text(text) => Ok(HashableValue::Text(text)),
        _ => plainerr("error: tried to use non-hashable value as a dictionary key")
    }
}

impl std::hash::Hash for HashableValue {
    fn hash<H: std::hash::Hasher>(&self, state : &mut H)
    {
        match self
        {
            HashableValue::Number(num) => {0.hash(state); pun_f64_as_u64(*num).hash(state);}
            HashableValue::Text(text)  => {1.hash(state); text.hash(state);}
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
