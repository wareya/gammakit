use super::*;

pub (crate) fn format_val(val : &Value) -> Option<String>
{
    match val
    {
        Value::Number(float) => Some(format!("{:.10}", float).trim_end_matches('0').trim_end_matches('.').to_string()),
        Value::Text(string) => Some(string.clone()),
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
                if let HashableValue::Text(text) = key
                {
                    ret.push_str(&format!("\"{}\"", escape(text)));
                    ret.push_str(": ");
                }
                else if let Some(part) = format_val(&hashval_to_val(key.clone()))
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
        Value::Set(set) =>
        {
            let mut ret = String::new();
            ret.push_str("set {");
            for (i, val) in set.iter().enumerate()
            {
                if let HashableValue::Text(text) = val
                {
                    ret.push_str(&format!("\"{}\"", escape(text)));
                }
                else if let Some(part) = format_val(&hashval_to_val(val.clone()))
                {
                    ret.push_str(&part);
                }
                else
                {
                    return None
                }
                if i+1 != set.len()
                {
                    ret.push_str(", ");
                }
            }
            ret.push_str("}");
            
            Some(ret)
        }
        Value::Instance(id) => Some(format!("<instance {}>", id)), // TODO: include object name?
        Value::Object(id) => Some(format!("<object {}>", id)), // TODO: use name?
        Value::Func(_) => Some("<function>".to_string()),
        Value::Generator(_) => Some("<generator>".to_string()),
        Value::Custom(custom) => Some(format!("<custom type discrim:{} storage:{}>", custom.discrim, custom.storage)),
        Value::Special(_) => Some("<special>".to_string()),
        Value::SubFunc(_) => Some("<subfunc reference>".to_string()),
    }
}

fn value_op_add(left : &Value, right : &Value) -> Result<Value, String>
{
    // TODO: string and array concatenation
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) => Ok(Value::Number(left+right)),
        (Value::Text(left), Value::Text(right)) => Ok(Value::Text(format!("{}{}", left, right))),
        _ => Err("types incompatible with addition".to_string())
    }
}
fn value_op_subtract(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) => Ok(Value::Number(left-right)),
        _ => Err("types incompatible with subtraction".to_string())
    }
}
fn value_op_multiply(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) => Ok(Value::Number(left*right)),
        (Value::Text(left), Value::Number(right)) => Ok(Value::Text(left.repeat(right.floor() as usize))),
        _ => Err("types incompatible with multiplication".to_string())
    }
}
fn value_op_divide(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) => Ok(Value::Number(left/right)),
        _ => Err("types incompatible with division".to_string())
    }
}
fn value_op_modulo(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(mut left), Value::Number(mut right)) =>
        {
            if right < 0.0
            {
                right = -right;
                left = -left;
            }
            
            let outval = ((left%right)+right)%right;
            Ok(Value::Number(outval))
        }
        _ => Err("types incompatible with modulo".to_string())
    }
}
pub (crate) fn float_booly(f : f64) -> bool
{
    f >= 0.5 // FIXME do we want to replicate this or can we get away with using f.round() != 0.0 instead?
}
pub (crate) fn bool_floaty(b : bool) -> f64
{
    if b {1.0} else {0.0}
}

