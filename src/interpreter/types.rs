use interpreter::*;

// internal types
#[derive(Debug)]
#[derive(Clone)]
pub(super) struct ControlData {
    pub controltype: u8,
    pub controlpoints: Vec<usize>,
    pub scopes: u16,
    pub other: Option<VecDeque<usize>> // in with(), a list of instance IDs
}
pub(super) struct Frame {
    pub code: Rc<Vec<u8>>,
    pub startpc: usize,
    pub pc: usize,
    pub endpc: usize,
    pub scopes: Vec<HashMap<String, Value>>,
    pub scopestarts: Vec<usize>,
    pub instancestack: Vec<usize>,
    pub controlstack: Vec<ControlData>,
    pub stack: Vec<Value>,
    pub isexpr: bool,
    pub currline: usize,
}
impl Frame {
    pub(super) fn new_root(code : Rc<Vec<u8>>) -> Frame
    {
        let codelen = code.len();
        Frame { code, startpc : 0, pc : 0, endpc : codelen, scopes : vec!(HashMap::<String, Value>::new()), scopestarts : Vec::new(), instancestack : Vec::new(), controlstack : Vec::new(), stack : Vec::new(), isexpr : false, currline : 0 }
    }
    pub(super) fn new_from_call(code : Rc<Vec<u8>>, startpc : usize, endpc : usize, isexpr : bool) -> Frame
    {
        Frame { code, startpc, pc : startpc, endpc, scopes : vec!(HashMap::<String, Value>::new()), scopestarts : Vec::new(), instancestack : Vec::new(), controlstack : Vec::new(), stack : Vec::new(), isexpr, currline : 0 }
    }
}

#[derive(Clone)]
#[derive(Debug)]
pub(super) struct FrameIdentity {
    pub code: Weak<Vec<u8>>,
    pub startpc: usize,
    pub endpc: usize,
    pub scopestarts: Vec<usize>,
}

impl FrameIdentity {
    pub(super) fn new(frame : &Frame) -> FrameIdentity
    {
        FrameIdentity { code : Rc::downgrade(&frame.code), startpc : frame.startpc, endpc : frame.endpc, scopestarts : frame.scopestarts.clone() }
    }
}

#[derive(Clone)]
#[derive(Debug)]
pub(super) struct FuncSpecLocation {
    pub outer_frames : Vec<FrameIdentity>,
    pub top_frame : FrameIdentity,
}


// inaccessible types
#[derive(Debug)]
#[derive(Clone)]
pub(super) struct FuncSpec {
    pub varnames: Vec<String>,
    pub code: Rc<Vec<u8>>,
    pub startaddr: usize,
    pub endaddr: usize,
    pub fromobj: bool,
    pub parentobj: usize,
    pub forcecontext: usize,
    pub location: FuncSpecLocation,
}
pub(super) struct ObjSpec {
    #[allow(unused)]
    pub ident: usize,
    pub name: String,
    pub functions: HashMap<String, FuncSpec>
}
pub(super) struct Instance {
    pub objtype: usize,
    pub ident: usize,
    pub variables: HashMap<String, Value>
}

// variable types (i.e. how to access a variable as an lvalue)
#[derive(Debug)]
#[derive(Clone)]
pub(super) struct ArrayVar { // for x[y]
    pub location: NonArrayVariable,
    pub indexes: VecDeque<Value>
}
#[derive(Debug)]
#[derive(Clone)]
pub(super) struct IndirectVar { // for x.y
    pub ident: usize, // id of an instance
    pub name: String
}
#[derive(Debug)]
#[derive(Clone)]
pub(super) struct DirectVar { // for x
    pub name: String
}
#[derive(Debug)]
#[derive(Clone)]
pub(super) enum Variable {
    Array(ArrayVar),
    Indirect(IndirectVar),
    Direct(DirectVar)
}
#[derive(Debug)]
#[derive(Clone)]
pub(super) enum NonArrayVariable {
    Indirect(IndirectVar), // x.y.z evaluates x.y before storing it as the instance identity under which to find y
    Direct(DirectVar),
    ActualArray(VecDeque<Value>) // for situations where the compiler doesn't know that EVALUATE is unnecessary, like func()[0]
}

// value types
#[derive(Debug)]
#[derive(Clone)]
pub(super) struct FuncVal {
    pub internal: bool,
    pub internalname: Option<String>,
    pub predefined: Option<HashMap<String, Value>>,
    pub userdefdata: Option<FuncSpec>
}
#[derive(Debug)]
#[derive(Clone)]
pub(super) enum Value {
    Number(f64),
    Text(String),
    Array(VecDeque<Value>),
    Dict(HashMap<HashableValue, Value>),
    Func(Box<FuncVal>),
    Var(Variable),
}

