#![allow(clippy::len_zero)]

use super::{strings::*, ast::*, bytecode::*, bookkeeping::*};
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Debug)]
pub (crate) struct DebugInfo
{
    pub (crate) last_line : usize,
    pub (crate) last_index : usize,
    pub (crate) last_type : String,
}

#[derive(Debug)]
pub struct Code
{
    pub (crate) code : Rc<Vec<u8>>,
    pub (crate) debug : Rc<HashMap<usize, DebugInfo>>,
    pub (crate) bookkeeping : Bookkeeping,
}

impl std::clone::Clone for Code
{
    fn clone(&self) -> Code
    {
        Code{code : Rc::clone(&self.code), debug : Rc::clone(&self.debug), bookkeeping : self.bookkeeping.refclone()}
    }
}

impl std::cmp::PartialEq for Code
{
    fn eq(&self, other : &Code) -> bool
    {
        self.code == other.code
    }
}
impl Eq for Code {}

impl Code
{
    pub (crate) fn new() -> Code
    {
        Code{code : Rc::new(Vec::new()), debug : Rc::new(HashMap::new()), bookkeeping : Bookkeeping::new()}
    }
    pub (crate) fn new_share_bookkeeping(other : &Code) -> Code
    {
        Code{code : Rc::new(Vec::new()), debug : Rc::new(HashMap::new()), bookkeeping : other.bookkeeping.refclone()}
    }
    fn compile_raw_string(&mut self, text : &str)
    {
        let code = Rc::get_mut(&mut self.code).unwrap();
        code.extend(text.bytes());
        code.push(0x00);
    }
    fn extend<I : IntoIterator<Item = u8>>(&mut self, iter : I)
    {
        Rc::get_mut(&mut self.code).unwrap().extend(iter);
    }
    fn push(&mut self, byte : u8)
    {
        Rc::get_mut(&mut self.code).unwrap().push(byte);
    }
    pub (crate) fn get_string_index(&self, string : &String) -> usize
    {
        self.bookkeeping.get_string_index(string)
    }
    pub (crate) fn get_string(&self, index : usize) -> String
    {
        self.bookkeeping.get_string(index)
    }
    pub (crate) fn len(&self) -> usize
    {
        self.code.len()
    }
    pub (crate) fn get<I : std::slice::SliceIndex<[u8]>>(&self, index : I) -> Option<&I::Output>
    {
        self.code.get(index)
    }
    fn add_debug_info(&mut self, pc : usize, last_line : usize, last_index : usize, last_type : &str)
    {
        Rc::get_mut(&mut self.debug).unwrap().insert(pc, DebugInfo{last_line, last_index, last_type : last_type.to_string()});
    }
    pub (crate) fn get_debug_info(&self, pc : usize) -> Option<&DebugInfo>
    {
        self.debug.get(&pc)
    }
}

impl<I: std::slice::SliceIndex<[u8]>> std::ops::Index<I> for Code
{
    type Output = I::Output;
    fn index(&self, index: I) -> &Self::Output
    {
        &self.code[index]
    }
}

impl<I: std::slice::SliceIndex<[u8]>> std::ops::IndexMut<I> for Code
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output
    {
        &mut Rc::get_mut(&mut self.code).unwrap()[index]
    }
}

type CompilerBinding = Fn(&mut CompilerState, &ASTNode) -> Result<(), String>;

fn minierr(mystr : &str) -> String
{
    mystr.to_string()
}

fn plainerr<T>(mystr : &str) -> Result<T, String>
{
    Err(mystr.to_string())
}

#[derive(Debug, Clone, Copy)]
enum Context {
    Unknown,
    Statement,
    Expr,
    Objdef,
}

struct CompilerState {
    code : Code,
    hooks : HashMap<String, Rc<RefCell<CompilerBinding>>>,
    scopedepth : usize,
    context : Context,
    last_line : usize,
    last_index : usize,
    last_type : String,
}

impl CompilerState {
    fn new() -> CompilerState
    {
        let mut ret = CompilerState{hooks: HashMap::new(), code: Code::new(), scopedepth: 0, context: Context::Unknown, last_line: 0, last_index: 0, last_type: "".to_string()};
        ret.insert_default_hooks();
        ret
    }
    fn new_share_bookkeeping(code : &Code) -> CompilerState
    {
        let mut ret = CompilerState{hooks: HashMap::new(), code: Code::new_share_bookkeeping(code), scopedepth: 0, context: Context::Unknown, last_line: 0, last_index: 0, last_type: "".to_string()};
        ret.insert_default_hooks();
        ret
    }
    
