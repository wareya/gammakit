#![allow(clippy::cast_lossless)]
#![allow(clippy::float_cmp)]
#![allow(clippy::type_complexity)]

use crate::interpreter::*;

pub (crate) mod ops;

pub (crate) use self::ops::*;

use std::collections::BTreeMap;

// note: for loops are controlled the same way as while loops

#[derive(Debug, Clone)]
pub (crate) struct WhileData {
    pub (super) variables: u64,
    pub (super) expr_start: usize, // continue destination
    pub (super) loop_start: usize,
    pub (super) loop_end: usize, // continue from here
}

#[derive(Debug, Clone)]
pub (crate) struct WithData {
    pub (super) variables: u64,
    pub (super) loop_start: usize,
    pub (super) loop_end: usize,
    pub (super) instances: Vec<Value>,
}

#[derive(Debug, Clone)]
pub (crate) enum ForEachValues {
    List(Vec<Value>),
    Gen(GeneratorState),
}

#[derive(Debug, Clone)]
pub (crate) struct ForEachData {
    pub (super) variables: u64,
    pub (super) loop_start: usize,
    pub (super) loop_end: usize,
    pub (super) values: ForEachValues,
}

#[derive(Debug, Clone)]
pub (crate) struct SwitchData {
    pub (super) variables: u64,
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
    pub (super) code: Code,
    pub (super) stack: Vec<StackValue>,
    pub (super) variables: Vec<Value>,
    pub (super) instancestack: Vec<usize>,
    pub (super) controlstack: Vec<Controller>,
    pub (super) startpc: usize,
    pub (super) pc: usize,
    pub (super) endpc: usize,
    pub (super) currline: usize,
    pub (super) isexpr: bool,
    pub (super) generator: bool,
}

// inaccessible types

#[derive(PartialEq, Eq, Debug, Clone)]
pub (crate) struct FuncSpec {
    pub (crate) code: Code,
    pub (crate) startaddr: usize,
    pub (crate) endaddr: usize,
    pub (crate) argcount: usize,
    pub (crate) parentobj: usize,
    pub (crate) forcecontext: usize, // the instance to use as context when executing an object function
    pub (crate) fromobj: bool, // function is associated with an object type and must be placed in the context of an instance to be used
    pub (crate) generator: bool,
}
#[derive(Debug, Clone)]
pub (crate) struct ObjSpec {
    pub (crate) ident: usize,
    pub (crate) variables: BTreeMap<usize, usize>, // mapping of name to index, zeroth index is always "id" (instance id)
    pub (crate) functions: BTreeMap<usize, FuncSpec>
}
#[derive(Debug, Clone)]
pub (crate) struct Instance {
    pub (super) variables: BTreeMap<usize, Value>,
    pub (super) objtype: usize,
    pub (super) ident: usize,
}

// variable types (i.e. how to access a variable as an lvalue)

// internal to ArrayVar
#[derive(Debug, Clone)]
pub (crate) enum NonArrayVariable {
    Indirect(IndirectVar), // x.y.z evaluates x.y before storing it as the instance identity under which to find y, but then (x.y).z is held as-is
    Global(usize),
    Direct(usize),
    ActualArray(Box<Vec<Value>>),
    ActualDict(Box<HashMap<HashableValue, Value>>),
    ActualText(Box<String>)
}

#[derive(Debug, Clone)]
pub (crate) struct ArrayVar { // for x[y]
    pub (super) indexes: Vec<HashableValue>,
    pub (super) location: NonArrayVariable,
}

impl ArrayVar {
    #[inline]
    pub (crate) fn new(location : NonArrayVariable, indexes : Vec<HashableValue>) -> ArrayVar
    {
        ArrayVar{location, indexes}
    }
}

#[derive(Debug, Clone)]
pub (crate) struct IndirectVar { // for x.y
    pub (super) ident: usize,
    pub (super) name: usize
}

#[derive(Debug, Clone)]
pub (crate) enum Variable {
    Array(ArrayVar),
    Indirect(IndirectVar),
    BareGlobal(usize),
    Global(usize),
    Direct(usize)
}

impl Variable {
    pub (crate) fn from_indirection(ident : usize, name : usize) -> Variable
    {
        Variable::Indirect(IndirectVar{ident, name})
    }
}

