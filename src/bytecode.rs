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

pub (crate) const BINOPAND : u64 = 0x20;
pub (crate) const BINOPOR : u64 = 0x21;
pub (crate) const BINOPEQ : u64 = 0x22;
pub (crate) const BINOPNEQ : u64 = 0x23;
pub (crate) const BINOPGEQ : u64 = 0x24;
pub (crate) const BINOPLEQ : u64 = 0x25;
pub (crate) const BINOPG : u64 = 0x26;
pub (crate) const BINOPL : u64 = 0x27;
pub (crate) const BINOPADD : u64 = 0x28;
pub (crate) const BINOPSUB : u64 = 0x29;
pub (crate) const BINOPMUL : u64 = 0x2A;
pub (crate) const BINOPDIV : u64 = 0x2B;
pub (crate) const BINOPMOD : u64 = 0x2C;

pub (crate) const UNOPNEG : u64 = 0x2E;
pub (crate) const UNOPNOT : u64 = 0x2F;

pub (crate) const FUNCEXPR : u64 = 0x30;
pub (crate) const INVOKEEXPR : u64 = 0x31;

pub (crate) const FUNCCALL : u64 = 0x38;
pub (crate) const INVOKE : u64 = 0x39;
pub (crate) const INVOKECALL : u64 = 0x3A;

pub (crate) const BINSTATE : u64 = 0x40;
pub (crate) const BINSTATEADD : u64 = 0x41;
pub (crate) const BINSTATESUB : u64 = 0x42;
pub (crate) const BINSTATEMUL : u64 = 0x43;
pub (crate) const BINSTATEDIV : u64 = 0x44;
pub (crate) const UNSTATEINCR : u64 = 0x48;
pub (crate) const UNSTATEDECR : u64 = 0x49;
pub (crate) const SETBAREGLOBAL : u64 = 0x4F;

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

#[allow(dead_code)]
pub (crate) fn op_to_name(op : u8) -> &'static str
{
    match op
    {
        0x00 => "NOP",
        
        0x10 => "PUSHFLT",
        0x11 => "PUSHSTR",
        0x12 => "PUSHNULL",
        
        0x15 => "PUSHVAR",
        0x16 => "PUSHGLOBAL",
        0x17 => "PUSHGLOBALVAL",
        0x18 => "PUSHGLOBALFUNC",
        0x19 => "PUSHBAREGLOBAL",
        0x1A => "PUSHINSTVAR",
        0x1C => "PUSHBIND",
        0x1D => "PUSHOBJ",
        0x1E => "PUSHSELF",
        0x1F => "PUSHOTHER",
        
        0x20 => "BINOPAND",
        0x21 => "BINOPOR",
        0x22 => "BINOPEQ",
        0x23 => "BINOPNEQ",
        0x24 => "BINOPGEQ",
        0x25 => "BINOPLEQ",
        0x26 => "BINOPG",
        0x27 => "BINOPL",
        0x28 => "BINOPADD",
        0x29 => "BINOPSUB",
        0x2A => "BINOPMUL",
        0x2B => "BINOPDIV",
        0x2C => "BINOPMOD",
        
        0x2E => "UNOPNEG",
        0x2F => "UNOPNOT",
        
        0x30 => "NEWVAR",
        0x31 => "UNSCOPE",
        0x32 => "FUNCEXPR",
        0x33 => "INVOKEEXPR",
        
        0x38 => "FUNCCALL",
        0x39 => "INVOKE",
        0x3A => "INVOKECALL",
        
        0x40 => "BINSTATE",
        0x41 => "BINSTATEADD",
        0x42 => "BINSTATESUB",
        0x43 => "BINSTATEMUL",
        0x44 => "BINSTATEDIV",
        0x48 => "UNSTATEINCR",
        0x49 => "UNSTATEDECR",
        0x4F => "SETBAREGLOBAL",
        
        0x50 => "EVALUATEVAR",
        0x51 => "EVALUATEBAREGLOBAL",
        0x52 => "EVALUATEINSTVAR",
        
        0x60 => "SWITCHCASE",
        0x61 => "SWITCHDEFAULT",
        0x62 => "SWITCHEXIT",
        
        0x70 => "COLLECTARRAY",
        0x71 => "COLLECTDICT",
        0x72 => "COLLECTSET",
        
        0x80 => "IF",
        0x82 => "WHILE",
        0x83 => "FOR",
        0x84 => "WITH",
        0x85 => "WITHAS",
        0x86 => "FOREACH",
        0x87 => "SWITCH",
        
        0x90 => "BREAK",
        0x91 => "CONTINUE",
        
        0xA0 => "INDIRECTION",
        0xA2 => "ARRAYEXPR",
        0xA3 => "DISMEMBER",
        
        0xAE => "EVALUATEINDIRECTION",
        0xAF => "EVALUATEARRAYEXPR",
        
        0xB0 => "FUNCDEF",
        0xB1 => "LAMBDA",
        0xB2 => "GENERATORDEF",
        
        0xC0 => "WHILETEST",
        0xC1 => "WHILELOOP",
        0xC2 => "WITHLOOP",
        0xC3 => "FOREACHLOOP",
        0xC4 => "FOREACHHEAD",
        
        0xD0 => "JUMPRELATIVE",
        0xD8 => "SHORTCIRCUITIFTRUE",
        0xD9 => "SHORTCIRCUITIFFALSE",
        
        0xF0 => "EXIT",
        0xF1 => "RETURN",
        0xF2 => "YIELD",
        
        _ => "___UNKNOWN",
    }
}



pub (crate) fn get_assignment_type(optext : &str) -> Option<u8>
{
    match optext
    { "="  => Some(BINSTATE as u8),
      "+=" => Some(BINSTATEADD as u8),
      "-=" => Some(BINSTATESUB as u8),
      "*=" => Some(BINSTATEMUL as u8),
      "/=" => Some(BINSTATEDIV as u8),
      _ => None
    }
}
pub (crate) fn get_binop_type(optext : &str) -> Option<u8>
{
    match optext
    { "and" |
      "&&"  => Some(BINOPAND as u8),
      "or"  |
      "||"  => Some(BINOPOR as u8),
      "=="  => Some(BINOPEQ as u8),
      "!="  => Some(BINOPNEQ as u8),
      ">="  => Some(BINOPGEQ as u8),
      "<="  => Some(BINOPLEQ as u8),
      ">"   => Some(BINOPG as u8),
      "<"   => Some(BINOPL as u8),
      "+"   => Some(BINOPADD as u8),
      "-"   => Some(BINOPSUB as u8),
      "*"   => Some(BINOPMUL as u8),
      "/"   => Some(BINOPDIV as u8),
      "%"   => Some(BINOPMOD as u8),
      _ => None
    }
}
pub (crate) fn get_unop_type(optext : &str) -> Option<u8>
{
    match optext
    { "-" => Some(UNOPNEG as u8),
      "!" => Some(UNOPNOT as u8),
      _ => None
    }
}
