#![allow(clippy::cast_lossless)]

pub (crate) fn pack_u64(num : u64) -> u64
{
    num
}
pub (crate) fn pack_f64(num : f64) -> u64
{
    num.to_bits()
}

pub (crate) const NOP : u64 = 0x00;

pub (crate) const PUSHFLT : u64 = 0x10;
pub (crate) const PUSHSTR : u64 = 0x11;
pub (crate) const PUSHNULL : u64 = 0x12;

pub (crate) const PUSHVAR : u64 = 0x15;
pub (crate) const PUSHGLOBAL : u64 = 0x16;
pub (crate) const PUSHGLOBALVAL : u64 = 0x17;
pub (crate) const PUSHGLOBALFUNC : u64 = 0x18;
pub (crate) const PUSHBAREGLOBAL : u64 = 0x19;
pub (crate) const PUSHINSTVAR : u64 = 0x1A;
pub (crate) const PUSHBIND : u64 = 0x1C;
pub (crate) const PUSHOBJ : u64 = 0x1D;
pub (crate) const PUSHSELF : u64 = 0x1E;
pub (crate) const PUSHOTHER : u64 = 0x1F;

pub (crate) const BINOP : u64 = 0x20;
pub (crate) const UNOP : u64 = 0x21;
pub (crate) const FUNCEXPR : u64 = 0x22;
pub (crate) const INVOKEEXPR : u64 = 0x28;

pub (crate) const NEWVAR : u64 = 0x30;
pub (crate) const UNSCOPE : u64 = 0x31;

pub (crate) const BINSTATE : u64 = 0x40;
pub (crate) const UNSTATE : u64 = 0x41;
pub (crate) const SETBAREGLOBAL : u64 = 0x42;
pub (crate) const FUNCCALL : u64 = 0x48;
pub (crate) const INVOKE : u64 = 0x4E;
pub (crate) const INVOKECALL : u64 = 0x4F;

pub (crate) const EVALUATEVAR : u64 = 0x50;
pub (crate) const EVALUATEBAREGLOBAL : u64 = 0x51;
pub (crate) const EVALUATEINSTVAR : u64 = 0x52;

pub (crate) const SWITCHCASE : u64 = 0x60;
pub (crate) const SWITCHDEFAULT : u64 = 0x61;
pub (crate) const SWITCHEXIT : u64 = 0x62;

pub (crate) const COLLECTARRAY : u64 = 0x70;
pub (crate) const COLLECTDICT : u64 = 0x71;
pub (crate) const COLLECTSET : u64 = 0x72;

pub (crate) const IF : u64 = 0x80;
pub (crate) const WHILE : u64 = 0x82;
pub (crate) const FOR : u64 = 0x83;
pub (crate) const WITH : u64 = 0x84;
pub (crate) const WITHAS : u64 = 0x85;
pub (crate) const FOREACH : u64 = 0x86;
pub (crate) const SWITCH : u64 = 0x87;

pub (crate) const BREAK : u64 = 0x90;
pub (crate) const CONTINUE : u64 = 0x91;

pub (crate) const INDIRECTION : u64 = 0xA0;
pub (crate) const ARRAYEXPR : u64 = 0xA2;
pub (crate) const DISMEMBER : u64 = 0xA3;

pub (crate) const EVALUATEINDIRECTION : u64 = 0xAE;
pub (crate) const EVALUATEARRAYEXPR : u64 = 0xAF;

pub (crate) const FUNCDEF : u64 = 0xB0;
pub (crate) const LAMBDA : u64 = 0xB1;
pub (crate) const GENERATORDEF : u64 = 0xB2;

pub (crate) const WHILETEST : u64 = 0xC0;
pub (crate) const WHILELOOP : u64 = 0xC1;
pub (crate) const WITHLOOP : u64 = 0xC2;
pub (crate) const FOREACHLOOP : u64 = 0xC3;
pub (crate) const FOREACHHEAD : u64 = 0xC4;

pub (crate) const JUMPRELATIVE : u64 = 0xD0;
pub (crate) const SHORTCIRCUITIFTRUE : u64 = 0xD8;
pub (crate) const SHORTCIRCUITIFFALSE : u64 = 0xD9;

pub (crate) const EXIT : u64 = 0xF0;
pub (crate) const RETURN : u64 = 0xF1;
pub (crate) const YIELD : u64 = 0xF2;



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
