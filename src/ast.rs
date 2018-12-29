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
    pub (crate) fn last_child<'a>(&'a self) -> Result<&'a ASTNode, Option<String>>
    {
        self.child(self.children.len()-1)
    }
    pub (crate) fn child<'a>(&'a self, n : usize) -> Result<&'a ASTNode, Option<String>>
    {
        if n >= self.children.len()
        {
            Err(Some(format!("internal error: tried to access child {} (zero-indexed) of ast node that only has {} children", n, self.children.len())))
        }
        else
        {
            Ok(&self.children[n])
        }
    }
    pub (crate) fn child_mut<'a>(&'a mut self, n : usize) -> Result<&'a mut ASTNode, Option<String>>
    {
        if n >= self.children.len()
        {
            Err(Some(format!("internal error: tried to access child {} (zero-indexed) of ast node that only has {} children", n, self.children.len())))
        }
        else
        {
            Ok(&mut self.children[n])
        }
    }
    pub (crate) fn child_slice<'a>(&'a self, start : isize, end : isize) -> Result<&'a[ASTNode], Option<String>>
    {
        let u_start = if start <  0 {self.children.len() - (-start as usize)} else {start as usize};
        let u_end   = if end   <= 0 {self.children.len() - (-end   as usize)} else {end   as usize};
        
        if u_start >= self.children.len() || u_end > self.children.len()
        {
            Err(Some(format!("internal error: tried to access child range {} to {} (zero-indexed) of ast node that only has {} children", u_start, u_end, self.children.len())))
        }
        else
        {
            Ok(&self.children[u_start..u_end])
        }
    }
}

pub (crate) fn dummy_astnode() -> ASTNode
{
    ASTNode{text: "".to_string(), line: 0, position: 0, isparent: false, children: Vec::new(), opdata: dummy_opdata()}
}

pub (crate) fn textualize_ast_node(ast : &ASTNode, depth : usize) -> Vec<String>
{
    let mut ret = Vec::new();
    let prefix = format!("{}", " ".repeat(depth));
    if ast.isparent
    {
        if ast.text == "name"
        {
            ret.push(format!("{}name({})", prefix, ast.children[0].text));
        }
        else if ast.text == "number"
        {
            ret.push(format!("{}number({})", prefix, ast.children[0].text));
        }
        else if ast.text == "string"
        {
            ret.push(format!("{}string({})", prefix, ast.children[0].text));
        }
        else
        {
            ret.push(format!("{}{} {} {}", prefix, ast.text, ast.line, ast.position));
            for child in &ast.children
            {
                ret.append(&mut textualize_ast_node(&child, depth+1));
            }
        }
    }
    else
    {
        ret.push(format!("{}{}", prefix, ast.text));
    }
    ret
}

pub (crate) fn textualize_ast(ast : &ASTNode) -> Vec<String>
{
    textualize_ast_node(&ast, 0)
}

pub (crate) fn print_ast_node(ast : &ASTNode, depth : usize)
{
    for line in textualize_ast_node(ast, depth)
    {
        println!("{}", line);
    }
}
pub (crate) fn print_ast(ast : &ASTNode)
{
    for line in textualize_ast(ast)
    {
        println!("{}", line);
    }
}
