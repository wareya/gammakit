#![allow(clippy::type_complexity)]
#![allow(clippy::cast_lossless)]

use crate::interpreter::*;
use crate::interpreter::types::ops::{float_booly, bool_floaty};

pub (crate) fn ast_to_dict(ast : &ASTNode) -> Value
{
    let mut astdict = HashMap::<HashableValue, Value>::new();
    
    macro_rules! to_key { ( $str:expr ) => { HashableValue::Text($str.to_string()) } }
    
    astdict.insert(to_key!("text"), Value::Text(ast.text.clone()));
    astdict.insert(to_key!("line"), Value::Number(ast.line as f64));
    astdict.insert(to_key!("position"), Value::Number(ast.line as f64));
    astdict.insert(to_key!("isparent"), Value::Number(bool_floaty(ast.isparent)));
    
    let children : VecDeque<Value> = ast.children.iter().map(|child| ast_to_dict(child)).collect();
    
    astdict.insert(to_key!("children"), Value::Array(children));
    
    let mut opdata = HashMap::<HashableValue, Value>::new();
    
    opdata.insert(to_key!("isop"), Value::Number(bool_floaty(ast.opdata.isop)));
    opdata.insert(to_key!("assoc"), Value::Number(ast.opdata.assoc as f64));
    opdata.insert(to_key!("precedence"), Value::Number(ast.opdata.precedence as f64));
    
    astdict.insert(to_key!("opdata"), Value::Dict(opdata));
    
    Value::Dict(astdict)
}

pub (crate) fn dict_to_ast(dict : &HashMap<HashableValue, Value>) -> Result<ASTNode, String>
{
    let mut ast = dummy_astnode();
    
    macro_rules! get { ( $as:ident, $dict:expr, $str:expr ) =>
    {
        match $dict.get(&HashableValue::Text($str.to_string())).ok_or_else(|| format!("error: tried to turn dict into ast, but dict lacked {} field", $str))?
        {
            Value::$as(this) => Ok(this),
            _ => Err(format!("error: tried to turn dict into ast, but dict's {} field was of the wrong type", $str))
        }
    } }
    
    ast.text = get!(Text, dict, "text")?.clone();
    ast.line = get!(Number, dict, "line")?.round() as usize;
    ast.position = get!(Number, dict, "position")?.round() as usize;
    ast.isparent = float_booly(*get!(Number, dict, "isparent")?);
    
    // ast.children from dummy_astnode() starts out extant but empty
    
    for child in get!(Array, dict, "children")?
    {
        let subnode = match_or_err!(child, Value::Dict(dict) => dict, minierr("error: values in list of children in ast node must be dictionaries that are themselves ast nodes"))?;
        ast.children.push(dict_to_ast(subnode)?);
    }
    
    let val_opdata = get!(Dict, dict, "opdata")?;
    ast.opdata.isop = float_booly(*get!(Number, val_opdata, "isop")?);
    ast.opdata.assoc = get!(Number, val_opdata, "assoc")?.round() as i32;
    ast.opdata.precedence = get!(Number, val_opdata, "precedence")?.round() as i32;
    
    Ok(ast)
}

impl Interpreter
{
    pub fn insert_normal_internal_func(&mut self, funcname : String, func : Rc<InternalFunction>)
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
        macro_rules! enrc { ( $y:ident ) => { Rc::new(Interpreter::$y) } }
        macro_rules! insert { ( $x:expr, $y:ident ) => { self.insert_normal_internal_func($x.to_string(), enrc!($y)); } }
        macro_rules! insert_noreturn { ( $x:expr, $y:ident ) => { self.insert_noreturn_internal_func($x.to_string(), enrc!($y)); } }
        
        insert!("print"                 , sim_func_print                );
        insert!("len"                   , sim_func_len                  );
        insert!("keys"                  , sim_func_keys                 );
        insert!("parse_text"            , sim_func_parse_text           );
        insert!("compile_text"          , sim_func_compile_text         );
        insert!("compile_ast"           , sim_func_compile_ast          );
        insert!("instance_create"       , sim_func_instance_create      );
        insert!("instance_add_variable" , sim_func_instance_add_variable);
        insert!("insert"                , sim_func_insert               );
        insert!("remove"                , sim_func_remove               );
        insert!("contains"              , sim_func_contains             );
        insert!("round"                 , sim_func_round                );
        insert!("floor"                 , sim_func_floor                );
        insert!("ceil"                  , sim_func_ceil                 );
        
