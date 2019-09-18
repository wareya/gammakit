#![allow(clippy::len_zero)]

use super::{strings::*, ast::*, bytecode::*};
use std::rc::Rc;
use std::collections::{HashMap, BTreeMap, BTreeSet};
use super::interpreter::GlobalState;
use super::interpreter::types::{FuncSpec, ObjSpec};

pub (crate) struct DebugInfo
{
    pub (crate) last_line : usize,
    pub (crate) last_index : usize,
    pub (crate) last_type : String,
}

impl std::fmt::Debug for DebugInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "DebugInfo {{ <redacted> }}")
    }
}

pub struct Code
{
    pub (crate) code : Rc<Vec<u8>>,
    pub (crate) debug : Rc<BTreeMap<usize, DebugInfo>>
}

impl std::clone::Clone for Code
{
    fn clone(&self) -> Code
    {
        Code{code : Rc::clone(&self.code), debug : Rc::clone(&self.debug)}
    }
}

impl std::cmp::PartialEq for Code
{
    fn eq(&self, other : &Code) -> bool
    {
        Rc::ptr_eq(&self.code, &other.code)
    }
}
impl Eq for Code {}

impl std::fmt::Debug for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "Code {{ <redacted> }}")
    }
}

impl Code
{
    pub (crate) fn new() -> Code
    {
        Code{code : Rc::new(Vec::new()), debug : Rc::new(BTreeMap::new())}
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
    pub (crate) fn len(&self) -> usize
    {
        self.code.len()
    }
    pub (crate) fn get<I : std::slice::SliceIndex<[u8]>>(&self, index : I) -> Option<&I::Output>
    {
        self.code.get(index)
    }
    pub (crate) unsafe fn as_ptr(&self) -> *const u8
    {
        self.code.as_ptr()
    }
    fn add_debug_info(&mut self, pc : usize, last_line : usize, last_index : usize, last_type : &str)
    {
        Rc::get_mut(&mut self.debug).unwrap().insert(pc, DebugInfo{last_line, last_index, last_type : last_type.to_string()});
    }
    pub (crate) fn get_debug_info(&self, pc : usize) -> Option<&DebugInfo>
    {
        self.debug.range(..=&pc).next_back().map(|x| x.1)
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
    Lvar,
    Objdef,
}

struct Scope {
    parent_size : usize,
    size : usize,
    identifiers : HashMap<usize, (usize, bool)>,
}

enum IdenLocation {
    Lexical(usize), // lexical scoped variable
    Function(usize), // user-defined function, declarations of which are subject to lexical scoping
    InstanceVar(usize), // instance variable as found from within method body or with() body
    BareGlobal(usize), // bare global variable
    GlobalFunc(usize), // global function
    Binding(usize), // binding
    Object(usize), // object name
    Selfref, // "self"
    Other, // "other"
}

impl Scope {
    fn new(parent_size : usize) -> Scope
    {
        Scope { parent_size, size : 0, identifiers : HashMap::new() }
    }
    fn add_identifier(&mut self, name : usize, isfunction : bool) -> Option<usize>
    {
        if self.identifiers.get(&name).is_some()
        {
            None
        }
        else
        {
            let new_index = self.parent_size + self.size;
            self.identifiers.insert(name, (new_index, isfunction));
            self.size += 1;
            Some(new_index)
        }
    }
    fn find_identifier(&self, name : usize) -> Option<IdenLocation>
    {
        if let Some((var, false)) = self.identifiers.get(&name)
        {
            Some(IdenLocation::Lexical(*var))
        }
        else if let Some((func, true)) = self.identifiers.get(&name)
        {
            Some(IdenLocation::Function(*func))
        }
        else
        {
            None
        }
    }
}

struct Frame {
    scopes : Vec<Scope>,
    objects : Vec<ObjSpec>,
}

impl Frame {
    fn new() -> Frame
    {
        Frame { scopes : vec!(Scope::new(0)), objects : vec!() }
    }
    fn total_size(&self) -> usize
    {
        let asdf = self.scopes.last().unwrap();
        asdf.parent_size + asdf.size
    }
    fn add_scope(&mut self)
    {
        self.scopes.push(Scope::new(self.total_size()));
    }
    fn pop_scope(&mut self) -> usize
    {
        self.scopes.pop();
        self.total_size()
    }
    fn add_identifier(&mut self, name : usize, isfunction : bool) -> Option<usize>
    {
        self.scopes.last_mut().unwrap().add_identifier(name, isfunction)
    }
    fn find_identifier(&self, name : usize) -> Option<IdenLocation>
    {
        for scope in self.scopes.iter().rev()
        {
            if let Some(index) = scope.find_identifier(name)
            {
                return Some(index);
            }
        }
        if let Some(myobj) = &self.objects.last()
        {
            if let Some(index) = myobj.variables.get(&name)
            {
                return Some(IdenLocation::InstanceVar(name)); // FIXME make this use exact index
            }
            if let Some(index) = myobj.functions.get(&name)
            {
                return Some(IdenLocation::InstanceVar(name)); // FIXME make this use exact index
            }
        }
        None
    }
}

type CompilerBinding<'a> = fn(&mut CompilerState<'a>, &ASTNode) -> Result<(), String>;

struct CompilerState<'a> {
    code : Code,
    hooks : HashMap<String, CompilerBinding<'a>>,
    context : Context,
    last_line : usize,
    last_index : usize,
    last_type : String,
    