pub (crate) fn value_equal(left : &Value, right : &Value) -> Result<bool, String>
{
    macro_rules! if_then_return_false { ( $x:expr ) => { if $x { return Ok(false); } } }
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) => Ok(left==right),
        (Value::Text(left), Value::Text(right)) => Ok(left==right),
        (Value::Array(left), Value::Array(right)) =>
        {
            if_then_return_false!(left.len() != right.len());
            for (left, right) in left.iter().zip(right.iter())
            {
                if_then_return_false!(!value_equal(left, right)?);
            }
            Ok(true)
        }
        (Value::Dict(left), Value::Dict(right)) =>
        {
            if_then_return_false!(left.len() != right.len());
            for (left_key, left_val) in left.iter()
            {
                if let Some(right_val) = right.get(&left_key)
                {
                    if_then_return_false!(!value_equal(left_val, right_val)?);
                }
                else
                {
                    return Ok(false)
                }
            }
            Ok(true)
        }
        (Value::Set(left), Value::Set(right)) =>
        {
            if_then_return_false!(left.len() != right.len());
            for left_val in left.iter()
            {
                if_then_return_false!(!right.contains(&left_val));
            }
            Ok(true)
        }
        (Value::Func(left), Value::Func(right)) =>
        {
            if_then_return_false!(
                left.internal != right.internal ||
                left.name != right.name ||
                left.userdefdata != right.userdefdata ||
                left.predefined.is_some() != right.predefined.is_some()
            );
            // only applies to lambdas
            if let (Some(left), Some(right)) = (&left.predefined, &right.predefined)
            {
                if_then_return_false!(left.len() != right.len());
                for (left_key, left_val) in left.iter()
                {
                    if let Some(right_val) = right.get(left_key)
                    {
                        let left_val = left_val.borrow();
                        let right_val = right_val.borrow();
                        if_then_return_false!(!value_equal(&left_val, &right_val)?);
                    }
                    else
                    {
                        return Ok(false)
                    }
                }
            }
            // if above block doesn't run then predefineds must be (None, None) because of left.predefined.is_some() != right.predefined.is_some()
            Ok(true)
        }
        // generators are never equal even in their default state
        (Value::Generator(_), Value::Generator(_)) => Ok(false),
        (Value::Instance(left), Value::Instance(right)) | (Value::Object(left), Value::Object(right)) => Ok(left==right),
        (Value::Special(left), Value::Special(right)) => Ok(std::mem::discriminant(left)==std::mem::discriminant(right)),
        (Value::Custom(left), Value::Custom(right)) => Ok(left.discrim == right.discrim && left.storage == right.storage),
        _ => Ok(false) // all non-matching type pairs test false
    }
}
// FIXME string/array/dict/generator/etc comparison
fn value_op_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    Ok(Value::Number(bool_floaty(value_equal(left, right)?)))
}
fn value_op_not_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    Ok(Value::Number(bool_floaty(!value_equal(left, right)?)))
}
fn value_op_greater_or_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) => Ok(Value::Number(bool_floaty(left >= right))),
        (Value::Text(left), Value::Text(right)) => Ok(Value::Number(bool_floaty(left >= right))),
        _ => value_op_equal(left, right)
    }
}
fn value_op_less_or_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) => Ok(Value::Number(bool_floaty(left <= right))),
        (Value::Text(left), Value::Text(right)) => Ok(Value::Number(bool_floaty(left <= right))),
        _ => value_op_equal(left, right)
    }
}
fn value_op_greater(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) => Ok(Value::Number(bool_floaty(left > right))),
        (Value::Text(left), Value::Text(right)) => Ok(Value::Number(bool_floaty(left > right))),
        _ => Ok(Value::Number(0.0))
    }
}
fn value_op_less(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) => Ok(Value::Number(bool_floaty(left < right))),
        (Value::Text(left), Value::Text(right)) => Ok(Value::Number(bool_floaty(left < right))),
        _ => Ok(Value::Number(0.0))
    }
}

fn value_op_and(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) => Ok(Value::Number(bool_floaty(float_booly(*left)&&float_booly(*right)))),
        _ => Err("types incompatible with logical and".to_string())
    }
}
fn value_op_or(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) => Ok(Value::Number(bool_floaty(float_booly(*left)||float_booly(*right)))),
        _ => Err("types incompatible with logical or".to_string())
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
    macro_rules! enbox { ( $x:ident ) => { Some(Box::new($x)) } }
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
        Value::Number(value) => Ok(Value::Number(-value)),
        _ => Err("type incompatible with negation".to_string())
    }
}
fn value_op_positive(value : &Value) -> Result<Value, String>
{
    match value
    {
        Value::Number(value) => Ok(Value::Number(*value)),
        _ => Err("type incompatible with positive".to_string())
    }
}
fn value_op_not(value : &Value) -> Result<Value, String>
{
    match value
    {
        Value::Number(value) => Ok(Value::Number(bool_floaty(!float_booly(*value)))),
        _ => Err("type incompatible with not operator".to_string())
    }
}
        
pub (crate) fn get_unop_function(op : u8) -> Option<Box<Fn(&Value) -> Result<Value, String>>>
{
    macro_rules! enbox { ( $x:ident ) => { Some(Box::new($x)) } }
    match op
    {
        0x10 => enbox!(value_op_negative),
        0x11 => enbox!(value_op_positive),
        0x20 => enbox!(value_op_not),
        // TODO: add "bitwise not"?
        _ => None
    }
}

fn value_op_increment(value : &Value) -> Result<Value, String>
{
    value_op_add(value, &Value::Number(1.0))
}
fn value_op_decrement(value : &Value) -> Result<Value, String>
{
    value_op_subtract(value, &Value::Number(1.0))
}
pub (crate) fn get_unstate_function(op : u8) -> Option<Box<Fn(&Value) -> Result<Value, String>>>
{
    macro_rules! enbox { ( $x:ident ) => { Some(Box::new($x)) } }
    match op
    {
        0x00 => enbox!(value_op_increment),
        0x01 => enbox!(value_op_decrement),
        _ => None
    }
}

pub (crate) fn value_truthy(imm : &Value) -> bool
{
    match imm
    {
        Value::Number(value) => float_booly(*value),
        Value::Generator(gen_state) => gen_state.frame.is_some(),
        _ => true
    }
}
