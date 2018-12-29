use super::*;

pub (crate) fn format_val(val : &Value) -> Option<String>
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

fn value_op_add(left : &Value, right : &Value) -> Result<Value, String>
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
fn value_op_subtract(left : &Value, right : &Value) -> Result<Value, String>
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
fn value_op_multiply(left : &Value, right : &Value) -> Result<Value, String>
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
fn value_op_divide(left : &Value, right : &Value) -> Result<Value, String>
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
fn value_op_modulo(left : &Value, right : &Value) -> Result<Value, String>
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
fn float_booly(f : f64) -> bool
{
    f >= 0.5 // FIXME do we want to replicate this or can we get away with using f.round() != 0.0 instead?
}
fn bool_floaty(b : bool) -> f64
{
    if b {1.0} else {0.0}
}
fn value_op_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
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
fn value_op_not_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(left != right)))
        }
        (Value::Text(left), Value::Text(right)) =>
        {
            Ok(Value::Number(bool_floaty(left != right)))
        }
        // TODO string comparison
        _ =>
        {
            Err("types incompatible with equal".to_string())
        }
    }
}
fn value_op_greater_or_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(left >= right)))
        }
        // TODO string comparison
        _ =>
        {
            Err("types incompatible with greater than or equal".to_string())
        }
    }
}
fn value_op_less_or_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(left <= right)))
        }
        // TODO string comparison
        _ =>
        {
            Err("types incompatible with less than or equal".to_string())
        }
    }
}
fn value_op_greater(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(left > right)))
        }
        // TODO string comparison
        _ =>
        {
            Err("types incompatible with greater than".to_string())
        }
    }
}
fn value_op_less(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(left < right)))
        }
        // TODO string comparison
        _ =>
        {
            Err("types incompatible with less than".to_string())
        }
    }
}
fn value_op_and(left : &Value, right : &Value) -> Result<Value, String>
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
fn value_op_or(left : &Value, right : &Value) -> Result<Value, String>
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

pub (crate) fn get_binop_function(op : u8) -> Option<Box<Fn(&Value, &Value) -> Result<Value, String>>>
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

fn value_op_negative(value : &Value) -> Result<Value, String>
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
fn value_op_positive(value : &Value) -> Result<Value, String>
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
fn value_op_not(value : &Value) -> Result<Value, String>
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

pub (crate) fn get_unop_function(op : u8) -> Option<Box<Fn(&Value) -> Result<Value, String>>>
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

pub (crate) fn value_truthy(imm : &Value) -> bool
{
    match imm
    {
        Value::Number(value) =>
        {
            float_booly(*value)
        }
        _ =>
        {
            true
        }
    }
}

// TODO: move these to bindings.rs or something

pub (crate) fn ast_to_dict(ast : &ASTNode) -> Value
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
    
    opdata.insert(to_key!("isop"), Value::Number(bool_floaty(ast.opdata.isop)));
    opdata.insert(to_key!("assoc"), Value::Number(ast.opdata.assoc as f64));
    opdata.insert(to_key!("precedence"), Value::Number(ast.opdata.precedence as f64));
    
    astdict.insert(to_key!("opdata"), Value::Dict(opdata));
    
    Value::Dict(astdict)
}

fn plainerr(mystr : &'static str) -> Result<ASTNode, Option<String>>
{
    Err(Some(mystr.to_string()))
}

pub (crate) fn dict_to_ast(dict : &HashMap<HashableValue, Value>) -> Result<ASTNode, Option<String>>
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
                return Err(Some(format!("error: tried to turn a dict into an ast but dict lacked \"{}\" field or the \"{}\" field was not {}", $str, $str, $errortext)));
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
        return plainerr("error: tried to turn a dict into an ast but dict lacked \"isparent\" field or the \"isparent\" field was not a number");
    }
    
    if let Some(Value::Array(val_children)) = get!(dict, "children")
    {
        // ast.children from dummy_astnode() starts out extant but empty
        for child in val_children
        {
            if let Value::Dict(dict) = child
            {
                ast.children.push(dict_to_ast(&dict)?);
            }
            else
            {
                return plainerr("error: values in list of children in ast node must be dictionaries that are themselves ast nodes");
            }
        }
    }
    else
    {
        return plainerr("error: tried to turn a dict into an ast but dict lacked \"children\" field or the \"children\" field was not a list");
    }
    
    if let Some(Value::Dict(val_opdata)) = get!(dict, "opdata")
    {
        if let Some(Value::Number(isop)) = get!(val_opdata, "isop")
        {
            ast.opdata.isop = float_booly(*isop);
        }
        else
        {
            return plainerr("error: tried to turn a dict into an ast but dict's opdata lacked \"isop\" field or the \"isop\" field was not a number");
        }
        if let Some(Value::Number(assoc)) = get!(val_opdata, "assoc")
        {
            ast.opdata.assoc = assoc.round() as i32;
        }
        else
        {
            return plainerr("error: tried to turn a dict into an ast but dict's opdata lacked \"assoc\" field or the \"assoc\" field was not a number");
        }
        if let Some(Value::Number(precedence)) = get!(val_opdata, "precedence")
        {
            ast.opdata.precedence = precedence.round() as i32;
        }
        else
        {
            return plainerr("error: tried to turn a dict into an ast but dict's opdata lacked \"precedence\" field or the \"precedence\" field was not a number");
        }
    }
    else
    {
        return plainerr("error: tried to turn a dict into an ast but dict lacked \"opdata\" field or the \"opdata\" field was not a dictionary");
    }
    
    Ok(ast)
}
