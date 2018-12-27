#![allow(clippy::len_zero)]

use std::hint::unreachable_unchecked;

use super::strings::*;
use super::ast::*;
use super::bytecode::*;

pub fn compile_statement(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize)
{
    code.push(LINENUM);
    code.extend(pack_u64(ast.line as u64));
    if !ast.children[0].isparent && !ast.children.last().unwrap().isparent && ast.children[0].text == "{" && ast.children.last().unwrap().text == "}"
    {
        code.push(SCOPE);
        for child in &ast.children[1..ast.children.len()-1]
        {
            code.extend(compile_astnode(child, scopedepth+1));
        }
        code.push(UNSCOPE);
        code.extend(pack_u16(scopedepth as u16));
        if scopedepth >= 0xFFFF
        {
            panic!("error: internal scope depth limit of 0xFFFF reached; nest your code less.");
        }
    }
    else if ast.children.len() == 3 && ast.children[1].isparent && ast.children[1].text == "binstateop"
    {
        let operator = &ast.children[1].children[0].text;
        code.extend(compile_astnode(&ast.children[0], scopedepth));
        code.extend(compile_astnode(&ast.children[2], scopedepth));
        code.push(BINSTATE);
        if let Some(op) = get_assignment_type(operator)
        {
            code.push(op);
        }
        else
        {
            println!("internal error: unhandled or unsupported type of binary statement {}", operator);
            print_ast(ast);
            assert!(false);
        }
    }
    else if ast.children[0].isparent
    {
        if ast.children[0].text == "withstatement"
        {
            let ast = &ast.children[0];
            
            let expr = compile_astnode(&ast.children[1], scopedepth);
            let sentinel = &ast.children[2].children[0].children[0];
            
            code.extend(expr);
            code.push(WITH);
            
            if !sentinel.isparent && sentinel.text == "{"
            {
                let block = compile_astnode(&ast.children[2].children[0], scopedepth);
                code.extend(pack_u64(block.len() as u64));
                code.extend(block);
            }
            else
            {
                let mut block = Vec::<u8>::new();
                block.push(SCOPE);
                block.extend(compile_astnode(&ast.children[2].children[0], scopedepth+1));
                block.push(UNSCOPE);
                block.extend(pack_u16(scopedepth as u16));
                code.extend(pack_u64(block.len() as u64));
                code.extend(block);
            }   
        }
        else if match ast.children[0].text.as_str() {"declaration" | "funccall" | "funcexpr" | "funcdef" | "objdef" => true , _ => false}
        {
            code.extend(compile_astnode(&ast.children[0], scopedepth));
        }
        else if ast.children[0].text == "condition"
        {
            code.extend(compile_astnode(&ast.children[0].children[0], scopedepth));
        }
        else if ast.children[0].text == "instruction"
        {
            if ast.children[0].children[0].text == "break"
            {
                code.push(BREAK);
            }
            else if ast.children[0].children[0].text == "continue"
            {
                code.push(CONTINUE);
            }
            else if ast.children[0].children[0].text == "return"
            {
                if ast.children[0].children.len() == 2
                {
                    code.extend(compile_astnode(&ast.children[0].children[1], scopedepth));
                }
                else if ast.children[0].children.len() == 1
                {
                    code.push(PUSHFLT);
                    code.extend(pack_f64(0.0));
                }
                else
                {
                    println!("internal error: broken return instruction");
                    print_ast(ast);
                    assert!(false);
                }
                code.push(RETURN);
            }
            else
            {
                println!("internal error: unhandled type of instruction");
                print_ast(ast);
                assert!(false);
            }
        }
        else
        {
            println!("internal error: unhandled type of statement");
            print_ast(ast);
            assert!(false);
        }
    }
    else
    {
        println!("internal error: statement child is not itself a parent/named node");
        print_ast(ast);
        assert!(false);
    }
}

fn compile_declaration(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize)
{
    for child in ast.children[1..].iter()
    {
        let name = &child.children[0].children[0].text;
        code.push(PUSHNAME);
        code.extend(name.bytes());
        code.push(0x00);
        if ast.children[0].text == "var"
        {
            code.push(DECLVAR);
        }
        else if ast.children[0].text == "far"
        {
            code.push(DECLFAR);
        }
        else
        {
            panic!("internal error: non-var/far prefix to declaration");
        }
        if child.children.len() == 3
        {
            code.push(PUSHNAME);
            code.extend(name.bytes());
            code.push(0x00);
            code.extend(compile_astnode(&child.children[2], scopedepth));
            code.push(BINSTATE);
            code.push(0x00);
        }
    }
}

