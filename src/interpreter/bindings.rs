#![allow(clippy::type_complexity)]
#![allow(clippy::cast_lossless)]

use crate::interpreter::*;
use crate::interpreter::types::ops::{float_booly, bool_floaty};

pub (crate) fn ast_to_dict(ast : &ASTNode) -> Value
{
    let mut astdict = HashMap::<HashableValue, Value>::new();
    
    macro_rules! to_key {
        ( $str:expr ) =>
        {
            HashableValue::Text($str.to_string())
        }
    }
    
    astdict.insert(to_key!("text"), Value::Text(ast.text.clone()));
    astdict.insert(to_key!("line"), Value::Number(ast.line as f64));
    astdict.insert(to_key!("position"), Value::Number(ast.line as f64));
    astdict.insert(to_key!("isparent"), Value::Number(bool_floaty(ast.isparent)));
    
    let mut children = VecDeque::<Value>::new();
    
    for child in &ast.children
    {
        children.push_back(ast_to_dict(&child));
    }
    
    astdict.insert(to_key!("children"), Value::Array(children));
    
    let mut opdata = HashMap::<HashableValue, Value>::new();
    
    opdata.insert(to_key!("isop"), Value::Number(bool_floaty(ast.opdata.isop)));
    opdata.insert(to_key!("assoc"), Value::Number(ast.opdata.assoc as f64));
    opdata.insert(to_key!("precedence"), Value::Number(ast.opdata.precedence as f64));
    
    astdict.insert(to_key!("opdata"), Value::Dict(opdata));
    
    Value::Dict(astdict)
}

pub (crate) fn dict_to_ast(dict : &HashMap<HashableValue, Value>) -> Result<ASTNode, Option<String>>
{
    let mut ast = dummy_astnode();
    
    macro_rules! get {
        ( $dict:expr, $str:expr ) =>
        {
            $dict.get(&HashableValue::Text($str.to_string()))
        }
    }
    
    macro_rules! handle {
        ( $into:expr, $dict:expr, $str:expr, $strident:ident, $subtype:ident, $helper:ident, $cast:ident, $errortext:expr ) =>
        {
            if let Some(Value::$subtype($strident)) = get!($dict, $str)
            {
                $into.$strident = $strident.$helper() as $cast;
            }
            else
            {
                return Err(Some(format!("error: tried to turn a dict into an ast but dict lacked \"{}\" field or the \"{}\" field was not {}", $str, $str, $errortext)));
            }
        }
    }
    
    handle!(ast, dict, "text", text, Text, clone, String, "a string");
    handle!(ast, dict, "line", line, Number, round, usize, "a number");
    handle!(ast, dict, "position", position, Number, round, usize, "a number");
    if let Some(Value::Number(isparent)) = get!(dict, "isparent")
    {
        ast.isparent = float_booly(*isparent);
    }
    else
    {
        return plainerr("error: tried to turn a dict into an ast but dict lacked \"isparent\" field or the \"isparent\" field was not a number");
    }
    
    if let Some(Value::Array(val_children)) = get!(dict, "children")
    {
        // ast.children from dummy_astnode() starts out extant but empty
        for child in val_children
        {
            if let Value::Dict(dict) = child
            {
                ast.children.push(dict_to_ast(&dict)?);
            }
            else
            {
                return plainerr("error: values in list of children in ast node must be dictionaries that are themselves ast nodes");
            }
        }
    }
    else
    {
        return plainerr("error: tried to turn a dict into an ast but dict lacked \"children\" field or the \"children\" field was not a list");
    }
    
    if let Some(Value::Dict(val_opdata)) = get!(dict, "opdata")
    {
        if let Some(Value::Number(isop)) = get!(val_opdata, "isop")
        {
            ast.opdata.isop = float_booly(*isop);
        }
        else
        {
            return plainerr("error: tried to turn a dict into an ast but dict's opdata lacked \"isop\" field or the \"isop\" field was not a number");
        }
        if let Some(Value::Number(assoc)) = get!(val_opdata, "assoc")
        {
            ast.opdata.assoc = assoc.round() as i32;
        }
        else
        {
            return plainerr("error: tried to turn a dict into an ast but dict's opdata lacked \"assoc\" field or the \"assoc\" field was not a number");
        }
        if let Some(Value::Number(precedence)) = get!(val_opdata, "precedence")
        {
            ast.opdata.precedence = precedence.round() as i32;
        }
        else
        {
            return plainerr("error: tried to turn a dict into an ast but dict's opdata lacked \"precedence\" field or the \"precedence\" field was not a number");
        }
    }
    else
    {
        return plainerr("error: tried to turn a dict into an ast but dict lacked \"opdata\" field or the \"opdata\" field was not a dictionary");
    }
    
    Ok(ast)
}

