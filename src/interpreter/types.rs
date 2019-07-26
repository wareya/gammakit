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
    pub (super) instances: Vec<Value>,
}

#[derive(Debug, Clone)]
pub (crate) enum ForEachValues {
    List(Vec<Value>),
    Gen(GeneratorState),
}

#[derive(Debug, Clone)]
pub (crate) struct ForEachData {
    pub (super) scopes: u16,
    pub (super) loop_start: usize,
    pub (super) loop_end: usize,
    pub (super) name: usize,
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
    pub (super) code: Code,
    pub (super) startpc: usize,
    pub (super) pc: usize,
    pub (super) endpc: usize,
    pub (super) currline: usize,
    pub (super) scopes: Vec<BTreeMap<usize, ValRef>>,
    pub (super) instancestack: Vec<usize>,
    pub (super) controlstack: Vec<Controller>,
    pub (super) stack: Vec<StackValue>,
    pub (super) isexpr: bool,
    pub (super) impassable: bool,
    pub (super) generator: bool,
}

// inaccessible types

#[derive(PartialEq, Eq, Debug, Clone)]
pub (crate) struct FuncSpec {
    pub (super) varnames: Vec<usize>,
    pub (super) code: Code,
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
    #[allow(unused)]
    pub (super) name: usize,
    pub (super) functions: BTreeMap<usize, FuncSpec>
}
pub (crate) struct Instance {
    pub (super) variables: BTreeMap<usize, ValRef>,
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
    Global(usize),
    Direct(usize),
    Selfref,
    Other,
}

impl Variable {
    pub (crate) fn from_indirection(ident : usize, name : usize) -> Variable
    {
        Variable::Indirect(IndirectVar{ident, name})
    }
    pub (crate) fn from_global(name : usize) -> Variable
    {
        Variable::Global(name)
    }
}

// value types
#[derive(Debug, Clone)]
// TODO split into InternalFuncVal and FuncVal
/// Intentionally opaque. Wrapped by Value.
pub struct FuncVal {
    pub (super) internal: bool,
    pub (super) name: Option<usize>,
    pub (super) predefined: Option<BTreeMap<usize, ValRef>>,
    pub (super) userdefdata: Option<FuncSpec>
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

/*
pub (crate) enum Reference {
    Mut(ValRef),
    Immut(StackValue)
}
*/

#[derive(Debug, Clone)]
/// Intentionally opaque. Wrapped by Value.
pub struct SubFuncVal {
    pub (super) source: StackValue,
    pub (super) name: usize
}

/// Stores typed values (e.g. variables after evaluation, raw literal values).
#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    Text(String),
    Array(Box<Vec<Value>>),
    Dict(Box<HashMap<HashableValue, Value>>),
    Set(Box<HashSet<HashableValue>>),
    Func(Box<FuncVal>),
    Generator(Box<GeneratorState>),
    Instance(usize),
    Object(usize),
    Custom(Custom),
    // cannot be assigned
    SubFunc(Box<SubFuncVal>),
}