impl Value
{
    pub(super) fn new_funcval(internal : bool, internalname : Option<String>, predefined : Option<HashMap<String, Value>>, userdefdata : Option<FuncSpec>) -> Value
    {
        Value::Func(Box::new(FuncVal{internal, internalname, predefined, userdefdata}))
    }
}

#[derive(Debug)]
#[derive(Clone)]
pub(super) enum HashableValue {
    Number(f64),
    Text(String),
}

pub(super) fn hashval_to_val(hashval : &HashableValue) -> Value
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

pub(super) fn format_val(val : &Value) -> Option<String>
{
    match val
    {
        Value::Number(float) =>
        {
            Some(format!("{:.10}", float).trim_right_matches('0').trim_right_matches('.').to_string())
        }
        Value::Text(string) =>
        {
            Some(string.clone())
        }
        Value::Array(array) =>
        {
            let mut ret = String::new();
            ret.push_str("[");
            for (i, val) in array.iter().enumerate()
            {
                if let Value::Text(text) = val
                {
                    ret.push_str(&format!("\"{}\"", escape(text)));
                }
                else if let Some(part) = format_val(val)
                {
                    ret.push_str(&part);
                }
                else
                {
                    return None
                }
                if i+1 != array.len()
                {
                    ret.push_str(", ");
                }
            }
            ret.push_str("]");
            
            Some(ret)
        }
        Value::Dict(dict) =>
        {
            let mut ret = String::new();
            ret.push_str("{");
            for (i, (key, val)) in dict.iter().enumerate()
            {
                if let Some(part) = format_val(&hashval_to_val(key))
                {
                    ret.push_str(&part);
                    ret.push_str(": ");
                }
                else
                {
                    return None
                }
                
                if let Value::Text(text) = val
                {
                    ret.push_str(&format!("\"{}\"", escape(text)));
                }
                else if let Some(part) = format_val(val)
                {
                    ret.push_str(&part);
                }
                else
                {
                    return None
                }
                if i+1 != dict.len()
                {
                    ret.push_str(", ");
                }
            }
            ret.push_str("}");
            
            Some(ret)
        }
        _ =>
        {
            None
        }
    }
}

pub(super) fn value_op_add(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(left+right))
        }
        // TODO: string and array concatenation
        _ =>
        {
            Err("types incompatible with addition".to_string())
        }
    }
}
pub(super) fn value_op_subtract(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(left-right))
        }
        _ =>
        {
            Err("types incompatible with subtraction".to_string())
        }
    }
}
pub(super) fn value_op_multiply(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(left*right))
        }
        _ =>
        {
            Err("types incompatible with multiplication".to_string())
        }
    }
}
pub(super) fn value_op_divide(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(left/right))
        }
        _ =>
        {
            Err("types incompatible with division".to_string())
        }
    }
}
pub(super) fn value_op_modulo(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(mut left), Value::Number(mut right)) =>
        {
            let negative_divisor = right < 0.0;
            if negative_divisor
            {
                right = -right;
                left = -left;
            }
            
            let outval = ((left%right)+right)%right;
            Ok(Value::Number(outval))
        }
        _ =>
        {
            Err("types incompatible with modulo".to_string())
        }
    }
}
pub(super) fn float_booly(f : f64) -> bool
{
    f >= 0.5 // FIXME do we want to replicate this or can we get away with using f.round() != 0.0 instead?
}
pub(super) fn bool_floaty(b : bool) -> f64
{
    if b {1.0} else {0.0}
}
pub(super) fn value_op_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            #[cfg_attr(feature = "cargo-clippy", allow(float_cmp))]
            Ok(Value::Number(bool_floaty(left==right)))
        }
        (Value::Text(left), Value::Text(right)) =>
        {
            Ok(Value::Number(bool_floaty(left==right)))
        }
        _ =>
        {
            Err("types incompatible with equal".to_string())
        }
    }
}
pub(super) fn value_op_not_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            #[cfg_attr(feature = "cargo-clippy", allow(float_cmp))]
            Ok(Value::Number(bool_floaty(left!=right)))
        }
        (Value::Text(left), Value::Text(right)) =>
        {
            Ok(Value::Number(bool_floaty(left!=right)))
        }
        // TODO string comparison
        _ =>
        {
            Err("types incompatible with equal".to_string())
        }
    }
}
pub(super) fn value_op_greater_or_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(left>=right)))
        }
        // TODO string comparison
        _ =>
        {
            Err("types incompatible with greater than or equal".to_string())
        }
    }
}
pub(super) fn value_op_less_or_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(left<=right)))
        }
        // TODO string comparison
        _ =>
        {
            Err("types incompatible with less than or equal".to_string())
        }
    }
}
pub(super) fn value_op_greater(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(left>right)))
        }
        // TODO string comparison
        _ =>
        {
            Err("types incompatible with greater than".to_string())
        }
    }
}
pub(super) fn value_op_less(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(left<right)))
        }
        // TODO string comparison
        _ =>
        {
            Err("types incompatible with less than".to_string())
        }
    }
}
pub(super) fn value_op_and(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(float_booly(*left)&&float_booly(*right))))
        }
        // TODO dicts
        _ =>
        {
            Err("types incompatible with logical and".to_string())
        }
    }
}
pub(super) fn value_op_or(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(float_booly(*left)||float_booly(*right))))
        }
        // TODO dicts
        _ =>
        {
            Err("types incompatible with logical or".to_string())
        }
    }
}


