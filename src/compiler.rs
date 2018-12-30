#![allow(clippy::len_zero)]

use super::strings::*;
use super::ast::*;
use super::bytecode::*;

fn plainerr<T>(mystr : &str) -> Result<T, Option<String>>
{
    Err(Some(mystr.to_string()))
}

fn compile_statement(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), Option<String>>
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
        if let Some(op) = get_assignment_type(operator)
        {
            code.push(op);
        }
        else
        {
            // panic!() fixme
            return plainerr(&format!("internal error: unhandled or unsupported type of binary statement {}", operator));
            //println!("internal error: unhandled or unsupported type of binary statement {}", operator);
            //print_ast(ast);
            //assert!(false);
        }
    }
    else if ast.child(0)?.isparent
    {
        if ast.child(0)?.text == "withstatement"
        {
            let ast = &ast.child(0)?;
            
            let expr = compile_astnode(ast.child(1)?, scopedepth)?;
            let sentinel = &ast.child(2)?.child(0)?.child(0)?;
            
            code.extend(expr);
            code.push(WITH);
            
            if !sentinel.isparent && sentinel.text == "{"
            {
                let block = compile_astnode(ast.child(2)?.child(0)?, scopedepth)?;
                code.extend(pack_u64(block.len() as u64));
                code.extend(block);
            }
            else
            {
                let mut block = Vec::<u8>::new();
                block.push(SCOPE);
                block.extend(compile_astnode(ast.child(2)?.child(0)?, scopedepth+1)?);
                block.push(UNSCOPE);
                block.extend(pack_u16(scopedepth as u16));
                code.extend(pack_u64(block.len() as u64));
                code.extend(block);
            }   
        }
        else if matches!(ast.child(0)?.text.as_str(), "declaration" | "funccall" | "funcexpr" | "funcdef" | "objdef")
        {
            code.extend(compile_astnode(ast.child(0)?, scopedepth)?);
        }
        else if ast.child(0)?.text == "condition"
        {
            code.extend(compile_astnode(ast.child(0)?.child(0)?, scopedepth)?);
        }
        else if ast.child(0)?.text == "instruction"
        {
            if ast.child(0)?.child(0)?.text == "break"
            {
                code.push(BREAK);
            }
            else if ast.child(0)?.child(0)?.text == "continue"
            {
                code.push(CONTINUE);
            }
            else if ast.child(0)?.child(0)?.text == "return"
            {
                if ast.child(0)?.children.len() == 2
                {
                    code.extend(compile_astnode(ast.child(0)?.child(1)?, scopedepth)?);
                }
                else if ast.child(0)?.children.len() == 1
                {
                    code.push(PUSHFLT);
                    code.extend(pack_f64(0.0));
                }
                else
                {
                    // panic!() fixme
                    return plainerr("internal error: broken return instruction");
                    //println!("internal error: broken return instruction");
                    //print_ast(ast);
                    //assert!(false);
                }
                code.push(RETURN);
            }
            else
            {
                // panic!() fixme
                return plainerr("internal error: unhandled type of instruction");
                //println!("internal error: unhandled type of instruction");
                //print_ast(ast);
                //assert!(false);
            }
        }
        else
        {
            // panic!() fixme
            return plainerr("internal error: unhandled type of statement");
            //println!("internal error: unhandled type of statement");
            //print_ast(ast);
            //assert!(false);
        }
    }
    else
    {
        // panic!() fixme
        return plainerr("internal error: statement child is not itself a parent/named node");
        //println!("internal error: statement child is not itself a parent/named node");
        //print_ast(ast);
        //assert!(false);
    }
    Ok(())
}

