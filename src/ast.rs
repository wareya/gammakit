use super::grammar::*;

#[derive(Clone)]
pub struct GrammarPoint {
    pub name: String,
    pub forms: Vec<GrammarForm>,
    pub istoken: bool,
}

#[derive(Clone)]
pub struct LexToken {
    pub text: String,
    pub line: usize,
    pub position: usize,
}

#[derive(Clone)]
pub struct OpData {
    pub isop: bool,
    pub assoc: i32,
    pub precedence: i32
}

pub fn dummy_opdata() -> OpData
{
    return OpData{isop: false, assoc: 0, precedence: 0};
}

#[derive(Clone)]
pub struct ASTNode {
    pub text: String,
    pub line: usize,
    pub position: usize,
    pub isparent: bool,
    pub children: Vec<ASTNode>,
    pub opdata: OpData,
}

pub fn dummy_astnode() -> ASTNode
{
    return ASTNode{text: "".to_string(), line: 0, position: 0, isparent: false, children: Vec::new(), opdata: dummy_opdata()};
}

pub fn print_ast_node(ast : &ASTNode, depth : usize)
{
    if depth > 0
    {
        print!("{}", " ".repeat(depth));
    }
    if ast.isparent
    {
        if ast.text == "name"
        {
            println!("name({})", ast.children[0].text);
            return;
        }
        else if ast.text == "number"
        {
            println!("number({})", ast.children[0].text);
            return;
        }
        else if ast.text == "string"
        {
            println!("string({})", ast.children[0].text);
            return;
        }
        else
        {
            println!("{} {} {}", ast.text, ast.line, ast.position);
            for child in &ast.children
            {
                print_ast_node(&child, depth+1);
            }
        }
    }
    else
    {
        println!("{}", ast.text);
    }
}

pub fn print_ast(ast : &ASTNode)
{
    print_ast_node(&ast, 0);
}
