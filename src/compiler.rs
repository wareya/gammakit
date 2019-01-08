#![allow(clippy::len_zero)]

use super::{strings::*, ast::*, bytecode::*};

fn minierr(mystr : &str) -> String
{
    mystr.to_string()
}

fn plainerr<T>(mystr : &str) -> Result<T, String>
{
    Err(mystr.to_string())
}

fn compile_statement(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    code.push(LINENUM);
    code.extend(pack_u64(ast.line as u64));
    if !ast.child(0)?.isparent && !ast.last_child()?.isparent && ast.child(0)?.text == "{" && ast.last_child()?.text == "}"
    {
        code.push(SCOPE);
        for child in ast.child_slice(1, -1)?
        {
            code.extend(compile_astnode(child, scopedepth+1)?);
        }
        code.push(UNSCOPE);
        code.extend(pack_u16(scopedepth as u16));
        if scopedepth >= 0xFFFF
        {
            return plainerr("error: internal scope depth limit of 0xFFFF reached; nest your code less.");
        }
    }
    else if ast.children.len() == 3 && ast.child(1)?.isparent && ast.child(1)?.text == "binstateop"
    {
        let operator = &ast.child(1)?.child(0)?.text;
        code.extend(compile_astnode(ast.child(0)?, scopedepth)?);
        code.extend(compile_astnode(ast.child(2)?, scopedepth)?);
        code.push(BINSTATE);
        let op = get_assignment_type(operator).ok_or_else(|| minierr(&format!("internal error: unhandled or unsupported type of binary statement {}", operator)))?;
        code.push(op);
    }
    else if ast.children.len() == 2 && ast.child(1)?.isparent && ast.child(1)?.text == "unstateop"
    {
        let operator = &ast.child(1)?.child(0)?.text;
        code.extend(compile_astnode(ast.child(0)?, scopedepth)?);
        code.push(UNSTATE);
        match operator.as_str()
        {
            "++" => code.push(0x00),
            "--" => code.push(0x01),
            _ => return Err(format!("internal error: unhandled or unsupported type of unary statement {}", operator))
        }
    }
    else if ast.child(0)?.isparent
    {
        match ast.child(0)?.text.as_str()
        {
            "withstatement" =>
            {
                let ast = &ast.child(0)?;
                
                let expr = compile_astnode(ast.child(1)?, scopedepth)?;
                let block = compile_astnode(ast.child(2)?, scopedepth)?;
                
                code.extend(expr);
                code.push(WITH);
                code.extend(pack_u64(block.len() as u64));
                code.extend(block);
            }
            "declaration" | "funccall" | "funcexpr" | "funcdef" | "objdef" | "invocation_call" | "foreach"  | "switch" =>
                code.extend(compile_astnode(ast.child(0)?, scopedepth)?),
            
            "condition" =>
                code.extend(compile_astnode(ast.child(0)?.child(0)?, scopedepth)?),
            
            "instruction" =>
                match ast.child(0)?.child(0)?.text.as_str()
                {
                    "break" => code.push(BREAK),
                    "continue" => code.push(CONTINUE),
                    "return" | "yield" =>
                    {
                        match ast.child(0)?.children.len()
                        {
                            2 => code.extend(compile_astnode(ast.child(0)?.child(1)?, scopedepth)?),
                            1 => 
                            {
                                code.push(PUSHFLT);
                                code.extend(pack_f64(0.0));
                            }
                            _ => return plainerr("internal error: broken return instruction")
                        }
                        match ast.child(0)?.child(0)?.text.as_str()
                        {
                            "return" => code.push(RETURN),
                            "yield" => code.push(YIELD),
                            _ => return plainerr("internal error: broken logic in compiling return/yield AST node")
                        }
                    }
                _ => return plainerr("internal error: unhandled type of instruction")
            }
            _ => return Err(format!("internal error: unhandled type of statement `{}`", ast.child(0)?.text))
        }
    }
    else
    {
        return plainerr("internal error: statement child is not itself a parent/named node");
    }
    Ok(())
}