impl Interpreter
{
    fn insert_normal_internal_func(&mut self, funcname : String, func : Rc<InternalFunction>)
    {
        self.internal_functions.insert(funcname, func);
    }
    fn insert_noreturn_internal_func(&mut self, funcname : String, func : Rc<InternalFunction>)
    {
        self.internal_functions_noreturn.insert(funcname.clone());
        self.internal_functions.insert(funcname, func);
    }
    
    pub (crate) fn insert_default_internal_functions(&mut self)
    {
        macro_rules! enrc {
            ( $y:ident ) =>
            {
                Rc::new(Interpreter::$y)
            }
        }
        macro_rules! insert {
            ( $x:expr, $y:ident ) =>
            {
                self.insert_normal_internal_func($x.to_string(), enrc!($y));
            }
        }
        macro_rules! insert_noreturn {
            ( $x:expr, $y:ident ) =>
            {
                self.insert_noreturn_internal_func($x.to_string(), enrc!($y));
            }
        }
        insert!("print"                 , sim_func_print                );
        insert!("len"                   , sim_func_len                  );
        insert!("keys"                  , sim_func_keys                 );
        insert!("parse_text"            , sim_func_parse_text           );
        insert!("compile_text"          , sim_func_compile_text         );
        insert!("compile_ast"           , sim_func_compile_ast          );
        insert!("instance_create"       , sim_func_instance_create      );
        insert!("instance_add_variable" , sim_func_instance_add_variable);
        
        insert_noreturn!("instance_execute", sim_func_instance_execute);
    }
    pub (crate) fn get_internal_function(&self, name : &str) -> Option<Rc<InternalFunction>>
    {
        match self.internal_functions.get(name)
        {
            Some(f) => Some(Rc::clone(f)),
            None => None
        }
    }
    pub (crate) fn internal_function_is_noreturn(&mut self, name : &str) -> bool
    {
        self.internal_functions_noreturn.contains(name)
    }
    // last argument is isexpr - as of the time of writing this comment, it's used exclusively by instance_execute
    // second return value is whether the frame was moved - necessary for weird functions like instance_create that implicly call user defined functions, because moving the frame to call user defined functions also moves the original stack
    pub (crate) fn sim_func_print(&mut self, args : Vec<Value>, _ : bool) -> Result<(Value, bool), Option<String>>
    {
        for arg in args
        {
            let formatted = format_val(&arg).ok_or(minierr("error: tried to print unprintable value"))?;
            println!("{}", formatted);
        }
        Ok((Value::Number(0.0), false))
    }
    pub (crate) fn sim_func_len(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), Option<String>>
    {
        if args.len() != 1
        {
            return Err(Some(format!("error: wrong number of arguments to len(); expected 1, got {}", args.len())));
        }
        let arg = args.pop().ok_or(minierr("internal error: this should be unreachable"))?;
        match arg
        {
            Value::Text(string) =>
            {
                Ok((Value::Number(string.chars().count() as f64), false))
            }
            Value::Array(array) =>
            {
                Ok((Value::Number(array.len() as f64), false))
            }
            Value::Dict(dict) =>
            {
                Ok((Value::Number(dict.keys().len() as f64), false))
            }
            _ =>
            {
                return plainerr("error: tried to take length of lengthless type");
            }
        }
    }
    pub (crate) fn sim_func_keys(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), Option<String>>
    {
        if args.len() != 1
        {
            return Err(Some(format!("error: wrong number of arguments to keys(); expected 1, got {}", args.len())));
        }
        let arg = args.pop().ok_or(minierr("internal error: this should be unreachable"))?;
        match arg
        {
            Value::Array(array) =>
            {
                let mut list = VecDeque::<Value>::new();
                for i in 0..array.len()
                {
                    list.push_back(Value::Number(i as f64));
                }
                return Ok((Value::Array(list), false));
            }
            Value::Dict(dict) =>
            {
                let mut list = VecDeque::<Value>::new();
                for key in dict.keys()
                {
                    list.push_back(hashval_to_val(key));
                }
                return Ok((Value::Array(list), false));
            }
            _ =>
            {
                return plainerr("error: tried to take length of lengthless type");
            }
        }
    }
    pub (crate) fn sim_func_instance_create(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), Option<String>>
    {
        if args.len() != 1
        {
            return Err(Some(format!("error: wrong number of arguments to instance_create(); expected 1, got {}", args.len())));
        }
        
        let object_id_f = self.list_pop_number(&mut args).or(plainerr("error: tried to use a non-number as an object id"))?;
        let object_id = object_id_f.round() as usize;
        let instance_id = self.global.instance_id as usize;
        let object = self.global.objects.get(&object_id).ok_or(Some(format!("error: tried to create instance of non-extant object type {}", object_id)))?;
        
        let mut variables = HashMap::new();
        // FIXME configurable default variables?
        variables.insert("x".to_string(), Value::Number(0.0));
        variables.insert("y".to_string(), Value::Number(0.0));
        let new = Instance { objtype : object_id, ident : instance_id, variables };
        self.global.instances.insert(instance_id, new); // FIXME: check for id clash
        
        if let Some(ref mut instance_list) = self.global.instances_by_type.get_mut(&object_id)
        {
            instance_list.push(instance_id); // gives no clash if there is no clash abovs
        }
        else
        {
            self.global.instances_by_type.insert(object_id, vec!(instance_id));
        }
        
        self.global.instance_id += 1;
        
        if let Some(function) = object.functions.get("create")
        {
            let pseudo_funcvar = FuncVal{internal : false, name : Some("create".to_string()), predefined : None, userdefdata : Some(function.clone())};
            self.jump_to_function(&function.clone(), Vec::new(), false, &pseudo_funcvar)?;
            self.top_frame.instancestack.push(instance_id);
            Ok((Value::Number(instance_id as f64), true))
        }
        else
        {
            Ok((Value::Number(instance_id as f64), false))
        }
    }
    pub (crate) fn sim_func_instance_add_variable(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), Option<String>>
    {
        if args.len() < 2 || args.len() > 3
        {
            return Err(Some(format!("error: wrong number of arguments to instance_add_variable(); expected 2 to 3, got {}", args.len())));
        }
        let instance_id_f = self.list_pop_number(&mut args).or(plainerr("error: first argument to instance_add_variable() must be a number"))?;
        let instance_id = instance_id_f.round() as usize;
        let name = self.list_pop_text(&mut args).or(plainerr("error: second argument to instance_add_variable() must be a string"))?;
        
        if !self.global.regex_holder.is_exact(r"[a-zA-Z_][a-zA-Z_0-9]*", &name)
        {
            return Err(Some(format!("error: tried to create a variable with an invalid identifier `{}`\n(note: must exactly match the regex [a-zA-Z_][a-zA-Z_0-9]*)", name)));
        }
        let inst = self.global.instances.get_mut(&instance_id).ok_or(Some(format!("error: tried to add variable to instance {} that doesn't exist", instance_id)))?;
        if inst.variables.contains_key(&name)
        {
            return plainerr("error: tried to add variable to instance that already had a variable with that name")
        }
        inst.variables.insert(name, args.pop().unwrap_or(Value::Number(0.0)));
        
        Ok((Value::Number(0.0), false))
    }
    pub (crate) fn sim_func_instance_execute(&mut self, mut args : Vec<Value>, isexpr : bool) -> Result<(Value, bool), Option<String>>
    {
        if args.len() < 2
        {
            return Err(Some(format!("error: wrong number of arguments to instance_execute(); expected 2 or more, got {}", args.len())));
        }
        let instance_id_f = self.list_pop_number(&mut args).or(plainerr("error: first argument to instance_execute() must be a number"))?;
        let instance_id = instance_id_f.round() as usize;
        let func = self.list_pop_func(&mut args).or(plainerr("error: second argument to instance_execute() must be a function"))?;
        
        if func.internal
        {
            return plainerr("error: unsupported: tried to use instance_execute() with an internal function");
        }
        let defdata = func.userdefdata.as_ref().ok_or(minierr("internal error: funcval was non-internal but had no userdefdata"))?;
        
        self.global.instances.get_mut(&instance_id).ok_or(Some(format!("error: tried to execute function with instance {} that doesn't exist", instance_id)))?;
        
        self.jump_to_function(defdata, args.into_iter().rev().collect(), isexpr, &func)?;
        self.top_frame.instancestack.push(instance_id);
        
        Ok((Value::Number(0.0), true))
    }
    pub (crate) fn sim_func_parse_text(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), Option<String>>
    {
        if args.len() != 1
        {
            return Err(Some(format!("error: wrong number of arguments to parse_text(); expected 1, got {}", args.len())));
        }
        
        let text = self.list_pop_text(&mut args).or(plainerr("error: first argument to parse_text() must be a string"))?;
        let parser = self.global.parser.as_mut().ok_or(minierr("error: parser was not loaded into interpreter"))?;
        
        let program_lines : Vec<String> = text.lines().map(|x| x.to_string()).collect();
        let tokens = parser.tokenize(&program_lines, true)?;
        
        let ast = parser.parse_program(&tokens, &program_lines, true)?.ok_or(minierr("error: string failed to parse"))?;
        
        Ok((ast_to_dict(&ast), false))
    }

