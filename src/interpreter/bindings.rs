#![allow(clippy::type_complexity)]

use crate::interpreter::*;

// last argument is isexpr - as of the time of writing this comment, it's used exclusively by instance_execute
// second return value is whether the frame was moved - necessary for weird functions like instance_create that implicly call user defined functions, because moving the frame to call user defined functions also moves the original stack

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
    pub (crate) fn sim_func_print(&mut self, args : Vec<Value>, _ : bool) -> Result<(Value, bool), Option<String>>
    {
        for arg in args
        {
            if let Some(string) = format_val(&arg)
            {
                println!("{}", string);
            }
            else
            {
                panic!("error: tried to print unprintable value");
            }
        }
        Ok((Value::Number(0.0), false))
    }
    pub (crate) fn sim_func_len(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), Option<String>>
    {
        if args.len() != 1
        {
            panic!("error: wrong number of arguments to len(); expected 1, got {}", args.len());
        }
        if let Some(arg) = args.pop()
        {
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
                    panic!("error: tried to take length of lengthless type");
                }
            }
        }
        else
        {
            panic!("internal error: failed to read argument for len() despite having the right number of arguments (this error should be unreachable!)");
        }
    }
    pub (crate) fn sim_func_keys(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), Option<String>>
    {
        if args.len() != 1
        {
            panic!("error: wrong number of arguments to keys(); expected 1, got {}", args.len());
        }
        if let Some(arg) = args.pop()
        {
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
                    panic!("error: tried to take length of lengthless type");
                }
            }
        }
        else
        {
            panic!("internal error: failed to read argument for keys() despite having the right number of arguments (this error should be unreachable!)");
        }
    }
    pub (crate) fn sim_func_instance_create(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), Option<String>>
    {
        if args.len() != 1
        {
            panic!("error: wrong number of arguments to instance_create(); expected 1, got {}", args.len());
        }
        if let Ok(object_id_f) = self.list_pop_number(&mut args)
        {
            let object_id = object_id_f.round() as usize;
            let instance_id = self.global.instance_id as usize;
            if let Some(object) = self.global.objects.get(&object_id)
            {
                let new = Instance { objtype : object_id, ident : instance_id, variables : hashmap!{"x".to_string() => Value::Number(0.0), "y".to_string() => Value::Number(0.0)} }; // FIXME configurable default variables?
                self.global.instances.insert(instance_id, new); // FIXME: check for id clash
                
                let mut dumbworkaround = true;
                if let Some(ref mut instance_list) = self.global.instances_by_type.get_mut(&object_id)
                {
                    instance_list.push(instance_id); // gives no clash if there is no clash abovs
                    dumbworkaround = false;
                }
                if dumbworkaround
                {
                    self.global.instances_by_type.insert(object_id, vec!(instance_id));
                }
                
                let mut frame_moved = false;
                
                if let Some(function) = object.functions.get("create")
                {
                    let pseudo_funcvar = FuncVal{internal : false, name : Some("create".to_string()), predefined : None, userdefdata : Some(function.clone())};
                    self.jump_to_function(&function.clone(), Vec::new(), false, &pseudo_funcvar)?;
                    self.top_frame.instancestack.push(instance_id);
                    frame_moved = true;
                }
                
                self.global.instance_id += 1;
                Ok((Value::Number(instance_id as f64), frame_moved))
            }
            else
            {
                panic!("error: tried to create instance of non-extant object type {}", object_id);
            }
        }
        else
        {
            panic!("error: tried to use a non-number as an object id");
        }
    }
    pub (crate) fn sim_func_instance_add_variable(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), Option<String>>
    {
        if args.len() < 2
        {
            panic!("error: wrong number of arguments to instance_add_variable(); expected 2 or more, got {}", args.len());
        }
        if let Ok(instance_id_f) = self.list_pop_number(&mut args)
        {
            let instance_id = instance_id_f.round() as usize;
            if let Ok(name) = self.list_pop_text(&mut args)
            {
                if !self.global.regex_holder.is_exact(r"[a-zA-Z_][a-zA-Z_0-9]*", &name)
                {
                    panic!("error: tried to create a variable with an invalid identifier `{}`\n(note: must exactly match the regex [a-zA-Z_][a-zA-Z_0-9]*)", name, )
                }
                let value : Value;
                if args.len() == 1
                {
                    if let Some(set_value) = args.pop()
                    {
                        value = set_value;
                    }
                    else
                    {
                        panic!("internal error: argument list was three values long but could not pop from it three times (this should be unreachable!)");
                    }
                }
                else
                {
                    value = Value::Number(0.0);
                }
                if let Some(inst) = self.global.instances.get_mut(&instance_id)
                {
                    if inst.variables.contains_key(&name)
                    {
                        panic!("error: tried to add variable to instance that already had a variable with that name")
                    }
                    inst.variables.insert(name, value);
                }
                else
                {
                    panic!("error: tried to add variable to instance {} that doesn't exist", instance_id);
                }
            }
            else
            {
                panic!("error: second argument to instance_add_variable() must be a string");
            }
        }
        else
        {
            panic!("error: first argument to instance_add_variable() must be a number");
        }
        Ok((Value::Number(0.0), false))
    }
    pub (crate) fn sim_func_instance_execute(&mut self, mut args : Vec<Value>, isexpr : bool) -> Result<(Value, bool), Option<String>>
    {
        if args.len() < 2
        {
            panic!("error: wrong number of arguments to instance_execute(); expected 2 or more, got {}", args.len());
        }
        if let Ok(instance_id_f) = self.list_pop_number(&mut args)
        {
            let instance_id = instance_id_f.round() as usize;
            if let Ok(func) = self.list_pop_func(&mut args)
            {
                if func.internal
                {
                    panic!("error: unsupported: tried to use instance_execute() with an internal function");
                }
                if let Some(ref defdata) = func.userdefdata
                {
                    if let Some(_inst) = self.global.instances.get_mut(&instance_id)
                    {
                        self.jump_to_function(defdata, args.into_iter().rev().collect(), isexpr, &func)?;
                        self.top_frame.instancestack.push(instance_id);
                    }
                    else
                    {
                        panic!("error: tried to add variable to instance {} that doesn't exist", instance_id);
                    }
                }
                else
                {
                    panic!("internal error: funcval was non-internal but had no userdefdata");
                }
            }
            else
            {
                panic!("error: second argument to instance_execute() must be a function");
            }
        }
        else
        {
            panic!("error: first argument to instance_execute() must be a number");
        }
        Ok((Value::Number(0.0), true))
    }
    pub (crate) fn sim_func_parse_text(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), Option<String>>
    {
        if args.len() != 1
        {
            panic!("error: wrong number of arguments to parse_text(); expected 1, got {}", args.len());
        }
        if let Ok(text) = self.list_pop_text(&mut args)
        {
            let program_lines : Vec<String> = text.lines().map(|x| x.to_string()).collect();
            if let Some(parser) = &mut self.global.parser
            {
                let tokens = parser.tokenize(&program_lines, true);
                if let Some(ref ast) = parser.parse_program(&tokens, &program_lines, true)
                {
                    Ok((ast_to_dict(ast), false))
                }
                else
                {
                    panic!("error: string failed to parse");
                }
            }
            else
            {
                panic!("error: first argument to parse_text() must be a string");
            }
        }
        else
        {
            panic!("error: parser was not loaded into interpreter");
        }
    }

    pub (crate) fn sim_func_compile_ast(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), Option<String>>
    {
        if args.len() != 1
        {
            panic!("error: wrong number of arguments to compile_ast(); expected 1, got {}", args.len());
        }
        if let Ok(dict) = self.list_pop_dict(&mut args)
        {
            let ast = dict_to_ast(&dict)?;
            
            let code = compile_bytecode(&ast);
            
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
        else
        {
            panic!("error: first argument to compile_ast() must be a dictionary");
        }
    }

    pub (crate) fn sim_func_compile_text(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), Option<String>>
    {
        if args.len() != 1
        {
            panic!("error: wrong number of arguments to compile_text(); expected 1, got {}", args.len());
        }
        if let Ok(text) = self.list_pop_text(&mut args)
        {
            let program_lines : Vec<String> = text.lines().map(|x| x.to_string()).collect();
            if let Some(parser) = &mut self.global.parser
            {
                let tokens = parser.tokenize(&program_lines, true);
                if let Some(ref ast) = parser.parse_program(&tokens, &program_lines, true)
                {
                    let code = compile_bytecode(ast);
                    
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
                else
                {
                    panic!("error: string failed to parse");
                }
            }
            else
            {
                panic!("error: first argument to compile_text() must be a string");
            }
        }
        else
        {
            panic!("error: parser was not loaded into interpreter");
        }
    }
}