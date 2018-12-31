use super::strings::*;
use super::bytecode::*;

fn pull_n<'a>(pc : &mut usize, code : &'a[u8], n : usize) -> Result<&'a[u8], Option<String>>
{
    *pc += n;
    code.get(*pc-n..*pc).ok_or_else(|| Some("error: tried to read past end of code".to_string()))
}
fn pull(pc : &mut usize, code : &[u8]) -> Result<u8, Option<String>>
{
    *pc += 1;
    code.get(*pc-1).ok_or_else(|| Some("error: tried to read past end of code".to_string())).map(|c| *c)
}
fn put_lit(ret : &mut Vec<String>, mystr : &str)
{
    ret.push(mystr.to_string());
}
fn pull_text(pc : &mut usize, code : &[u8]) -> Result<String, Option<String>>
{
    let mut bytes = Vec::<u8>::new();
    
    let mut c = pull(pc, &code)?;
    while c != 0 && *pc < code.len() // FIXME check if this should be < or <= (will only affect malformed bytecode, but still)
    {
        bytes.push(c);
        c = pull(pc, &code)?;
    }
    
    std::str::from_utf8(&bytes).or_else(|_| Err(Some("error: tried to decode invalid utf-8".to_string()))).map(|mystr| escape(mystr))
}
fn pull_text_unescaped(pc : &mut usize, code : &[u8]) -> Result<String, Option<String>>
{
    let mut bytes = Vec::<u8>::new();
    
    let mut c = pull(pc, &code)?;
    while c != 0 && *pc < code.len() // FIXME check if this should be < or <= (will only affect malformed bytecode, but still)
    {
        bytes.push(c);
        c = pull(pc, &code)?;
    }
    
    std::str::from_utf8(&bytes).or_else(|_| Err(Some("error: tried to decode invalid utf-8".to_string()))).map(|mystr| mystr.to_string())
}
fn disassemble_op(op : u8, code : &[u8], mut pc : usize, ret : &mut Vec<String>) -> Result<usize, Option<String>>
{
    macro_rules! pull_n { ($n:expr) => { pull_n(&mut pc, &code, $n)? } }
    macro_rules! pull { () => { pull(&mut pc, &code)? } }
    macro_rules! pull_text { () => { pull_text(&mut pc, &code)? } }
    macro_rules! pull_text_unescaped { () => { pull_text_unescaped(&mut pc, &code)? } }
    macro_rules! put_lit { ($x:expr) => { put_lit(ret, $x) } }
    macro_rules! append_sub { ($x:expr) => { ret.append(&mut $x.into_iter().map(|line| format!("    {}", line)).collect()) } }
    
    match op
    {
        NOP =>
        {
            put_lit!("NOP");
        }
        PUSHFLT =>
        {
            let num = unpack_f64(&pull_n!(8))?;
            ret.push(format!("PUSHFLT {}", num));
        }
        PUSHSHORT =>
        {
            let num = unpack_u16(&pull_n!(2))?;
            ret.push(format!("PUSHSHORT {}", num));
        }
        PUSHSTR =>
        {
            ret.push(format!("PUSHSTR \"{}\"", pull_text_unescaped!()));
        }
        PUSHVAR =>
        {
            ret.push(format!("PUSHVAR \"{}\"", pull_text!()));
        }
        PUSHNAME =>
        {
            ret.push(format!("PUSHNAME \"{}\"", pull_text!()));
        }
        BINOP =>
        {
            let immediate = pull!();
            ret.push(format!("BINOP {}",
            match immediate
            {
                0x10 => "&&",
                0x11 => "||",
                0x20 => "==",
                0x21 => "!=",
                0x22 => ">=",
                0x23 => "<=",
                0x24 => ">" ,
                0x25 => "<" ,
                0x30 => "+" ,
                0x31 => "-" ,
                0x40 => "*" ,
                0x41 => "/" ,
                0x42 => "%" ,
                _ => "<unknown>"
            }));
        }
        UNOP =>
        {
            let immediate = pull!();
            ret.push(format!("UNOP {}",
            match immediate
            {
                0x10 => "-",
                0x11 => "+",
                _ => "<unknown>"
            }));
        }
        FUNCEXPR =>
        {
            put_lit!("FUNCEXPR");
        }
        DECLVAR =>
        {
            put_lit!("DECLVAR");
        }
        DECLFAR =>
        {
            put_lit!("DECLFAR");
        }
        BINSTATE =>
        {
            let immediate = pull!();
            ret.push(format!("BINSTATE {}",
            match immediate
            {
                0x00 => "=" ,
                0x30 => "+=" ,
                0x31 => "-=" ,
                0x40 => "*=" ,
                0x41 => "/=" ,
                0x42 => "%=" ,
                _ => "<unknown>"
            }));
        }
        UNSTATE =>
        {
            put_lit!("UNSTATE <unimplemented>");
        }
        FUNCCALL =>
        {
            put_lit!("FUNCCALL");
        }
        SCOPE =>
        {
            put_lit!("SCOPE");
        }
        UNSCOPE =>
        {
            let num = unpack_u16(&pull_n!(2))?;
            ret.push(format!("UNSCOPE {}", num));
        }
        COLLECTARRAY =>
        {
            let num = unpack_u16(&pull_n!(2))?;
            ret.push(format!("COLLECTARRAY {}", num));
        }
        COLLECTDICT =>
        {
            let num = unpack_u16(&pull_n!(2))?;
            ret.push(format!("COLLECTDICT {}", num));
        }
        IF =>
        {
            let num_1 = unpack_u64(&pull_n!(8))? as usize;
            let num_2 = unpack_u64(&pull_n!(8))? as usize;
            let expr_disassembly = disassemble_bytecode(code, pc, pc+num_1)?;
            let code_disassembly = disassemble_bytecode(code, pc+num_1, pc+num_1+num_2)?;
            pc += num_1;
            pc += num_2;
            put_lit!("IF");
            put_lit!("(");
            append_sub!(expr_disassembly);
            put_lit!(")");
            put_lit!("{");
            append_sub!(code_disassembly);
            put_lit!("}");
        }
        IFELSE =>
        {
            let num_1 = unpack_u64(&pull_n!(8))? as usize;
            let num_2 = unpack_u64(&pull_n!(8))? as usize;
            let num_3 = unpack_u64(&pull_n!(8))? as usize;
            let expr_disassembly = disassemble_bytecode(code, pc, pc+num_1)?;
            let code_disassembly = disassemble_bytecode(code, pc+num_1, pc+num_1+num_2)?;
            let else_disassembly = disassemble_bytecode(code, pc+num_1+num_2, pc+num_1+num_2+num_3)?;
            pc += num_1;
            pc += num_2;
            pc += num_3;
            put_lit!("IFELSE");
            put_lit!("(");
            append_sub!(expr_disassembly);
            put_lit!(")");
            put_lit!("{");
            append_sub!(code_disassembly);
            put_lit!("}");
            put_lit!("{");
            append_sub!(else_disassembly);
            put_lit!("}");
        }
        WHILE =>
        {
            let num_1 = unpack_u64(&pull_n!(8))? as usize;
            let num_2 = unpack_u64(&pull_n!(8))? as usize;
            let expr_disassembly = disassemble_bytecode(code, pc, pc+num_1)?;
            let code_disassembly = disassemble_bytecode(code, pc+num_1, pc+num_1+num_2)?;
            pc += num_1;
            pc += num_2;
            put_lit!("WHILE");
            put_lit!("(");
            append_sub!(expr_disassembly);
            put_lit!(")");
            put_lit!("{");
            append_sub!(code_disassembly);
            put_lit!("}");
        }
        FOR =>
        {
            let num_1 = unpack_u64(&pull_n!(8))? as usize;
            let num_2 = unpack_u64(&pull_n!(8))? as usize;
            let num_3 = unpack_u64(&pull_n!(8))? as usize;
            let expr_disassembly = disassemble_bytecode(code, pc, pc+num_1)?;
            let post_disassembly = disassemble_bytecode(code, pc+num_1, pc+num_1+num_2)?;
            let code_disassembly = disassemble_bytecode(code, pc+num_1+num_2, pc+num_1+num_2+num_3)?;
            pc += num_1;
            pc += num_2;
            pc += num_3;
            put_lit!("FOR");
            put_lit!("(");
            append_sub!(expr_disassembly);
            put_lit!(")");
            put_lit!("(");
            append_sub!(post_disassembly);
            put_lit!(")");
            put_lit!("{");
            append_sub!(code_disassembly);
            put_lit!("}");
        }
        WITH =>
        {
            let num = unpack_u64(&pull_n!(8))? as usize;
            let code_disassembly = disassemble_bytecode(code, pc, pc+num)?;
            pc += num;
            put_lit!("WITH");
            put_lit!("{");
            append_sub!(code_disassembly);
            put_lit!("}");
        }
        BREAK =>
        {
            put_lit!("BREAK");
        }
        CONTINUE =>
        {
            put_lit!("CONTINUE");
        }
        INDIRECTION =>
        {
            put_lit!("INDIRECTION");
        }
        EVALUATION =>
        {
            put_lit!("EVALUATION");
        }
        ARRAYEXPR =>
        {
            put_lit!("ARRAYEXPR");
        }
        FUNCDEF =>
        {
            let name = pull_text!();
            let num_1 = unpack_u16(&pull_n!(2))?;
            let num_2 = unpack_u64(&pull_n!(8))? as usize;
            ret.push(format!("FUNCDEF {}", name));
            put_lit!("(");
            for _ in 0..num_1
            {
                ret.push(format!("    {}", pull_text!()));
            }
            put_lit!(")");
            let code_disassembly = disassemble_bytecode(code, pc, pc+num_2)?;
            pc += num_2;
            put_lit!("{");
            append_sub!(code_disassembly);
            put_lit!("}");
        }
        LAMBDA =>
        {
            let num_1 = unpack_u16(&pull_n!(2))?;
            let num_2 = unpack_u16(&pull_n!(2))?;
            let num_3 = unpack_u64(&pull_n!(8))? as usize;
            put_lit!("LAMBDA");
            ret.push(format!("[{}]", num_1));
            put_lit!("(");
            for _ in 0..num_2
            {
                ret.push(format!("    {}", pull_text!()));
            }
            put_lit!(")");
            let code_disassembly = disassemble_bytecode(code, pc, pc+num_3)?;
            pc += num_3;
            put_lit!("{");
            append_sub!(code_disassembly);
            put_lit!("}");
        }
        OBJDEF =>
        {
            let objname = pull_text!();
            
            let immediate = pull_n!(2);
            let num = unpack_u16(&immediate)?;
            
            ret.push(format!("OBJDEF {}", objname));
            put_lit!("{");
            
            for _ in 0..num
            {
                let name = pull_text!();
                let num_1 = unpack_u16(&pull_n!(2))?;
                let num_2 = unpack_u64(&pull_n!(8))? as usize;
                ret.push(format!("    FUNCTION {}", name));
                put_lit!("    (");
                for _ in 0..num_1
                {
                    ret.push(format!("    {}", pull_text!()));
                }
                put_lit!("    )");
                let code_disassembly = disassemble_bytecode(code, pc, pc+num_2)?;
                pc += num_2;
                put_lit!("    {");
                for line in code_disassembly
                {
                    ret.push(format!("        {}", line));
                }
                put_lit!("    }");
            }
            put_lit!("}");
        }
        EXIT =>
        {
            put_lit!("EXIT");
        }
        RETURN =>
        {
            put_lit!("RETURN");
        }
        LINENUM =>
        {
            let num = unpack_u64(&pull_n!(8))?;
            ret.push(format!("LINE {}", num));
        }
        _ =>
        {
            put_lit!("<unknown>");
        }
    }
    Ok(pc)
}

pub fn disassemble_bytecode(code : &[u8], mut pc : usize, mut end : usize) -> Result<Vec<String>, Option<String>>
{
    let mut ret = Vec::<String>::new();
    
    if end == 0
    {
        end = code.len()
    }
    else if end >= code.len()
    {
        return Err(Some(format!("end value {} is past actual end of code {} in disassembler", end, code.len())));
    }
    
    while let Some(op) = code.get(pc)
    {
        pc = disassemble_op(*op, code, pc+1, &mut ret)?;
        if pc >= end
        {
            break;
        }
    }
    
    Ok(ret)
}
