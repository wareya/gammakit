use super::*;
use crate::interpreter::variableaccess::ValueLoc;

#[inline]
fn compiler_bytecode_desync_error<S : ToString>(text : S) -> String
{
    if cfg!(compiler_bytecode_desync_debugging)
    {
        text.to_string()
    }
    else
    {
        panic!(text.to_string())
    }
}

pub (crate) fn format_val(val : &Value) -> Option<String>
{
    match val
    {
        Value::Null => Some("<null>".to_string()),
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
        Value::InternalFunc(_) => Some("<internal function>".to_string()),
        Value::Generator(_) => Some("<generator>".to_string()),
        Value::Custom(custom) => Some(format!("<custom type discrim:{} storage:{}>", custom.discrim, custom.storage)),
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
            Ok(left.userdefdata == right.userdefdata)
        }
        (Value::InternalFunc(left), Value::InternalFunc(right)) =>
        {
            Ok(left.nameindex == right.nameindex)
        }
        // generators are never equal even in their default state
        (Value::Generator(_), Value::Generator(_)) => Ok(false),
        (Value::Instance(left), Value::Instance(right)) | (Value::Object(left), Value::Object(right)) => Ok(left==right),
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

fn inplace_value_op_add(mut left : ValueLoc, right : &Value) -> Result<(), String>
{
    // TODO: string and array.as_ref() concatenation
    match (left.as_mut()?, right)
    {
        (Value::Number(ref mut left), Value::Number(right)) =>
        {
            let newval = *left+right;
            *left = newval;
            Ok(())
        }
        (Value::Text(left), Value::Text(right)) =>
        {
            let newval = format!("{}{}", left, right);
            *left = newval;
            Ok(())
        }
        _ => Err("types incompatible with addition".to_string())
    }
}
fn inplace_value_op_subtract(mut left : ValueLoc, right : &Value) -> Result<(), String>
{
    match (left.as_mut()?, right)
    {
        (Value::Number(ref mut left), Value::Number(right)) =>
        {
            let newval = *left-right;
            *left = newval;
            Ok(())
        }
        _ => Err("types incompatible with subtraction".to_string())
    }
}
fn inplace_value_op_multiply(mut left : ValueLoc, right : &Value) -> Result<(), String>
{
    match (left.as_mut()?, right)
    {
        (Value::Number(ref mut left), Value::Number(right)) =>
        {
            let newval = *left*right;
            *left = newval;
            Ok(())
        }
        (Value::Text(left), Value::Number(right)) =>
        {
            let newval = left.repeat(right.floor() as usize);
            *left = newval;
            Ok(())
        }
        _ => Err("types incompatible with multiplication".to_string())
    }
}
fn inplace_value_op_divide(mut left : ValueLoc, right : &Value) -> Result<(), String>
{
    match (left.as_mut()?, right)
    {
        (Value::Number(ref mut left), Value::Number(right)) =>
        {
            let newval = *left/right;
            *left = newval;
            Ok(())
        }
        _ => Err("types incompatible with division".to_string())
    }
}
fn inplace_value_op_modulo(mut left : ValueLoc, right : &Value) -> Result<(), String>
{
    match (left.as_mut()?, right)
    {
        (Value::Number(ref mut left), Value::Number(mut right)) =>
        {
            if right < 0.0
            {
                right = -right;
                *left = -*left;
            }
            
            let outval = ((*left%right)+right)%right;
            *left = outval;
            Ok(())
        }
        _ => Err("types incompatible with modulo".to_string())
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

pub (crate) fn do_binop_function(op : u8, left : &Value, right : &Value) -> Result<Value, String>
{
    match op
    {
        0x10 => value_op_and(left, right),
        0x11 => value_op_or(left, right),
        0x20 => value_op_equal(left, right),
        0x21 => value_op_not_equal(left, right),
        0x22 => value_op_greater_or_equal(left, right),
        0x23 => value_op_less_or_equal(left, right),
        0x24 => value_op_greater(left, right),
        0x25 => value_op_less(left, right),
        0x30 => value_op_add(left, right),
        0x31 => value_op_subtract(left, right),
        0x40 => value_op_multiply(left, right),
        0x41 => value_op_divide(left, right),
        0x42 => value_op_modulo(left, right),
        _ => Err(compiler_bytecode_desync_error(format!("internal error: unknown binary operation 0x{:02X}", op)))
    }
}
pub (crate) fn do_binstate_function(op : u8, left : ValueLoc, right : &Value) -> Result<(), String>
{
    match op
    {
        0x30 => inplace_value_op_add(left, right),
        0x31 => inplace_value_op_subtract(left, right),
        0x40 => inplace_value_op_multiply(left, right),
        0x41 => inplace_value_op_divide(left, right),
        0x42 => inplace_value_op_modulo(left, right),
        _ => Err(compiler_bytecode_desync_error(format!("internal error: unknown in-place binary operation 0x{:02X}", op)))
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

pub (crate) fn do_unop_function(op : u8, val : &Value) -> Result<Value, String>
{
    match op
    {
        0x10 => value_op_negative(val),
        0x11 => value_op_positive(val),
        0x20 => value_op_not(val),
        // TODO: add "bitwise not"?
        _ => Err(compiler_bytecode_desync_error(format!("internal error: unknown unary operation 0x{:02X}", op)))
    }
}

fn inplace_value_op_increment(mut value : ValueLoc) -> Result<(), String>
{
    match value.as_mut()?
    {
        Value::Number(ref mut value) =>
        {
            *value += 1.0;
            Ok(())
        }
        _ => Err("type incompatible with incrementation".to_string())
    }
}
fn inplace_value_op_decrement(mut value : ValueLoc) -> Result<(), String>
{
    match value.as_mut()?
    {
        Value::Number(ref mut value) =>
        {
            *value -= 1.0;
            Ok(())
        }
        _ => Err("type incompatible with decrementation".to_string())
    }
}
pub (crate) fn do_unstate_function(op : u8,  val : ValueLoc) -> Result<(), String>
{
    match op
    {
        0x00 => inplace_value_op_increment(val),
        0x01 => inplace_value_op_decrement(val),
        _ => Err(compiler_bytecode_desync_error(format!("internal error: unknown unary state operator 0x{:02X}", op)))
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