        insert_noreturn!("instance_execute", sim_func_instance_execute);
    }
    pub (crate) fn get_internal_function(&self, name : &str) -> Option<Rc<InternalFunction>>
    {
        match_or_none!(self.internal_functions.get(name), Some(f) => Rc::clone(f))
    }
    pub (crate) fn internal_function_is_noreturn(&mut self, name : &str) -> bool
    {
        self.internal_functions_noreturn.contains(name)
    }
    // last argument is isexpr - as of the time of writing this comment, it's used exclusively by instance_execute
    // second return value is whether the frame was moved - necessary for weird functions like instance_create that implicly call user defined functions, because moving the frame to call user defined functions also moves the original stack
    pub (crate) fn sim_func_print(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), String>
    {
        for arg in args.drain(..).rev()
        {
            let formatted = format_val(&arg).ok_or_else(|| minierr("error: tried to print unprintable value"))?;
            println!("{}", formatted);
        }
        Ok((Value::Number(0.0), false))
    }
    pub (crate) fn sim_func_len(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to len(); expected 1, got {}", args.len()));
        }
        let arg = args.pop().ok_or_else(|| minierr("internal error: this should be unreachable"))?;
        match arg
        {
            Value::Text(string) => Ok((Value::Number(string.chars().count() as f64), false)),
            Value::Array(array) => Ok((Value::Number(array.len() as f64), false)),
            Value::Dict(dict) => Ok((Value::Number(dict.keys().len() as f64), false)),
            _ => plainerr("error: tried to take length of lengthless type")
        }
    }
    pub (crate) fn sim_func_keys(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to keys(); expected 1, got {}", args.len()));
        }
        let arg = args.pop().ok_or_else(|| minierr("internal error: this should be unreachable"))?;
        match arg
        {
            Value::Array(array) =>
            {
                let list = (0..array.len()).map(|i| Value::Number(i as f64)).collect();
                Ok((Value::Array(list), false))
            }
            Value::Dict(dict) =>
            {
                let list = dict.keys().map(|key| hashval_to_val(key)).collect();
                Ok((Value::Array(list), false))
            }
            _ => plainerr("error: tried to take length of lengthless type")
        }
    }
    pub (crate) fn sim_func_insert(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), String>
    {
        if !matches!(args.len(), 3 | 2)
        {
            return Err(format!("error: wrong number of arguments to insert(); expected 3 or 2, got {}", args.len()));
        }
        let collection = args.pop().ok_or_else(|| minierr("internal error: this should be unreachable"))?;
        let key = args.pop().ok_or_else(|| minierr("internal error: this should be unreachable"))?;
        match collection
        {
            Value::Array(mut array) =>
            {
                let value = args.pop().ok_or_else(|| minierr("error: insert() with an array also requires a value to insert at the given index"))?;
                let index = match_or_err!(key, Value::Number(index) => index.round() as isize, minierr("error: tried to insert into an array with a non-number index"))?;
                if index < 0 || index as usize > array.len()
                {
                    return plainerr("error: tried to insert into an array at an out-of-range index");
                }
                array.insert(index as usize, value);
                Ok((Value::Array(array), false))
            }
            Value::Dict(mut dict) =>
            {
                let value = args.pop().ok_or_else(|| minierr("error: insert() with a dict also requires a value to insert at the given key"))?;
                dict.insert(val_to_hashval(key)?, value);
                Ok((Value::Dict(dict), false))
            }
            Value::Set(mut set) =>
            {
                if !args.is_empty()
                {
                    return plainerr("error: insert() with a set must not be called with a third argument");
                }
                set.insert(val_to_hashval(key)?);
                Ok((Value::Set(set), false))
            }
            _ => plainerr("error: insert() must be called with an array, dictionary, or set as the first argument")
        }
    }
    pub (crate) fn sim_func_remove(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), String>
    {
        if args.len() != 2
        {
            return Err(format!("error: wrong number of arguments to remove(); expected 2, got {}", args.len()));
        }
        let collection = args.pop().ok_or_else(|| minierr("internal error: this should be unreachable"))?;
        let key = args.pop().ok_or_else(|| minierr("internal error: this should be unreachable"))?;
        match collection
        {
            Value::Array(mut array) =>
            {
                let index = match_or_err!(key, Value::Number(index) => index.round() as isize, minierr("error: tried to remove from an array with a non-number index"))?;
                if index < 0 || index as usize > array.len()
                {
                    return plainerr("error: tried to remove from an array at an out-of-range index");
                }
                array.remove(index as usize);
                Ok((Value::Array(array), false))
            }
            Value::Dict(mut dict) =>
            {
                dict.remove(&val_to_hashval(key)?);
                Ok((Value::Dict(dict), false))
            }
            Value::Set(mut set) =>
            {
                set.remove(&val_to_hashval(key)?);
                Ok((Value::Set(set), false))
            }
            _ => plainerr("error: remove() must be called with an array, dictionary, or set as its argument")
        }
    }
    pub (crate) fn sim_func_contains(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), String>
    {
        if args.len() != 2
        {
            return Err(format!("error: wrong number of arguments to contains(); expected 2, got {}", args.len()));
        }
        let collection = args.pop().ok_or_else(|| minierr("internal error: this should be unreachable"))?;
        let key = args.pop().ok_or_else(|| minierr("internal error: this should be unreachable"))?;
        match collection
        {
            Value::Dict(dict) => Ok((Value::Number(bool_floaty(dict.contains_key(&val_to_hashval(key)?))), false)),
            Value::Set (set ) => Ok((Value::Number(bool_floaty(set .contains    (&val_to_hashval(key)?))), false)),
            _ => plainerr("error: remove() must be called with an array, dictionary, or set as its argument")
        }
    }
    pub (crate) fn sim_func_round(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), String>
    {
        let val = args.pop().ok_or_else(|| minierr("error: wrong number of arguments to round(); expected 1, got 0"))?;
        let num = match_or_err!(val, Value::Number(num) => num, minierr("error: round() must be called with a number as its argument"))?;
        Ok((Value::Number(num.round()), false))
    }
    pub (crate) fn sim_func_ceil(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), String>
    {
        let val = args.pop().ok_or_else(|| minierr("error: wrong number of arguments to ceil(); expected 1, got 0"))?;
        let num = match_or_err!(val, Value::Number(num) => num, minierr("error: ceil() must be called with a number as its argument"))?;
        Ok((Value::Number(num.ceil()), false))
    }
    pub (crate) fn sim_func_floor(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), String>
    {
        let val = args.pop().ok_or_else(|| minierr("error: wrong number of arguments to floor(); expected 1, got 0"))?;
        let num = match_or_err!(val, Value::Number(num) => num, minierr("error: floor() must be called with a number as its argument"))?;
        Ok((Value::Number(num.floor()), false))
    }
    pub (crate) fn sim_func_instance_create(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to instance_create(); expected 1, got {}", args.len()));
        }
        
        let object_id = self.list_pop_object(&mut args).or_else(|_| plainerr("error: first argument to instance_create() must be an object"))?;
        let instance_id = self.global.instance_id as usize;
        let object = self.global.objects.get(&object_id).ok_or_else(|| format!("error: tried to create instance of non-extant object type {}", object_id))?;
        
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
            Ok((Value::Instance(instance_id), true))
        }
        else
        {
            Ok((Value::Instance(instance_id), false))
        }
    }
    pub (crate) fn sim_func_instance_add_variable(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), String>
    {
        if args.len() < 2 || args.len() > 3
        {
            return Err(format!("error: wrong number of arguments to instance_add_variable(); expected 2 to 3, got {}", args.len()));
        }
        let instance_id = self.list_pop_instance(&mut args).or_else(|_| plainerr("error: first argument to instance_add_variable() must be an instance"))?;
        let name = self.list_pop_text(&mut args).or_else(|_| plainerr("error: second argument to instance_add_variable() must be a string"))?;
        
        if !self.global.regex_holder.is_exact(r"[a-zA-Z_][a-zA-Z_0-9]*", &name)
        {
            return Err(format!("error: tried to create a variable with an invalid identifier `{}`\n(note: must exactly match the regex [a-zA-Z_][a-zA-Z_0-9]*)", name));
        }
        let inst = self.global.instances.get_mut(&instance_id).ok_or_else(|| format!("error: tried to add variable to instance {} that doesn't exist", instance_id))?;
        if inst.variables.contains_key(&name)
        {
            return plainerr("error: tried to add variable to instance that already had a variable with that name")
        }
        inst.variables.insert(name, args.pop().unwrap_or(Value::Number(0.0)));
        
        Ok((Value::Number(0.0), false))
    }
    pub (crate) fn sim_func_instance_execute(&mut self, mut args : Vec<Value>, isexpr : bool) -> Result<(Value, bool), String>
    {
        if args.len() < 2
        {
            return Err(format!("error: wrong number of arguments to instance_execute(); expected 2 or more, got {}", args.len()));
        }
        let instance_id = self.list_pop_instance(&mut args).or_else(|_| plainerr("error: first argument to instance_execute() must be a number"))?;
        let func = self.list_pop_func(&mut args).or_else(|_| plainerr("error: second argument to instance_execute() must be a function"))?;
        
        if func.internal
        {
            return plainerr("error: unsupported: tried to use instance_execute() with an internal function");
        }
        let defdata = func.userdefdata.as_ref().ok_or_else(|| minierr("internal error: funcval was non-internal but had no userdefdata"))?;
        if defdata.generator
        {
            return plainerr("error: cannot use instance_execute with a generator");
        }
        self.global.instances.get_mut(&instance_id).ok_or_else(|| format!("error: tried to execute function with instance {} that doesn't exist", instance_id))?;
        
        self.jump_to_function(defdata, args.into_iter().rev().collect(), isexpr, &func)?;
        self.top_frame.instancestack.push(instance_id);
        
        Ok((Value::Number(0.0), true))
    }
    pub (crate) fn sim_func_parse_text(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to parse_text(); expected 1, got {}", args.len()));
        }
        
        let text = self.list_pop_text(&mut args).or_else(|_| plainerr("error: first argument to parse_text() must be a string"))?;
        let parser = self.global.parser.as_mut().ok_or_else(|| minierr("error: parser was not loaded into interpreter"))?;
        
        let program_lines : Vec<String> = text.lines().map(|x| x.to_string()).collect();
        let tokens = parser.tokenize(&program_lines, true)?;
        
        let ast = parser.parse_program(&tokens, &program_lines, true)?.ok_or_else(|| minierr("error: string failed to parse"))?;
        
        Ok((ast_to_dict(&ast), false))
    }

    pub (crate) fn sim_func_compile_ast(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to compile_ast(); expected 1, got {}", args.len()));
        }
        
        let dict = self.list_pop_dict(&mut args).or_else(|_| plainerr("error: first argument to compile_ast() must be a dictionary"))?;
        let ast = dict_to_ast(&dict)?;
        let code = compile_bytecode(&ast)?;
        
        // endaddr at the start because Rc::new() moves `code`
        Ok(
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
              impassable : true,
              generator : false,
            }
            )), false)
        )
    }

    pub (crate) fn sim_func_compile_text(&mut self, mut args : Vec<Value>, _ : bool) -> Result<(Value, bool), String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to compile_text(); expected 1, got {}", args.len()));
        }
        let text = self.list_pop_text(&mut args).or_else(|_| plainerr("error: first argument to compile_text() must be a string"))?;
        
        let program_lines : Vec<String> = text.lines().map(|x| x.to_string()).collect();
        let parser = self.global.parser.as_mut().ok_or_else(|| minierr("error: parser was not loaded into interpreter"))?;
        
        let tokens = parser.tokenize(&program_lines, true)?;
        let ast = parser.parse_program(&tokens, &program_lines, true)?.ok_or_else(|| minierr("error: string failed to parse"))?;
        
        let code = compile_bytecode(&ast)?;
        
        // endaddr at the start because Rc::new() moves `code`
        Ok(
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
              impassable : true,
              generator : false,
            }
            )), false)
        )
    }
}