fn compile_declaration(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    for child in ast.child_slice(1, 0)?
    {
        let name = &child.child(0)?.child(0)?.text;
        code.push(PUSHNAME);
        code.extend(name.bytes());
        code.push(0x00);
        match ast.child(0)?.text.as_str()
        {
            "var" => code.push(DECLVAR),
            "far" => code.push(DECLFAR),
            "globalvar" => code.push(DECLGLOBALVAR),
            _ => return plainerr("internal error: non-var/far prefix to declaration")
        }
        if child.children.len() == 3
        {
            code.push(PUSHNAME);
            code.extend(name.bytes());
            code.push(0x00);
            code.extend(compile_astnode(child.child(2)?, scopedepth)?);
            code.push(BINSTATE);
            code.push(0x00);
        }
    }
    Ok(())
}

fn compile_function(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    if ast.child(1)?.children.len() > 0
    {
        let children = &ast.child(1)?.child(0)?.children;
        if children.len() > 0xFFFF
        {
            return plainerr("internal error: more than 0xFFFF (around 65000) arguments to single function");
        }
        for child in children
        {
            code.extend(compile_astnode(child, scopedepth)?);
        }
        code.push(PUSHSHORT);
        code.extend(pack_u16(children.len() as u16))
    }
    else
    {
        code.push(PUSHSHORT);
        code.extend(pack_u16(0))
    }
    code.extend(compile_astnode(ast.child(0)?, scopedepth)?);
    
    if ast.text == "funccall"
    {
        code.push(FUNCCALL);
    }
    else
    {
        code.push(FUNCEXPR);
    }
    Ok(())
}

fn compile_block(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    let sentinel = &ast.child(0)?.child(0)?;
    if !sentinel.isparent && sentinel.text == "{"
    {
        code.extend(compile_astnode(ast.child(0)?, scopedepth)?);
    }
    else
    {
        code.push(SCOPE);
        code.extend(compile_astnode(ast.child(0)?, scopedepth+1)?);
        code.push(UNSCOPE);
        code.extend(pack_u16(scopedepth as u16));
    }
    Ok(())
}

fn compile_nakedblock(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    code.push(SCOPE);
    
    for child in &ast.children
    {
        code.extend(compile_astnode(child, scopedepth+1)?);
    }
    
    code.push(UNSCOPE);
    code.extend(pack_u16(scopedepth as u16));
    Ok(())
}

fn compile_ifcondition(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    code.extend(compile_astnode(ast.child(1)?, scopedepth)?);
    
    let block = compile_astnode(ast.child(2)?, scopedepth)?;
    
    if ast.children.len() == 3
    {
        code.push(IF);
        code.extend(pack_u64(block.len() as u64));
        code.extend(block);
    }
    else if ast.children.len() == 5 && ast.child(3)?.text == "else"
    {
        let block2 = compile_astnode(ast.child(4)?, scopedepth)?;
        code.push(IFELSE);
        code.extend(pack_u64(block.len() as u64));
        code.extend(pack_u64(block2.len() as u64));
        code.extend(block);
        code.extend(block2);
    }
    else
    {
        return plainerr("internal error: broken if condition");
    }
    Ok(())
}

struct Case {
    labels: Vec<u8>,
    block: Vec<u8>
}

impl Case {
    // compiles switchcase and switchdefault blocks; in the case of default, the label vector is blank
    fn compile(ast : &ASTNode, which : u16, scopedepth : usize) -> Result<Case, String>
    {
        if !ast.isparent || !matches!(ast.text.as_str(), "switchcase" | "switchdefault")
        {
            return plainerr("error: tried to compile a non-switchcase/switchdefault ast node as a switch case")
        }
        let mut labels = vec!();
        for node in ast.child_slice(1, -2)? // implicitly causes switchdefault to have 0 labels
        {
            labels.extend(compile_astnode(node, scopedepth)?);
            labels.push(SWITCHCASE);
            labels.extend(pack_u16(which));
        }
        if labels.len() == 0
        {
            labels.push(SWITCHDEFAULT);
            labels.extend(pack_u16(which));
        }
        let mut block = compile_astnode(ast.last_child()?, scopedepth)?;
        block.push(SWITCHEXIT);
        Ok(Case{labels, block})
    }
}