    pub (crate) fn sim_func_compile_ast(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), Option<String>>
    {
        if args.len() != 1
        {
            return Err(Some(format!("error: wrong number of arguments to compile_ast(); expected 1, got {}", args.len())));
        }
        
        let dict = self.list_pop_dict(&mut args).or(plainerr("error: first argument to compile_ast() must be a dictionary"))?;
        let ast = dict_to_ast(&dict)?;
        let code = compile_bytecode(&ast)?;
        
        // endaddr at the start because Rc::new() moves `code`
        return Ok(
        ( Value::new_funcval
          ( false,
            None,
            None,
            Some(FuncSpec
            { endaddr : code.len(), // must be before code : Rc::new(code)
              varnames : Vec::new(),
              code : Rc::new(code),
              startaddr : 0,
              fromobj : false,
              parentobj : 0,
              forcecontext : 0,
              location : self.build_funcspec_location(),
              impassable : true,
            }
            )), false)
        );
    }

    pub (crate) fn sim_func_compile_text(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), Option<String>>
    {
        if args.len() != 1
        {
            return Err(Some(format!("error: wrong number of arguments to compile_text(); expected 1, got {}", args.len())));
        }
        let text = self.list_pop_text(&mut args).or(plainerr("error: first argument to compile_text() must be a string"))?;
        
        let program_lines : Vec<String> = text.lines().map(|x| x.to_string()).collect();
        let parser = self.global.parser.as_mut().ok_or(minierr("error: parser was not loaded into interpreter"))?;
        
        let tokens = parser.tokenize(&program_lines, true)?;
        let ast = parser.parse_program(&tokens, &program_lines, true)?.ok_or(minierr("error: string failed to parse"))?;
        
        let code = compile_bytecode(&ast)?;
        
        // endaddr at the start because Rc::new() moves `code`
        return Ok(
        ( Value::new_funcval
          ( false,
            None,
            None,
            Some(FuncSpec
            { endaddr : code.len(), // must be before code : Rc::new(code)
              varnames : Vec::new(),
              code : Rc::new(code),
              startaddr : 0,
              fromobj : false,
              parentobj : 0,
              forcecontext : 0,
              location : self.build_funcspec_location(),
              impassable : true
            }
            )), false)
        );
    }
}