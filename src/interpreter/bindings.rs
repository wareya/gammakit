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
    
    if let Some(precedence) = ast.precedence
    {
        astdict.insert(to_key!("precedence"), Value::Number(precedence as f64));
    }
    
    Value::Dict(astdict)
}

pub (crate) fn dict_to_ast(dict : &HashMap<HashableValue, Value>) -> Result<ASTNode, String>
{
    let mut ast = dummy_astnode();
    
    macro_rules! get { ( $as:ident, $dict:expr, $str:expr ) =>
    {
        match $dict.get(&HashableValue::Text($str.to_string()))
        {
            Some(Value::$as(this)) => Ok(this),
            Some(_) => Err(format!("error: tried to turn dict into ast, but dict's {} field was of the wrong type", $str)),
            _ => Err(format!("error: tried to turn dict into ast, but dict lacked {} field", $str))
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
    
    ast.precedence = get!(Number, dict, "precedence").map(|x| x.round() as u64).ok();
    
    Ok(ast)
}

impl Interpreter
{
    pub fn insert_binding(&mut self, funcname : String, func : Rc<RefCell<Binding>>)
    {
        self.simple_bindings.remove(&funcname);
        self.bindings.insert(funcname, func);
    }
    pub fn insert_simple_binding(&mut self, funcname : String, func : Rc<RefCell<SimpleBinding>>)
    {
        self.bindings.remove(&funcname);
        self.simple_bindings.insert(funcname, func);
    }
    
    pub fn insert_default_bindings(&mut self)
    {
        macro_rules! enrc { ( $y:ident ) => { Rc::new(RefCell::new(Interpreter::$y)) } }
        macro_rules! insert { ( $x:expr, $y:ident ) => { self.insert_binding($x.to_string(), enrc!($y)); } }
        
        insert!("print"                 , sim_func_print                );
        insert!("printraw"              , sim_func_printraw             );
        insert!("len"                   , sim_func_len                  );
        insert!("keys"                  , sim_func_keys                 );
        insert!("parse_text"            , sim_func_parse_text           );
        insert!("compile_text"          , sim_func_compile_text         );
        insert!("compile_ast"           , sim_func_compile_ast          );
        insert!("instance_create"       , sim_func_instance_create      );
        insert!("instance_exists"       , sim_func_instance_exists      );
        insert!("instance_kill"         , sim_func_instance_kill        );
        insert!("insert"                , sim_func_insert               );
        insert!("remove"                , sim_func_remove               );
        insert!("contains"              , sim_func_contains             );
        insert!("round"                 , sim_func_round                );
        insert!("floor"                 , sim_func_floor                );
        insert!("ceil"                  , sim_func_ceil                 );
    }
    pub (crate) fn get_binding(&self, name : &str) -> Option<Rc<RefCell<Binding>>>
    {
        match_or_none!(self.bindings.get(name), Some(f) => Rc::clone(f))
    }
    pub (crate) fn get_simple_binding(&self, name : &str) -> Option<Rc<RefCell<SimpleBinding>>>
    {
        match_or_none!(self.simple_bindings.get(name), Some(f) => Rc::clone(f))
    }
    pub (crate) fn sim_func_print(&mut self, mut args : VecDeque<Value>) -> Result<Value, String>
    {
        for arg in args.drain(..)
        {
            let formatted = format_val(&arg).ok_or_else(|| minierr("error: tried to print unprintable value"))?;
            println!("{}", formatted);
        }
        Ok(Value::Number(0.0))
    }
    pub (crate) fn sim_func_printraw(&mut self, mut args : VecDeque<Value>) -> Result<Value, String>
    {
        for arg in args.drain(..)
        {
            let formatted = format_val(&arg).ok_or_else(|| minierr("error: tried to print unprintable value"))?;
            print!("{}", formatted);
        }
        Ok(Value::Number(0.0))
    }
    pub (crate) fn sim_func_len(&mut self, mut args : VecDeque<Value>) -> Result<Value, String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to len(); expected 1, got {}", args.len()));
        }
        let arg = args.pop_front().ok_or_else(|| minierr("internal error: this should be unreachable"))?;
        match arg
        {
            Value::Text(string) => Ok(Value::Number(string.chars().count() as f64)),
            Value::Array(array) => Ok(Value::Number(array.len() as f64)),
            Value::Dict(dict) => Ok(Value::Number(dict.keys().len() as f64)),
            Value::Set(set) => Ok(Value::Number(set.len() as f64)),
            _ => plainerr("error: tried to take length of lengthless type")
        }
    }
    pub (crate) fn sim_func_keys(&mut self, mut args : VecDeque<Value>) -> Result<Value, String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to keys(); expected 1, got {}", args.len()));
        }
        let arg = args.pop_front().ok_or_else(|| minierr("internal error: this should be unreachable"))?;
        match arg
        {
            Value::Array(array) =>
            {
                Ok(Value::Array((0..array.len()).map(|i| Value::Number(i as f64)).collect()))
            }
            Value::Dict(mut dict) =>
            {
                Ok(Value::Array(dict.drain().map(|(key, _)| hashval_to_val(key)).collect()))
            }
            _ => plainerr("error: tried to take length of lengthless type")
        }
    }
    pub (crate) fn sim_func_insert(&mut self, mut args : VecDeque<Value>) -> Result<Value, String>
    {
        if !matches!(args.len(), 3 | 2)
        {
            return Err(format!("error: wrong number of arguments to insert(); expected 3 or 2, got {}", args.len()));
        }
        let collection = args.pop_front().ok_or_else(|| minierr("internal error: this should be unreachable"))?;
        let key = args.pop_front().ok_or_else(|| minierr("internal error: this should be unreachable"))?;
        match collection
        {
            Value::Array(mut array) =>
            {
                let value = args.pop_front().ok_or_else(|| minierr("error: insert() with an array also requires a value to insert at the given index"))?;
                let index = match_or_err!(key, Value::Number(index) => index.round() as isize, minierr("error: tried to insert into an array with a non-number index"))?;
                if index < 0 || index as usize > array.len()
                {
                    return plainerr("error: tried to insert into an array at an out-of-range index");
                }
                array.insert(index as usize, value);
                Ok(Value::Array(array))
            }
            Value::Dict(mut dict) =>
            {
                let value = args.pop_front().ok_or_else(|| minierr("error: insert() with a dict also requires a value to insert at the given key"))?;
                dict.insert(val_to_hashval(key)?, value);
                Ok(Value::Dict(dict))
            }
            Value::Set(mut set) =>
            {
                if !args.is_empty()
                {
                    return plainerr("error: insert() with a set must not be called with a third argument");
                }
                set.insert(val_to_hashval(key)?);
                Ok(Value::Set(set))
            }
            _ => plainerr("error: insert() must be called with an array, dictionary, or set as the first argument")
        }
    }
    pub (crate) fn sim_func_remove(&mut self, mut args : VecDeque<Value>) -> Result<Value, String>
    {
        if args.len() != 2
        {
            return Err(format!("error: wrong number of arguments to remove(); expected 2, got {}", args.len()));
        }
        let collection = args.pop_front().ok_or_else(|| minierr("internal error: this should be unreachable"))?;
        let key = args.pop_front().ok_or_else(|| minierr("internal error: this should be unreachable"))?;
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
                Ok(Value::Array(array))
            }
            Value::Dict(mut dict) =>
            {
                dict.remove(&val_to_hashval(key)?);
                Ok(Value::Dict(dict))
            }
            Value::Set(mut set) =>
            {
                set.remove(&val_to_hashval(key)?);
                Ok(Value::Set(set))
            }
            _ => plainerr("error: remove() must be called with an array, dictionary, or set as its argument")
        }
    }
    pub (crate) fn sim_func_contains(&mut self, mut args : VecDeque<Value>) -> Result<Value, String>
    {
        if args.len() != 2
        {
            return Err(format!("error: wrong number of arguments to contains(); expected 2, got {}", args.len()));
        }
        let collection = args.pop_front().ok_or_else(|| minierr("internal error: this should be unreachable"))?;
        let key = args.pop_front().ok_or_else(|| minierr("internal error: this should be unreachable"))?;
        match collection
        {
            Value::Dict(dict) => Ok(Value::Number(bool_floaty(dict.contains_key(&val_to_hashval(key)?)))),
            Value::Set (set ) => Ok(Value::Number(bool_floaty(set .contains    (&val_to_hashval(key)?)))),
            _ => plainerr("error: remove() must be called with an array, dictionary, or set as its argument")
        }
    }
    pub (crate) fn sim_func_round(&mut self, mut args : VecDeque<Value>) -> Result<Value, String>
    {
        let val = args.pop_front().ok_or_else(|| minierr("error: wrong number of arguments to round(); expected 1, got 0"))?;
        let num = match_or_err!(val, Value::Number(num) => num, minierr("error: round() must be called with a number as its argument"))?;
        Ok(Value::Number(num.round()))
    }
    pub (crate) fn sim_func_ceil(&mut self, mut args : VecDeque<Value>) -> Result<Value, String>
    {
        let val = args.pop_front().ok_or_else(|| minierr("error: wrong number of arguments to ceil(); expected 1, got 0"))?;
        let num = match_or_err!(val, Value::Number(num) => num, minierr("error: ceil() must be called with a number as its argument"))?;
        Ok(Value::Number(num.ceil()))
    }
    pub (crate) fn sim_func_floor(&mut self, mut args : VecDeque<Value>) -> Result<Value, String>
    {
        let val = args.pop_front().ok_or_else(|| minierr("error: wrong number of arguments to floor(); expected 1, got 0"))?;
        let num = match_or_err!(val, Value::Number(num) => num, minierr("error: floor() must be called with a number as its argument"))?;
        Ok(Value::Number(num.floor()))
    }
    pub (crate) fn sim_func_instance_create(&mut self, mut args : VecDeque<Value>) -> Result<Value, String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to instance_create(); expected 1, got {}", args.len()));
        }
        
        let object_id = self.deque_pop_front_object(&mut args).or_else(|_| plainerr("error: first argument to instance_create() must be an object"))?;
        
        let instance_id = self.global.instance_id as usize;
        if self.global.instances.len() == !0usize
        {
            return plainerr("error: ran out of instance id space");
        }
        let object = self.global.objects.get(&object_id).ok_or_else(|| format!("error: tried to create instance of non-extant object type {}", object_id))?;
        
        let mut variables = HashMap::new();
        variables.insert("id".to_string(), Value::Instance(instance_id));
        self.global.instances.insert(instance_id, Instance { objtype : object_id, ident : instance_id, variables });
        
        if let Some(ref mut instance_list) = self.global.instances_by_type.get_mut(&object_id)
        {
            instance_list.insert(instance_id);
        }
        else
        {
            let mut instance_list = BTreeSet::new();
            instance_list.insert(instance_id);
            self.global.instances_by_type.insert(object_id, instance_list);
        }
        
        if let Some(function) = object.functions.get("create")
        {
            let mut mydata = function.clone();
            mydata.forcecontext = instance_id;
            let pseudo_funcvar = FuncVal{internal : false, name : Some("create".to_string()), predefined : None, userdefdata : Some(mydata)};
            self.call_function(pseudo_funcvar, VecDeque::new(), false)?;
        }
        
        while self.global.instances.contains_key(&self.global.instance_id)
        {
            self.global.instance_id += 1;
        }
        
        Ok(Value::Instance(instance_id))
    }
    pub (crate) fn sim_func_instance_exists(&mut self, mut args : VecDeque<Value>) -> Result<Value, String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to instance_create(); expected 1, got {}", args.len()));
        }
        
        // FIXME
        let instance_id = self.deque_pop_front_instance(&mut args).or_else(|_| plainerr("error: first argument to instance_create() must be an object"))?;
        
        Ok(Value::Number(bool_floaty(self.global.instances.contains_key(&instance_id))))
    }
    pub (crate) fn sim_func_instance_kill(&mut self, mut args : VecDeque<Value>) -> Result<Value, String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to instance_create(); expected 1, got {}", args.len()));
        }
        
        // FIXME
        let instance_id = self.deque_pop_front_instance(&mut args).or_else(|_| plainerr("error: first argument to instance_create() must be an object"))?;
        
        if let Some(inst) = self.global.instances.remove(&instance_id)
        {
            if let Some(ref mut instance_list) = self.global.instances_by_type.get_mut(&inst.objtype)
            {
                instance_list.remove(&instance_id);
            }
        }
        
        Ok(Value::Number(0.0))
    }
    pub (crate) fn sim_func_parse_text(&mut self, mut args : VecDeque<Value>) -> Result<Value, String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to parse_text(); expected 1, got {}", args.len()));
        }
        
        // FIXME
        let text = self.deque_pop_front_text(&mut args).or_else(|_| plainerr("error: first argument to parse_text() must be a string"))?;
        let parser = self.global.parser.as_mut().ok_or_else(|| minierr("error: parser was not loaded into interpreter"))?;
        
        let program_lines : Vec<String> = text.lines().map(|x| x.to_string()).collect();
        let tokens = parser.tokenize(&program_lines, true)?;
        
        let ast = parser.parse_program(&tokens, &program_lines, true)?.ok_or_else(|| minierr("error: string failed to parse"))?;
        
        Ok(ast_to_dict(&ast))
    }

    pub (crate) fn sim_func_compile_ast(&mut self, mut args : VecDeque<Value>) -> Result<Value, String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to compile_ast(); expected 1, got {}", args.len()));
        }
        
        let dict = self.deque_pop_front_dict(&mut args).or_else(|_| plainerr("error: first argument to compile_ast() must be a dictionary"))?;
        let ast = dict_to_ast(&dict)?;
        let code = compile_bytecode(&ast)?;
        
        // endaddr at the start because Rc::new() moves `code`
        Ok
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
            })
        ) )
    }

    pub (crate) fn sim_func_compile_text(&mut self, mut args : VecDeque<Value>) -> Result<Value, String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to compile_text(); expected 1, got {}", args.len()));
        }
        let text = self.deque_pop_front_text(&mut args).or_else(|_| plainerr("error: first argument to compile_text() must be a string"))?;
        
        let program_lines : Vec<String> = text.lines().map(|x| x.to_string()).collect();
        let parser = self.global.parser.as_mut().ok_or_else(|| minierr("error: parser was not loaded into interpreter"))?;
        
        let tokens = parser.tokenize(&program_lines, true)?;
        let ast = parser.parse_program(&tokens, &program_lines, true)?.ok_or_else(|| minierr("error: string failed to parse"))?;
        
        let code = compile_bytecode(&ast)?;
        
        // endaddr at the start because Rc::new() moves `code`
        Ok
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
            })
        ) )
    }
}