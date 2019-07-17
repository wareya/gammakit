#![allow(clippy::type_complexity)]
#![allow(clippy::cast_lossless)]

use crate::interpreter::*;
use crate::interpreter::types::ops::{float_booly, bool_floaty};

pub trait VecHelpers<Value> {
    /// Slow; use extract() instead
    fn pop_front(&mut self) -> Option<Value>;
    /// If the given element exists, extracts it by value, replacing what was there with Value::Number(0.0)
    /// Otherwise returns None
    fn extract(&mut self, index : usize) -> Option<Value>;
    /// For numbers.
    fn extract_num(&mut self, index : usize) -> Result<f64, String>;
    /// Same as extract(), but returns Err(...message that the error should be unreachable...) on out-of-range.
    fn expect_extract(&mut self, index : usize) -> Result<Value, String>;
}

impl VecHelpers<Value> for Vec<Value> {
    fn pop_front(&mut self) -> Option<Value>
    {
        if self.is_empty() { None } else { Some(self.remove(0)) }
    }
    fn extract(&mut self, index : usize) -> Option<Value>
    {
        if index < self.len()
        {
            let mut val = Value::Number(0.0);
            std::mem::swap(&mut self[index], &mut val);
            Some(val)
        }
        else
        {
            None
        }
    }
    fn extract_num(&mut self, index : usize) -> Result<f64, String>
    {
        let val = self.extract(index).ok_or_else(|| format!("error: wrong number of arguments; expected at least {}", index+1))?;
        match_or_err!(val, Value::Number(num) => num, minierr("error: expected a number, got something else"))
    }
    fn expect_extract(&mut self, index : usize) -> Result<Value, String>
    {
        self.extract(index).ok_or_else(|| minierr("internal error: error that should be unreachable in expect_extract"))
    }
}

pub (crate) fn ast_to_dict(ast : &ASTNode) -> Value
{
    let mut astdict = HashMap::<HashableValue, Value>::new();
    
    macro_rules! to_key { ( $str:expr ) => { HashableValue::Text($str.to_string()) } }
    
    astdict.insert(to_key!("text"), Value::Text(ast.text.clone()));
    astdict.insert(to_key!("line"), Value::Number(ast.line as f64));
    astdict.insert(to_key!("position"), Value::Number(ast.line as f64));
    astdict.insert(to_key!("isparent"), Value::Number(bool_floaty(ast.isparent)));
    
    let children : Vec<Value> = ast.children.iter().map(|child| ast_to_dict(child)).collect();
    
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
    
    // FIXME make this not use get! or something
    ast.precedence = get!(Number, dict, "precedence").map(|x| x.round() as u64).ok();
    
    Ok(ast)
}

