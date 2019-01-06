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
                if let HashableValue::Text(text) = key
                {
                    ret.push_str(&format!("\"{}\"", escape(text)));
                    ret.push_str(": ");
                }
                else if let Some(part) = format_val(&hashval_to_val(key))
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
            ret.push_str("{");
            for (i, val) in set.iter().enumerate()
            {
                if let HashableValue::Text(text) = val
                {
                    ret.push_str(&format!("\"{}\"", escape(text)));
                }
                else if let Some(part) = format_val(&hashval_to_val(val))
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
        Value::Func(_) | Value::Generator(_) | Value::Special(_) => None
    }
}

fn value_op_add(left : &Value, right : &Value) -> Result<Value, String>
{
    // TODO: string and array concatenation
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) => Ok(Value::Number(left+right)),
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
            let negative_divisor = right < 0.0;
            if negative_divisor
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
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) => Ok(left==right),
        (Value::Text(left), Value::Text(right)) => Ok(left==right),
        _ => Ok(false)
    }
}
// FIXME string/array/dict/generator/etc comparison
fn value_op_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    Ok(Value::Number(bool_floaty(value_equal(left, right)?)))
}
fn value_op_not_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) => Ok(Value::Number(bool_floaty(left != right))),
        (Value::Text(left), Value::Text(right)) => Ok(Value::Number(bool_floaty(left != right))),
        _ => Ok(Value::Number(0.0))
    }
}
fn value_op_greater_or_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) => Ok(Value::Number(bool_floaty(left >= right))),
        _ => Ok(Value::Number(0.0))
    }
}
fn value_op_less_or_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) => Ok(Value::Number(bool_floaty(left <= right))),
        _ => Ok(Value::Number(0.0))
    }
}
fn value_op_greater(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) => Ok(Value::Number(bool_floaty(left > right))),
        _ => Ok(Value::Number(0.0))
    }
}
fn value_op_less(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) => Ok(Value::Number(bool_floaty(left < right))),
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
        // TODO: add "not" and "bitwise not"
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