/*
BINOP_TYPES = \
{ 0x10: "&&" , # this is not an endorsement of using && instead of and in code
  0x11: "||" , # likewise for || and or
  0x20: "==" ,
  0x21: "!=" ,
  0x22: ">=" ,
  0x23: "<=" ,
  0x24: ">"  ,
  0x25: "<"  ,
  0x30: "+"  ,
  0x31: "-"  ,
  0x40: "*"  ,
  0x41: "/"  ,
  0x42: "%"  ,
}
*/

pub(super) fn get_binop_function(op : u8) -> Option<Box<Fn(&Value, &Value) -> Result<Value, String>>>
{
    macro_rules! enbox {
        ( $x:ident ) =>
        {
            Some(Box::new($x))
        }
    }
    match op
    {
        0x10 => enbox!(value_op_and),
        0x11 => enbox!(value_op_or),
        0x20 => enbox!(value_op_equal),
        0x21 => enbox!(value_op_not_equal),
        0x22 => enbox!(value_op_greater_or_equal),
        0x23 => enbox!(value_op_less_or_equal),
        0x24 => enbox!(value_op_greater),
        0x25 => enbox!(value_op_less),
        0x30 => enbox!(value_op_add),
        0x31 => enbox!(value_op_subtract),
        0x40 => enbox!(value_op_multiply),
        0x41 => enbox!(value_op_divide),
        0x42 => enbox!(value_op_modulo),
        _ => None
    }
}

pub(super) fn value_op_negative(value : &Value) -> Result<Value, String>
{
    match value
    {
        Value::Number(value) =>
        {
            Ok(Value::Number(-value))
        }
        _ =>
        {
            Err("type incompatible with negation".to_string())
        }
    }
}
pub(super) fn value_op_positive(value : &Value) -> Result<Value, String>
{
    match value
    {
        Value::Number(value) =>
        {
            Ok(Value::Number(*value))
        }
        _ =>
        {
            Err("type incompatible with positive".to_string())
        }
    }
}
pub(super) fn value_op_not(value : &Value) -> Result<Value, String>
{
    match value
    {
        Value::Number(value) =>
        {
            Ok(Value::Number(bool_floaty(!float_booly(*value))))
        }
        _ =>
        {
            Err("type incompatible with positive".to_string())
        }
    }
}

pub(super) fn get_unop_function(op : u8) -> Option<Box<Fn(&Value) -> Result<Value, String>>>
{
    macro_rules! enbox {
        ( $x:ident ) =>
        {
            Some(Box::new($x))
        }
    }
    match op
    {
        0x10 => enbox!(value_op_negative),
        0x11 => enbox!(value_op_positive),
        0x20 => enbox!(value_op_not),
        // TODO: add "not" and "bitwise not"
        _ => None
    }
}

pub(super) fn value_truthy(imm : &Value) -> bool
{
    match imm
    {
        Value::Number(value) =>
        {
            float_booly(*value)
        }
        // TODO: string and array concatenation
        _ =>
        {
            true
        }
    }
}

