#![allow(clippy::cast_lossless)]

pub (crate) fn pack_u16(num : u16) -> Vec<u8>
{
    num.to_le_bytes().to_vec()
}
pub (crate) fn unpack_u16(vec : &[u8]) -> Result<u16, String>
{
    if vec.len() != 2
    {
        return Err("tried to unpack u16 from buffer of size other than 2 bytes".to_string());
    }
    let mut ret = [0u8, 0u8];
    ret.copy_from_slice(vec);
    Ok(u16::from_le_bytes(ret))
}

pub (crate) fn pack_u64(num : u64) -> Vec<u8>
{
    num.to_le_bytes().to_vec()
}
pub (crate) fn unpack_u64(vec : &[u8]) ->  Result<u64, String>
{
    if vec.len() != 8
    {
        return Err("tried to unpack u64 from buffer of size other than 8 bytes".to_string());
    }
    let mut ret = [0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
    ret.copy_from_slice(vec);
    Ok(u64::from_le_bytes(ret))
}

pub (crate) fn pack_f64(num : f64) -> Vec<u8>
{
    pack_u64(num.to_bits())
}

pub (crate) fn unpack_f64(vec : &[u8]) ->  Result<f64, String>
{
    if vec.len() != 8
    {
        return Err("tried to unpack f64 from buffer of size other than 8 bytes".to_string());
    }
    let mut ret = [0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
    ret.copy_from_slice(vec);
    let ret = u64::from_le_bytes(ret);
    Ok(f64::from_bits(ret))
}

pub (crate) const NOP : u8 = 0x00;

pub (crate) const PUSHFLT : u8 = 0x10;
pub (crate) const PUSHSHORT : u8 = 0x11;
pub (crate) const PUSHSTR : u8 = 0x12;
pub (crate) const PUSHVAR : u8 = 0x13;
pub (crate) const PUSHNAME : u8 = 0x14;

pub (crate) const BINOP : u8 = 0x20;
pub (crate) const UNOP : u8 = 0x21;
pub (crate) const FUNCEXPR : u8 = 0x22;
pub (crate) const INVOKEEXPR : u8 = 0x28;

pub (crate) const DECLVAR : u8 = 0x30;
pub (crate) const DECLFAR : u8 = 0x31;
pub (crate) const DECLGLOBALVAR : u8 = 0x32;

pub (crate) const BINSTATE : u8 = 0x40;
pub (crate) const UNSTATE : u8 = 0x41;
pub (crate) const FUNCCALL : u8 = 0x42;
pub (crate) const INVOKECALL : u8 = 0x48;

pub (crate) const INVOKE : u8 = 0x50;

pub (crate) const SCOPE : u8 = 0x60;
pub (crate) const UNSCOPE : u8 = 0x61;
pub (crate) const SWITCHCASE : u8 = 0x68;
pub (crate) const SWITCHDEFAULT : u8 = 0x69;
pub (crate) const SWITCHEXIT : u8 = 0x6F;

pub (crate) const COLLECTARRAY : u8 = 0x70;
pub (crate) const COLLECTDICT : u8 = 0x71;
pub (crate) const COLLECTSET : u8 = 0x72;

pub (crate) const IF : u8 = 0x80;
pub (crate) const WHILE : u8 = 0x82;
pub (crate) const FOR : u8 = 0x83;
pub (crate) const WITH : u8 = 0x84;
pub (crate) const FOREACH : u8 = 0x85;
pub (crate) const SWITCH : u8 = 0x86;

pub (crate) const BREAK : u8 = 0x90;
pub (crate) const CONTINUE : u8 = 0x91;
pub (crate) const SHORTCIRCUITIFTRUE : u8 = 0x98;
pub (crate) const SHORTCIRCUITIFFALSE : u8 = 0x99;

pub (crate) const INDIRECTION : u8 = 0xA0;
pub (crate) const EVALUATION : u8 = 0xA1;
pub (crate) const ARRAYEXPR : u8 = 0xA2;
pub (crate) const DISMEMBER : u8 = 0xA3;

pub (crate) const FUNCDEF : u8 = 0xB0;
pub (crate) const LAMBDA : u8 = 0xB1;
pub (crate) const OBJDEF : u8 = 0xB2;
pub (crate) const GLOBALFUNCDEF : u8 = 0xB3;
pub (crate) const SUBFUNCDEF : u8 = 0xB4;
pub (crate) const GENERATORDEF : u8 = 0xB5;

pub (crate) const WHILETEST : u8 = 0xC0;
pub (crate) const WHILELOOP : u8 = 0xC1;
pub (crate) const WITHLOOP : u8 = 0xC2;
pub (crate) const FOREACHLOOP : u8 = 0xC3;
pub (crate) const FOREACHHEAD : u8 = 0xC4;

pub (crate) const JUMPRELATIVE : u8 = 0xD0;

pub (crate) const EXIT : u8 = 0xF0;
pub (crate) const RETURN : u8 = 0xF1;
pub (crate) const YIELD : u8 = 0xF2;

pub (crate) const LINENUM : u8 = 0xF8;
pub (crate) const DEBUGINFO : u8 = 0xFF;


pub (crate) fn get_assignment_type(optext : &str) -> Option<u8>
{
    match optext
    { "="  => Some(0x00),
      "+=" => Some(0x30),
      "-=" => Some(0x31),
      "*=" => Some(0x40),
      "/=" => Some(0x41),
      _ => None
    }
}
pub (crate) fn get_binop_type(optext : &str) -> Option<u8>
{
    match optext
    { "and" => Some(0x10),
      "&&"  => Some(0x10),
      "or"  => Some(0x11),
      "||"  => Some(0x11),
      "=="  => Some(0x20),
      "!="  => Some(0x21),
      ">="  => Some(0x22),
      "<="  => Some(0x23),
      ">"   => Some(0x24),
      "<"   => Some(0x25),
      "+"   => Some(0x30),
      "-"   => Some(0x31),
      "*"   => Some(0x40),
      "/"   => Some(0x41),
      "%"   => Some(0x42),
      _ => None
    }
}
pub (crate) fn get_unop_type(optext : &str) -> Option<u8>
{
    match optext
    { "-" => Some(0x10),
      "+" => Some(0x11),
      "!" => Some(0x20),
      _ => None
    }
}
