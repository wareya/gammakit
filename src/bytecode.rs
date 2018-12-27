#![allow(clippy::cast_lossless)]

/*
fn pack_u8(num : u8) -> Vec<u8>
{
    return vec!(num);
}
fn unpack_u8(vec : &[u8]) -> u8
{
    assert!(vec.len() == 1);
    return vec[0];
}
*/

pub (crate) fn pack_u16(num : u16) -> Vec<u8>
{
    vec!(((num>>8)&0xFF) as u8, (num&0xFF) as u8)
}
pub (crate) fn unpack_u16(vec : &[u8]) -> u16
{
    assert!(vec.len() == 2);
    (vec[1] as u16) | ((vec[0] as u16)<<8)
}

/*
fn pack_u32(num : u32) -> Vec<u8>
{
    return vec!((num&0xFF) as u8, ((num>>8)&0xFF) as u8, ((num>>16)&0xFF) as u8, ((num>>24)&0xFF) as u8);
}
fn unpack_u32(vec : &Vec<u8>) -> u32
{
    assert!(vec.len() == 4);
    return (vec[0] as u32) | ((vec[1] as u32)<<8) | ((vec[2] as u32)<<16) | ((vec[3] as u32)<<24);
}
*/


pub (crate) fn pack_u64(num : u64) -> Vec<u8>
{
    vec!(((num>>56)&0xFF) as u8, ((num>>48)&0xFF) as u8, ((num>>40)&0xFF) as u8, ((num>>32)&0xFF) as u8,
         ((num>>24)&0xFF) as u8, ((num>>16)&0xFF) as u8, ((num>> 8)&0xFF) as u8, ((num    )&0xFF) as u8)
}
pub (crate) fn unpack_u64(vec : &[u8]) -> u64
{
    assert!(vec.len() == 8);
    (   vec[7] as u64)      | ((vec[6] as u64)<< 8) | ((vec[5] as u64)<<16) | ((vec[4] as u64)<<24)
    | ((vec[3] as u64)<<32) | ((vec[2] as u64)<<40) | ((vec[1] as u64)<<48) | ((vec[0] as u64)<<56)
}

pub (crate) fn pun_f64_as_u64(num : f64) -> u64
{
    unsafe { std::mem::transmute(num) }
}

pub (crate) fn pack_f64(num : f64) -> Vec<u8>
{
    pack_u64(pun_f64_as_u64(num))
}

pub (crate) fn unpack_f64(vec : &[u8]) -> f64
{
    assert!(vec.len() == 8);
    let num = unpack_u64(vec);
    f64::from_bits(num)
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
pub (crate) const DECLVAR : u8 = 0x30;
pub (crate) const DECLFAR : u8 = 0x31;
pub (crate) const BINSTATE : u8 = 0x32;
pub (crate) const UNSTATE : u8 = 0x33;
pub (crate) const FUNCCALL : u8 = 0x34;
pub (crate) const SCOPE : u8 = 0x60;
pub (crate) const UNSCOPE : u8 = 0x61;
pub (crate) const COLLECTARRAY : u8 = 0x70;
pub (crate) const COLLECTDICT : u8 = 0x71;
pub (crate) const IF : u8 = 0x80;
pub (crate) const IFELSE : u8 = 0x81;
pub (crate) const WHILE : u8 = 0x82;
pub (crate) const FOR : u8 = 0x83;
pub (crate) const WITH : u8 = 0x84;
pub (crate) const BREAK : u8 = 0x90;
pub (crate) const CONTINUE : u8 = 0x91;
pub (crate) const INDIRECTION : u8 = 0xA0;
pub (crate) const EVALUATION : u8 = 0xA1;
pub (crate) const ARRAYEXPR : u8 = 0xA2;
pub (crate) const FUNCDEF : u8 = 0xB0;
pub (crate) const LAMBDA : u8 = 0xB1;
pub (crate) const OBJDEF : u8 = 0xB2;
pub (crate) const EXIT : u8 = 0xF0;
pub (crate) const RETURN : u8 = 0xF1;
pub (crate) const LINENUM : u8 = 0xF8;

pub (crate) fn get_assignment_type(optext : &str) -> Option<u8>
{
    match optext
    { "="    => Some(0x00),
      "+="   => Some(0x30),
      "-="   => Some(0x31),
      "*="   => Some(0x40),
      "/="   => Some(0x41),
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