fn compile_declaration(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), Option<String>>
{
    for child in ast.child_slice(1, 0)?
    {
        let name = &child.child(0)?.child(0)?.text;
        code.push(PUSHNAME);
        code.extend(name.bytes());
        code.push(0x00);
        if ast.child(0)?.text == "var"
        {
            code.push(DECLVAR);
        }
        else if ast.child(0)?.text == "far"
        {
            code.push(DECLFAR);
        }
        else
        {
            return plainerr("internal error: non-var/far prefix to declaration");
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

fn compile_function(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), Option<String>>
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
            //print_ast(child)
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
    // code.push(0x00); // FIXME this was wrong
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

fn compile_ifcondition(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), Option<String>>
{
    let expr = compile_astnode(ast.child(1)?, scopedepth)?;
    let sentinel = &ast.child(2)?.child(0)?.child(0)?;
    let mut block : Vec<u8>;
    if !sentinel.isparent && sentinel.text == "{"
    {
        block = compile_astnode(ast.child(2)?.child(0)?, scopedepth)?;
    }
    else
    {
        block = Vec::<u8>::new();
        block.push(SCOPE);
        block.extend(compile_astnode(ast.child(2)?.child(0)?, scopedepth+1)?);
        block.push(UNSCOPE);
        block.extend(pack_u16(scopedepth as u16));
    }
    if ast.children.len() == 3
    {
        code.push(IF);
        code.extend(pack_u64(expr.len() as u64));
        code.extend(pack_u64(block.len() as u64));
        code.extend(expr);
        code.extend(block);
    }
    else if ast.children.len() == 5 && ast.child(3)?.text == "else"
    {
        let sentinel = &ast.child(4)?.child(0)?.child(0)?;
        let mut block2 : Vec<u8>;
        if !sentinel.isparent && sentinel.text == "{"
        {
            block2 = compile_astnode(ast.child(4)?.child(0)?, scopedepth)?;
        }
        else
        {
            block2 = Vec::<u8>::new();
            block2.push(SCOPE);
            block2.extend(compile_astnode(ast.child(4)?.child(0)?, scopedepth+1)?);
            block2.push(UNSCOPE);
            block2.extend(pack_u16(scopedepth as u16));
        }
        code.push(IFELSE);
        code.extend(pack_u64(expr.len() as u64));
        code.extend(pack_u64(block.len() as u64));
        code.extend(pack_u64(block2.len() as u64));
        code.extend(expr);
        code.extend(block);
        code.extend(block2);
    }
    else
    {
        // panic!() fixme
        return plainerr("internal error: broken if condition");
        //println!("internal error: broken if condition");
        //print_ast(ast);
        //assert!(false);
    }
    Ok(())
}
fn compile_whilecondition(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), Option<String>>
{
    let expr = compile_astnode(ast.child(1)?, scopedepth)?;
    // FIXME: make this a subroutine lmao
    let sentinel = &ast.child(2)?.child(0)?.child(0)?;
    let mut block : Vec<u8>;
    if !sentinel.isparent && sentinel.text == "{"
    {
        block = compile_astnode(ast.child(2)?.child(0)?, scopedepth)?;
    }
    else
    {
        block = Vec::<u8>::new();
        block.push(SCOPE);
        block.extend(compile_astnode(ast.child(2)?.child(0)?, scopedepth+1)?);
        block.push(UNSCOPE);
        block.extend(pack_u16(scopedepth as u16))
    }
    code.push(WHILE);
    code.extend(pack_u64(expr.len() as u64));
    code.extend(pack_u64(block.len() as u64));
    code.extend(expr);
    code.extend(block);
    Ok(())
}
fn compile_forcondition(ast : &ASTNode, code : &mut Vec<u8>, mut scopedepth : usize) -> Result<(), Option<String>>
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
    
    // FIXME: make this a subroutine lmao
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
    code.extend(pack_u64(expr.len() as u64));
    code.extend(pack_u64(post.len() as u64));
    code.extend(pack_u64(block.len() as u64));
    code.extend(expr);
    code.extend(post);
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
fn compile_expr(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), Option<String>>
{
    if ast.children.len() == 1
    {
        code.extend(compile_astnode(ast.child(0)?, scopedepth)?);
    }
    else
    {
        // panic!() fixme
        return plainerr("internal error: unhandled form of expression");
        //println!("internal error: unhandled form of expression");
        //print_ast(ast);
        //assert!(false);
    }
    Ok(())
}
fn compile_simplexpr(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), Option<String>>
{
    if ast.children.len() == 3 && !ast.child(0)?.isparent && !ast.child(2)?.isparent && ast.child(0)?.text == "(" && ast.child(2)?.text == ")"
    { 
        code.extend(compile_astnode(ast.child(1)?, scopedepth)?);
    }
    else
    {
        // panic!() fixme
        return plainerr("internal error: unhandled form of expression");
        //println!("internal error: unhandled form of expression");
        //print_ast(ast);
        //assert!(false);
    }
    Ok(())
}
fn compile_number(ast : &ASTNode, code : &mut Vec<u8>, _scopedepth : usize) -> Result<(), Option<String>>
{
    if ast.children.len() == 1
    {
        code.push(PUSHFLT);
        if let Ok(float) = ast.child(0)?.text.parse::<f64>()
        {
            code.extend(pack_f64(float));
        }
        else
        {
            // panic!() fixme
            return plainerr(&format!("internal error: text `{}` cannot be converted to a floating point number by rust", ast.child(0)?.text));
            //println!("internal error: text `{}` cannot be converted to a floating point number by rust", ast.child(0)?.text);
            //print_ast(ast);
            //assert!(false);
        }
    }
    else
    {
        // panic!() fixme
        return plainerr("internal error: unhandled form of expression");
        //println!("internal error: unhandled form of expression");
        //print_ast(ast);
        //assert!(false);
    }
    Ok(())
}
fn compile_string(ast : &ASTNode, code : &mut Vec<u8>, _scopedepth : usize) -> Result<(), Option<String>>
{
    if ast.children.len() == 1
    {
        code.push(PUSHSTR);
        let text = slice(&ast.child(0)?.text, 1, -1);
        // FIXME handle \ escapes in text
        code.extend(unescape(&text).bytes());
        code.push(0x00);
    }
    else
    {
        // panic!() fixme
        return plainerr("internal error: unhandled form of expression");
        //println!("internal error: unhandled form of expression");
        //print_ast(ast);
        //assert!(false);
    }
    Ok(())
}
fn compile_name(ast : &ASTNode, code : &mut Vec<u8>, _scopedepth : usize) -> Result<(), Option<String>>
{
    if ast.children.len() == 1
    {
        code.push(PUSHVAR);
        code.extend(ast.child(0)?.text.bytes());
        code.push(0x00);
    }
    else
    {
        // panic!() fixme
        return plainerr("internal error: unhandled form of expression");
        //println!("internal error: unhandled form of expression");
        //print_ast(ast);
        //assert!(false);
    }
    Ok(())
}

fn compile_lvar(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), Option<String>>
{
    if ast.children.len() == 1
    {
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
    }
    else
    {
        // panic!() fixme
        return plainerr("internal error: malformed lvar reference node");
        //println!("internal error: malformed lvar reference node");
        //print_ast(ast);
        //assert!(false);
    }
    Ok(())
}
fn compile_rvar(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), Option<String>>
{
    if ast.children.len() == 1
    {
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
    }
    else
    {
        // panic!() fixme
        return plainerr("internal error: malformed rvar reference node");
        //println!("internal error: malformed rvar reference node");
        //print_ast(ast);
        //assert!(false);
    }
    Ok(())
}
fn compile_funcdef(ast : &ASTNode, code : &mut Vec<u8>, _scopedepth : usize) -> Result<(), Option<String>>
{
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
    
    code.push(FUNCDEF);
    code.extend(name.bytes());
    code.push(0x00);
    code.extend(pack_u16(args.len() as u16));
    code.extend(pack_u64(body.len() as u64));
    code.extend(argbytes);
    code.extend(body);
    
    Ok(())
}
fn compile_lambda(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), Option<String>>
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
fn compile_objdef(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), Option<String>>
{
    let funcs = ast.child_slice(3, -1)?;
    let mut childcode = Vec::<u8>::new();
    for child in funcs.iter()
    {
        let code = compile_astnode(child, scopedepth)?;
        if let Some(without_first_byte) = code.get(1..) // cut off the FUNCDEF byte
        {
            childcode.extend(without_first_byte);
        }
        else
        {
            return plainerr("internal error: compile_astnode for child function of objdef somehow didn't have even a single byte of code");
        }
    }
    code.push(OBJDEF);
    code.extend(ast.child(1)?.child(0)?.text.bytes());
    code.push(0x00);
    code.extend(pack_u16(funcs.len() as u16));
    code.extend(childcode);
    
    Ok(())
}
fn compile_arraybody(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), Option<String>>
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
fn compile_dictbody(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), Option<String>>
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
fn compile_arrayexpr(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), Option<String>>
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
fn compile_indirection(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), Option<String>>
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
fn compile_lhunop(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize) -> Result<(), Option<String>>
{
    if ast.children.len() == 0
    {
        // panic!() fixme
        return plainerr("internal error: lhunop has no children");
        //println!("internal error: lhunop has no children");
        //print_ast(ast);
        //assert!(false);
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
        
        if let Some(op) = get_unop_type(slice(&operator, 0, 1).as_str())
        {
            code.push(op);
        }
        else
        {
            // panic!() fixme
            return plainerr("internal error: unhandled type of unary expression");
            //println!("internal error: unhandled type of unary expression");
            //print_ast(ast);
            //assert!(false);
        }
    }
    
    Ok(())
}
fn compile_astnode(ast : &ASTNode, scopedepth : usize) -> Result<Vec<u8>, Option<String>>
{
    if !ast.isparent
    {
        // panic!() fixme
        plainerr("error: tried to compile non-parent ast node")
        //println!("error: tried to compile non-parent ast node");
        //print_ast(ast);
        //assert!(false);
    }
    else
    {
        let mut code = Vec::<u8>::new();
        
        if ast.text.starts_with("binexpr_")
        {
            assert!(ast.children.len() == 3);
            code.extend(compile_astnode(ast.child(0)?, scopedepth)?);
            code.extend(compile_astnode(ast.child(2)?, scopedepth)?);
            code.push(BINOP);
            if let Some(op) = get_binop_type(ast.child(1)?.child(0)?.text.as_str())
            {
                code.push(op);
            }
            else
            {
                // panic!() fixme
                return plainerr("internal error: unhandled type of binary expression");
                //println!("internal error: unhandled type of binary expression");
                //print_ast(ast);
                //assert!(false);
            }
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
                {
                    compile_statement(ast, &mut code, scopedepth)?;
                }
                "declaration" =>
                {
                    compile_declaration(ast, &mut code, scopedepth)?;
                }
                "name" =>
                {
                    compile_name(ast, &mut code, scopedepth)?;
                }
                "funccall" | "funcexpr" =>
                {
                    compile_function(ast, &mut code, scopedepth)?;
                }
                "ifcondition" =>
                {
                    compile_ifcondition(ast, &mut code, scopedepth)?;
                }
                "whilecondition" =>
                {
                    compile_whilecondition(ast, &mut code, scopedepth)?;
                }
                "forcondition" =>
                {
                    compile_forcondition(ast, &mut code, scopedepth)?;
                }
                "expr" =>
                {
                    compile_expr(ast, &mut code, scopedepth)?;
                }
                "simplexpr" =>
                {
                    compile_simplexpr(ast, &mut code, scopedepth)?;
                }
                "number" =>
                {
                    compile_number(ast, &mut code, scopedepth)?;
                }
                "string" =>
                {
                    compile_string(ast, &mut code, scopedepth)?;
                }
                "lvar" =>
                {
                    compile_lvar(ast, &mut code, scopedepth)?;
                }
                "rvar" =>
                {
                    compile_rvar(ast, &mut code, scopedepth)?;
                }
                "funcdef" =>
                {
                    compile_funcdef(ast, &mut code, scopedepth)?;
                }
                "lambda" =>
                {
                    compile_lambda(ast, &mut code, scopedepth)?;
                }
                "objdef" =>
                {
                    compile_objdef(ast, &mut code, scopedepth)?;
                }
                "arraybody" =>
                {
                    compile_arraybody(ast, &mut code, scopedepth)?;
                }
                "dictbody" =>
                {
                    compile_dictbody(ast, &mut code, scopedepth)?;
                }
                "arrayexpr" =>
                {
                    compile_arrayexpr(ast, &mut code, scopedepth)?;
                }
                "indirection" =>
                {
                    compile_indirection(ast, &mut code, scopedepth)?;
                }
                "lhunop" =>
                {
                    compile_lhunop(ast, &mut code, scopedepth)?;
                }
                _ =>
                {
                    // panic!() fixme
                    return plainerr("internal error: unhandled ast node type in compiler");
                    //println!("internal error: unhandled ast node type in compiler");
                    //print_ast(ast);
                    //assert!(false);
                }
            }
        }
        Ok(code)
    }
}

pub fn compile_bytecode(ast : &ASTNode) -> Result<Vec<u8>, Option<String>>
{
    compile_astnode(ast, 0)
}