pub(super) fn ast_to_dict(ast : &ASTNode) -> Value
{
    let mut astdict = HashMap::<HashableValue, Value>::new();
    
    macro_rules! to_key {
        ( $str:expr ) =>
        {
            HashableValue::Text($str.to_string())
        }
    }
    
    astdict.insert(to_key!("text"), Value::Text(ast.text.clone()));
    astdict.insert(to_key!("line"), Value::Number(ast.line as f64));
    astdict.insert(to_key!("position"), Value::Number(ast.line as f64));
    astdict.insert(to_key!("isparent"), Value::Number(bool_floaty(ast.isparent)));
    
    let mut children = VecDeque::<Value>::new();
    
    for child in &ast.children
    {
        children.push_back(ast_to_dict(&child));
    }
    
    astdict.insert(to_key!("children"), Value::Array(children));
    
    let mut opdata = HashMap::<HashableValue, Value>::new();
    
    /*
    struct OpData {
        isop: bool,
        assoc: i32,
        precedence: i32
    }
    */
    
    opdata.insert(to_key!("isop"), Value::Number(bool_floaty(ast.opdata.isop)));
    #[cfg_attr(feature = "cargo-clippy", allow(cast_lossless))]
    opdata.insert(to_key!("assoc"), Value::Number(ast.opdata.assoc as f64));
    #[cfg_attr(feature = "cargo-clippy", allow(cast_lossless))]
    opdata.insert(to_key!("precedence"), Value::Number(ast.opdata.precedence as f64));
    
    astdict.insert(to_key!("opdata"), Value::Dict(opdata));
    
    return Value::Dict(astdict);
}

pub(super) fn dict_to_ast(dict : &HashMap<HashableValue, Value>) -> ASTNode
{
    let mut ast = dummy_astnode();
    
    macro_rules! get {
        ( $dict:expr, $str:expr ) =>
        {
            $dict.get(&HashableValue::Text($str.to_string()))
        }
    }
    
    macro_rules! handle {
        ( $into:expr, $dict:expr, $str:expr, $strident:ident, $subtype:ident, $helper:ident, $cast:ident, $errortext:expr ) =>
        {
            if let Some(Value::$subtype($strident)) = get!($dict, $str)
            {
                $into.$strident = $strident.$helper() as $cast;
            }
            else
            {
                panic!("error: tried to turn a dict into an ast but dict lacked \"{}\" field or the \"{}\" field was not {}", $str, $str, $errortext);
            }
        }
    }
    
    handle!(ast, dict, "text", text, Text, clone, String, "a string");
    handle!(ast, dict, "line", line, Number, round, usize, "a number");
    handle!(ast, dict, "position", position, Number, round, usize, "a number");
    if let Some(Value::Number(isparent)) = get!(dict, "isparent")
    {
        ast.isparent = float_booly(*isparent);
    }
    else
    {
        panic!("error: tried to turn a dict into an ast but dict lacked \"isparent\" field or the \"isparent\" field was not a number");
    }
    
    if let Some(Value::Array(val_children)) = get!(dict, "children")
    {
        // ast.children from dummy_astnode() starts out extant but empty
        for child in val_children
        {
            if let Value::Dict(dict) = child
            {
                ast.children.push(dict_to_ast(&dict));
            }
            else
            {
                panic!("error: values in list of children in ast node must be dictionaries that are themselves ast nodes");
            }
        }
    }
    else
    {
        panic!("error: tried to turn a dict into an ast but dict lacked \"children\" field or the \"children\" field was not a list");
    }
    
    if let Some(Value::Dict(val_opdata)) = get!(dict, "opdata")
    {
        if let Some(Value::Number(isop)) = get!(val_opdata, "isop")
        {
            ast.opdata.isop = float_booly(*isop);
        }
        else
        {
            panic!("error: tried to turn a dict into an ast but dict's opdata lacked \"isop\" field or the \"isop\" field was not a number");
        }
        if let Some(Value::Number(assoc)) = get!(val_opdata, "assoc")
        {
            ast.opdata.assoc = assoc.round() as i32;
        }
        else
        {
            panic!("error: tried to turn a dict into an ast but dict's opdata lacked \"assoc\" field or the \"assoc\" field was not a number");
        }
        if let Some(Value::Number(precedence)) = get!(val_opdata, "precedence")
        {
            ast.opdata.precedence = precedence.round() as i32;
        }
        else
        {
            panic!("error: tried to turn a dict into an ast but dict's opdata lacked \"precedence\" field or the \"precedence\" field was not a number");
        }
    }
    else
    {
        panic!("error: tried to turn a dict into an ast but dict lacked \"opdata\" field or the \"opdata\" field was not a dictionary");
    }
    
    return ast;
}
