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

fn compile_push_float_one() -> Vec<u8>
{
    let mut code : Vec<u8> = Vec::new();
    code.push(PUSHFLT);
    code.extend(pack_f64(1.0));
    code
}
fn compile_push_float_zero() -> Vec<u8>
{
    let mut code : Vec<u8> = Vec::new();
    code.push(PUSHFLT);
    code.extend(pack_f64(0.0));
    code
}

fn compile_string_with_prefix(code : &mut Vec<u8>, prefix : u8, text : &str)
{
    code.push(prefix);
    code.extend(text.bytes());
    code.push(0x00);
}

fn compile_unscope(code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    if scopedepth >= 0xFFFF
    {
        return plainerr("error: internal scope depth limit of 0xFFFF reached; nest your code less.");
    }
    code.push(UNSCOPE);
    code.extend(pack_u16(scopedepth as u16));
    Ok(())
}

fn compile_binstate(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    let operator = &ast.child(1)?.child(0)?.text;
    let op = get_assignment_type(operator).ok_or_else(|| minierr(&format!("internal error: unhandled or unsupported type of binary statement {}", operator)))?;
    
    code.extend(compile_astnode(ast.child(0)?, scopedepth)?);
    code.extend(compile_astnode(ast.child(2)?, scopedepth)?);
    code.push(BINSTATE);
    code.push(op);
    
    Ok(())
}
fn compile_unstate(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
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
    
    Ok(())
}
fn compile_with(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    let expr = compile_astnode(ast.child(1)?, scopedepth)?;
    let mut block = compile_astnode(ast.child(2)?, scopedepth)?;
    block.push(WITHLOOP);
    
    code.extend(expr);
    code.push(WITH);
    code.extend(pack_u64(block.len() as u64));
    code.extend(block);
    
    Ok(())
}

