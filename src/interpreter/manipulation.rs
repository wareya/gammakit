use interpreter::*;

macro_rules! list_pop_generic {
    ( $list:expr, $x:ident ) =>
    {
        if let Some(val) = $list.pop()
        {
            if let Value::$x(ret) = val
            {
                Ok(ret)
            }
            else
            {
                Err(1)
            }
        }
        else
        {
            Err(0)
        }
    }
}

impl Interpreter
{
    pub(super) fn get_code(&self) -> Rc<Vec<u8>>
    {
        return Rc::clone(&self.top_frame.code);
    }
    pub(super) fn get_pc(&self) -> usize
    {
        self.top_frame.pc
    }
    pub(super) fn set_pc(&mut self, new : usize)
    {
        self.top_frame.pc = new;
    }
    pub(super) fn add_pc(&mut self, new : usize)
    {
        self.top_frame.pc += new;
    }
    
    pub(super) fn pull_from_code(&mut self, n : usize) -> Vec<u8>
    {
        let vec = self.get_code()[self.get_pc()..self.get_pc()+n].to_vec();
        self.add_pc(n);
        return vec;
    }
    pub(super) fn pull_single_from_code(&mut self) -> u8
    {
        let vec = self.get_code()[self.get_pc()];
        self.add_pc(1);
        return vec;
    }
    
    pub(super) fn list_pop_number(&mut self, args : &mut Vec<Value>) -> Result<f64, i32> // second val: 0: no value on stack; 1: value on stack was of the wrong type
    {
        list_pop_generic!(args, Number)
    }
    pub(super) fn list_pop_text(&mut self, args : &mut Vec<Value>) -> Result<String, i32>
    {
        list_pop_generic!(args, Text)
    }
    pub(super) fn list_pop_func(&mut self, args : &mut Vec<Value>) -> Result<FuncVal, i32>
    {
        match list_pop_generic!(args, Func)
        {
            Ok(s) => Ok(*s),
            Err(e) => Err(e)
        }
    }
    pub(super) fn list_pop_dict(&mut self, args : &mut Vec<Value>) -> Result<HashMap<HashableValue, Value>, i32>
    {
        list_pop_generic!(args, Dict)
    }
    
    pub(super) fn read_string(&mut self) -> String
    {
        let code = self.get_code();
        if self.get_pc() >= code.len()
        {
            return "".to_string();
        }
        
        let mut bytes = Vec::<u8>::new();
        
        let mut c = self.pull_single_from_code();
        while c != 0 && self.get_pc() < code.len() // FIXME check if this should be < or <= (will only affect malformed bytecode, but still)
        {
            bytes.push(c);
            c = self.pull_single_from_code();
        }
        
        if let Ok(res) = std::str::from_utf8(&bytes)
        {
            return res.to_string();
        }
        else
        {
            return "".to_string();
        }
    }
    pub(super) fn read_function(&mut self) -> (String, FuncSpec)
    {
        let code = self.get_code();
        
        let name = self.read_string();
        
        let argcount = unpack_u16(&self.pull_from_code(2));
        
        let bodylen = unpack_u64(&self.pull_from_code(8)) as usize;
        
        let mut args = Vec::<String>::new();
        for _ in 0..argcount
        {
            args.push(self.read_string());
        }
        
        let startaddr = self.get_pc();
        self.add_pc(bodylen);
        
        return (name, FuncSpec { varnames : args, code : Rc::clone(&code), startaddr, endaddr : startaddr + bodylen, fromobj : false, parentobj : 0, forcecontext : 0, location : self.build_funcspec_location() } );
    }
    
    pub(super) fn read_lambda(&mut self) -> (HashMap<String, Value>, FuncSpec)
    {
        let code = self.get_code();
        
        let capturecount = unpack_u16(&self.pull_from_code(2)) as usize;
        
        if self.top_frame.stack.len() < capturecount*2
        {
            panic!("internal error: not enough values on stack to satisfy requirements of read_lambda (need {}, have {})", capturecount*2, self.top_frame.stack.len());
        }
        
        let mut captures = HashMap::<String, Value>::new();
        for _i in 0..capturecount
        {
            if let Ok(val) = self.stack_pop_any()
            {
                if let Ok(name) = self.stack_pop_text()
                {
                    if captures.contains_key(&name)
                    {
                        panic!("error: duplicate capture variable name `{}` in lambda capture expression", name);
                    }
                    captures.insert(name, val);
                }
                else
                {
                    panic!("internal error: read_lambda failed to collect capture name from stack");
                }
            }
            else
            {
                panic!("internal error: read_lambda failed to collect capture value from stack");
            }
        }
        
        let argcount = unpack_u16(&self.pull_from_code(2));
        
        let bodylen = unpack_u64(&self.pull_from_code(8)) as usize;
        
        let mut args = Vec::<String>::new();
        for _ in 0..argcount
        {
            args.push(self.read_string());
        }
        
        let startaddr = self.get_pc();
        self.add_pc(bodylen);
        
        return (captures, FuncSpec { varnames : args, code : Rc::clone(&code), startaddr, endaddr : startaddr + bodylen, fromobj : false, parentobj : 0, forcecontext : 0, location : self.build_funcspec_location() } );
    }
    
    pub(super) fn stack_pop_number(&mut self) -> Result<f64, i32>
    {
        list_pop_generic!(self.top_frame.stack, Number)
    }
    pub(super) fn stack_pop_text(&mut self) -> Result<String, i32>
    {
        list_pop_generic!(self.top_frame.stack, Text)
    }
    /*
    fn stack_pop_var(&mut self) -> Result<Variable, i32>
    {
        list_pop_generic!(self.top_frame.stack, Var)
    }
    */
    pub(super) fn stack_pop_name(&mut self) -> Result<String, i32>
    {
        let var = list_pop_generic!(self.top_frame.stack, Var)?;
        if let Variable::Direct(DirectVar{name:text}) = var
        {
            Ok(text)
        }
        else
        {
            Err(1)
        }
    }
    pub(super) fn stack_pop_any(&mut self) -> Result<Value, i32>
    {
        if let Some(val) = self.top_frame.stack.pop()
        {
            Ok(val)
        }
        else
        {
            Err(0)
        }
    }
    
    pub(super) fn drain_scopes(&mut self, desired_depth : u16)
    {
        while self.top_frame.scopes.len() > desired_depth as usize
        {
            self.top_frame.scopes.pop();
            self.top_frame.scopestarts.pop();
        }
    }
    pub(super) fn pop_controlstack_until_loop(&mut self)
    {
        let mut foundloop = false;
        
        if let Some(controller) = self.top_frame.controlstack.last()
        {
            if controller.controltype == WHILE || controller.controltype == FOR // TODO: add WITH
            {
                foundloop = true;
            }
        }
        
        if !foundloop
        {
            self.top_frame.controlstack.pop();
            self.pop_controlstack_until_loop();
        }
    }
}