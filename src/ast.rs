use super::grammar::*;

#[derive(Clone)]
pub (crate) struct GrammarPoint {
    pub (crate) name: String,
    pub (crate) forms: Vec<GrammarForm>,
    pub (crate) istoken: bool,
}

#[derive(Clone)]
pub struct LexToken {
    pub (crate) text: String,
    pub (crate) line: usize,
    pub (crate) position: usize,
}

#[derive(Clone)]
pub (crate) struct OpData {
    pub (crate) isop: bool,
    pub (crate) assoc: i32,
    pub (crate) precedence: i32
}

pub (crate) fn dummy_opdata() -> OpData
{
    OpData{isop: false, assoc: 0, precedence: 0}
}

#[derive(Clone)]
pub struct ASTNode {
    pub (crate) text: String,
    pub (crate) line: usize,
    pub (crate) position: usize,
    pub (crate) isparent: bool,
    pub (crate) children: Vec<ASTNode>,
    pub (crate) opdata: OpData,
}

pub (crate) fn dummy_astnode() -> ASTNode
{
    ASTNode{text: "".to_string(), line: 0, position: 0, isparent: false, children: Vec::new(), opdata: dummy_opdata()}
}

pub (crate) fn print_ast_node(ast : &ASTNode, depth : usize)
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

pub (crate) fn print_ast(ast : &ASTNode)
{
    print_ast_node(&ast, 0);
}