#[derive(Debug)]
pub struct ValRef {
    reference: Rc<RefCell<Value>>,
    indexes: Option<Vec<HashableValue>>,
    readonly: bool,
}
impl ValRef {
    pub fn from_val(val : Value) -> ValRef
    {
        ValRef{reference : Rc::new(RefCell::new(val)), indexes : None, readonly : false}
    }
    pub fn from_val_readonly(val : Value) -> ValRef
    {
        ValRef{reference : Rc::new(RefCell::new(val)), indexes : None, readonly : true}
    }
    pub fn from_val_indexed(val : Value, indexes : Vec<HashableValue>) -> ValRef
    {
        ValRef{reference : Rc::new(RefCell::new(val)), indexes : Some(indexes), readonly : false}
    }
    pub fn from_val_indexed_readonly(val : Value, indexes : Vec<HashableValue>) -> ValRef
    {
        ValRef{reference : Rc::new(RefCell::new(val)), indexes : Some(indexes), readonly : true}
    }
    pub (crate) fn from_ref(reference : Rc<RefCell<Value>>, indexes : Vec<HashableValue>, readonly : bool) -> ValRef
    {
        ValRef{reference, indexes : Some(indexes), readonly}
    }
    pub fn refclone(&self) -> ValRef
    {
        ValRef{reference : Rc::clone(&self.reference), indexes : self.indexes.clone(), readonly : self.readonly}
    }
    pub fn borrow(&self) -> Result<std::cell::Ref<Value>, String>
    {
        Ok(self.reference.borrow())
    }
    pub fn borrow_mut(&self) -> Result<std::cell::RefMut<Value>, String>
    {
        if self.readonly
        {
            return Err("error: tried to borrow to a read-only value".to_string());
        }
        Ok(self.reference.borrow_mut())
    }
    pub fn extract_ref(&self) -> Result<Rc<RefCell<Value>>, String>
    {
        if self.readonly
        {
            return Err("error: tried to borrow to a read-only value".to_string());
        }
        Ok(Rc::clone(&self.reference))
    }
    pub fn to_val(&self) -> Result<Value, String>
    {
        if let Some(indexes) = &self.indexes
        {
            use super::variableaccess::return_indexed;
            return_indexed(&*self.borrow()?, indexes)
        }
        else
        {
            Ok((*self.reference).clone().into_inner())
        }
    }
    pub fn assign(&self, val : Value) -> Result<(), String>
    {
        if self.readonly
        {
            return Err("error: tried to assign to a read-only value".to_string());
        }
        match val
        {
            Value::SubFunc(_) => Err("error: tried to assign the result of the dismember operator (->) to a variable (you probably forgot the argument list)".to_string()),
            val =>
            {
                if let Some(indexes) = &self.indexes
                {
                    use super::variableaccess::assign_indexed;
                    assign_indexed(val, &mut self.reference.borrow_mut(), indexes)
                }
                else
                {
                    let mut var = self.reference.borrow_mut();
                    *var = val;
                    Ok(())
                }
            }
        }
    }
}

impl Clone for ValRef {
    fn clone(&self) -> ValRef
    {
        ValRef{reference: Rc::new(RefCell::new(self.to_val().unwrap())), indexes: self.indexes.clone(), readonly: self.readonly}
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
}

// implementations

impl Frame {
    pub (super) fn new_root(code : &Code) -> Frame
    {
        let codelen = code.len();
        Frame { code : code.clone(), startpc : 0, pc : 0, endpc : codelen, scopes : vec!(BTreeMap::new()), instancestack : Vec::new(), controlstack : Vec::new(), stack : Vec::new(), isexpr : false, currline : 0, impassable: true, generator: false }
    }
    pub (super) fn new_from_call(code : &Code, startpc : usize, endpc : usize, isexpr : bool, outer : Option<&Frame>, generator : bool) -> Frame
    {
        let instancestack = if let Some(outer) = outer { outer.instancestack.clone() } else { Vec::new() };
        Frame { code : code.clone(), startpc, pc : startpc, endpc, scopes : vec!(BTreeMap::new()), instancestack, controlstack : Vec::new(), stack : Vec::new(), isexpr, currline : 0, impassable : outer.is_none(), generator }
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
    pub (crate) fn new_funcval(internal : bool, name : Option<usize>, predefined : Option<BTreeMap<usize, ValRef>>, userdefdata : Option<FuncSpec>) -> Value
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
            HashableValue::Number(num) => {0.hash(state); num.to_bits().hash(state);}
            HashableValue::Text(text)  => {1.hash(state); text.hash(state);}
        }
    }
}
impl std::cmp::PartialEq for HashableValue {
    fn eq(&self, other : &HashableValue) -> bool
    {
        match (self, other)
        {
            (HashableValue::Number(left), HashableValue::Number(right)) => left.to_bits() == right.to_bits(),
            (HashableValue::Text(left), HashableValue::Text(right)) => left == right,
            _ => false
        }
    }
}

impl std::cmp::Eq for HashableValue { }