impl Interpreter
{
    /// Insert a normal binding that needs access to the interpreter.
    ///
    /// The reference-counter wrapping is required to pass functions.
    ///
    /// The reference cell wrapping is required to support lambdas that have closure over mutable references, because that closure may indirectly include the interpreter itself. See magmakit for examples.
    pub fn insert_binding(&mut self, funcname : String, func : Rc<RefCell<Binding>>)
    {
        self.simple_bindings.remove(&funcname);
        self.bindings.insert(funcname, func);
    }
    /// Insert a normal binding that does not need access to the interpreter.
    pub fn insert_simple_binding(&mut self, funcname : String, func : Rc<RefCell<SimpleBinding>>)
    {
        self.bindings.remove(&funcname);
        self.simple_bindings.insert(funcname, func);
    }
    /// Insert an associated function ("arrow" function) binding.
    pub fn insert_arrow_binding(&mut self, funcname : String, func : Rc<RefCell<ArrowBinding>>)
    {
        self.arrow_bindings.insert(funcname, func);
    }
    /// Inserts or reinserts the default bindings. These SHOULD be safe, but if you're paranoid or you're making a very restrictive implementation of gammakit, you can feel free not to call this after initializing the interpreter.
    pub fn insert_default_bindings(&mut self)
    {
        macro_rules! insert { ( $x:expr, $y:ident ) => { self.insert_binding($x.to_string(), Rc::new(RefCell::new(Interpreter::$y))); } }
        
        insert!("print"                 , sim_func_print                );
        insert!("printraw"              , sim_func_printraw             );
        insert!("parse_text"            , sim_func_parse_text           );
        insert!("compile_text"          , sim_func_compile_text         );
        insert!("compile_ast"           , sim_func_compile_ast          );
        insert!("instance_create"       , sim_func_instance_create      );
        insert!("instance_exists"       , sim_func_instance_exists      );
        insert!("instance_kill"         , sim_func_instance_kill        );
        insert!("round"                 , sim_func_round                );
        insert!("floor"                 , sim_func_floor                );
        insert!("ceil"                  , sim_func_ceil                 );
        
        insert!("sqrt"                  , sim_func_sqrt                 );
        insert!("pow"                   , sim_func_pow                  );
        insert!("log"                   , sim_func_log                  );
        insert!("ln"                    , sim_func_ln                   );
        
        macro_rules! insert_arrow { ( $x:expr, $y:ident ) => { self.insert_arrow_binding($x.to_string(), Rc::new(RefCell::new(Interpreter::$y))); } }
        
        insert_arrow!("len"             , sim_subfunc_len               );
        insert_arrow!("keys"            , sim_subfunc_keys              );
        insert_arrow!("slice"           , sim_subfunc_slice             );
        insert_arrow!("contains"        , sim_subfunc_contains          );
        
        insert_arrow!("insert"          , sim_subfunc_insert            );
        insert_arrow!("remove"          , sim_subfunc_remove            );
        
        insert_arrow!("push"            , sim_subfunc_push              );
        insert_arrow!("pop"             , sim_subfunc_pop               );
    }
    pub (crate) fn get_binding(&self, name : &str) -> Option<Rc<RefCell<Binding>>>
    {
        match_or_none!(self.bindings.get(name), Some(f) => Rc::clone(f))
    }
    pub (crate) fn get_simple_binding(&self, name : &str) -> Option<Rc<RefCell<SimpleBinding>>>
    {
        match_or_none!(self.simple_bindings.get(name), Some(f) => Rc::clone(f))
    }
    pub (crate) fn get_arrow_binding(&self, name : &str) -> Option<Rc<RefCell<ArrowBinding>>>
    {
        match_or_none!(self.arrow_bindings.get(name), Some(f) => Rc::clone(f))
    }
    pub (crate) fn sim_func_print(&mut self, mut args : Vec<Value>) -> Result<Value, String>
    {
        for arg in args.drain(..)
        {
            println!("{}", format_val(&arg).ok_or_else(|| minierr("error: tried to print unprintable value"))?);
        }
        Ok(Value::Number(0.0))
    }
    pub (crate) fn sim_func_printraw(&mut self, mut args : Vec<Value>) -> Result<Value, String>
    {
        for arg in args.drain(..)
        {
            print!("{}", format_val(&arg).ok_or_else(|| minierr("error: tried to print unprintable value"))?);
        }
        Ok(Value::Number(0.0))
    }
    pub (crate) fn sim_func_round(&mut self, mut args : Vec<Value>) -> Result<Value, String>
    {
        let num = args.extract_num(0)?;
        Ok(Value::Number(num.round()))
    }
    pub (crate) fn sim_func_ceil(&mut self, mut args : Vec<Value>) -> Result<Value, String>
    {
        let num = args.extract_num(0)?;
        Ok(Value::Number(num.ceil()))
    }
    pub (crate) fn sim_func_floor(&mut self, mut args : Vec<Value>) -> Result<Value, String>
    {
        let num = args.extract_num(0)?;
        Ok(Value::Number(num.floor()))
    }
    pub (crate) fn sim_func_sqrt(&mut self, mut args : Vec<Value>) -> Result<Value, String>
    {
        let num = args.extract_num(0)?;
        Ok(Value::Number(num.sqrt()))
    }
    pub (crate) fn sim_func_pow(&mut self, mut args : Vec<Value>) -> Result<Value, String>
    {
        let num1 = args.extract_num(0)?;
        let num2 = args.extract_num(1)?;
        Ok(Value::Number(num1.powf(num2)))
    }
    pub (crate) fn sim_func_log(&mut self, mut args : Vec<Value>) -> Result<Value, String>
    {
        let num1 = args.extract_num(0)?;
        let num2 = args.extract_num(1)?;
        Ok(Value::Number(num1.log(num2)))
    }
    pub (crate) fn sim_func_ln(&mut self, mut args : Vec<Value>) -> Result<Value, String>
    {
        let num = args.extract_num(0)?;
        Ok(Value::Number(num.ln()))
    }
    pub (crate) fn sim_func_instance_create(&mut self, mut args : Vec<Value>) -> Result<Value, String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to instance_create(); expected 1, got {}", args.len()));
        }
        
        let object_id = self.vec_pop_front_object(&mut args).ok_or_else(|| minierr("error: first argument to instance_create() must be an object"))?;
        
        let instance_id = self.global.instance_id as usize;
        if self.global.instances.len() == !0usize
        {
            return plainerr("error: ran out of instance id space");
        }
        let object = self.global.objects.get(&object_id).ok_or_else(|| format!("error: tried to create instance of non-extant object type {}", object_id))?;
        
        let mut variables = HashMap::new();
        variables.insert("id".to_string(), ValRefSimple::from_val(Value::Instance(instance_id)));
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
            self.call_function(pseudo_funcvar, Vec::new(), false)?;
        }
        
        while self.global.instances.contains_key(&self.global.instance_id)
        {
            self.global.instance_id += 1;
        }
        
        Ok(Value::Instance(instance_id))
    }
    pub (crate) fn sim_func_instance_exists(&mut self, mut args : Vec<Value>) -> Result<Value, String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to instance_create(); expected 1, got {}", args.len()));
        }
        
        let instance_id = self.vec_pop_front_instance(&mut args).ok_or_else(|| minierr("error: first argument to instance_exists() must be an instance"))?;
        
        Ok(Value::Number(bool_floaty(self.global.instances.contains_key(&instance_id))))
    }
    pub (crate) fn sim_func_instance_kill(&mut self, mut args : Vec<Value>) -> Result<Value, String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to instance_create(); expected 1, got {}", args.len()));
        }
        
        let instance_id = self.vec_pop_front_instance(&mut args).ok_or_else(|| minierr("error: first argument to instance_kill() must be an instance"))?;
        
        if let Some(inst) = self.global.instances.remove(&instance_id)
        {
            if let Some(ref mut instance_list) = self.global.instances_by_type.get_mut(&inst.objtype)
            {
                instance_list.remove(&instance_id);
            }
        }
        
        Ok(Value::Number(0.0))
    }
    pub (crate) fn sim_func_parse_text(&mut self, mut args : Vec<Value>) -> Result<Value, String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to parse_text(); expected 1, got {}", args.len()));
        }
        
        let text = self.vec_pop_front_text(&mut args).ok_or_else(|| minierr("error: first argument to parse_text() must be a string"))?;
        let parser = self.global.parser.as_mut().ok_or_else(|| minierr("error: parser was not loaded into interpreter"))?;
        
        let program_lines : Vec<String> = text.lines().map(|x| x.to_string()).collect();
        let tokens = parser.tokenize(&program_lines, true)?;
        
        let ast = parser.parse_program(&tokens, &program_lines, true)?.ok_or_else(|| minierr("error: string failed to parse"))?;
        
        Ok(ast_to_dict(&ast))
    }

    pub (crate) fn sim_func_compile_ast(&mut self, mut args : Vec<Value>) -> Result<Value, String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to compile_ast(); expected 1, got {}", args.len()));
        }
        
        let dict = self.vec_pop_front_dict(&mut args).ok_or_else(|| minierr("error: first argument to compile_ast() must be a dictionary"))?;
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

    pub (crate) fn sim_func_compile_text(&mut self, mut args : Vec<Value>) -> Result<Value, String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to compile_text(); expected 1, got {}", args.len()));
        }
        let text = self.vec_pop_front_text(&mut args).ok_or_else(|| minierr("error: first argument to compile_text() must be a string"))?;
        
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
    
    pub (crate) fn sim_subfunc_len(myself : Box<ValRef>, args : Vec<Value>) -> Result<Value, String>
    {
        if !args.is_empty()
        {
            return Err(format!("error: wrong number of arguments to len(); expected 0, got {}", args.len()));
        }
        
        Ok(match *myself.borrow()?
        {
            Value::Text(ref string) => Value::Number(string.chars().count() as f64),
            Value::Array(ref array) => Value::Number(array.len() as f64),
            Value::Dict(ref dict) => Value::Number(dict.keys().len() as f64),
            Value::Set(ref set) => Value::Number(set.len() as f64),
            _ => return plainerr("error: tried to take length of lengthless type")
        })
    }
    pub (crate) fn sim_subfunc_keys(myself : Box<ValRef>, args : Vec<Value>) -> Result<Value, String>
    {
        if !args.is_empty()
        {
            return Err(format!("error: wrong number of arguments to keys(); expected 0, got {}", args.len()));
        }
        
        Ok(match *myself.borrow()?
        {
            Value::Array(ref array) => Value::Array((0..array.len()).map(|i| Value::Number(i as f64)).collect()),
            Value::Dict(ref dict) => Value::Array(dict.iter().map(|(key, _)| hashval_to_val(key.clone())).collect()),
            _ => return plainerr("error: tried to take length of lengthless type")
        })
    }
    pub (crate) fn sim_subfunc_slice(myself : Box<ValRef>, mut args : Vec<Value>) -> Result<Value, String>
    {
        if args.len() != 2
        {
            return Err(format!("error: wrong number of arguments to slice(); expected 2, got {}", args.len()));
        }
        let start = args.expect_extract(0)?;
        let end = args.expect_extract(1)?;
        let start = match_or_err!(start, Value::Number(start) => start.round() as i64, minierr("error: start and end indexes passed to slice() must be numbers"))?;
        let end = match_or_err!(end, Value::Number(end) => end.round() as i64, minierr("error: start and end indexes passed to slice() must be numbers"))?;
        
        Ok(match *myself.borrow()?
        {
            Value::Text(ref string) => slice_any(&string.chars().collect::<Vec<char>>(), start, end).map(|array| Value::Text(array.iter().cloned().collect())).ok_or_else(|| minierr("error: slice() on string went out of range"))?,
            Value::Array(ref array) => slice_any(&array, start, end).map(|array| Value::Array(array.to_vec())).ok_or_else(|| minierr("error: slice() on array went out of range"))?,
            _ => return plainerr("error: tried to slice lengthless type")
        })
    }
    pub (crate) fn sim_subfunc_contains(myself : Box<ValRef>, mut args : Vec<Value>) -> Result<Value, String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to contains(); expected 1, got {}", args.len()));
        }
        let key = args.expect_extract(0)?;
        Ok(match *myself.borrow()?
        {
            Value::Dict(ref dict) => Value::Number(bool_floaty(dict.contains_key(&val_to_hashval(key)?))),
            Value::Set (ref set ) => Value::Number(bool_floaty(set .contains    (&val_to_hashval(key)?))),
            _ => return plainerr("error: remove() must be called with an array, dictionary, or set as its argument")
        })
    }
    pub (crate) fn sim_subfunc_insert(myself : Box<ValRef>, mut args : Vec<Value>) -> Result<Value, String>
    {
        match *myself.borrow_mut()?
        {
            Value::Text(ref mut string) =>
            {
                if args.len() != 2
                {
                    return Err(format!("error: wrong number of arguments to insert() on a string; expected 2, got {}", args.len()));
                }
                let key = args.expect_extract(0)?;
                if let Value::Text(value) = args.expect_extract(1)?
                {
                    // FIXME use codepoint indexes for this
                    let chars : Vec<char> = string.chars().collect();
                    
                    let index = match_or_err!(key, Value::Number(index) => index.round() as isize, minierr("error: tried to insert into a string with a non-number index"))?;
                    let index = if index < 0 {chars.len() - (-index as usize)} else {index as usize} as usize;
                    
                    let left = chars.get(0..index).ok_or_else(|| minierr("error: tried to insert into a string at an out-of-range index"))?.iter().collect::<String>();
                    let right = chars.get(index..chars.len()).ok_or_else(|| minierr("error: tried to insert into a string at an out-of-range index"))?.iter().collect::<String>();
                    
                    let newstr = format!("{}{}{}", left, value, right);
                    *string = newstr;
                    
                    return Ok(Value::Number(0.0));
                }
                plainerr("error: tried to insert a non-string into a string with insert()")
            }
            Value::Array(ref mut array) =>
            {
                if args.len() != 2
                {
                    return Err(format!("error: wrong number of arguments to insert() on an array; expected 2, got {}", args.len()));
                }
                let key = args.expect_extract(0)?;
                let value = args.expect_extract(1)?;
                let index = match_or_err!(key, Value::Number(index) => index.round() as isize, minierr("error: tried to insert into an array with a non-number index"))?;
                if index < 0 || index as usize > array.len()
                {
                    return plainerr("error: tried to insert into an array at an out-of-range index");
                }
                array.insert(index as usize, value);
                Ok(Value::Number(0.0))
            }
            Value::Dict(ref mut dict) =>
            {
                if args.len() != 2
                {
                    return Err(format!("error: wrong number of arguments to insert() on a dict; expected 2, got {}", args.len()));
                }
                let key = args.expect_extract(0)?;
                let value = args.expect_extract(1)?;
                dict.insert(val_to_hashval(key)?, value);
                Ok(Value::Number(0.0))
            }
            Value::Set(ref mut set) =>
            {
                if args.len() != 1
                {
                    return Err(format!("error: wrong number of arguments to insert() on a set; expected 1, got {}", args.len()));
                }
                let key = args.expect_extract(0)?;
                set.insert(val_to_hashval(key)?);
                Ok(Value::Number(0.0))
            }
            _ => plainerr("error: insert() must be called with an array, dictionary, set, or string as the first argument")
        }
    }
    pub (crate) fn sim_subfunc_push(myself : Box<ValRef>, mut args : Vec<Value>) -> Result<Value, String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to push(); expected 1, got {}", args.len()));
        }
        let value = args.expect_extract(0)?;
        match *myself.borrow_mut()?
        {
            Value::Text(ref mut string) =>
            {
                if let Value::Text(value) = value
                {
                    *string = format!("{}{}", string, value);
                    return Ok(Value::Number(0.0));
                }
                plainerr("error: tried to concatenate a non-string to a string with push()")
            }
            Value::Array(ref mut array) =>
            {
                array.push(value);
                Ok(Value::Number(0.0))
            }
            _ => plainerr("error: push() must be called with an array or string as the first argument")
        }
    }
    pub (crate) fn sim_subfunc_remove(myself : Box<ValRef>, mut args : Vec<Value>) -> Result<Value, String>
    {
        if args.len() != 1
        {
            return Err(format!("error: wrong number of arguments to remove(); expected 1, got {}", args.len()));
        }
        let key = args.expect_extract(0)?;
        match *myself.borrow_mut()?
        {
            Value::Text(ref mut string) =>
            {
                let mut chars : Vec<char> = string.chars().collect();
                
                let index = match_or_err!(key, Value::Number(index) => index.round() as isize, minierr("error: tried to remove from a string with a non-number index"))?;
                let index = if index < 0 {chars.len() - (-index as usize)} else {index as usize} as usize;
                
                let mid = chars.get(index..=index).ok_or_else(|| minierr("error: tried to remove from a string at an out-of-range index"))?.iter().collect::<String>();
                chars.drain(index..=index);
                *string = chars.iter().collect();
                Ok(Value::Text(mid))
            }
            Value::Array(ref mut array) =>
            {
                let index = match_or_err!(key, Value::Number(index) => index.round() as isize, minierr("error: tried to remove from an array with a non-number index"))?;
                if index < 0 || index as usize > array.len()
                {
                    return plainerr("error: tried to remove from an array at an out-of-range index");
                }
                let removed = array.remove(index as usize);
                Ok(removed)
            }
            Value::Dict(ref mut dict) =>
            {
                if let Some(removed) = dict.remove(&val_to_hashval(key)?)
                {
                    Ok(removed)
                }
                else
                {
                    plainerr("error: tried to remove non-extant key from dict")
                }
            }
            Value::Set(ref mut set) =>
            {
                if set.remove(&val_to_hashval(key.clone())?)
                {
                    Ok(Value::Number(0.0))
                }
                else
                {
                    plainerr("error: tried to remove non-extant value from set")
                }
            }
            _ => plainerr("error: remove() must be called with an array, dictionary, or set as its argument")
        }
    }
    pub (crate) fn sim_subfunc_pop(myself : Box<ValRef>, args : Vec<Value>) -> Result<Value, String>
    {
        if !args.is_empty()
        {
            return Err(format!("error: wrong number of arguments to pop(); expected 0, got {}", args.len()));
        }
        match *myself.borrow_mut()?
        {
            Value::Array(ref mut array) =>
            {
                let ret = array.pop().ok_or_else(|| minierr("error: tried to call pop() on an empty array"))?;
                Ok(ret)
            }
            _ => plainerr("error: pop() must be called with an array as the first argument")
        }
    }
}