    fn add_hook<T : ToString>(&mut self, name: &T, fun : &'static CompilerBinding)
    {
        self.hooks.insert(name.to_string(), Rc::new(RefCell::new(fun)));
    }
    fn insert_default_hooks(&mut self)
    {
        self.add_hook(&"program", &CompilerState::compile_program);
        self.add_hook(&"blankstatement", &CompilerState::compile_nop);
        self.add_hook(&"statement", &CompilerState::compile_statement);
        self.add_hook(&"funccall", &CompilerState::compile_funccall);
        self.add_hook(&"name", &CompilerState::compile_name);
        self.add_hook(&"rhunexpr_right", &CompilerState::compile_children);
        self.add_hook(&"funcargs", &CompilerState::compile_funcargs);
        self.add_hook(&"expr", &CompilerState::compile_children);
        self.add_hook(&"lhunop", &CompilerState::compile_children);
        self.add_hook(&"simplexpr", &CompilerState::compile_children);
        self.add_hook(&"supersimplexpr", &CompilerState::compile_children);
        self.add_hook(&"string", &CompilerState::compile_push_string);
        self.add_hook(&"condition", &CompilerState::compile_children);
        self.add_hook(&"barestatement", &CompilerState::compile_children);
        self.add_hook(&"block", &CompilerState::compile_block);
        self.add_hook(&"nakedblock", &CompilerState::compile_nakedblock);
        self.add_hook(&"whilecondition", &CompilerState::compile_whilecondition);
        self.add_hook(&"parenexpr", &CompilerState::compile_parenexpr);
        self.add_hook(&"number", &CompilerState::compile_number);
        self.add_hook(&"statementlist", &CompilerState::compile_statementlist);
        self.add_hook(&"instruction", &CompilerState::compile_instruction);
        self.add_hook(&"objdef", &CompilerState::compile_objdef);
        self.add_hook(&"funcdef", &CompilerState::compile_funcdef);
        self.add_hook(&"withstatement", &CompilerState::compile_with);
        self.add_hook(&"declaration", &CompilerState::compile_declaration);
        self.add_hook(&"binstate", &CompilerState::compile_binstate);
        self.add_hook(&"unstate", &CompilerState::compile_unstate);
        self.add_hook(&"lvar", &CompilerState::compile_lvar);
        self.add_hook(&"rvar", &CompilerState::compile_rvar);
        self.add_hook(&"rhunexpr", &CompilerState::compile_rhunexpr);
        self.add_hook(&"unary", &CompilerState::compile_unary);
        self.add_hook(&"indirection", &CompilerState::compile_indirection);
        self.add_hook(&"dictindex", &CompilerState::compile_dictindex);
        self.add_hook(&"binexpr_0", &CompilerState::compile_binexpr);
        self.add_hook(&"binexpr_1", &CompilerState::compile_binexpr);
        self.add_hook(&"binexpr_2", &CompilerState::compile_binexpr);
        self.add_hook(&"binexpr_3", &CompilerState::compile_binexpr);
        self.add_hook(&"lambda", &CompilerState::compile_lambda);
        self.add_hook(&"arraybody", &CompilerState::compile_arraybody);
        self.add_hook(&"arrayindex", &CompilerState::compile_arrayindex);
        self.add_hook(&"ifcondition", &CompilerState::compile_ifcondition);
        self.add_hook(&"dismember", &CompilerState::compile_dismember);
        self.add_hook(&"dictbody", &CompilerState::compile_dictbody);
        self.add_hook(&"forcondition", &CompilerState::compile_forcondition);
        self.add_hook(&"forheaderstatement", &CompilerState::compile_children);
        self.add_hook(&"forheaderexpr", &CompilerState::compile_children);
        self.add_hook(&"invocation_expr", &CompilerState::compile_invocation_expr);
        self.add_hook(&"setbody", &CompilerState::compile_setbody);
        self.add_hook(&"foreach", &CompilerState::compile_foreach);
        self.add_hook(&"switch", &CompilerState::compile_switch);
        self.add_hook(&"ternary", &CompilerState::compile_ternary);
    }
    
    fn compile_push_float(&mut self, float : f64)
    {
        self.code.push(PUSHFLT);
        self.code.extend(pack_f64(float));
    }
    fn compile_raw_string(&mut self, text : &str)
    {
        self.code.compile_raw_string(text);
    }
    fn compile_string_with_prefix(&mut self, prefix : u8, text : &str)
    {
        self.code.push(prefix);
        self.compile_raw_string(text);
    }
    fn compile_string_index(&mut self, text : &String)
    {
        let myu64 = pack_u64(self.code.get_string_index(text) as u64);
        self.code.extend(myu64);
    }
    fn compile_string_index_with_prefix(&mut self, prefix : u8, text : &String)
    {
        self.code.push(prefix);
        self.compile_string_index(text);
    }

    fn compile_unscope(&mut self) -> Result<(), String>
    {
        if self.scopedepth >= 0xFFFF
        {
            return plainerr("error: internal scope depth limit of 0xFFFF reached; nest your code less.");
        }
        self.code.push(UNSCOPE);
        self.code.extend(pack_u16(self.scopedepth as u16));
        Ok(())
    }
    fn compile_context_wrapped(&mut self, context : Context, fun : &Fn(&mut CompilerState) -> Result<(), String>) -> Result<(), String>
    {
        let old_context = self.context;
        self.context = context;
        
        fun(self)?;
        
        self.context = old_context;
        
        Ok(())
    }
    fn compile_scope_wrapped(&mut self, fun : &Fn(&mut CompilerState) -> Result<(), String>) -> Result<(), String>
    {
        self.code.push(SCOPE);
        self.scopedepth += 1;
        
        fun(self)?;
        
        self.scopedepth -= 1;
        self.compile_unscope()
    }
    fn compile_any(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.last_line = ast.line;
        self.last_index = ast.position;
        self.last_type = ast.text.clone();
        
        self.code.add_debug_info(self.code.len(), self.last_line, self.last_index, &self.last_type);
        
        let hook = Rc::clone(self.hooks.get(&ast.text).ok_or_else(|| minierr(&format!("internal error: no handler for AST node with name `{}`", ast.text)))?);
        let hook = hook.try_borrow().or_else(|_| Err(format!("internal error: hook for AST node type `{}` is already in use", ast.text)))?;
        hook(self, ast)
    }
    fn compile_nop(&mut self, _ast : &ASTNode) -> Result<(), String>
    {
        Ok(())
    }
    fn compile_program(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_children(ast)?;
        self.code.push(EXIT);
        Ok(())
    }
    fn compile_children(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        for child in &ast.children
        {
            self.compile_any(&child)?;
        }
        Ok(())
    }
    fn compile_nth_child(&mut self, ast : &ASTNode, n : usize) -> Result<(), String>
    {
        self.compile_any(ast.child(n)?)
    }
    fn compile_last_child(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_any(ast.last_child()?)
    }
    fn compile_statement(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_nth_child(ast, 0)
    }
    
    fn compile_rhunexpr(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        if ast.child(0)?.text == "name" && ast.child(0)?.child(0)?.text == "global" && ast.child(1)?.child(0)?.text == "indirection"
        {
            if matches!(self.context, Context::Expr)
            {
                self.compile_pushglobalval(&ast.child(1)?.child(0)?.child(1)?.child(0)?.text)?;
            }
            else
            {
                self.compile_pushglobal(&ast.child(1)?.child(0)?.child(1)?.child(0)?.text)?;
            }
            if ast.children.len() > 2
            {
                for child in ast.child_slice(2, -1)?
                {
                    //eprintln!("{:?}", child);
                    if child.text == "name"
                    {
                        self.compile_pushname(&child.child(0)?.text)?;
                    }
                    else
                    {
                        self.compile_any(child)?;
                    }
                }
                self.compile_last_child(ast)?;
            }
            Ok(())
        }
        else
        {
            for child in ast.child_slice(0, -1)?
            {
                //eprintln!("{:?}", child);
                if child.text == "name"
                {
                    self.compile_pushname(&child.child(0)?.text)?;
                }
                else
                {
                    self.compile_any(child)?;
                }
            }
            self.compile_last_child(ast)
        }
    }
    fn compile_funccall(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        if ast.last_child()?.child(0)?.text.as_str() != "funcargs"
        {
            return match ast.last_child()?.child(0)?.text.as_str()
            {
                "dismember" => plainerr("error: tried to use a -> expression as a statement"),
                "arrayindex" => plainerr("error: tried to use a [] expression as a statement"),
                "indirection" => plainerr("error: tried to use a . expression as a statement"),
                _ => plainerr("error: tried to use an unknown form of expression as a statement")
            };
        }
        
        for child in ast.child_slice(0, -1)?
        {
            if child.text == "name"
            {
                self.compile_pushname(&child.child(0)?.text)?;
            }
            else
            {
                self.compile_any(child)?;
            }
        }
        self.compile_context_wrapped(Context::Statement, &|x| x.compile_last_child(ast))
    }
    fn compile_pushvar(&mut self, string : &String) -> Result<(), String>
    {
        self.compile_string_index_with_prefix(PUSHVAR, string);
        Ok(())
    }
    fn compile_pushname(&mut self, string : &String) -> Result<(), String>
    {
        self.compile_string_index_with_prefix(PUSHNAME, string);
        Ok(())
    }
    fn compile_pushglobal(&mut self, string : &String) -> Result<(), String>
    {
        self.compile_string_index_with_prefix(PUSHGLOBAL, string);
        Ok(())
    }
    fn compile_pushglobalval(&mut self, string : &String) -> Result<(), String>
    {
        self.compile_string_index_with_prefix(PUSHGLOBALVAL, string);
        Ok(())
    }
    fn compile_pushstr(&mut self, string : &String) -> Result<(), String>
    {
        self.compile_string_with_prefix(PUSHSTR, string);
        Ok(())
    }
    fn compile_name(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_pushvar(&ast.child(0)?.text)?;
        Ok(())
    }
    fn compile_funcargs(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_context_wrapped(Context::Unknown, &|x|
        {
            let args = &ast.child(1)?.children;
            if args.len() > 0xFFFF
            {
                return plainerr("internal error: more than 0xFFFF (around 65000) arguments to single function");
            }
            for child in args
            {
                x.compile_any(child)?;
            }
            x.code.push(PUSHSHORT);
            x.code.extend(pack_u16(args.len() as u16));
            Ok(())
        })?;
        match self.context
        {
            Context::Statement => self.code.push(FUNCCALL),
            _ => self.code.push(FUNCEXPR)
        }
        
        Ok(())
    }
    fn rewrite_code(&mut self, location : usize, subcode : Vec<u8>) -> Result<(), String>
    {
        if self.code.len() < location+subcode.len()
        {
            return plainerr("internal error: tried to rewrite code past end of code");
        }
        for (i, c) in self.code[location..location+subcode.len()].iter_mut().enumerate()
        {
            *c = subcode[i];
        }
        Ok(())
    }
    fn compile_binexpr(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        if ast.children.len() != 3
        {
            return plainerr("error: binexpr_ nodes must have exactly three children");
        }
        self.compile_nth_child(ast, 0)?;
        let op = get_binop_type(ast.child(1)?.child(0)?.text.as_str()).ok_or_else(|| minierr("internal error: unhandled type of binary expression"))?;
        
        let mut rewrite_location_jumplen = 0;
        if op == 0x10 // and
        {
            self.code.push(SHORTCIRCUITIFFALSE);
            rewrite_location_jumplen = self.code.len();
            self.code.extend(pack_u64(0));
        }
        else if op == 0x11 // or
        {
            self.code.push(SHORTCIRCUITIFTRUE);
            rewrite_location_jumplen = self.code.len();
            self.code.extend(pack_u64(0));
        }
        
        let position_1 = self.code.len();
        
        self.compile_nth_child(ast, 2)?;
        self.code.push(BINOP);
        self.code.push(op);
        
        let position_2 = self.code.len();
        let jump_distance = position_2 - position_1;
        
        if rewrite_location_jumplen > 0
        {
            self.rewrite_code(rewrite_location_jumplen, pack_u64(jump_distance as u64))?;
        }
        Ok(())
    }
    fn compile_push_string(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_pushstr(&unescape(&slice(&ast.child(0)?.text, 1, -1)))?;
        Ok(())
    }
    fn compile_whilecondition(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.code.push(WHILE);
        let rewrite_location_exprlen = self.code.len();
        self.code.extend(pack_u64(0));
        let rewrite_location_codelen = self.code.len();
        self.code.extend(pack_u64(0));
        
        let point_1 = self.code.len();
        
        self.compile_nth_child(ast, 1)?;
        self.code.push(WHILETEST);
        
        let point_2 = self.code.len();
        
        self.compile_nth_child(ast, 2)?;
        self.code.push(WHILELOOP);
        
        let point_3 = self.code.len();
        
        let expr_len = point_2 - point_1;
        let code_len = point_3 - point_2;
        
        self.rewrite_code(rewrite_location_exprlen, pack_u64(expr_len as u64))?;
        self.rewrite_code(rewrite_location_codelen, pack_u64(code_len as u64))?;
        
        Ok(())
    }
    fn compile_parenexpr(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_nth_child(ast, 1)
    }
    fn compile_number(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        if ast.children.len() != 1
        {
            return plainerr("internal error: unhandled form of number");
        }
        self.code.push(PUSHFLT);
        let float = match ast.child(0)?.text.as_str()
        {
            "true" => 1.0,
            "false" => 0.0,
            _ => ast.child(0)?.text.parse::<f64>().or_else(|_| Err(format!("internal error: text `{}` cannot be converted to a floating point number by rust", ast.child(0)?.text)))?
        };
        self.code.extend(pack_f64(float));
        Ok(())
    }
    fn compile_statementlist(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        if ast.children.len() < 2
        {
            return plainerr("internal error: unhandled form of statement list");
        }
        self.compile_scope_wrapped(&|x| 
        {
            for child in ast.child_slice(1, -1)?
            {
                x.compile_any(child)?;
            }
            Ok(())
        })
    }
    fn compile_block(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        let sentinel = &ast.child(0)?.child(0)?;
        if sentinel.isparent && sentinel.text == "statementlist"
        {
            self.compile_nth_child(ast, 0)?;
        }
        else
        {
            self.compile_scope_wrapped(&|x| x.compile_nth_child(ast, 0))?;
        }
        Ok(())
    }

    fn compile_nakedblock(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_scope_wrapped(&|x|
        {
            for child in &ast.children
            {
                x.compile_any(child)?;
            }
            Ok(())
        })
    }
    fn compile_instruction(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        // FIXME move to function
        match ast.child(0)?.text.as_str()
        {
            "break" => self.code.push(BREAK),
            "continue" => self.code.push(CONTINUE),
            "return" | "yield" =>
            {
                match ast.children.len()
                {
                    2 => self.compile_nth_child(ast, 1)?,
                    1 => self.compile_push_float(0.0),
                    _ => return plainerr("internal error: broken return/yield instruction")
                }
                match ast.child(0)?.text.as_str()
                {
                    "return" => self.code.push(RETURN),
                    "yield" => self.code.push(YIELD),
                    _ => return plainerr("internal error: broken logic in compiling return/yield AST node")
                }
            }
            _ => return plainerr("internal error: unhandled type of instruction")
        }
        Ok(())
    }
    fn compile_objdef(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_string_index_with_prefix(OBJDEF, &ast.child(1)?.child(0)?.text);
        let funcs = ast.child_slice(3, -1)?;
        if funcs.len() > 0xFFFF
        {
            return plainerr("error: can only have 0xFFFF (around 65000) functions to a single object");
        }
        self.code.extend(pack_u16(funcs.len() as u16));
        
        self.compile_context_wrapped(Context::Objdef, &|x|
        {
            for child in funcs.iter()
            {
                x.compile_any(child)?;
            }
            Ok(())
        })
    }
    fn compile_funcdef(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        let kind = &ast.child(0)?.child(0)?.text;
        let name = &ast.child(1)?.child(0)?.text;
        
        if !matches!(self.context, Context::Objdef)
        {
            match kind.as_str()
            {
                "def" => self.code.push(FUNCDEF),
                "globaldef" => self.code.push(GLOBALFUNCDEF),
                "subdef" => self.code.push(SUBFUNCDEF),
                "generator" => self.code.push(GENERATORDEF),
                _ => return plainerr("error: first token of funcdef must be \"def\" | \"globaldef\" | \"subdef\" | \"generator\"")
            }
        }
        
        self.compile_context_wrapped(Context::Unknown, &|x|
        {
            x.compile_string_index(&name);
            x.code.extend(pack_u16(ast.child(3)?.children.len() as u16));
            
            let body_len_position = x.code.len();
            x.code.extend(pack_u64(0 as u64));
            
            for arg in &ast.child(3)?.children
            {
                x.compile_string_index(&arg.child(0)?.text);
            }
            
            let position_1 = x.code.len();
            
            for statement in &ast.child(6)?.children
            {
                let oldscopedepth = x.scopedepth;
                x.scopedepth = 0;
                x.compile_any(&statement)?;
                x.scopedepth = oldscopedepth;
            }
            x.code.push(EXIT);
            
            let position_2 = x.code.len();
            
            let body_len = position_2 - position_1;
            
            x.rewrite_code(body_len_position, pack_u64(body_len as u64))?;
            
            Ok(())
        })?;
        Ok(())
    }
    
    fn compile_with(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        
        self.compile_nth_child(ast, 1)?;
        self.code.push(WITH);
        
        let len_position = self.code.len();
        self.code.extend(pack_u64(0));
        
        let position_1 = self.code.len();
        self.compile_nth_child(ast, 2)?;
        self.code.push(WITHLOOP);
        
        let position_2 = self.code.len();
        
        let block_len = position_2 - position_1;
        self.rewrite_code(len_position, pack_u64(block_len as u64))?;
        
        Ok(())
    }
    fn compile_declaration(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        let decl_type = ast.child(0)?.text.as_str();
        
        for child in ast.child_slice(1, 0)?
        {
            let name = &child.child(0)?.child(0)?.text;
            
            // evaluate right hand side of assignment, if there is one, BEFORE declaring the variable
            if child.children.len() == 3
            {
                match decl_type
                {
                    "globalvar" => self.compile_pushglobal(&name)?,
                    _ => self.compile_pushname(&name)?
                }
                self.compile_nth_child(child, 2)?;
            }
            
            // declare the variable
            self.compile_pushname(&name)?;
            match decl_type
            {
                "var" => self.code.push(DECLVAR),
                "far" => self.code.push(DECLFAR),
                "globalvar" => self.code.push(DECLGLOBALVAR),
                _ => return plainerr("internal error: non-var/far prefix to declaration")
            }
            
            // perform the assignment to the newly-declared variable
            if child.children.len() == 3
            {
                self.code.push(BINSTATE);
                self.code.push(0x00);
            }
        }
        Ok(())
    }
    fn compile_binstate(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        let operator = &ast.child(1)?.child(0)?.text;
        let op = get_assignment_type(operator).ok_or_else(|| minierr(&format!("internal error: unhandled or unsupported type of binary statement {}", operator)))?;
        
        self.compile_nth_child(ast, 0)?;
        self.compile_nth_child(ast, 2)?;
        self.code.push(BINSTATE);
        self.code.push(op);
        
        Ok(())
    }
    fn compile_unstate(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        let operator = &ast.child(1)?.child(0)?.text;
        
        self.compile_nth_child(ast, 0)?;
        self.code.push(UNSTATE);
        match operator.as_str()
        {
            "++" => self.code.push(0x00),
            "--" => self.code.push(0x01),
            _ => return Err(format!("internal error: unhandled or unsupported type of unary statement {}", operator))
        }
        
        Ok(())
    }
    fn compile_lvar(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        if ast.child(0)?.text == "name"
        {
            self.compile_pushname(&ast.child(0)?.child(0)?.text)?;
        }
        else
        {
            self.compile_nth_child(ast, 0)?;
        }
        Ok(())
    }

    fn compile_rvar(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_context_wrapped(Context::Expr, &|x| x.compile_nth_child(ast, 0))
    }
    fn compile_unary(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        let operator = &ast.child(0)?.child(0)?.text;
        
        self.compile_nth_child(ast, 1)?;
        self.code.push(UNOP);
        
        let op = get_unop_type(slice(&operator, 0, 1).as_str()).ok_or_else(|| minierr("internal error: unhandled type of unary expression"))?;
        self.code.push(op);
        
        Ok(())
    }
    fn compile_indirection(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_pushname(&ast.child(1)?.child(0)?.text)?;
        self.code.push(INDIRECTION);
        
        if matches!(self.context, Context::Expr)
        {
            self.code.push(EVALUATION);
        }
        
        Ok(())
    }
    fn compile_dismember(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_pushname(&ast.child(1)?.child(0)?.text)?;
        self.code.push(DISMEMBER);
        Ok(())
    }
    fn compile_dictindex(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_pushstr(&ast.child(1)?.child(0)?.text)?;
        self.code.push(ARRAYEXPR);
        
        if matches!(self.context, Context::Expr)
        {
            self.code.push(EVALUATION);
        }
        
        Ok(())
    }
    fn compile_arrayindex(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_context_wrapped(Context::Unknown, &|x| x.compile_nth_child(ast, 1))?;
        self.code.push(ARRAYEXPR);
        
        if matches!(self.context, Context::Expr)
        {
            self.code.push(EVALUATION);
        }
        
        Ok(())
    }
    fn compile_lambda(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        let captures : &Vec<ASTNode> = &ast.child(1)?.children;
        let args : &Vec<ASTNode> = &ast.child(4)?.children;
        let statements : &Vec<ASTNode> = &ast.child(7)?.children;
        
        let mut capturenames = Vec::new();
        
        for capture in captures
        {
            capturenames.push(self.code.get_string_index(&capture.child(0)?.child(0)?.text));
            self.compile_nth_child(capture, 2)?;
        }
        
        self.code.push(LAMBDA);
        self.code.extend(pack_u64(captures.len() as u64));
        for capture_name_index in capturenames.iter().rev()
        {
            self.code.extend(pack_u64(*capture_name_index as u64));
        }
        self.code.extend(pack_u16(args.len() as u16));
        let len_position = self.code.len();
        self.code.extend(pack_u64(0 as u64));
          
        for arg in args
        {
            self.compile_string_index(&arg.child(0)?.text);
        }
        
        let position_1 = self.code.len();
        
        for statement in statements
        {
            self.compile_any(statement)?;
        }
                
        self.code.push(EXIT);
        
        let position_2 = self.code.len();
        let body_len = position_2 - position_1;
        
        self.rewrite_code(len_position, pack_u64(body_len as u64))
    }
    fn compile_arraybody(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        let mut elementcount = 0;
        for expression in ast.child_slice(1, -1)?
        {
            if expression.text == "unusedcomma"
            {
                break;
            }
            self.compile_any(expression)?;
            elementcount += 1;
        }
        self.code.push(COLLECTARRAY);
        self.code.extend(pack_u16(elementcount as u16));
        
        Ok(())
    }
    fn compile_dictbody(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        let mut elementcount = 0;
        for expression in ast.child_slice(1, -1)?
        {
            if expression.text == "unusedcomma"
            {
                break;
            }
            self.compile_nth_child(expression, 0)?;
            self.compile_nth_child(expression, 2)?;
            elementcount += 1;
        }
        self.code.push(COLLECTDICT);
        self.code.extend(pack_u16(elementcount as u16));
        
        Ok(())
    }
    fn compile_setbody(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        let mut elementcount = 0;
        for expression in ast.child_slice(2, -1)?
        {
            if expression.text == "unusedcomma"
            {
                break;
            }
            self.compile_any(expression)?;
            elementcount += 1;
        }
        self.code.push(COLLECTSET);
        self.code.extend(pack_u16(elementcount as u16));
        
        Ok(())
    }

    fn compile_ifcondition(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_nth_child(ast, 1)?;
        
        if ast.children.len() == 3
        {
            self.code.push(IF);
            let body_len_position = self.code.len();
            self.code.extend(pack_u64(0));
            let position_1 = self.code.len();
            self.compile_nth_child(ast, 2)?;
            let position_2 = self.code.len();
            let body_len = position_2 - position_1;
            self.rewrite_code(body_len_position, pack_u64(body_len as u64))
        }
        else if ast.children.len() == 5 && ast.child(3)?.text == "else"
        {
            self.code.push(IF);
            let body_len_position = self.code.len();
            self.code.extend(pack_u64(0));
            
            let position_1 = self.code.len();
            self.compile_nth_child(ast, 2)?;
            self.code.push(JUMPRELATIVE);
            let else_len_position = self.code.len();
            self.code.extend(pack_u64(0));
            let position_2 = self.code.len();
            let body_len = position_2 - position_1;
            self.rewrite_code(body_len_position, pack_u64(body_len as u64))?;
            
            let position_1 = self.code.len();
            self.compile_nth_child(ast, 4)?;
            let position_2 = self.code.len();
            let else_len = position_2 - position_1;
            self.rewrite_code(else_len_position, pack_u64(else_len as u64))
        }
        else
        {
            plainerr("internal error: broken if condition")
        }
    }
    fn compile_forcondition(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        let header = ast.child(2)?;
        let init_exists = !header.child(0)?.children.is_empty();
        let expr_exists = !header.child(2)?.children.is_empty();
        
        
        // for loops act almost exactly like while loops,
        // except that the "post" execution expression is a prefix to the loop test expression,
        // but it is skipped over the first time the loop is entered
        
        // for loops need an extra layer of scope around them if they have an init statement
        
        if init_exists
        {
            self.code.push(SCOPE);
            self.scopedepth += 1;
            self.compile_nth_child(header, 0)?;
        }
        
        self.code.push(FOR);
        let post_len_rewrite_pos = self.code.len();
        self.code.extend(pack_u64(0));
        let expr_len_rewrite_pos = self.code.len();
        self.code.extend(pack_u64(0));
        let block_len_rewrite_pos = self.code.len();
        self.code.extend(pack_u64(0));
        
        let position_1 = self.code.len();
        
        self.compile_nth_child(header, 4)?;
        
        let position_2 = self.code.len();
        
        if expr_exists
        {
            self.compile_nth_child(header, 2)?;
        }
        else
        {
            self.compile_push_float(1.0);
        }
        self.code.push(WHILETEST);
        
        let position_3 = self.code.len();
        self.compile_last_child(ast)?;
        self.code.push(WHILELOOP);
        
        let position_4 = self.code.len();
        
        let post_len = position_2 - position_1;
        let expr_len = position_3 - position_2;
        let block_len = position_4 - position_3;
        
        self.rewrite_code(post_len_rewrite_pos, pack_u64(post_len as u64))?;
        self.rewrite_code(expr_len_rewrite_pos, pack_u64(expr_len as u64))?;
        self.rewrite_code(block_len_rewrite_pos, pack_u64(block_len as u64))?;
        
        // for loops need an extra layer of scope around them if they have an init statement
        if init_exists
        {
            self.scopedepth -= 1;
            self.compile_unscope()?;
        }
        Ok(())
    }
    fn compile_invocation_expr(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        if ast.children.len() != 2
        {
            return plainerr("error: invocation must have exactly two children");
        }
        
        self.compile_nth_child(ast, 1)?;
        self.code.push(INVOKE);
        self.code.push(INVOKEEXPR);
        
        Ok(())
    }
    fn compile_foreach(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        if !ast.child(2)?.isparent || ast.child(2)?.text != "name"
        {
            return plainerr("error: child index 2 of `foreach` must be a `name`");
        }
        
        self.compile_scope_wrapped(&|x|
        {
            x.compile_pushname(&ast.child(2)?.child(0)?.text)?;
            
            x.compile_nth_child(ast, 4)?;
            x.code.push(FOREACH);
            let block_len_rewrite_pos = x.code.len();
            x.code.extend(pack_u64(0));
            let position_1 = x.code.len();
            x.code.push(FOREACHHEAD);
            x.compile_nth_child(ast, 6)?;
            x.code.push(FOREACHLOOP);
            let position_2 = x.code.len();
            let block_len = position_2 - position_1;
            x.rewrite_code(block_len_rewrite_pos, pack_u64(block_len as u64))
        })
    }
    
    fn compile_switch_case_labels(&mut self, ast : &ASTNode, which : u16) -> Result<(), String>
    {
        if !ast.isparent || !matches!(ast.text.as_str(), "switchcase" | "switchdefault")
        {
            return plainerr("error: tried to compile a non-switchcase/switchdefault ast node as a switch case")
        }
        for node in ast.child_slice(1, -2)? // implicitly causes switchdefault to have 0 labels
        {
            self.compile_any(node)?;
            self.code.push(SWITCHCASE);
            self.code.extend(pack_u16(which));
        }
        if ast.child_slice(1, -2)?.is_empty()
        {
            self.code.push(SWITCHDEFAULT);
            self.code.extend(pack_u16(which));
        }
        Ok(())
    }
    
    fn compile_switch_case_block(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        if !ast.isparent || !matches!(ast.text.as_str(), "switchcase" | "switchdefault")
        {
            return plainerr("error: tried to compile a non-switchcase/switchdefault ast node as a switch case")
        }
        self.compile_last_child(ast)?;
        self.code.push(SWITCHEXIT);
        Ok(())
    }

    fn compile_switch(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_nth_child(ast, 2)?;
        
        // SWITCH (u8)
        // num cases (blocks) (u16)
        // case block locations... (relative to end of "num cases") (u64s, numbered by "num cases")
        // switch exit location (relative to end of "num cases") (single u64)
        // case label expressions... (arbitrary)
        // case blocks... (arbitrary)
        
        self.code.push(SWITCH);
        let cases = &ast.child(5)?.children;
        let num_case_blocks = cases.len();
        self.code.extend(pack_u16(num_case_blocks as u16));
        let case_block_reference_point = self.code.len();
        
        let block_count_rewrite_pos = self.code.len();
        for _ in 0..=num_case_blocks
        {
            self.code.extend(pack_u64(0));
        }
        
        for (i, node) in cases.iter().enumerate()
        {
            self.compile_switch_case_labels(node, i as u16)?
        }
        self.code.push(SWITCHEXIT);
        let mut case_block_positions = Vec::new();
        for node in cases
        {
            case_block_positions.push(self.code.len() - case_block_reference_point);
            self.compile_switch_case_block(node)?
        }
        case_block_positions.push(self.code.len() - case_block_reference_point);
        if case_block_positions.len() > 0xFFFF
        {
            return plainerr("error: switches may have a maximum of 0xFFFF (65000ish) labels");
        }
        for (i, position) in case_block_positions.iter().enumerate()
        {
            self.rewrite_code(block_count_rewrite_pos + i*8, pack_u64(*position as u64))?;
        }
        Ok(())
    }

    fn compile_ternary(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_nth_child(ast, 0)?;
        self.code.push(IF);
        let block1_len_rewrite_pos = self.code.len();
        self.code.extend(pack_u64(0));
        
        let position_1 = self.code.len();
        self.compile_nth_child(ast, 2)?;
        self.code.push(JUMPRELATIVE);
        let block2_len_rewrite_pos = self.code.len();
        self.code.extend(pack_u64(0));
        let position_2 = self.code.len();
        
        let position_3 = self.code.len();
        self.compile_nth_child(ast, 4)?;
        let position_4 = self.code.len();
        
        let block1_len = position_2 - position_1;
        let block2_len = position_4 - position_3;
        
        self.rewrite_code(block1_len_rewrite_pos, pack_u64(block1_len as u64))?;
        self.rewrite_code(block2_len_rewrite_pos, pack_u64(block2_len as u64))?;
        
        Ok(())
    }
}

/// Compiles an AST into bytes.
pub fn compile_bytecode(ast : &ASTNode) -> Result<Code, String>
{
    let mut state = CompilerState::new();
    if let Err(err) = state.compile_any(ast)
    {
        eprintln!("compiler hit an error on line {}, position {}, node type {}:", state.last_line, state.last_index, state.last_type);
        Err(err)
    }
    else
    {
        Ok(state.code)
    }
}
/// Same as compile_bytecode, but shares bookkeeping with an existing compilation result. Necessary for dynamically compiling subroutines and executing them from other code.
pub fn compile_bytecode_share_bookkeeping(code : &Code, ast : &ASTNode) -> Result<Code, String>
{
    let mut state = CompilerState::new_share_bookkeeping(code);
    if let Err(err) = state.compile_any(ast)
    {
        eprintln!("compiler hit an error on line {}, position {}, node type {}:", state.last_line, state.last_index, state.last_type);
        Err(err)
    }
    else
    {
        Ok(state.code)
    }
}