fn compile_switch(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    code.extend(compile_astnode(ast.child(1)?, scopedepth)?);
    
    // SWITCH (u8)
    // num cases (u16)
    // case block locations... (relative to end of "num cases") (u64s)
    // case label expressions... (arbitrary)
    // case blocks... (arbitrary)
    
    code.push(SWITCH);
    code.extend(pack_u16((ast.children.len()-2) as u16));
    
    let mut labels = vec!();
    let mut blocks = vec!();
    
    for node in ast.child_slice(2, 0)?
    {
        let case = Case::compile(node, blocks.len() as u16, scopedepth)?;
        labels.extend(case.labels);
        
        blocks.push(case.block);
    }
    labels.push(SWITCHEXIT);
    if blocks.len() != ast.children.len()-2
    {
        return plainerr("error: broken switch node");
    }
    if blocks.len() > 0xFFFF
    {
        return plainerr("error: switches may have a maximum of 0x10000 (65000ish) labels");
    }
    let mut case_block_offset = labels.len() + (blocks.len()+1)*8;
    for block in &blocks
    {
        code.extend(pack_u64(case_block_offset as u64));
        case_block_offset += block.len();
    }
    code.extend(pack_u64(case_block_offset as u64));
    
    code.extend(labels);
    code.extend(blocks.drain(..).flatten());
    
    Ok(())
}
fn compile_whilecondition(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    let expr = compile_astnode(ast.child(1)?, scopedepth)?;
    let block = compile_astnode(ast.child(2)?, scopedepth)?;
    code.push(WHILE);
    code.extend(pack_u64(expr.len() as u64));
    code.extend(pack_u64(block.len() as u64));
    code.extend(expr);
    code.extend(block);
    Ok(())
}
fn compile_foreach(ast : &ASTNode, code : &mut Vec<u8>, mut scopedepth : usize) -> Result<(), String>
{
    if !ast.child(1)?.isparent || ast.child(1)?.text != "name"
    {
        return plainerr("error: second child (index 1) of `foreach` must be a `name`");
    }
    
    code.push(SCOPE);
    scopedepth += 1;
    
    let block = compile_astnode(ast.child(3)?, scopedepth)?;
    
    code.push(PUSHNAME);
    code.extend(ast.child(1)?.child(0)?.text.bytes());
    code.push(0x00);
    code.extend(compile_astnode(ast.child(2)?, scopedepth)?);
    code.push(FOREACH);
    code.extend(pack_u64(block.len() as u64));
    code.extend(block);
    
    scopedepth -= 1;
    code.push(UNSCOPE);
    code.extend(pack_u16(scopedepth as u16));
    
    Ok(())
}
fn compile_forcondition(ast : &ASTNode, code : &mut Vec<u8>, mut scopedepth : usize) -> Result<(), String>
{
    // FIXME make this not disgusting
    let mut header_node_0 = None;
    let mut header_node_1 = None;
    let mut header_node_2 = None;
    
    let mut header_index = 0;
    for node in ast.child_slice(2, -2)?
    {
        if node.isparent
        {
            match header_index
            {
                0 => header_node_0 = Some(node),
                1 => header_node_1 = Some(node),
                2 => header_node_2 = Some(node),
                _ => return plainerr("internal error: too many parts to for condition head")
            };
        }
        else if node.text == ";"
        {
            header_index += 1;
        }
        else
        {
            return plainerr("internal error: unexpected literal child in head of for condition");
        }
    }
    if header_index != 2
    {
        return plainerr("internal error: too many parts to for condition head");
    }
    
    // FOR loops need an extra layer of scope around them if they have an init statement
    if let Some(ref init) = header_node_0
    {
        code.push(SCOPE);
        scopedepth += 1;
        code.extend(compile_astnode(&init, scopedepth)?);
    }
    
    // FIXME: expr needs to just test true if it's zero length
    let expr = if let Some(ref expr) = header_node_1 {compile_astnode(&expr, scopedepth)?} else {Vec::<u8>::new()};
    
    let mut block : Vec<u8>;
    let post : Vec<u8>;
    
    // custom block compiler
    let sentinel = &ast.last_child()?.child(0)?.child(0)?;
    if !sentinel.isparent && sentinel.text == "{"
    {
        block = compile_astnode(ast.last_child()?.child(0)?, scopedepth)?;
        post = if let Some(ref body) = header_node_2 {compile_astnode(&body, scopedepth)?} else {Vec::<u8>::new()};
    }
    else
    {
        block = Vec::<u8>::new();
        block.push(SCOPE);
        block.extend(compile_astnode(ast.last_child()?.child(0)?, scopedepth+1)?);
        post = if let Some(ref body) = header_node_2 {compile_astnode(&body, scopedepth+1)?} else {Vec::<u8>::new()};
        block.push(UNSCOPE);
        block.extend(pack_u16(scopedepth as u16));
    }
    
    code.push(FOR);
    code.extend(pack_u64(post.len() as u64));
    code.extend(pack_u64(expr.len() as u64));
    code.extend(pack_u64(block.len() as u64));
    code.extend(post);
    code.extend(expr);
    code.extend(block);
    
    // FOR loops need an extra layer of scope around them if they have an init statement
    if header_node_0.is_some()
    {
        scopedepth -= 1;
        code.push(UNSCOPE);
        code.extend(pack_u16(scopedepth as u16));
    }
    Ok(())
}
fn compile_expr(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    if ast.children.len() != 1
    {
        return plainerr("internal error: unhandled form of expression");
    }
    code.extend(compile_astnode(ast.child(0)?, scopedepth)?);
    Ok(())
}
fn compile_simplexpr(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    if ast.children.len() != 3 || ast.child(0)?.isparent || ast.child(2)?.isparent || ast.child(0)?.text != "(" || ast.child(2)?.text != ")"
    { 
        return plainerr("internal error: unhandled form of simplexpr");
    }
    code.extend(compile_astnode(ast.child(1)?, scopedepth)?);
    Ok(())
}
fn compile_number(ast : &ASTNode, code : &mut Vec<u8>, _scopedepth : usize) -> Result<(), String>
{
    if ast.children.len() != 1
    {
        return plainerr("internal error: unhandled form of expression");
    }
    code.push(PUSHFLT);
    let float = ast.child(0)?.text.parse::<f64>().or_else(|_| Err(format!("internal error: text `{}` cannot be converted to a floating point number by rust", ast.child(0)?.text)))?;
    code.extend(pack_f64(float));
    Ok(())
}
fn compile_string(ast : &ASTNode, code : &mut Vec<u8>, _scopedepth : usize) -> Result<(), String>
{
    if ast.children.len() != 1
    {
        return plainerr("internal error: unhandled form of expression");
    }
    code.push(PUSHSTR);
    let text = slice(&ast.child(0)?.text, 1, -1);
    code.extend(unescape(&text).bytes());
    code.push(0x00);
    Ok(())
}
fn compile_name(ast : &ASTNode, code : &mut Vec<u8>, _scopedepth : usize) -> Result<(), String>
{
    if ast.children.len() != 1
    {
        return plainerr("internal error: unhandled form of expression");
    }
    code.push(PUSHVAR);
    code.extend(ast.child(0)?.text.bytes());
    code.push(0x00);
    Ok(())
}