fn compile_function(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize)
{
    if ast.children[1].children.len() > 0
    {
        let children = &ast.children[1].children[0].children;
        if children.len() > 0xFFFF
        {
            panic!("internal error: more than 0xFFFF (around 65000) arguments to single function");
        }
        for child in children
        {
            //print_ast(child)
            code.extend(compile_astnode(child, scopedepth));
        }
        code.push(PUSHSHORT);
        code.extend(pack_u16(children.len() as u16))
    }
    else
    {
        code.push(PUSHSHORT);
        code.extend(pack_u16(0))
    }
    code.extend(compile_astnode(&ast.children[0], scopedepth));
    // code.push(0x00); // FIXME this was wrong
    if ast.text == "funccall"
    {
        code.push(FUNCCALL);
    }
    else
    {
        code.push(FUNCEXPR);
    }
}

fn compile_ifcondition(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize)
{
    let expr = compile_astnode(&ast.children[1], scopedepth);
    let sentinel = &ast.children[2].children[0].children[0];
    let mut block : Vec<u8>;
    if !sentinel.isparent && sentinel.text == "{"
    {
        block = compile_astnode(&ast.children[2].children[0], scopedepth);
    }
    else
    {
        block = Vec::<u8>::new();
        block.push(SCOPE);
        block.extend(compile_astnode(&ast.children[2].children[0], scopedepth+1));
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
    else if ast.children.len() == 5 && ast.children[3].text == "else"
    {
        let sentinel = &ast.children[4].children[0].children[0];
        let mut block2 : Vec<u8>;
        if !sentinel.isparent && sentinel.text == "{"
        {
            block2 = compile_astnode(&ast.children[4].children[0], scopedepth);
        }
        else
        {
            block2 = Vec::<u8>::new();
            block2.push(SCOPE);
            block2.extend(compile_astnode(&ast.children[4].children[0], scopedepth+1));
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
        println!("internal error: broken if condition");
        print_ast(ast);
        assert!(false);
    }
}
fn compile_whilecondition(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize)
{
    let expr = compile_astnode(&ast.children[1], scopedepth);
    // FIXME: make this a subroutine lmao
    let sentinel = &ast.children[2].children[0].children[0];
    let mut block : Vec<u8>;
    if !sentinel.isparent && sentinel.text == "{"
    {
        block = compile_astnode(&ast.children[2].children[0], scopedepth);
    }
    else
    {
        block = Vec::<u8>::new();
        block.push(SCOPE);
        block.extend(compile_astnode(&ast.children[2].children[0], scopedepth+1));
        block.push(UNSCOPE);
        block.extend(pack_u16(scopedepth as u16))
    }
    code.push(WHILE);
    code.extend(pack_u64(expr.len() as u64));
    code.extend(pack_u64(block.len() as u64));
    code.extend(expr);
    code.extend(block);
}
fn compile_forcondition(ast : &ASTNode, code : &mut Vec<u8>, mut scopedepth : usize)
{
    let mut header_nodes : Vec<Option<&ASTNode>> = vec!(None, None, None);
    let mut header_index = 0;
    for node in ast.children[2..ast.children.len()-2].iter()
    {
        if node.isparent
        {
            header_nodes[header_index] = Some(&node);
        }
        else if !node.isparent && node.text == ";"
        {
            header_index += 1;
        }
    }
    
    // FOR loops need an extra layer of scope around them if they have an init statement
    if let Some(ref init) = header_nodes[0]
    {
        code.push(SCOPE);
        scopedepth += 1;
        code.extend(compile_astnode(&init, scopedepth));
    }
    
    // FIXME: expr needs to just test true if it's zero length
    let expr = if let Some(ref expr) = header_nodes[1] {compile_astnode(&expr, scopedepth)} else {Vec::<u8>::new()};
    
    let mut block : Vec<u8>;
    let post : Vec<u8>;
    
    // FIXME: make this a subroutine lmao
    let sentinel = &ast.children.last().unwrap().children[0].children[0];
    if !sentinel.isparent && sentinel.text == "{"
    {
        block = compile_astnode(&ast.children.last().unwrap().children[0], scopedepth);
        post = if let Some(ref body) = header_nodes[2] {compile_astnode(&body, scopedepth)} else {Vec::<u8>::new()};
    }
    else
    {
        block = Vec::<u8>::new();
        block.push(SCOPE);
        block.extend(compile_astnode(&ast.children.last().unwrap().children[0], scopedepth+1));
        post = if let Some(ref body) = header_nodes[2] {compile_astnode(&body, scopedepth+1)} else {Vec::<u8>::new()};
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
    if let Some(ref _init) = header_nodes[0]
    {
        scopedepth -= 1;
        code.push(UNSCOPE);
        code.extend(pack_u16(scopedepth as u16));
    }
}
fn compile_expr(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize)
{
    if ast.children.len() == 1
    {
        code.extend(compile_astnode(&ast.children[0], scopedepth));
    }
    else
    {
        println!("internal error: unhandled form of expression");
        print_ast(ast);
        assert!(false);
    }
}
fn compile_simplexpr(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize)
{
    if ast.children.len() == 3 && !ast.children[0].isparent && !ast.children[2].isparent && ast.children[0].text == "(" && ast.children[2].text == ")"
    { 
        code.extend(compile_astnode(&ast.children[1], scopedepth));
    }
    else
    {
        println!("internal error: unhandled form of expression");
        print_ast(ast);
        assert!(false);
    }
}
fn compile_number(ast : &ASTNode, code : &mut Vec<u8>, _scopedepth : usize)
{
    if ast.children.len() == 1
    {
        code.push(PUSHFLT);
        if let Ok(float) = ast.children[0].text.parse::<f64>()
        {
            code.extend(pack_f64(float));
        }
        else
        {
            println!("internal error: text `{}` cannot be converted to a floating point number by rust", ast.children[0].text);
            print_ast(ast);
            assert!(false);
        }
    }
    else
    {
        println!("internal error: unhandled form of expression");
        print_ast(ast);
        assert!(false);
    }
}
fn compile_string(ast : &ASTNode, code : &mut Vec<u8>, _scopedepth : usize)
{
    if ast.children.len() == 1
    {
        code.push(PUSHSTR);
        let text = slice(&ast.children[0].text, 1, -1);
        // FIXME handle \ escapes in text
        code.extend(unescape(&text).bytes());
        code.push(0x00);
    }
    else
    {
        println!("internal error: unhandled form of expression");
        print_ast(ast);
        assert!(false);
    }
}
fn compile_name(ast : &ASTNode, code : &mut Vec<u8>, _scopedepth : usize)
{
    if ast.children.len() == 1
    {
        code.push(PUSHVAR);
        code.extend(ast.children[0].text.bytes());
        code.push(0x00);
    }
    else
    {
        println!("internal error: unhandled form of expression");
        print_ast(ast);
        assert!(false);
    }
}

fn compile_lvar(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize)
{
    if ast.children.len() == 1
    {
        if ast.children[0].text == "name"
        {
            code.push(PUSHNAME);
            code.extend(ast.children[0].children[0].text.bytes());
            code.push(0x00);
        }
        else
        {
            code.extend(compile_astnode(&ast.children[0], scopedepth))
        }
    }
    else
    {
        println!("internal error: malformed lvar reference node");
        print_ast(ast);
        assert!(false);
    }
}
fn compile_rvar(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize)
{
    if ast.children.len() == 1
    {
        if ast.children[0].text == "name"
        {
            code.push(PUSHVAR);
            code.extend(ast.children[0].children[0].text.bytes());
            code.push(0x00);
        }
        else
        {
            code.extend(compile_astnode(&ast.children[0], scopedepth));;
            if ast.children[0].isparent && match ast.children[0].text.as_str() { "indirection" | "arrayexpr" => true, _ => false }
            {
                code.push(EVALUATION);
            }
        }
    }
    else
    {
        println!("internal error: malformed rvar reference node");
        print_ast(ast);
        assert!(false);
    }
}
fn compile_funcdef(ast : &ASTNode, code : &mut Vec<u8>, _scopedepth : usize)
{
    let name = &ast.children[1].children[0].text;
    
    let mut args = Vec::<&ASTNode>::new();
    for child in ast.children[3..].iter()
    {
        if !child.isparent && child.text == ")"
        {
            break;
        }
        args.push(&child);
    }
    
    let mut statements = Vec::<&ASTNode>::new();
    for child in ast.children[5+args.len()..].iter()
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
        argbytes.extend(arg.children[0].text.bytes());
        argbytes.push(0x00);
    }
    
    let mut body = Vec::<u8>::new();
    for statement in &statements
    {
        body.extend(compile_astnode(&statement, 0))
    }
    body.push(EXIT);
    
    code.push(FUNCDEF);
    code.extend(name.bytes());
    code.push(0x00);
    code.extend(pack_u16(args.len() as u16));
    code.extend(pack_u64(body.len() as u64));
    code.extend(argbytes);
    code.extend(body);
}
fn compile_lambda(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize)
{
    let mut  captures = Vec::<&ASTNode>::new();
    for child in ast.children[0].children[1..ast.children[0].children.len()-1].iter()
    {
        captures.push(&child);
    }
    
    let mut args = Vec::<&ASTNode>::new();
    for child in ast.children[1].children[1..ast.children[1].children.len()-1].iter()
    {
        args.push(&child);
    }
    
    let mut  statements = Vec::<&ASTNode>::new();
    for child in ast.children[2].children[1..ast.children[2].children.len()-1].iter()
    {
        statements.push(&child);
    }
                   
    let mut argbytes = Vec::<u8>::new();
    for arg in &args
    {
        argbytes.extend(arg.children[0].text.bytes());
        argbytes.push(0x00);
    }
    
    let mut body = Vec::<u8>::new();
    for statement in &statements
    {
        body.extend(compile_astnode(statement, 0))
    }
            
    body.push(EXIT);
    
    let mut capturebytes = Vec::<u8>::new();
    for capture in &captures
    {
        capturebytes.push(PUSHSTR);
        capturebytes.extend(capture.children[0].children[0].text.bytes());
        capturebytes.push(0x00);
        capturebytes.extend(compile_astnode(&capture.children[2], scopedepth));
    }
    
    code.extend(capturebytes);
    code.push(LAMBDA);
    code.extend(pack_u16(captures.len() as u16));
    code.extend(pack_u16(args.len() as u16));
    code.extend(pack_u64(body.len() as u64));
    code.extend(argbytes);
    code.extend(body);
}
fn compile_objdef(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize)
{
    let funcs = &ast.children[3..ast.children.len()-1];
    let mut childcode = Vec::<u8>::new();
    for child in funcs.iter()
    {
        childcode.extend(compile_astnode(&child, scopedepth)[1..].iter()) // cut off the FUNCDEF byte
    }
    code.push(OBJDEF);
    code.extend(ast.children[1].children[0].text.bytes());
    code.push(0x00);
    code.extend(pack_u16(funcs.len() as u16));
    code.extend(childcode);
}
fn compile_arraybody(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize)
{
    let mut elementcount = 0;
    let mut childexprs = Vec::<u8>::new();
    for expression in ast.children[1..ast.children.len()-1].iter()
    {
        if expression.text == "unusedcomma"
        {
            break
        }
        childexprs.extend(compile_astnode(&expression, scopedepth));
        elementcount += 1;
    }
    code.extend(childexprs);
    code.push(COLLECTARRAY);
    code.extend(pack_u16(elementcount as u16));
}
fn compile_dictbody(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize)
{
    let mut elementcount = 0;
    let mut childexprs = Vec::<u8>::new();
    for expression in ast.children[1..ast.children.len()-1].iter()
    {
        if expression.text == "unusedcomma"
        {
            break;
        }
        childexprs.extend(compile_astnode(&expression.children[0], scopedepth));
        childexprs.extend(compile_astnode(&expression.children[2], scopedepth));
        elementcount += 1;
    }
    code.extend(childexprs);
    code.push(COLLECTDICT);
    code.extend(pack_u16(elementcount as u16));
}
fn compile_arrayexpr(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize)
{
    if ast.children[0].isparent && ast.children[0].text == "name"
    {
        code.push(PUSHNAME);
        code.extend(ast.children[0].children[0].text.bytes());
        code.push(0x00);
    }
    else
    {
        code.extend(compile_astnode(&ast.children[0], scopedepth));
    }
    code.extend(compile_astnode(&ast.children[1].children[1], scopedepth));
    code.push(ARRAYEXPR);
}
fn compile_indirection(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize)
{
    code.extend(compile_astnode(&ast.children[0], scopedepth));
    if ast.children[0].text == "indirection"
    {
        code.push(EVALUATION);
    }
    code.push(PUSHNAME);
    code.extend(ast.children[1].children[0].text.bytes());
    code.push(0x00);
    code.push(INDIRECTION);
}
fn compile_lhunop(ast : &ASTNode, code : &mut Vec<u8>, scopedepth : usize)
{
    if ast.children.len() == 0
    {
        println!("internal error: lhunop has no children");
        print_ast(ast);
        assert!(false);
    }
    else if ast.children.len() == 1
    {
        code.extend(compile_astnode(&ast.children[0], scopedepth));
    }
    else
    {
        let operator = &ast.children[0].children[0].text;
        
        code.extend(compile_astnode(&ast.children[1], scopedepth));
        code.push(UNOP);
        
        if let Some(op) = get_unop_type(slice(&operator, 0, 1).as_str())
        {
            code.push(op);
        }
        else
        {
            println!("internal error: unhandled type of unary expression");
            print_ast(ast);
            assert!(false);
        }
    }
}
pub fn compile_astnode(ast : &ASTNode, scopedepth : usize) -> Vec<u8>
{
    if !ast.isparent
    {
        println!("error: tried to compile non-parent ast node");
        print_ast(ast);
        assert!(false);
        unsafe { unreachable_unchecked() };
    }
    else
    {
        let mut code = Vec::<u8>::new();
        
        if ast.text.starts_with("binexpr_")
        {
            assert!(ast.children.len() == 3);
            code.extend(compile_astnode(&ast.children[0], scopedepth));
            code.extend(compile_astnode(&ast.children[2], scopedepth));
            code.push(BINOP);
            if let Some(op) = get_binop_type(ast.children[1].children[0].text.as_str())
            {
                code.push(op);
            }
            else
            {
                println!("internal error: unhandled type of binary expression");
                print_ast(ast);
                assert!(false);
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
                        code.extend(compile_astnode(&child, scopedepth));
                    }
                    code.push(EXIT);
                }
                "statement" | "barestatement" =>
                {
                    compile_statement(ast, &mut code, scopedepth);
                }
                "declaration" =>
                {
                    compile_declaration(ast, &mut code, scopedepth);
                }
                "name" =>
                {
                    compile_name(ast, &mut code, scopedepth);
                }
                "funccall" | "funcexpr" =>
                {
                    compile_function(ast, &mut code, scopedepth);
                }
                "ifcondition" =>
                {
                    compile_ifcondition(ast, &mut code, scopedepth);
                }
                "whilecondition" =>
                {
                    compile_whilecondition(ast, &mut code, scopedepth);
                }
                "forcondition" =>
                {
                    compile_forcondition(ast, &mut code, scopedepth);
                }
                "expr" =>
                {
                    compile_expr(ast, &mut code, scopedepth);
                }
                "simplexpr" =>
                {
                    compile_simplexpr(ast, &mut code, scopedepth);
                }
                "number" =>
                {
                    compile_number(ast, &mut code, scopedepth);
                }
                "string" =>
                {
                    compile_string(ast, &mut code, scopedepth);
                }
                "lvar" =>
                {
                    compile_lvar(ast, &mut code, scopedepth);
                }
                "rvar" =>
                {
                    compile_rvar(ast, &mut code, scopedepth);
                }
                "funcdef" =>
                {
                    compile_funcdef(ast, &mut code, scopedepth);
                }
                "lambda" =>
                {
                    compile_lambda(ast, &mut code, scopedepth);
                }
                "objdef" =>
                {
                    compile_objdef(ast, &mut code, scopedepth);
                }
                "arraybody" =>
                {
                    compile_arraybody(ast, &mut code, scopedepth);
                }
                "dictbody" =>
                {
                    compile_dictbody(ast, &mut code, scopedepth);
                }
                "arrayexpr" =>
                {
                    compile_arrayexpr(ast, &mut code, scopedepth);
                }
                "indirection" =>
                {
                    compile_indirection(ast, &mut code, scopedepth);
                }
                "lhunop" =>
                {
                    compile_lhunop(ast, &mut code, scopedepth);
                }
                _ =>
                {
                    println!("internal error: unhandled ast node type in compiler");
                    print_ast(ast);
                    assert!(false);
                }
            }
        }
        code
    }
}

pub fn compile_bytecode(ast : &ASTNode) -> Vec<u8>
{
    compile_astnode(ast, 0)
}
