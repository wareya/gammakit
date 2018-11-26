use super::strings::*;
use super::bytecode::*;

pub fn disassemble_bytecode(code : &Vec<u8>, mut pc : usize, mut end : usize) -> Vec<String>
{
    let mut ret = Vec::<String>::new();
    
    if end == 0
    {
        end = code.len()
    }
    if end > code.len()
    {
        panic!("end value {} is past actual end of code {} in disassembler", end, code.len());
    }
    
    macro_rules! pull_n {
        ( $n:expr ) =>
        {
            {
                pc += $n;
                code[pc-$n..pc].iter().cloned().collect::<Vec<u8>>() as Vec<u8>
            }
        }
    }
    macro_rules! pull {
        ( ) =>
        {
            {
                pc += 1;
                code[pc-1]
            }
        }
    }
    macro_rules! put_lit {
        ( $x:expr ) =>
        {
            ret.push($x.to_string())
        }
    }
    
    macro_rules! pull_text {
        ( ) =>
        {
            {
                let mut bytes = Vec::<u8>::new();
                
                let mut c = pull!();
                while c != 0 && pc < code.len() // FIXME check if this should be < or <= (will only affect malformed bytecode, but still)
                {
                    bytes.push(c);
                    c = pull!();
                }
                
                if let Ok(res) = std::str::from_utf8(&bytes)
                {
                    escape(res)
                }
                else
                {
                    "<invalid utf-8>".to_string()
                }
            }
        }
    }
    macro_rules! pull_text_unescaped {
        ( ) =>
        {
            {
                let mut bytes = Vec::<u8>::new();
                
                let mut c = pull!();
                while c != 0 && pc < code.len() // FIXME check if this should be < or <= (will only affect malformed bytecode, but still)
                {
                    bytes.push(c);
                    c = pull!();
                }
                
                if let Ok(res) = std::str::from_utf8(&bytes)
                {
                    res.to_string()
                }
                else
                {
                    "<invalid utf-8>".to_string()
                }
            }
        }
    }
    
    while pc < end
    {
        let op = code[pc];
        pc += 1;
        
        match op
        {
            NOP =>
            {
                put_lit!("NOP");
            }
            PUSHFLT =>
            {
                let immediate = pull_n!(8);
                let num = unpack_f64(&immediate);
                ret.push(format!("PUSHFLT {}", num));
            }
            PUSHSHORT =>
            {
                let immediate = pull_n!(2);
                let num = unpack_u16(&immediate);
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
                    }
                ));
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
                    }
                ));
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
                    }
                ));
            }
            UNSTATE =>
            {
                put_lit!("UNSTATE <unimplemented>");
            }
            FUNCCALL =>
            {
                put_lit!("FUNCCALL");
            }
            /*
            JUMP =>
            {
                put_lit!("JUMP <unimplemented>");
            }
            BFALSE =>
            {
                put_lit!("BFALSE <unimplemented>");
            }
            */
            SCOPE =>
            {
                put_lit!("SCOPE");
            }
            UNSCOPE =>
            {
                let immediate = pull_n!(2);
                let num = unpack_u16(&immediate);
                ret.push(format!("UNSCOPE {}", num));
            }
            COLLECTARRAY =>
            {
                let immediate = pull_n!(2);
                let num = unpack_u16(&immediate);
                ret.push(format!("COLLECTARRAY {}", num));
            }
            COLLECTDICT =>
            {
                let immediate = pull_n!(2);
                let num = unpack_u16(&immediate);
                ret.push(format!("COLLECTDICT {}", num));
            }
            IF =>
            {
                let immediate_1 = pull_n!(8);
                let immediate_2 = pull_n!(8);
                let num_1 = unpack_u64(&immediate_1) as usize;
                let num_2 = unpack_u64(&immediate_2) as usize;
                let expr_disassembly = disassemble_bytecode(code, pc, pc+num_1);
                let code_disassembly = disassemble_bytecode(code, pc+num_1, pc+num_1+num_2);
                pc += num_1;
                pc += num_2;
                put_lit!("IF");
                put_lit!("(");
                for line in expr_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!(")");
                put_lit!("{");
                for line in code_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!("}");
            }
            IFELSE =>
            {
                let immediate_1 = pull_n!(8);
                let immediate_2 = pull_n!(8);
                let immediate_3 = pull_n!(8);
                let num_1 = unpack_u64(&immediate_1) as usize;
                let num_2 = unpack_u64(&immediate_2) as usize;
                let num_3 = unpack_u64(&immediate_3) as usize;
                let expr_disassembly = disassemble_bytecode(code, pc, pc+num_1);
                let code_disassembly = disassemble_bytecode(code, pc+num_1, pc+num_1+num_2);
                let else_disassembly = disassemble_bytecode(code, pc+num_1+num_2, pc+num_1+num_2+num_3);
                pc += num_1;
                pc += num_2;
                pc += num_3;
                put_lit!("IFELSE");
                put_lit!("(");
                for line in expr_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!(")");
                put_lit!("{");
                for line in code_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!("}");
                put_lit!("{");
                for line in else_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!("}");
            }
            WHILE =>
            {
                let immediate_1 = pull_n!(8);
                let immediate_2 = pull_n!(8);
                let num_1 = unpack_u64(&immediate_1) as usize;
                let num_2 = unpack_u64(&immediate_2) as usize;
                let expr_disassembly = disassemble_bytecode(code, pc, pc+num_1);
                let code_disassembly = disassemble_bytecode(code, pc+num_1, pc+num_1+num_2);
                pc += num_1;
                pc += num_2;
                put_lit!("WHILE");
                put_lit!("(");
                for line in expr_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!(")");
                put_lit!("{");
                for line in code_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!("}");
            }
            FOR =>
            {
                let immediate_1 = pull_n!(8);
                let immediate_2 = pull_n!(8);
                let immediate_3 = pull_n!(8);
                let num_1 = unpack_u64(&immediate_1) as usize;
                let num_2 = unpack_u64(&immediate_2) as usize;
                let num_3 = unpack_u64(&immediate_3) as usize;
                let expr_disassembly = disassemble_bytecode(code, pc, pc+num_1);
                let post_disassembly = disassemble_bytecode(code, pc+num_1, pc+num_1+num_2);
                let code_disassembly = disassemble_bytecode(code, pc+num_1+num_2, pc+num_1+num_2+num_3);
                pc += num_1;
                pc += num_2;
                pc += num_3;
                put_lit!("FOR");
                put_lit!("(");
                for line in expr_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!(")");
                put_lit!("(");
                for line in post_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!(")");
                put_lit!("{");
                for line in code_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!("}");
            }
            WITH =>
            {
                let immediate = pull_n!(8);
                let num = unpack_u64(&immediate) as usize;
                let code_disassembly = disassemble_bytecode(code, pc, pc+num);
                pc += num;
                put_lit!("WITH");
                put_lit!("{");
                for line in code_disassembly
                {
                    ret.push(format!("    {}", line));
                }
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
            /*
            ARRAYREF =>
            {
                
            }
            */
            FUNCDEF =>
            {
                let name = pull_text!();
                let immediate_1 = pull_n!(2);
                let immediate_2 = pull_n!(8);
                let num_1 = unpack_u16(&immediate_1);
                let num_2 = unpack_u64(&immediate_2) as usize;
                ret.push(format!("FUNCDEF {}", name));
                put_lit!("(");
                for _ in 0..num_1
                {
                    ret.push(format!("    {}", pull_text!()));
                }
                put_lit!(")");
                let code_disassembly = disassemble_bytecode(code, pc, pc+num_2);
                pc += num_2;
                put_lit!("{");
                for line in code_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!("}");
            }
            LAMBDA =>
            {
                let immediate_1 = pull_n!(2);
                let immediate_2 = pull_n!(2);
                let immediate_3 = pull_n!(8);
                let num_1 = unpack_u16(&immediate_1);
                let num_2 = unpack_u16(&immediate_2);
                let num_3 = unpack_u64(&immediate_3) as usize;
                put_lit!("LAMBDA");
                ret.push(format!("[{}]", num_1));
                put_lit!("(");
                for _ in 0..num_2
                {
                    ret.push(format!("    {}", pull_text!()));
                }
                put_lit!(")");
                let code_disassembly = disassemble_bytecode(code, pc, pc+num_3);
                pc += num_3;
                put_lit!("{");
                for line in code_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!("}");
            }
            OBJDEF =>
            {
                let objname = pull_text!();
                
                let immediate = pull_n!(2);
                let num = unpack_u16(&immediate);
                
                ret.push(format!("OBJDEF {}", objname));
                put_lit!("{");
                
                for _ in 0..num
                {
                    let name = pull_text!();
                    let immediate_1 = pull_n!(2);
                    let immediate_2 = pull_n!(8);
                    let num_1 = unpack_u16(&immediate_1);
                    let num_2 = unpack_u64(&immediate_2) as usize;
                    ret.push(format!("    FUNCTION {}", name));
                    put_lit!("    (");
                    for _ in 0..num_1
                    {
                        ret.push(format!("    {}", pull_text!()));
                    }
                    put_lit!("    )");
                    let code_disassembly = disassemble_bytecode(code, pc, pc+num_2);
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
                let immediate = pull_n!(8);
                let num = unpack_u64(&immediate);
                ret.push(format!("LINE {}", num));
            }
            _ =>
            {
                put_lit!("<unknown>");
            }
        }
    }
    
    return ret;
}
