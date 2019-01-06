use crate::interpreter::*;

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
    pub (crate) fn get_code(&self) -> Rc<Vec<u8>>
    {
        Rc::clone(&self.top_frame.code)
    }
    pub (crate) fn get_pc(&self) -> usize
    {
        self.top_frame.pc
    }
    pub (crate) fn set_pc(&mut self, new : usize)
    {
        self.top_frame.pc = new;
    }
    pub (crate) fn add_pc(&mut self, new : usize)
    {
        self.top_frame.pc += new;
    }
    
    pub (crate) fn pull_from_code(&mut self, n : usize) -> Result<Vec<u8>, String>
    {
        let vec = self.get_code().get(self.get_pc()..self.get_pc()+n).map(|v| v.to_vec()).ok_or_else(|| minierr("error: tried to access past end of code"))?;
        self.add_pc(n);
        Ok(vec)
    }
    pub (crate) fn pull_single_from_code(&mut self) -> Result<u8, String>
    {
        let byte = self.get_code().get(self.get_pc()).cloned().ok_or_else(|| minierr("error: tried to access past end of code"))?;
        self.add_pc(1);
        Ok(byte)
    }
    
    // second val: 0: no value on stack; 1: value on stack was of the wrong type
    pub (crate) fn list_pop_number(&mut self, args : &mut Vec<Value>) -> Result<f64, i32>
    {
        list_pop_generic!(args, Number)
    }
    pub (crate) fn list_pop_text(&mut self, args : &mut Vec<Value>) -> Result<String, i32>
    {
        list_pop_generic!(args, Text)
    }
    pub (crate) fn list_pop_func(&mut self, args : &mut Vec<Value>) -> Result<FuncVal, i32>
    {
        match list_pop_generic!(args, Func)
        {
            Ok(s) => Ok(*s),
            Err(e) => Err(e)
        }
    }
    pub (crate) fn list_pop_dict(&mut self, args : &mut Vec<Value>) -> Result<HashMap<HashableValue, Value>, i32>
    {
        list_pop_generic!(args, Dict)
    }
    pub (crate) fn read_u16(&mut self) -> Result<u16, String>
    {
        Ok(unpack_u16(&self.pull_from_code(2)?)?)
    }
    pub (crate) fn read_usize(&mut self) -> Result<usize, String>
    {
        Ok(unpack_u64(&self.pull_from_code(8)?)? as usize)
    }
    
    pub (crate) fn read_string(&mut self) -> Result<String, String>
    {
        let code = self.get_code();
        if self.get_pc() >= code.len()
        {
            return Err("error: tried to decode a string past the end of code".to_string());
        }
        
        let mut bytes = Vec::<u8>::new();
        
        let mut c = self.pull_single_from_code()?;
        while c != 0 && self.get_pc() < code.len()
        {
            bytes.push(c);
            c = self.pull_single_from_code()?;
        }
        
        let res = std::str::from_utf8(&bytes).or_else(|_| plainerr("error: tried to decode a string that was not utf-8"))?;
        Ok(res.to_string())
    }
    pub (crate) fn read_function(&mut self, subroutine : bool, generator : bool) -> Result<(String, FuncSpec), String>
    {
        let code = self.get_code();
        
        let name = self.read_string()?;
        
        let argcount = self.read_u16()?;
        
        let bodylen = unpack_u64(&self.pull_from_code(8)?)? as usize;
        
        let mut args = Vec::<String>::new();
        for _ in 0..argcount
        {
            args.push(self.read_string()?);
        }
        
        let startaddr = self.get_pc();
        self.add_pc(bodylen);
        
        Ok((name, FuncSpec { varnames : args, code : Rc::clone(&code), startaddr, endaddr : startaddr + bodylen, fromobj : false, parentobj : 0, forcecontext : 0, impassable : !subroutine, generator }))
    }
    
    pub (crate) fn read_lambda(&mut self) -> Result<(HashMap<String, Value>, FuncSpec), String>
    {
        let code = self.get_code();
        
        let capturecount = self.read_u16()? as usize;
        
        if self.top_frame.stack.len() < capturecount*2
        {
            return Err(format!("internal error: not enough values on stack to satisfy requirements of read_lambda (need {}, have {})", capturecount*2, self.top_frame.stack.len()));
        }
        
        let mut captures = HashMap::<String, Value>::new();
        for _i in 0..capturecount
        {
            let val = self.stack_pop_val().ok_or_else(|| minierr("internal error: read_lambda failed to collect capture value from stack"))?;
            let name = self.stack_pop_text().ok_or_else(|| minierr("internal error: read_lambda failed to collect capture name from stack"))?;
            
            if captures.contains_key(&name)
            {
                return Err(format!("error: duplicate capture variable name `{}` in lambda capture expression", name));
            }
            captures.insert(name, val);
        }
        
        let argcount = self.read_u16()?;
        
        let bodylen = unpack_u64(&self.pull_from_code(8)?)? as usize;
        
        let mut args = Vec::<String>::new();
        for _ in 0..argcount
        {
            args.push(self.read_string()?);
        }
        
        let startaddr = self.get_pc();
        self.add_pc(bodylen);
        
        Ok((captures, FuncSpec { varnames : args, code : Rc::clone(&code), startaddr, endaddr : startaddr + bodylen, fromobj : false, parentobj : 0, forcecontext : 0, impassable : true, generator : false }))
    }
    
    pub (crate) fn stack_pop_number(&mut self) -> Option<f64>
    {
        match_or_none!(self.stack_pop_val(), Some(Value::Number(val)) => val)
    }
    pub (crate) fn stack_pop_text(&mut self) -> Option<String>
    {
        match_or_none!(self.stack_pop_val(), Some(Value::Text(val)) => val)
    }
    pub (crate) fn stack_pop_name(&mut self) -> Option<String>
    {
        match_or_none!(self.stack_pop_var(), Some(Variable::Direct(DirectVar{name:text})) => text)
    }
    
    pub (crate) fn drain_scopes(&mut self, desired_depth : u16)
    {
        while self.top_frame.scopes.len() > desired_depth as usize
        {
            self.top_frame.scopes.pop();
            self.top_frame.scopestarts.pop();
        }
    }
    pub (crate) fn pop_controlstack_until_loop(&mut self)
    {
        while let Some(controller) = self.top_frame.controlstack.last()
        {
            if matches!(controller, Controller::While(_)) // NOTE: the while controller also handles for loops // TODO: let WITH and FOREACH support break/continue
            {
                break;
            }
            else
            {
                self.top_frame.controlstack.pop();
            }
        }
    }
}