    globalstate : &'a mut GlobalState,
    
    frames : Vec<Frame>,
}


impl<'a> CompilerState<'a> {
    fn new(globalstate : &'a mut GlobalState) -> CompilerState<'a>
    {
        let mut ret = CompilerState {
            hooks: HashMap::new(),
            code: Code::new(),
            context: Context::Unknown,
            
            last_line: 0,
            last_index: 0,
            last_type: "".to_string(),
            
            globalstate,
            
            frames : vec!(Frame::new()),
        };
        ret.insert_default_hooks();
        ret
    }
    
    fn trap_error(&self, err : Result<(), String>) -> Result<(), String>
    {
        if let Err(err) = err
        {
            eprintln!("compiler hit an error on line {}, position {}, node type `{}`:", self.last_line, self.last_index, self.last_type);
            Err(err)
        }
        else
        {
            Ok(())
        }
    }
    
    pub (crate) fn get_string_index(&mut self, string : &String) -> usize
    {
        self.globalstate.get_string_index(string)
    }
    pub (crate) fn get_string(&mut self, index : usize) -> String
    {
        self.globalstate.get_string(index)
    }
    
    fn add_scope(&mut self)
    {
        self.frames.last_mut().unwrap().add_scope()
    }
    fn pop_scope(&mut self) -> usize
    {
        self.frames.last_mut().unwrap().pop_scope()
    }
    fn add_variable(&mut self, name : &String) -> Option<usize>
    {
        let id = self.get_string_index(name);
        self.frames.last_mut().unwrap().add_identifier(id, false)
    }
    fn add_function(&mut self, name : &String) -> Option<usize>
    {
        let id = self.get_string_index(name);
        self.frames.last_mut().unwrap().add_identifier(id, true)
    }
    fn find_identifier(&mut self, name : &String) -> Option<IdenLocation>
    {
        match name.as_str()
        {
            "self" => return Some(IdenLocation::Selfref),
            "other" => return Some(IdenLocation::Other),
            _ => {},
        }
        let index = self.get_string_index(name);
        if let Some(var) = self.frames.last().unwrap().find_identifier(index)
        {
            return Some(var);
        }
        if self.globalstate.bindings.contains_key(&index)
        || self.globalstate.trivial_bindings.contains_key(&index)
        || self.globalstate.simple_bindings.contains_key(&index)
        || self.globalstate.trivial_simple_bindings.contains_key(&index)
        {
            return Some(IdenLocation::Binding(index));
        }
        if self.globalstate.barevariables.contains_key(&index)
        {
            return Some(IdenLocation::BareGlobal(index));
        }
        if self.globalstate.objects.contains_key(&index)
        {
            return Some(IdenLocation::Object(index));
        }
        if self.globalstate.functions.contains_key(&index)
        {
            return Some(IdenLocation::GlobalFunc(index));
        }
        return None;
    }
    fn open_frame(&mut self)
    {
        assert!(self.frames.len() >= 1);
        self.frames.push(Frame::new());
    }
    fn close_frame(&mut self)
    {
        self.frames.pop();
        assert!(self.frames.len() >= 1);
    }
    
    fn add_hook<T : ToString>(&mut self, name: &T, fun : CompilerBinding<'a>)
    {
        self.hooks.insert(name.to_string(), fun);
    }
    fn insert_default_hooks(&mut self)
    {
        self.add_hook(&"program", CompilerState::compile_program);
        self.add_hook(&"blankstatement", CompilerState::compile_nop);
        self.add_hook(&"statement", CompilerState::compile_statement);
        self.add_hook(&"funccall", CompilerState::compile_funccall);
        self.add_hook(&"name", CompilerState::compile_name);
        self.add_hook(&"rhunexpr_right", CompilerState::compile_children);
        self.add_hook(&"funcargs", CompilerState::compile_funcargs);
        self.add_hook(&"expr", CompilerState::compile_children);
        self.add_hook(&"lhunop", CompilerState::compile_children);
        self.add_hook(&"simplexpr", CompilerState::compile_children);
        self.add_hook(&"supersimplexpr", CompilerState::compile_children);
        self.add_hook(&"string", CompilerState::compile_push_string);
        self.add_hook(&"condition", CompilerState::compile_children);
        self.add_hook(&"barestatement", CompilerState::compile_children);
        self.add_hook(&"block", CompilerState::compile_block);
        self.add_hook(&"nakedblock", CompilerState::compile_nakedblock);
        self.add_hook(&"whilecondition", CompilerState::compile_whilecondition);
        self.add_hook(&"parenexpr", CompilerState::compile_parenexpr);
        self.add_hook(&"number", CompilerState::compile_number);
        self.add_hook(&"statementlist", CompilerState::compile_statementlist);
        self.add_hook(&"instruction", CompilerState::compile_instruction);
        self.add_hook(&"objdef", CompilerState::compile_objdef);
        self.add_hook(&"funcdef", CompilerState::compile_funcdef);
        self.add_hook(&"globalfuncdef", CompilerState::compile_globalfuncdef);
        self.add_hook(&"withstatement", CompilerState::compile_with);
        self.add_hook(&"withasstatement", CompilerState::compile_withas);
        self.add_hook(&"declaration", CompilerState::compile_declaration);
        self.add_hook(&"bareglobaldec", CompilerState::compile_bareglobaldec);
        self.add_hook(&"binstate", CompilerState::compile_binstate);
        self.add_hook(&"unstate", CompilerState::compile_unstate);
        self.add_hook(&"lvar", CompilerState::compile_lvar);
        self.add_hook(&"rvar", CompilerState::compile_rvar);
        self.add_hook(&"rhunexpr", CompilerState::compile_rhunexpr);
        self.add_hook(&"unary", CompilerState::compile_unary);
        self.add_hook(&"indirection", CompilerState::compile_indirection);
        self.add_hook(&"dictindex", CompilerState::compile_dictindex);
        self.add_hook(&"binexpr_0", CompilerState::compile_binexpr);
        self.add_hook(&"binexpr_1", CompilerState::compile_binexpr);
        self.add_hook(&"binexpr_2", CompilerState::compile_binexpr);
        self.add_hook(&"binexpr_3", CompilerState::compile_binexpr);
        self.add_hook(&"lambda", CompilerState::compile_lambda);
        self.add_hook(&"arraybody", CompilerState::compile_arraybody);
        self.add_hook(&"arrayindex", CompilerState::compile_arrayindex);
        self.add_hook(&"ifcondition", CompilerState::compile_ifcondition);
        self.add_hook(&"dismember", CompilerState::compile_dismember);
        self.add_hook(&"dictbody", CompilerState::compile_dictbody);
        self.add_hook(&"forcondition", CompilerState::compile_forcondition);
        self.add_hook(&"forheaderstatement", CompilerState::compile_children);
        self.add_hook(&"forheaderexpr", CompilerState::compile_children);
        self.add_hook(&"invocation_expr", CompilerState::compile_invocation_expr);
        self.add_hook(&"setbody", CompilerState::compile_setbody);
        self.add_hook(&"foreach", CompilerState::compile_foreach);
        self.add_hook(&"switch", CompilerState::compile_switch);
        self.add_hook(&"ternary", CompilerState::compile_ternary);
    }
    fn compile_u16(&mut self, num : u16) -> usize
    {
        while self.code.len() % 2 != 0
        {
            self.code.push(0);
        }
        self.code.extend(pack_u16(num));
        self.code.len() - 2
    }
    fn compile_u64(&mut self, num : u64) -> usize
    {
        while self.code.len() % 8 != 0
        {
            self.code.push(0);
        }
        self.code.extend(pack_u64(num));
        self.code.len() - 8
    }
    fn compile_f64(&mut self, num : f64) -> usize
    {
        while self.code.len() % 8 != 0
        {
            self.code.push(0);
        }
        self.code.extend(pack_f64(num));
        self.code.len() - 8
    }
    
    fn compile_push_float(&mut self, float : f64)
    {
        self.code.push(PUSHFLT);
        self.compile_f64(float);
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
        let index = self.get_string_index(text) as u64;
        self.compile_u64(index);
    }
    fn compile_string_index_with_prefix(&mut self, prefix : u8, text : &String)
    {
        self.code.push(prefix);
        self.compile_string_index(text);
    }

    fn compile_unscope(&mut self, var_stack_length : usize) -> Result<(), String>
    {
        self.code.push(UNSCOPE);
        self.compile_u64(var_stack_length as u64);
        Ok(())
    }
    fn compile_context_wrapped(&mut self, context : Context, fun : &dyn Fn(&mut CompilerState) -> Result<(), String>) -> Result<(), String>
    {
        let old_context = self.context;
        self.context = context;
        
        fun(self)?;
        
        self.context = old_context;
        
        Ok(())
    }
    fn compile_scope_wrapped(&mut self, fun : &dyn Fn(&mut CompilerState) -> Result<(), String>) -> Result<(), String>
    {
        self.add_scope();
        
        fun(self)?;
        
        let var_stack_length = self.pop_scope();
        self.compile_unscope(var_stack_length)
    }
    fn compile_any(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.last_line = ast.line;
        self.last_index = ast.position;
        self.last_type = ast.text.clone();
        
        self.code.add_debug_info(self.code.len(), self.last_line, self.last_index, &self.last_type);
        
        let hook = self.hooks.get(&ast.text).ok_or_else(|| minierr(&format!("internal error: no handler for AST node with name `{}`", ast.text)))?;
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
            let mut need_mutable_context = matches!(self.context, Context::Lvar);
            if ast.children.len() >= 3 && !matches!(ast.child(2)?.child(0)?.text.as_str(), "funcargs" | "indirection")
            {
                need_mutable_context = true;
            }
            if need_mutable_context
            {
                self.compile_pushglobal(&ast.child(1)?.child(0)?.child(1)?.child(0)?.text)?;
            }
            else
            {
                self.compile_pushglobalval(&ast.child(1)?.child(0)?.child(1)?.child(0)?.text)?;
            }
            if ast.children.len() > 2
            {
                for child in ast.child_slice(2, -1)?
                {
                    self.compile_context_wrapped(Context::Lvar, &|x|
                    {
                        if child.text == "name"
                        {
                            x.compile_pushname(&child.child(0)?.text)
                        }
                        else
                        {
                            x.compile_any(child)
                        }
                    })?;
                }
                self.compile_last_child(ast)?;
            }
            Ok(())
        }
        else
        {
            for child in ast.child_slice(0, -1)?
            {
                self.compile_context_wrapped(Context::Lvar, &|x|
                {
                    if child.text == "name"
                    {
                        x.compile_pushname(&child.child(0)?.text)
                    }
                    else
                    {
                        x.compile_any(child)
                    }
                })?;
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
                _ => plainerr("internal error: tried to use an unknown form of expression as a statement")
            };
        }
        
        if ast.child(0)?.text == "name" && ast.child(0)?.child(0)?.text == "global" && ast.child(1)?.child(0)?.text == "indirection"
        {
            let mut need_mutable_context = matches!(self.context, Context::Lvar);
            if ast.children.len() >= 3 && !matches!(ast.child(2)?.child(0)?.text.as_str(), "funcargs" | "indirection")
            {
                need_mutable_context = true;
            }
            if need_mutable_context
            {
                self.compile_pushglobal(&ast.child(1)?.child(0)?.child(1)?.child(0)?.text)?;
            }
            else
            {
                self.compile_pushglobalval(&ast.child(1)?.child(0)?.child(1)?.child(0)?.text)?;
            }
            if ast.children.len() > 2
            {
                for child in ast.child_slice(2, -1)?
                {
                    self.compile_context_wrapped(Context::Lvar, &|x|
                    {
                        if child.text == "name"
                        {
                            x.compile_pushname(&child.child(0)?.text)
                        }
                        else
                        {
                            x.compile_any(child)
                        }
                    })?;
                }
                self.compile_context_wrapped(Context::Statement, &|x| x.compile_last_child(ast))?;
            }
            Ok(())
        }
        else
        {
            for child in ast.child_slice(0, -1)?
            {
                self.compile_context_wrapped(Context::Lvar, &|x|
                {
                    if child.text == "name"
                    {
                        x.compile_pushname(&child.child(0)?.text)
                    }
                    else
                    {
                        x.compile_any(child)
                    }
                })?;
            }
            self.compile_context_wrapped(Context::Statement, &|x| x.compile_last_child(ast))
        }
        
    }
    fn compile_indirection(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        if !matches!(self.context, Context::Lvar)
        {
            self.code.push(EVALUATEINDIRECTION);
        }
        else
        {
            self.code.push(INDIRECTION);
        }
        self.compile_string_index(&ast.child(1)?.child(0)?.text);
        
        Ok(())
    }
    fn compile_dismember(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.code.push(DISMEMBER);
        self.compile_string_index(&ast.child(1)?.child(0)?.text);
        Ok(())
    }
    fn compile_dictindex(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_pushstr(&ast.child(1)?.child(0)?.text)?;
        if !matches!(self.context, Context::Lvar)
        {
            self.code.push(EVALUATEARRAYEXPR);
        }
        else
        {
            self.code.push(ARRAYEXPR);
        }
        
        Ok(())
    }
    fn compile_arrayindex(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_context_wrapped(Context::Unknown, &|x| x.compile_nth_child(ast, 1))?;
        if !matches!(self.context, Context::Lvar)
        {
            self.code.push(EVALUATEARRAYEXPR);
        }
        else
        {
            self.code.push(ARRAYEXPR);
        }
        
        Ok(())
    }
    fn compile_funcargs(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        let args = &ast.child(1)?.children;
        self.compile_context_wrapped(Context::Unknown, &|x|
        {
            for child in args
            {
                x.compile_any(child)?;
            }
            Ok(())
        })?;
        match self.context
        {
            Context::Statement => self.code.push(FUNCCALL),
            _ => self.code.push(FUNCEXPR)
        }
        self.compile_u64(args.len() as u64);
        
        Ok(())
    }
    fn compile_pushname(&mut self, string : &String) -> Result<(), String>
    {
        if string.as_str() == "global"
        {
            return Err(format!("internal error: attempted to compile `global` with PUSHNAME ({:?})", (self.last_line, self.last_index, &self.last_type)))
        }
        if let Some(var) = self.find_identifier(string)
        {
            match var
            {
                IdenLocation::Lexical(index) |
                IdenLocation::Function(index) => // FIXME always evaluate function?
                {
                    if !matches!(self.context, Context::Lvar)
                    {
                        self.code.push(EVALUATEVAR);
                        self.compile_u64(index as u64);
                    }
                    else
                    {
                        self.code.push(PUSHVAR);
                        self.compile_u64(index as u64);
                    }
                }
                IdenLocation::InstanceVar(index) =>
                {
                    if !matches!(self.context, Context::Lvar)
                    {
                        self.code.push(EVALUATEINSTVAR);
                        self.compile_u64(index as u64);
                    }
                    else
                    {
                        self.code.push(PUSHINSTVAR);
                        self.compile_u64(index as u64);
                    }
                }
                IdenLocation::Binding(index) =>
                {
                    self.code.push(PUSHBIND);
                    self.compile_u64(index as u64);
                }
                IdenLocation::BareGlobal(index) =>
                {
                    if !matches!(self.context, Context::Lvar)
                    {
                        self.code.push(EVALUATEBAREGLOBAL);
                        self.compile_u64(index as u64);
                    }
                    else
                    {
                        self.code.push(PUSHBAREGLOBAL);
                        self.compile_u64(index as u64);
                    }
                }
                IdenLocation::GlobalFunc(index) =>
                {
                    self.code.push(PUSHGLOBALFUNC);
                    self.compile_u64(index as u64);
                }
                IdenLocation::Object(index) =>
                {
                    self.code.push(PUSHOBJ);
                    self.compile_u64(index as u64);
                }
                IdenLocation::Selfref => self.code.push(PUSHSELF),
                IdenLocation::Other   => self.code.push(PUSHOTHER),
                _ => return Err(format!("not implemented yet kjawefawlefs"))
            }
        }
        else
        {
            return Err(format!("error: unknown identifier `{}`", string))
        }
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
        self.compile_pushname(&ast.child(0)?.text)
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
            rewrite_location_jumplen = self.compile_u64(0);
        }
        else if op == 0x11 // or
        {
            self.code.push(SHORTCIRCUITIFTRUE);
            rewrite_location_jumplen = self.compile_u64(0);
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
        let rewrite_location_exprlen = self.compile_u64(0);
        let rewrite_location_codelen = self.compile_u64(0);
        
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
        self.compile_f64(float);
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
        let nameindex = self.get_string_index(&ast.child(1)?.child(0)?.text);
        
        let parts = &ast.child(3)?;
        
        let mut var_index = 0;
        let mut variables = BTreeMap::new();
        
        variables.insert(self.get_string_index(&"id".to_string()), var_index);
        var_index += 1;
        
        let mut incomplete_object = ObjSpec { ident : nameindex, variables, functions : BTreeMap::new() };
        
        for part in &parts.children
        {
            if part.child(0)?.text == "objvardef"
            {
                for varname in part.child(0)?.child_slice(1, -1)?
                {
                    let varname = &varname.child(0)?.text;
                    let varnameindex = self.get_string_index(&varname);
                    if !incomplete_object.variables.contains_key(&varnameindex)
                    {
                        incomplete_object.variables.insert(varnameindex, var_index);
                    }
                    else
                    {
                        return Err(format!("error: redeclared identifier `{}`", varname))?;
                    }
                    var_index += 1;
                }
            }
        }
        
        self.globalstate.objects.insert(nameindex, incomplete_object.clone());
        self.globalstate.instances_by_type.insert(nameindex, BTreeSet::new());
        
        let mut dummy_functions = BTreeMap::new();
        for part in &parts.children
        {
            if part.child(0)?.text == "objfuncdef"
            {
                let def = &part.child(0)?;
                let funcname = &def.child(1)?.child(0)?.text;
                let argcount = def.child(3)?.children.len();
                
                let func = FuncSpec {
                    startaddr : 0,
                    endaddr : 0,
                    code : Code::new(),
                    argcount : argcount,
                    parentobj : nameindex,
                    forcecontext : 0,
                    fromobj : true,
                    generator : false,
                };
                
                dummy_functions.insert(self.get_string_index(funcname), func);
            }
        }
        
        incomplete_object.functions = dummy_functions;
        self.globalstate.objects.insert(nameindex, incomplete_object.clone());
        
        let mut functions = BTreeMap::new();
        for part in &parts.children
        {
            if part.child(0)?.text == "objfuncdef"
            {
                let def = &part.child(0)?;
                let funcname = &def.child(1)?.child(0)?.text;
                
                let oldcode = self.code.clone();
                self.code = Code::new();
                self.open_frame();
                self.frames.last_mut().unwrap().objects.push(incomplete_object.clone());
                
                let argcount = def.child(3)?.children.len();
                self.add_function(funcname).ok_or_else(|| format!("error: redeclared identifier `{}`", funcname))?;
                for arg in &def.child(3)?.children
                {
                    let name = &arg.child(0)?.text;
                    self.add_variable(name).ok_or_else(|| format!("error: redeclared identifier `{}`", name))?;
                }
                if funcname == "create" && argcount != 0
                {
                    return Err("error: create event must have 0 arguments".to_string());
                }
                for statement in &def.child(6)?.children
                {
                    self.compile_any(&statement)?;
                }
                self.code.push(EXIT);
                
                self.close_frame();
                let funccode = self.code.clone();
                self.code = oldcode;
                
                let func = FuncSpec {
                    startaddr : 0,
                    endaddr : funccode.code.len(),
                    code : funccode,
                    argcount,
                    parentobj : nameindex,
                    forcecontext : 0,
                    fromobj : true,
                    generator : false,
                };
                
                functions.insert(self.get_string_index(funcname), func);
            }
        }
        
        incomplete_object.functions = functions;
        let complete_object = incomplete_object;
        self.globalstate.objects.insert(nameindex, complete_object);
        
        Ok(())
    }
    fn compile_funcdef(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        let kind = &ast.child(0)?.child(0)?.text;
        let name = &ast.child(1)?.child(0)?.text;
        
        let prefix;
        match kind.as_str()
        {
            "def" =>
            {
                prefix = FUNCDEF;
                self.add_function(name).ok_or_else(|| format!("error: redeclared identifier `{}`", name))?;
            }
            "generator" =>
            {
                prefix = GENERATORDEF;
                self.add_function(name).ok_or_else(|| format!("error: redeclared identifier `{}`", name))?;
            }
            _ => return plainerr("error: first token of funcdef must be \"def\" | \"generator\"")
        }
        
        self.code.push(prefix);
        
        self.compile_u16(ast.child(3)?.children.len() as u16);
        
        let body_len_position = self.compile_u64(0 as u64);
        
        let position_1 = self.code.len();
        
        self.open_frame();
        self.add_function(name).ok_or_else(|| format!("error: redeclared identifier `{}`", name))?;
        for child in &ast.child(3)?.children
        {
            let name = &child.child(0)?.text;
            self.add_variable(name).ok_or_else(|| format!("error: redeclared identifier `{}`", name))?;
        }
        
        for statement in &ast.child(6)?.children
        {
            self.compile_any(&statement)?;
        }
        self.code.push(EXIT);
        
        self.close_frame();
        
        let position_2 = self.code.len();
        
        let body_len = position_2 - position_1;
        
        self.rewrite_code(body_len_position, pack_u64(body_len as u64))?;
        
        Ok(())
    }
    fn compile_globalfuncdef(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        let name = &ast.child(1)?.child(0)?.text;
        let nameindex = self.get_string_index(name);
        
        if self.globalstate.functions.contains_key(&nameindex)
        {
            return Err(format!("error: redeclared global function `{}`", name));
        }
        
        let oldcode = self.code.clone();
        self.code = Code::new();
        self.open_frame();
        self.add_function(name).ok_or_else(|| format!("error: redeclared identifier `{}`", name))?;
        let mut argcount = 0;
        for arg in &ast.child(3)?.children
        {
            let name = &arg.child(0)?.text;
            self.add_variable(name).ok_or_else(|| format!("error: redeclared identifier `{}`", name))?;
            argcount += 1;
        }
        for statement in &ast.child(6)?.children
        {
            self.compile_any(&statement)?;
        }
        self.code.push(EXIT);
        
        self.close_frame();
        let funccode = self.code.clone();
        self.code = oldcode;
        
        let func = FuncSpec {
            startaddr : 0,
            endaddr : funccode.code.len(),
            code : funccode,
            argcount,
            parentobj : 0,
            forcecontext : 0,
            fromobj : false,
            generator : false,
        };
        
        self.globalstate.insert_globalfunc(nameindex, func);
        
        Ok(())
    }
    
    fn compile_with(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        let name = &ast.child(2)?.child(0)?.text;
        let index = self.get_string_index(name);
        self.code.push(WITH);
        self.compile_u64(index as u64);
        
        let len_position = self.compile_u64(0);
        
        let position_1 = self.code.len();
        let myobj = self.globalstate.objects.get(&index).ok_or_else(|| format!("error: unknown object type `{}`", name))?;
        self.frames.last_mut().unwrap().objects.push(myobj.clone());
        self.compile_scope_wrapped(&|x|
        {
            x.compile_nth_child(ast, 4)?;
            x.code.push(WITHLOOP);
            Ok(())
        })?;
        self.frames.last_mut().unwrap().objects.pop();
        let position_2 = self.code.len();
        let block_len = position_2 - position_1;
        self.rewrite_code(len_position, pack_u64(block_len as u64))?;
        
        Ok(())
    }
    fn compile_withas(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_nth_child(ast, 2)?;
        self.code.push(WITHAS);
        
        let obj_name = &ast.child(4)?.child(0)?.text;
        let obj_index = self.get_string_index(obj_name);
        
        let len_position = self.compile_u64(0);
        
        let position_1 = self.code.len();
        let myobj = self.globalstate.objects.get(&obj_index).ok_or_else(|| format!("error: unknown object type `{}`", obj_name))?;
        self.frames.last_mut().unwrap().objects.push(myobj.clone());
        self.compile_scope_wrapped(&|x|
        {
            x.compile_nth_child(ast, 6)?;
            x.code.push(WITHLOOP);
            Ok(())
        })?;
        self.frames.last_mut().unwrap().objects.pop();
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
            
            if child.children.len() == 3
            {
                match decl_type
                {
                    "var" =>
                    {
                        self.compile_nth_child(child, 2)?;
                        
                        self.add_variable(name).ok_or_else(|| format!("error: redeclared identifier `{}`", name))?;
                        self.code.push(NEWVAR);
                        
                        self.compile_context_wrapped(Context::Lvar, &|x| x.compile_pushname(&child.child(0)?.child(0)?.text))?;
                        self.code.push(BINSTATE);
                        self.code.push(0x00);
                    }
                    "globalvar" =>
                    {
                        let nameindex = self.get_string_index(name);
                        if self.globalstate.variables.contains_key(&nameindex)
                        {
                            return Err(format!("error: redeclared bare global variable `{}`", name))?;;
                        }
                        self.globalstate.insert_global(nameindex);
                        
                        self.compile_nth_child(child, 2)?;
                        self.compile_pushglobal(&child.child(0)?.child(0)?.text)?;
                        self.code.push(BINSTATE);
                        self.code.push(0x00);
                    }
                    _ => return plainerr("internal error: unknown prefix to compound variable declaration")
                }
            }
            else
            {
                // declare the variable
                match decl_type
                {
                    "var" =>
                    {
                        self.add_variable(name).ok_or_else(|| format!("error: redeclared identifier `{}`", name))?;
                        self.code.push(NEWVAR);
                    }
                    "globalvar" =>
                    {
                        let nameindex = self.get_string_index(name);
                        if self.globalstate.variables.contains_key(&nameindex)
                        {
                            return Err(format!("error: redeclared bare global variable `{}`", name))?;;
                        }
                        self.globalstate.insert_global(nameindex);
                    }
                    _ => return plainerr("internal error: unknown prefix to variable declaration")
                }
            }
        }
        Ok(())
    }
    fn compile_bareglobaldec(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        // does not allow reassignment, so there's only one syntax, and it requires an expression
        let name = &ast.child(2)?.child(0)?.text;
        let nameindex = self.get_string_index(name);
        if self.globalstate.barevariables.contains_key(&nameindex)
        {
            return Err(format!("error: redeclared bare global variable `{}`", name))?;;
        }
        self.globalstate.insert_bare_global(nameindex);
        
        self.compile_nth_child(ast, 4)?;
        self.code.push(SETBAREGLOBAL);
        self.compile_string_index(&ast.child(2)?.child(0)?.text);
        Ok(())
    }
    fn compile_binstate(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        let operator = &ast.child(1)?.child(0)?.text;
        let op = get_assignment_type(operator).ok_or_else(|| minierr(&format!("internal error: unhandled or unsupported type of binary statement {}", operator)))?;
        
        self.compile_nth_child(ast, 2)?;
        self.compile_nth_child(ast, 0)?;
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
        self.compile_context_wrapped(Context::Lvar, &|x|
        {
            x.compile_nth_child(ast, 0)
        })
    }

