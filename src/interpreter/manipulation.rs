use crate::interpreter::*;

macro_rules! vec_pop_front_generic { ( $list:expr, $x:ident ) =>
{
    if !$list.is_empty()
    {
        match $list.remove(0)
        {
            Value::$x(ret) => Some(ret),
            _ => None
        }
    }
    else
    {
        None
    }
} }

impl Interpreter
{
    #[inline]
    pub (crate) fn get_pc(&self) -> usize
    {
        self.top_frame.pc
    }
    #[inline]
    pub (crate) fn set_pc(&mut self, new : usize)
    {
        self.top_frame.pc = new;
    }
    #[inline]
    pub (crate) fn add_pc(&mut self, new : usize)
    {
        self.top_frame.pc += new;
    }
    #[inline]
    pub (crate) fn sub_pc(&mut self, new : usize)
    {
        self.top_frame.pc -= new;
    }
    #[inline]
    pub (crate) fn round_up_pc_2(&mut self)
    {
        self.top_frame.pc = (((self.top_frame.pc-1)>>1)+1)<<1;
    }
    #[inline]
    pub (crate) fn round_up_pc_8(&mut self)
    {
        self.top_frame.pc = (((self.top_frame.pc-1)>>3)+1)<<3;
    }
    
    #[inline]
    pub (crate) fn pull_2_from_code(&mut self) -> Result<[u8; 2], String>
    {
        if self.top_frame.pc+2 > self.top_frame.code.len()
        {
            if cfg!(code_bounds_debugging)
            {
                return plainerr("error: tried to access past end of code");
            }
            else
            {
                panic!("error: tried to access past end of code; {:?}", self.top_frame);
            }
        }
        self.round_up_pc_2();
        let pc = self.top_frame.pc;
        self.top_frame.pc += 2;
        unsafe
        {
            let vec = self.top_frame.code.get_unchecked(pc..pc+2);
            Ok([vec[0], vec[1]])
        }
    }
    #[inline]
    pub (crate) fn pull_8_from_code(&mut self) -> Result<[u8; 8], String>
    {
        if self.top_frame.pc+8 > self.top_frame.code.len()
        {
            if cfg!(code_bounds_debugging)
            {
                return plainerr("error: tried to access past end of code");
            }
            else
            {
                panic!("error: tried to access past end of code; {:?}", self.top_frame);
            }
        }
        self.round_up_pc_8();
        let pc = self.top_frame.pc;
        self.top_frame.pc += 8;
        unsafe
        {
            let vec = self.top_frame.code.get_unchecked(pc..pc+8);
            Ok([vec[0], vec[1], vec[2], vec[3], vec[4], vec[5], vec[6], vec[7]])
        }
    }
    #[inline]
    pub (crate) fn pull_single_from_code(&mut self) -> Result<u8, String>
    {
        if self.top_frame.pc+1 > self.top_frame.code.len()
        {
            if cfg!(code_bounds_debugging)
            {
                return plainerr("error: tried to access past end of code");
            }
            else
            {
                panic!("error: tried to access past end of code; {:?}", self.top_frame);
            }
        }
        let pc = self.top_frame.pc;
        self.top_frame.pc += 1;
        unsafe
        {
            Ok(*self.top_frame.code.code.get_unchecked(pc))
        }
    }
    
    pub (crate) fn vec_pop_front_instance(&mut self, args : &mut Vec<Value>) -> Option<usize>
    {
        vec_pop_front_generic!(args, Instance)
    }
    pub (crate) fn vec_pop_front_object(&mut self, args : &mut Vec<Value>) -> Option<usize>
    {
        vec_pop_front_generic!(args, Object)
    }
    pub (crate) fn vec_pop_front_text(&mut self, args : &mut Vec<Value>) -> Option<String>
    {
        vec_pop_front_generic!(args, Text)
    }
    pub (crate) fn vec_pop_front_dict(&mut self, args : &mut Vec<Value>) -> Option<Box<HashMap<HashableValue, Value>>>
    {
        vec_pop_front_generic!(args, Dict)
    }
    pub (crate) fn read_u16(&mut self) -> Result<u16, String>
    {
        Ok(unpack_u16(self.pull_2_from_code()?))
    }
    pub (crate) fn read_usize(&mut self) -> Result<usize, String>
    {
        Ok(unpack_u64(self.pull_8_from_code()?) as usize)
    }
    pub (crate) fn read_float(&mut self) -> Result<f64, String>
    {
        Ok(unpack_f64(self.pull_8_from_code()?))
    }
    pub (crate) fn read_string_index(&mut self) -> Result<usize, String>
    {
        self.read_usize()
    }
    