// value types
#[derive(Debug, Clone)]
pub struct FuncVal {
    pub (super) predefined: Option<Vec<Value>>,
    pub (super) userdefdata: FuncSpec
}

#[derive(Debug, Clone)]
pub struct InternalFuncVal {
    pub (super) nameindex : usize
}

#[derive(Debug, Clone)]
/// Intentionally opaque. Wrapped by Value.
pub struct GeneratorState {
    pub (super) frame: Option<Frame>, // stores code, pc, and stacks; becomes None after the generator returns/finalizes or exits through its bottom
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
    pub (super) name: usize
}

/// Stores typed values (e.g. variables after evaluation, raw literal values).
#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Number(f64),
    Text(String),
    Array(Box<Vec<Value>>),
    Dict(Box<HashMap<HashableValue, Value>>),
    Set(Box<HashSet<HashableValue>>),
    InternalFunc(InternalFuncVal),
    Func(Box<FuncVal>),
    Generator(Box<GeneratorState>),
    Instance(usize),
    Object(usize),
    Custom(Custom),
    // cannot be assigned
    SubFunc(Box<SubFuncVal>),
}

impl Value {
    pub fn default() -> Value
    {
        Value::Null
    }
}

#[derive(Debug, Clone)]
pub (crate) enum StackValue {
    Val(Value),
    Var(Variable),
}

#[derive(Debug, Clone)]
/// Enum of only the kinds of Value that can be used as dict keys or set entries.
pub enum HashableValue {
    Number(f64),
    Text(String),
    Instance(usize),
}

// implementations

impl Frame {
    pub (super) fn new_root(code : &Code) -> Frame
    {
        let codelen = code.len();
        Frame { code : code.clone(), startpc : 0, pc : 0, endpc : codelen, variables : fat_vec(), instancestack : Vec::new(), controlstack : fat_vec(), stack : fat_vec(), isexpr : false, currline : 0, generator: false }
    }
    pub (super) fn new_from_call(code : &Code, startpc : usize, endpc : usize, isexpr : bool, generator : bool) -> Frame
    {
        Frame { code : code.clone(), startpc, pc : startpc, endpc, variables : fat_vec(), instancestack : Vec::new(), controlstack : fat_vec(), stack : fat_vec(), isexpr, currline : 0, generator }
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
    pub (crate) fn new_funcval(predefined : Option<Vec<Value>>, userdefdata : FuncSpec) -> Value
    {
        Value::Func(Box::new(FuncVal{predefined, userdefdata}))
    }
}

pub (crate) fn hashval_to_val(hashval : HashableValue) -> Value
{
    match hashval
    {
        HashableValue::Number(val)   => Value::Number(val),
        HashableValue::Text(val)     => Value::Text(val),
        HashableValue::Instance(val) => Value::Instance(val),
    }
}
pub (crate) fn val_to_hashval(val : Value) -> Result<HashableValue, String>
{
    match val
    {
        Value::Number(num)  => Ok(HashableValue::Number(num)),
        Value::Text(text)   => Ok(HashableValue::Text(text)),
        Value::Instance(id) => Ok(HashableValue::Instance(id)),
        _ => plainerr("error: tried to use non-hashable value as a dictionary key")
    }
}

impl std::hash::Hash for HashableValue {
    fn hash<H: std::hash::Hasher>(&self, state : &mut H)
    {
        match self
        {
            HashableValue::Number(num)     => {0.hash(state); num.to_bits().hash(state);}
            HashableValue::Text(text)      => {1.hash(state); text.hash(state);}
            HashableValue::Instance(text)  => {2.hash(state); text.hash(state);}
        }
    }
}
impl std::cmp::PartialEq for HashableValue {
    fn eq(&self, other : &HashableValue) -> bool
    {
        match (self, other)
        {
            (HashableValue::Number(left)  , HashableValue::Number(right))   => left.to_bits() == right.to_bits(),
            (HashableValue::Text(left)    , HashableValue::Text(right))     => left == right,
            (HashableValue::Instance(left), HashableValue::Instance(right)) => left == right,
            _ => false
        }
    }
}

impl std::cmp::Eq for HashableValue { }