    fn compile_rvar(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_nth_child(ast, 0)
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
    fn compile_lambda(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        let captures : &Vec<ASTNode> = &ast.child(1)?.children;
        let args : &Vec<ASTNode> = &ast.child(4)?.children;
        let statements : &Vec<ASTNode> = &ast.child(7)?.children;
        
        let mut capture_names = Vec::new();
        
        for capture in captures
        {
            capture_names.push(capture.child(0)?.child(0)?.text.clone());
            self.compile_nth_child(capture, 2)?;
        }
        
        self.open_frame();
        
        self.add_function(&"lambda_self".to_string()).ok_or_else(|| format!("error: redeclared identifier `lambda_self`"))?;
        
        self.code.push(LAMBDA);
        self.compile_u64(captures.len() as u64);
        for capture_name in capture_names.iter().rev()
        {
            self.add_variable(capture_name).ok_or_else(|| format!("error: redeclared identifier `{}`", capture_name))?;;
        }
        self.compile_u16(args.len() as u16);
        let len_position = self.compile_u64(0 as u64);
          
        for arg in args
        {
            let name = &arg.child(0)?.text;
            self.add_variable(name).ok_or_else(|| format!("error: redeclared identifier `{}`", name))?;
        }
        
        let position_1 = self.code.len();
        
        for statement in statements
        {
            self.compile_any(statement)?;
        }
                
        self.code.push(EXIT);
        
        self.close_frame();
        
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
        self.compile_u16(elementcount as u16);
        
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
        self.compile_u16(elementcount as u16);
        
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
        self.compile_u16(elementcount as u16);
        
        Ok(())
    }

    fn compile_ifcondition(&mut self, ast : &ASTNode) -> Result<(), String>
    {
        self.compile_nth_child(ast, 1)?;
        
        if ast.children.len() == 3
        {
            self.code.push(IF);
            let body_len_position = self.compile_u64(0);
            let position_1 = self.code.len();
            self.compile_nth_child(ast, 2)?;
            let position_2 = self.code.len();
            let body_len = position_2 - position_1;
            self.rewrite_code(body_len_position, pack_u64(body_len as u64))
        }
        else if ast.children.len() == 5 && ast.child(3)?.text == "else"
        {
            self.code.push(IF);
            let body_len_position = self.compile_u64(0);
            
            let position_1 = self.code.len();
            self.compile_nth_child(ast, 2)?;
            self.code.push(JUMPRELATIVE);
            let else_len_position = self.compile_u64(0);
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
            self.add_scope();
            self.compile_nth_child(header, 0)?;
        }
        
        self.code.push(FOR);
        let post_len_rewrite_pos = self.compile_u64(0);
        let expr_len_rewrite_pos = self.compile_u64(0);
        let block_len_rewrite_pos = self.compile_u64(0);
        
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
            let var_stack_length = self.pop_scope();
            self.compile_unscope(var_stack_length)?;
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
            x.compile_nth_child(ast, 4)?;
            
            x.code.push(FOREACH);
            let block_len_rewrite_pos = x.compile_u64(0);
            let position_1 = x.code.len();
            
            x.code.push(FOREACHHEAD);
            x.add_variable(&ast.child(2)?.child(0)?.text);
            
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
            self.compile_u16(which);
        }
        if ast.child_slice(1, -2)?.is_empty()
        {
            self.code.push(SWITCHDEFAULT);
            self.compile_u16(which);
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
        self.compile_u16(num_case_blocks as u16);
        let case_block_reference_point = self.code.len();
        
        for _ in 0..=num_case_blocks
        {
            self.compile_u64(0);
        }
        let block_count_rewrite_pos = self.code.len() - 8*(num_case_blocks+1);
        
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
        let block1_len_rewrite_pos = self.compile_u64(0);
        
        let position_1 = self.code.len();
        self.compile_nth_child(ast, 2)?;
        self.code.push(JUMPRELATIVE);
        let block2_len_rewrite_pos = self.compile_u64(0);
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

pub fn compile_bytecode<'a>(ast : &ASTNode, global : &'a mut GlobalState) -> Result<Code, String>
{
    let mut state = CompilerState::new(global);
    let signal = state.compile_any(ast);
    state.trap_error(signal)?;
    Ok(state.code)
}
