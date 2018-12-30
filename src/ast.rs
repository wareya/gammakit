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

impl ASTNode {
    pub (crate) fn last_child(&'_ self) -> Result<&'_ ASTNode, Option<String>>
    {
        self.child(self.children.len()-1)
    }
    pub (crate) fn child(&'_ self, n : usize) -> Result<&'_ ASTNode, Option<String>>
    {
        self.children.get(n).ok_or_else(|| Some(format!("internal error: tried to access child {} (zero-indexed) of ast node that only has {} children", n, self.children.len())))
    }
    pub (crate) fn child_mut(&'_ mut self, n : usize) -> Result<&'_ mut ASTNode, Option<String>>
    {
        let len = self.children.len();
        self.children.get_mut(n).ok_or_else(|| Some(format!("internal error: tried to access child {} (zero-indexed) of ast node that only has {} children", n, len)))
    }
    pub (crate) fn child_slice(&'_ self, start : isize, end : isize) -> Result<&'_[ASTNode], Option<String>>
    {
        let u_start = if start <  0 {self.children.len() - (-start as usize)} else {start as usize};
        let u_end   = if end   <= 0 {self.children.len() - (-end   as usize)} else {end   as usize};
        
        self.children.get(u_start..u_end).ok_or_else(|| Some(format!("internal error: tried to access child range {} to {} (zero-indexed) of ast node that only has {} children", u_start, u_end, self.children.len())))
    }
}

pub (crate) fn dummy_astnode() -> ASTNode
{
    ASTNode{text: "".to_string(), line: 0, position: 0, isparent: false, children: Vec::new(), opdata: dummy_opdata()}
}

pub (crate) fn textualize_ast_node(ast : &ASTNode, depth : usize) -> Result<Vec<String>, Option<String>>
{
    let mut ret = Vec::new();
    let prefix = " ".repeat(depth).to_string();
    if ast.isparent
    {
        if matches!(ast.text.as_str(), "name" | "number" | "string")
        {
            ret.push(format!("{}{}({})", prefix, ast.text, ast.child(0)?.text));
        }
        else
        {
            ret.push(format!("{}{} {} {}", prefix, ast.text, ast.line, ast.position));
            for child in &ast.children
            {
                ret.append(&mut textualize_ast_node(&child, depth+1)?);
            }
        }
    }
    else
    {
        ret.push(format!("{}{}", prefix, ast.text));
    }
    Ok(ret)
}

pub (crate) fn textualize_ast(ast : &ASTNode) -> Result<Vec<String>, Option<String>>
{
    textualize_ast_node(&ast, 0)
}

pub (crate) fn print_ast_node(ast : &ASTNode, depth : usize) -> Result<(), Option<String>>
{
    println!("{}", textualize_ast_node(ast, depth)?.join("\n"));
    Ok(())
}
pub (crate) fn print_ast(ast : &ASTNode) -> Result<(), Option<String>>
{
    println!("{}", textualize_ast(ast)?.join("\n"));
    Ok(())
}