    #[inline]
    pub (crate) fn get_string_index(&self, string : &String) -> usize
    {
        self.top_frame.code.get_string_index(string)
    }
    #[inline]
    pub (crate) fn get_indexed_string(&self, index : usize) -> String
    {
        self.top_frame.code.get_string(index)
    }
    
    pub (crate) fn read_string(&mut self) -> Result<String, String>
    {
        let start = self.get_pc();
        if start >= self.top_frame.code.len()
        {
            if cfg!(code_bounds_debugging)
            {
                return Err("error: tried to decode a string past the end of code".to_string());
            }
            else
            {
                panic!("error: tried to decode a string past the end of code; {:?}", self.top_frame);
            }
        }
        
        let mut end = start+1;
        while end < self.top_frame.code.len() && self.top_frame.code[end] != 0
        {
            end += 1;
        }
        
        self.set_pc(end+1);
        Ok(String::from_utf8_lossy(&self.top_frame.code[start..end]).to_string())
    }
    pub (crate) fn read_function(&mut self, subroutine : bool, generator : bool) -> Result<(usize, FuncSpec), String>
    {
        let name = self.read_string_index()?;
        let argcount = self.read_u16()? as usize;
        let bodylen = self.read_usize()?;
        
        let mut args = Vec::with_capacity(argcount);
        for _ in 0..argcount
        {
            args.push(self.read_string_index()?);
        }
        
        let startaddr = self.get_pc();
        self.add_pc(bodylen);
        
        Ok((name, FuncSpec { varnames : args, code : self.top_frame.code.clone(), startaddr, endaddr : startaddr + bodylen, fromobj : false, parentobj : 0, forcecontext : 0, impassable : !subroutine, generator }))
    }
    
    pub (crate) fn read_lambda(&mut self) -> Result<(BTreeMap<usize, ValRef>, FuncSpec), String>
    {
        let capturecount = self.read_usize()?;
        
        if self.top_frame.stack.len() < capturecount
        {
            return Err(format!("internal error: not enough values on stack to satisfy requirements of read_lambda (need {}, have {})", capturecount, self.top_frame.stack.len()));
        }
        
        let mut captures = BTreeMap::new();
        for _i in 0..capturecount
        {
            let val = self.stack_pop_val().ok_or_else(|| minierr("internal error: read_lambda failed to collect capture value from stack"))?;
            let name = self.read_usize()?;
            
            if captures.contains_key(&name)
            {
                return Err(format!("error: duplicate capture variable name `{}` in lambda capture expression", name));
            }
            captures.insert(name, ValRef::from_val(val));
        }
        
        let argcount = self.read_u16()? as usize;
        let bodylen = self.read_usize()?;
        
        let mut args = Vec::with_capacity(argcount);
        for _ in 0..argcount
        {
            args.push(self.read_string_index()?);
        }
        
        let startaddr = self.get_pc();
        self.add_pc(bodylen);
        
        Ok((captures, FuncSpec { varnames : args, code : self.top_frame.code.clone(), startaddr, endaddr : startaddr + bodylen, fromobj : false, parentobj : 0, forcecontext : 0, impassable : true, generator : false }))
    }
    
    #[inline]
    pub (crate) fn stack_pop_name(&mut self) -> Option<usize>
    {
        match_or_none!(self.stack_pop_var(), Some(Variable::Direct(name)) => name)
    }
    
    #[inline]
    pub (crate) fn drain_scopes(&mut self, desired_depth : u16)
    {
        while self.top_frame.scopes.len() > desired_depth as usize
        {
            self.top_frame.scopes.pop();
        }
    }
    #[inline]
    pub (crate) fn pop_controlstack_until_loop(&mut self)
    {
        while let Some(controller) = self.top_frame.controlstack.last()
        {
            if matches!(controller, Controller::While(_)) // NOTE: the while controller also handles for loops // TODO: let WITH and FOREACH support break/continue
            {
                break;
            }
            self.top_frame.controlstack.pop();
        }
    }
}