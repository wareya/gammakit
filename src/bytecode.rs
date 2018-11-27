/*
Q: u64
q: i64
l/i: 32
h: 16
b/c: 8
f: f32
d: f64
*/

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

pub fn pack_u16(num : u16) -> Vec<u8>
{
    return vec!(((num>>8)&0xFF) as u8, (num&0xFF) as u8);
}
pub fn unpack_u16(vec : &[u8]) -> u16
{
    assert!(vec.len() == 2);
    return (vec[1] as u16) | ((vec[0] as u16)<<8);
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



pub fn pack_u64(num : u64) -> Vec<u8>
{
    return vec!(((num>>56)&0xFF) as u8, ((num>>48)&0xFF) as u8, ((num>>40)&0xFF) as u8, ((num>>32)&0xFF) as u8,
                ((num>>24)&0xFF) as u8, ((num>>16)&0xFF) as u8, ((num>> 8)&0xFF) as u8, ((num    )&0xFF) as u8,
    );
}
pub fn unpack_u64(vec : &[u8]) -> u64
{
    assert!(vec.len() == 8);
    return ( vec[7] as u64) | ((vec[6] as u64)<<8) | ((vec[5] as u64)<<16) | ((vec[4] as u64)<<24)
     | ((vec[3] as u64)<<32) | ((vec[2] as u64)<<40) | ((vec[1] as u64)<<48) | ((vec[0] as u64)<<56);
}

pub fn pack_f64(num : f64) -> Vec<u8>
{
    let as_u64 : u64 = unsafe { std::mem::transmute(num) };
    return pack_u64(as_u64);
}

pub fn unpack_f64(vec : &[u8]) -> f64
{
    assert!(vec.len() == 8);
    let num = unpack_u64(vec);
    let as_f64 = f64::from_bits(num);
    return as_f64;
}

pub fn pun_f64_as_u64(num : f64) -> u64
{
    unsafe { std::mem::transmute(num) }
}


pub const NOP : u8 = 0x00;
pub const PUSHFLT : u8 = 0x10;
pub const PUSHSHORT : u8 = 0x11;
pub const PUSHSTR : u8 = 0x12;
pub const PUSHVAR : u8 = 0x13;
pub const PUSHNAME : u8 = 0x14;
pub const BINOP : u8 = 0x20;
pub const UNOP : u8 = 0x21;
pub const FUNCEXPR : u8 = 0x22;
pub const DECLVAR : u8 = 0x30;
pub const DECLFAR : u8 = 0x31;
pub const BINSTATE : u8 = 0x32;
pub const UNSTATE : u8 = 0x33;
pub const FUNCCALL : u8 = 0x34;
pub const SCOPE : u8 = 0x60;
pub const UNSCOPE : u8 = 0x61;
pub const COLLECTARRAY : u8 = 0x70;
pub const COLLECTDICT : u8 = 0x71;
pub const IF : u8 = 0x80;
pub const IFELSE : u8 = 0x81;
pub const WHILE : u8 = 0x82;
pub const FOR : u8 = 0x83;
pub const WITH : u8 = 0x84;
pub const BREAK : u8 = 0x90;
pub const CONTINUE : u8 = 0x91;
pub const INDIRECTION : u8 = 0xA0;
pub const EVALUATION : u8 = 0xA1;
pub const ARRAYEXPR : u8 = 0xA2;
pub const FUNCDEF : u8 = 0xB0;
pub const LAMBDA : u8 = 0xB1;
pub const OBJDEF : u8 = 0xB2;
pub const EXIT : u8 = 0xF0;
pub const RETURN : u8 = 0xF1;
pub const LINENUM : u8 = 0xF8;