fn compile_statement(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    code.push(LINENUM);
    code.extend(pack_u64(ast.line as u64));
    
    if ast.child(0)?.isparent
    {
        match ast.child(0)?.text.as_str()
        {
            "blankstatement" => {}
            "binstate" =>
                compile_binstate(ast.child(0)?, code, scopedepth)?,
            "unstate" =>
                compile_unstate(ast.child(0)?, code, scopedepth)?,
            "withstatement" =>
                compile_with(ast.child(0)?, code, scopedepth)?,
            "declaration" | "funccall" | "funcexpr" | "funcdef" | "objdef" | "invocation_call" | "foreach"  | "switch" | "statementlist" =>
                code.extend(compile_astnode(ast.child(0)?, scopedepth)?),
            "condition" =>
                code.extend(compile_astnode(ast.child(0)?.child(0)?, scopedepth)?),
            "instruction" =>
                // FIXME move to function
                match ast.child(0)?.child(0)?.text.as_str()
                {
                    "break" => code.push(BREAK),
                    "continue" => code.push(CONTINUE),
                    "return" | "yield" =>
                    {
                        match ast.child(0)?.children.len()
                        {
                            2 => code.extend(compile_astnode(ast.child(0)?.child(1)?, scopedepth)?),
                            1 => code.extend(compile_push_float_zero()),
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
        
        // evaluate right hand side of assignment, if there is one, BEFORE declaring the variable
        if child.children.len() == 3
        {
            match ast.child(0)?.text.as_str()
            {
                "globalvar" =>
                {
                    compile_string_with_prefix(code, PUSHVAR, "global");
                    compile_string_with_prefix(code, PUSHNAME, &name);
                    code.push(INDIRECTION);
                }
                _ => compile_string_with_prefix(code, PUSHNAME, &name)
            }
            code.extend(compile_astnode(child.child(2)?, scopedepth)?);
        }
        
        // declare the variable
        compile_string_with_prefix(code, PUSHNAME, &name);
        match ast.child(0)?.text.as_str()
        {
            "var" => code.push(DECLVAR),
            "far" => code.push(DECLFAR),
            "globalvar" => code.push(DECLGLOBALVAR),
            _ => return plainerr("internal error: non-var/far prefix to declaration")
        }
        
        // perform the assignment to the newly-declared variable
        if child.children.len() == 3
        {
            code.push(BINSTATE);
            code.push(0x00);
        }
    }
    Ok(())
}

fn compile_arrayindex(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    code.extend(compile_astnode(ast.child(1)?, scopedepth)?);
    code.push(ARRAYEXPR);
    
    Ok(())
}
fn compile_indirection(ast : &ASTNode, code : &mut Vec<u8>, _scopedepth : usize) -> Result<(), String>
{
    compile_string_with_prefix(code, PUSHNAME, &ast.child(1)?.child(0)?.text); // FIXME make this use PUSHSTR
    code.push(INDIRECTION);
    
    Ok(())
}
fn compile_funcargs(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize, left_is_var : bool) -> Result<(), String>
{
    if left_is_var
    {
        code.push(EVALUATION);
    }
    let args = &ast.child(1)?.children;
    if args.len() > 0xFFFF
    {
        return plainerr("internal error: more than 0xFFFF (around 65000) arguments to single function");
    }
    for child in args
    {
        code.extend(compile_astnode(child, scopedepth)?);
    }
    code.push(PUSHSHORT);
    code.extend(pack_u16(args.len() as u16));
    code.push(FUNCEXPR);
    
    Ok(())
}

fn compile_dismember(ast : &ASTNode, code : &mut Vec<u8>, _scopedepth : usize) -> Result<(), String>
{
    compile_string_with_prefix(code, PUSHNAME, &ast.child(1)?.child(0)?.text); // FIXME make this use PUSHSTR
    code.push(DISMEMBER);
    
    Ok(())
}

fn rhunexpr_left_type_is_variable(node : &ASTNode) -> bool
{
    match node.text.as_str()
    {
        "name" => true,
        "rhunexpr_right" =>
        {
            match node.child(0)
            {
                Ok(node) => matches!(node.text.as_str(), "indirection" | "arrayindex"),
                _ => false
            }
        }
        _ => false
    }
}

fn compile_rhunexpr_inner(nodes : &[ASTNode], code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    match nodes.len()
    {
        0 => return plainerr("error: rhunexpr has no children"),
        1 =>
        {
            let node = &nodes[0];
            if node.isparent && node.text == "name"
            {
                compile_string_with_prefix(code, PUSHNAME, &node.child(0)?.text);
            }
            else
            {
                code.extend(compile_astnode(node, scopedepth)?);
            }
        }
        _ =>
        {
            compile_rhunexpr_inner(&nodes[..nodes.len()-1], code, scopedepth)?;
            let left_is_var = rhunexpr_left_type_is_variable(&nodes[nodes.len()-2]);
            let end = nodes[nodes.len()-1].child(0)?;
            match end.text.as_str()
            {
                "funcargs" => compile_funcargs(end, code, scopedepth, left_is_var)?,
                "dismember" => compile_dismember(end, code, scopedepth)?,
                "arrayindex" => compile_arrayindex(end, code, scopedepth)?,
                "indirection" => compile_indirection(end, code, scopedepth)?,
                _ => return plainerr("error: rhunexpr contains unknown final node")
            }
        }
    }
    Ok(())
}

fn compile_rhunexpr(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    compile_rhunexpr_inner(&ast.children[..], code, scopedepth)
}
fn compile_funccall(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    let mut subcode = vec!();
    compile_rhunexpr(ast, &mut subcode, scopedepth)?;
    
    match subcode.pop()
    {
        Some(FUNCEXPR) => subcode.push(FUNCCALL),
        Some(_) => return plainerr("internal error: tried to use an indexing expression or indirection expression as a statement (this is supposed to be caught by parse_tweak_ast)"),
        None => return plainerr("internal error: compiled child of funccall node was empty")
    }
    code.extend(subcode);
    Ok(())
}

fn compile_rvar(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    let mut subcode = compile_astnode(ast.child(0)?, scopedepth)?;
    
    match subcode.pop()
    {
        Some(INDIRECTION) => subcode.extend(&[INDIRECTION, EVALUATION]),
        Some(ARRAYEXPR) => subcode.extend(&[ARRAYEXPR, EVALUATION]),
        Some(other) => subcode.push(other),
        None => return plainerr("internal error: compiled child of rvar node was empty")
    }
    code.extend(subcode);
    Ok(())
}
fn compile_lvar(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    if ast.child(0)?.text == "name"
    {
        compile_string_with_prefix(code, PUSHNAME, &ast.child(0)?.child(0)?.text);
    }
    else
    {
        code.extend(compile_astnode(ast.child(0)?, scopedepth)?);
    }
    Ok(())
}

fn compile_block(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    let sentinel = &ast.child(0)?.child(0)?;
    if sentinel.isparent && sentinel.text == "statementlist"
    {
        code.extend(compile_astnode(ast.child(0)?, scopedepth)?);
    }
    else
    {
        code.push(SCOPE);
        code.extend(compile_astnode(ast.child(0)?, scopedepth+1)?);
        compile_unscope(code, scopedepth)?;
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
    
    compile_unscope(code, scopedepth)?;
    Ok(())
}

fn compile_ternary(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    code.extend(compile_astnode(ast.child(0)?, scopedepth)?);
    
    let mut block = compile_astnode(ast.child(2)?, scopedepth)?;

    let block2 = compile_astnode(ast.child(4)?, scopedepth)?;
    block.push(JUMPRELATIVE);
    block.extend(pack_u64(block2.len() as u64));
    
    code.push(IFELSE);
    code.extend(pack_u64(block.len() as u64));
    code.extend(block);
    code.extend(block2);
    
    Ok(())
}

fn compile_ifcondition(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    code.extend(compile_astnode(ast.child(1)?, scopedepth)?);
    
    let mut block = compile_astnode(ast.child(2)?, scopedepth)?;
    
    if ast.children.len() == 3
    {
        code.push(IF);
        code.extend(pack_u64(block.len() as u64));
        code.extend(block);
    }
    else if ast.children.len() == 5 && ast.child(3)?.text == "else"
    {
        let block2 = compile_astnode(ast.child(4)?, scopedepth)?;
        block.push(JUMPRELATIVE);
        block.extend(pack_u64(block2.len() as u64));
        code.push(IFELSE);
        code.extend(pack_u64(block.len() as u64));
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
    code.extend(compile_astnode(ast.child(2)?, scopedepth)?);
    
    // SWITCH (u8)
    // num cases (u16)
    // case block locations... (relative to end of "num cases") (u64s)
    // case label expressions... (arbitrary)
    // case blocks... (arbitrary)
    
    
    let cases = ast.child(5)?;
    
    code.push(SWITCH);
    code.extend(pack_u16(cases.children.len() as u16));
    
    let mut labels = vec!();
    let mut blocks = vec!();
    
    for node in &cases.children
    {
        let case = Case::compile(node, blocks.len() as u16, scopedepth)?;
        labels.extend(case.labels);
        
        blocks.push(case.block);
    }
    labels.push(SWITCHEXIT);
    if blocks.len() != cases.children.len()
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
    let mut expr = compile_astnode(ast.child(1)?, scopedepth)?;
    expr.push(WHILETEST);
    let mut block = compile_astnode(ast.child(2)?, scopedepth)?;
    block.push(WHILELOOP);
    
    code.push(WHILE);
    code.extend(pack_u64(expr.len() as u64));
    code.extend(pack_u64(block.len() as u64));
    code.extend(expr);
    code.extend(block);
    Ok(())
}
fn compile_foreach(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    if !ast.child(2)?.isparent || ast.child(2)?.text != "name"
    {
        return plainerr("error: child index 2 of `foreach` must be a `name`");
    }
    
    code.push(SCOPE);
    
    let mut block = vec!(FOREACHHEAD);
    block.extend(compile_astnode(ast.child(6)?, scopedepth+1)?);
    block.push(FOREACHLOOP);
    
    // use expression
    compile_string_with_prefix(code, PUSHNAME, &ast.child(2)?.child(0)?.text);
    
    code.extend(compile_astnode(ast.child(4)?, scopedepth+1)?);
    code.push(FOREACH);
    code.extend(pack_u64(block.len() as u64));
    code.extend(block);
    
    compile_unscope(code, scopedepth)?;
    
    Ok(())
}
fn compile_forcondition(ast : &ASTNode, code : &mut Vec<u8>, mut scopedepth : usize) -> Result<(), String>
{
    // FIXME make this not disgusting
    let mut header_init = None;
    let mut header_expr = None;
    let mut header_post = None;
    
    let mut header_index = 0;
    for node in ast.child_slice(2, -2)?
    {
        if node.isparent
        {
            match header_index
            {
                0 => header_init = Some(node),
                1 => header_expr = Some(node),
                2 => header_post = Some(node),
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
        return plainerr("internal error: wrong number of parts parts to for condition head");
    }
    
    // for loops act almost exactly like while loops,
    // except that the "post" execution expression is a prefix to the loop test expression,
    // but it is skipped over the first time the loop is entered
    
    // for loops need an extra layer of scope around them if they have an init statement
    if let Some(ref init) = header_init
    {
        code.push(SCOPE);
        scopedepth += 1;
        code.extend(compile_astnode(&init, scopedepth)?);
    }
    
    let mut expr = match header_expr
    {
        Some(ref expr) => compile_astnode(&expr, scopedepth)?,
        _ => compile_push_float_one()
    };
    let post = match header_post
    {
        Some(ref body) => compile_astnode(&body, scopedepth)?,
        _ => Vec::<u8>::new()
    };
    let mut block = compile_astnode(ast.last_child()?, scopedepth)?;
    
    expr.push(WHILETEST);
    block.push(WHILELOOP);
    
    code.push(FOR);
    code.extend(pack_u64(post.len() as u64));
    code.extend(pack_u64(expr.len() as u64));
    code.extend(pack_u64(block.len() as u64));
    code.extend(post);
    code.extend(expr);
    code.extend(block);
    
    // for loops need an extra layer of scope around them if they have an init statement
    if header_init.is_some()
    {
        scopedepth -= 1;
        compile_unscope(code, scopedepth)?;
    }
    Ok(())
}
fn compile_expr(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    if ast.children.len() != 1
    {
        return plainerr("internal error: unhandled form of expr");
    }
    code.extend(compile_astnode(ast.child(0)?, scopedepth)?);
    Ok(())
}
fn compile_parenexpr(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), String>
{
    code.extend(compile_astnode(ast.child(1)?, scopedepth)?);
    Ok(())
}
fn compile_number(ast : &ASTNode, code : &mut Vec<u8>, _scopedepth : usize) -> Result<(), String>
{
    if ast.children.len() != 1
    {
        return plainerr("internal error: unhandled form of number");
    }
    code.push(PUSHFLT);
    let float = ast.child(0)?.text.parse::<f64>().or_else(|_| Err(format!("internal error: text `{}` cannot be converted to a floating point number by rust", ast.child(0)?.text)))?;
    code.extend(pack_f64(float));
    Ok(())
}
fn compile_string(ast : &ASTNode, code : &mut Vec<u8>, _scopedepth : usize) -> Result<(), String>
{
    compile_string_with_prefix(code, PUSHSTR, &unescape(&slice(&ast.child(0)?.text, 1, -1)));
    Ok(())
}
fn compile_name(ast : &ASTNode, code : &mut Vec<u8>, _scopedepth : usize) -> Result<(), String>
{
    compile_string_with_prefix(code, PUSHVAR, &ast.child(0)?.text);
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
    let captures : Vec<&ASTNode> = ast.child(0)?.child_slice(1, -1)?.iter().collect();
    let args : Vec<&ASTNode> = ast.child(1)?.child_slice(1, -1)?.iter().collect();
    let statements : Vec<&ASTNode> = ast.child(2)?.child_slice(1, -1)?.iter().collect();
                   
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
        compile_string_with_prefix(code, PUSHSTR, &capture.child(0)?.child(0)?.text);
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
    compile_string_with_prefix(code, OBJDEF, &ast.child(1)?.child(0)?.text);
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
            break;
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
        panic!("error: tried to compile non-parent ast node");
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
        
        let mut finalcode = compile_astnode(ast.child(2)?, scopedepth)?;
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
            "statementlist" =>
            // FIXME move to function
                if ast.children.len() >= 3
                {
                    code.push(SCOPE);
                    for child in ast.child_slice(1, -1)?
                    {
                        code.extend(compile_astnode(child, scopedepth+1)?);
                    }
                    compile_unscope(&mut code, scopedepth)?;
                },
            "declaration" =>
                compile_declaration(ast, &mut code, scopedepth)?,
            "name" =>
                compile_name(ast, &mut code, scopedepth)?,
            "rhunexpr" =>
                compile_rhunexpr(ast, &mut code, scopedepth)?,
            "funccall" =>
                compile_funccall(ast, &mut code, scopedepth)?,
            "ternary" =>
                compile_ternary(ast, &mut code, scopedepth)?,
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
            "expr" | "simplexpr" =>
                compile_expr(ast, &mut code, scopedepth)?,
            "parenexpr" =>
                compile_parenexpr(ast, &mut code, scopedepth)?,
            "number" =>
                compile_number(ast, &mut code, scopedepth)?,
            "string" =>
                compile_string(ast, &mut code, scopedepth)?,
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
            "lvar" =>
                compile_lvar(ast, &mut code, scopedepth)?,
            "rvar" =>
                compile_rvar(ast, &mut code, scopedepth)?,
            _ =>
                //Err(format!("internal error: unhandled ast node type `{}` in compiler", ast.text))?,
                panic!("internal error: unhandled ast node type `{}` in compiler", ast.text)
        }
    }
    Ok(code)
}

/// Compiles an AST into bytecode.
pub fn compile_bytecode(ast : &ASTNode) -> Result<Vec<u8>, String>
{
    compile_astnode(ast, 0)
}