fn compile_lvar(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    if ast.children.len() != 1
    {
        return plainerr("internal error: malformed lvar reference node");
    }
    if ast.child(0)?.text == "name"
    {
        code.push(PUSHNAME);
        code.extend(ast.child(0)?.child(0)?.text.bytes());
        code.push(0x00);
    }
    else
    {
        code.extend(compile_astnode(ast.child(0)?, scopedepth)?)
    }
    Ok(())
}
fn compile_rvar(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    if ast.children.len() != 1
    {
        return plainerr("internal error: malformed rvar reference node");
    }
    if ast.child(0)?.text == "name"
    {
        code.push(PUSHVAR);
        code.extend(ast.child(0)?.child(0)?.text.bytes());
        code.push(0x00);
    }
    else
    {
        code.extend(compile_astnode(ast.child(0)?, scopedepth)?);;
        if ast.child(0)?.isparent && matches!(ast.child(0)?.text.as_str(), "indirection" | "arrayexpr")
        {
            code.push(EVALUATION);
        }
    }
    Ok(())
}
fn compile_funcdef(ast : &ASTNode, code : &mut Vec<u8>, _scopedepth : usize) -> Result<(), String>
{
    let kind = &ast.child(0)?.child(0)?.text;
    let name = &ast.child(1)?.child(0)?.text;
    
    let mut args = Vec::<&ASTNode>::new();
    for child in ast.child_slice(3, 0)?
    {
        if !child.isparent && child.text == ")"
        {
            break;
        }
        args.push(&child);
    }
    
    let mut statements = Vec::<&ASTNode>::new();
    for child in ast.child_slice(5+args.len() as isize, 0)?
    {
        if !child.isparent && child.text == "}"
        {
            break;
        }
        statements.push(&child);
    }
                   
    let mut argbytes = Vec::<u8>::new();
    for arg in &args
    {
        argbytes.extend(arg.child(0)?.text.bytes());
        argbytes.push(0x00);
    }
    
    let mut body = Vec::<u8>::new();
    for statement in &statements
    {
        body.extend(compile_astnode(&statement, 0)?)
    }
    body.push(EXIT);
    
    match kind.as_str()
    {
        "def" => code.push(FUNCDEF),
        "globaldef" => code.push(GLOBALFUNCDEF),
        "subdef" => code.push(SUBFUNCDEF),
        "generator" => code.push(GENERATORDEF),
        _ => return plainerr("error: first token of funcdef must be \"def\" | \"globaldef\" | \"subdef\" | \"generator\"")
    }
    code.extend(name.bytes());
    code.push(0x00);
    code.extend(pack_u16(args.len() as u16));
    code.extend(pack_u64(body.len() as u64));
    code.extend(argbytes);
    code.extend(body);
    
    Ok(())
}
fn compile_lambda(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    let mut captures = Vec::<&ASTNode>::new();
    for child in ast.child(0)?.child_slice(1, -1)?
    {
        captures.push(&child);
    }
    
    let mut args = Vec::<&ASTNode>::new();
    for child in ast.child(1)?.child_slice(1, -1)?
    {
        args.push(&child);
    }
    
    let mut statements = Vec::<&ASTNode>::new();
    for child in ast.child(2)?.child_slice(1, -1)?
    {
        statements.push(&child);
    }
                   
    let mut argbytes = Vec::<u8>::new();
    for arg in &args
    {
        argbytes.extend(arg.child(0)?.text.bytes());
        argbytes.push(0x00);
    }
    
    let mut body = Vec::<u8>::new();
    for statement in &statements
    {
        body.extend(compile_astnode(statement, 0)?)
    }
            
    body.push(EXIT);
    
    let mut capturebytes = Vec::<u8>::new();
    for capture in &captures
    {
        capturebytes.push(PUSHSTR);
        capturebytes.extend(capture.child(0)?.child(0)?.text.bytes());
        capturebytes.push(0x00);
        capturebytes.extend(compile_astnode(capture.child(2)?, scopedepth)?);
    }
    
    code.extend(capturebytes);
    code.push(LAMBDA);
    code.extend(pack_u16(captures.len() as u16));
    code.extend(pack_u16(args.len() as u16));
    code.extend(pack_u64(body.len() as u64));
    code.extend(argbytes);
    code.extend(body);
    
    Ok(())
}
fn compile_objdef(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    let funcs = ast.child_slice(3, -1)?;
    let mut childcode = Vec::<u8>::new();
    for child in funcs.iter()
    {
        let code = compile_astnode(child, scopedepth)?;
        let first_byte = code.get(0).ok_or_else(|| minierr("internal error: compile_astnode for child function of objdef somehow didn't have even a single byte of code"))?;
        if *first_byte != FUNCDEF
        {
            return plainerr("error: functions inside of an object definition must be defined with \"def\", not \"globaldef\" or \"subdef\"");
        }
        // cut off the FUNCDEF byte
        let without_first_byte = code.get(1..).ok_or_else(|| minierr("internal error: compile_astnode for child function of objdef somehow didn't have even a single byte of code"))?;
        childcode.extend(without_first_byte);
    }
    code.push(OBJDEF);
    code.extend(ast.child(1)?.child(0)?.text.bytes());
    code.push(0x00);
    code.extend(pack_u16(funcs.len() as u16));
    code.extend(childcode);
    
    Ok(())
}
fn compile_arraybody(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    let mut elementcount = 0;
    let mut childexprs = Vec::<u8>::new();
    for expression in ast.child_slice(1, -1)?
    {
        if expression.text == "unusedcomma"
        {
            break
        }
        childexprs.extend(compile_astnode(expression, scopedepth)?);
        elementcount += 1;
    }
    code.extend(childexprs);
    code.push(COLLECTARRAY);
    code.extend(pack_u16(elementcount as u16));
    
    Ok(())
}
fn compile_dictbody(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    let mut elementcount = 0;
    let mut childexprs = Vec::<u8>::new();
    for expression in ast.child_slice(1, -1)?
    {
        if expression.text == "unusedcomma"
        {
            break;
        }
        childexprs.extend(compile_astnode(expression.child(0)?, scopedepth)?);
        childexprs.extend(compile_astnode(expression.child(2)?, scopedepth)?);
        elementcount += 1;
    }
    code.extend(childexprs);
    code.push(COLLECTDICT);
    code.extend(pack_u16(elementcount as u16));
    
    Ok(())
}
fn compile_setbody(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    let mut elementcount = 0;
    let mut childexprs = Vec::<u8>::new();
    for expression in ast.child_slice(2, -1)?
    {
        if expression.text == "unusedcomma"
        {
            break;
        }
        childexprs.extend(compile_astnode(expression, scopedepth)?);
        elementcount += 1;
    }
    code.extend(childexprs);
    code.push(COLLECTSET);
    code.extend(pack_u16(elementcount as u16));
    
    Ok(())
}
fn compile_arrayexpr(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    if ast.child(0)?.isparent && ast.child(0)?.text == "name"
    {
        code.push(PUSHNAME);
        code.extend(ast.child(0)?.child(0)?.text.bytes());
        code.push(0x00);
    }
    else
    {
        code.extend(compile_astnode(ast.child(0)?, scopedepth)?);
    }
    code.extend(compile_astnode(ast.child(1)?.child(1)?, scopedepth)?);
    code.push(ARRAYEXPR);
    
    Ok(())
}
fn compile_indirection(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    code.extend(compile_astnode(ast.child(0)?, scopedepth)?);
    if ast.child(0)?.text == "indirection"
    {
        code.push(EVALUATION);
    }
    code.push(PUSHNAME);
    code.extend(ast.child(1)?.child(0)?.text.bytes());
    code.push(0x00);
    code.push(INDIRECTION);
    
    Ok(())
}
fn compile_lhunop(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    if ast.children.len() == 0
    {
        return plainerr("internal error: lhunop has no children");
    }
    else if ast.children.len() == 1
    {
        code.extend(compile_astnode(ast.child(0)?, scopedepth)?);
    }
    else
    {
        let operator = &ast.child(0)?.child(0)?.text;
        
        code.extend(compile_astnode(ast.child(1)?, scopedepth)?);
        code.push(UNOP);
        
        let op = get_unop_type(slice(&operator, 0, 1).as_str()).ok_or_else(|| minierr("internal error: unhandled type of unary expression"))?;
        code.push(op);
    }
    
    Ok(())
}
fn compile_invocation_call(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    if ast.children.len() != 2
    {
        return plainerr("error: invocation must have exactly two children");
    }
    
    code.extend(compile_astnode(ast.child(1)?, scopedepth)?);
    code.push(INVOKE);
    code.push(INVOKECALL);
    
    Ok(())
}
fn compile_invocation_expr(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    if ast.children.len() != 2
    {
        return plainerr("error: invocation must have exactly two children");
    }
    
    code.extend(compile_astnode(ast.child(1)?, scopedepth)?);
    code.push(INVOKE);
    code.push(INVOKEEXPR);
    
    Ok(())
}
fn compile_astnode(ast : &ASTNode, scopedepth : usize) -> Result<Vec<u8>, String>
{
    if !ast.isparent
    {
        return plainerr("error: tried to compile non-parent ast node");
    }
    let mut code = Vec::<u8>::new();
    
    if ast.text.starts_with("binexpr_")
    {
        if ast.children.len() != 3
        {
            return plainerr("error: binexpr_ nodes must have exactly three children");
        }
        code.extend(compile_astnode(ast.child(0)?, scopedepth)?);
        let op = get_binop_type(ast.child(1)?.child(0)?.text.as_str()).ok_or_else(|| minierr("internal error: unhandled type of binary expression"))?;
        
        let mut finalcode =  compile_astnode(ast.child(2)?, scopedepth)?;
        finalcode.push(BINOP);
        finalcode.push(op);
        
        if op == 0x10 // and
        {
            code.push(SHORTCIRCUITIFFALSE);
            code.extend(pack_u64(finalcode.len() as u64));
        }
        else if op == 0x11 // or
        {
            code.push(SHORTCIRCUITIFTRUE);
            code.extend(pack_u64(finalcode.len() as u64));
        }
        code.extend(finalcode);
    }
    else
    {
        match ast.text.as_str()
        {
            "program" =>
            {
                for child in &ast.children
                {
                    code.extend(compile_astnode(&child, scopedepth)?);
                }
                code.push(EXIT);
            }
            "statement" | "barestatement" =>
                compile_statement(ast, &mut code, scopedepth)?,
            "declaration" =>
                compile_declaration(ast, &mut code, scopedepth)?,
            "name" =>
                compile_name(ast, &mut code, scopedepth)?,
            "funccall" | "funcexpr" =>
                compile_function(ast, &mut code, scopedepth)?,
            "ifcondition" =>
                compile_ifcondition(ast, &mut code, scopedepth)?,
            "whilecondition" =>
                compile_whilecondition(ast, &mut code, scopedepth)?,
            "forcondition" =>
                compile_forcondition(ast, &mut code, scopedepth)?,
            "foreach" =>
                compile_foreach(ast, &mut code, scopedepth)?,
            "switch" =>
                compile_switch(ast, &mut code, scopedepth)?,
            "expr" =>
                compile_expr(ast, &mut code, scopedepth)?,
            "simplexpr" =>
                compile_simplexpr(ast, &mut code, scopedepth)?,
            "number" =>
                compile_number(ast, &mut code, scopedepth)?,
            "string" =>
                compile_string(ast, &mut code, scopedepth)?,
            "lvar" =>
                compile_lvar(ast, &mut code, scopedepth)?,
            "rvar" =>
                compile_rvar(ast, &mut code, scopedepth)?,
            "funcdef" =>
                compile_funcdef(ast, &mut code, scopedepth)?,
            "lambda" =>
                compile_lambda(ast, &mut code, scopedepth)?,
            "objdef" =>
                compile_objdef(ast, &mut code, scopedepth)?,
            "arraybody" =>
                compile_arraybody(ast, &mut code, scopedepth)?,
            "dictbody" =>
                compile_dictbody(ast, &mut code, scopedepth)?,
            "setbody" =>
                compile_setbody(ast, &mut code, scopedepth)?,
            "arrayexpr" =>
                compile_arrayexpr(ast, &mut code, scopedepth)?,
            "indirection" =>
                compile_indirection(ast, &mut code, scopedepth)?,
            "lhunop" =>
                compile_lhunop(ast, &mut code, scopedepth)?,
            "invocation_call" =>
                compile_invocation_call(ast, &mut code, scopedepth)?,
            "invocation_expr" =>
                compile_invocation_expr(ast, &mut code, scopedepth)?,
            "block" =>
                compile_block(ast, &mut code, scopedepth)?,
            "nakedblock" =>
                compile_nakedblock(ast, &mut code, scopedepth)?,
            _ =>
                Err(format!("internal error: unhandled ast node type `{}` in compiler", ast.text))?,
        }
    }
    Ok(code)
}

/// Compiles an AST into bytecode.
pub fn compile_bytecode(ast : &ASTNode) -> Result<Vec<u8>, String>
{
    compile_astnode(ast, 0)
}
