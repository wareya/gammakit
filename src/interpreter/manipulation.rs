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

macro_rules! past_end_of_code_err { ($self:expr) =>
{
    if cfg!(code_bounds_debugging)
    {
        plainerr("error: tried to access past end of code")
    }
    else
    {
        panic!("error: tried to access past end of code; {:?}", $self.top_frame);
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
    pub (crate) fn pull_single_from_code(&mut self) -> Result<u8, String>
    {
        let r = self.top_frame.code[self.top_frame.pc];
        self.top_frame.pc += 1;
        Ok(r as u8)
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
    
    #[inline]
    pub (crate) fn read_usize(&mut self) -> Result<usize, String>
    {
        let r = self.top_frame.code[self.top_frame.pc] as usize;
        self.top_frame.pc += 1;
        Ok(r)
    }
    #[inline]
    pub (crate) fn read_float(&mut self) -> Result<f64, String>
    {
        let r = f64::from_bits(self.top_frame.code[self.top_frame.pc]);
        self.top_frame.pc += 1;
        Ok(r)
    }
    
    #[allow(clippy::ptr_arg)]
    #[inline]
    pub (crate) fn get_string_index(&mut self, string : &String) -> usize
    {
        self.global.get_string_index(string)
    }
    #[inline]
    pub (crate) fn get_indexed_string(&self, index : usize) -> String
    {
        self.global.get_string(index)
    }
    
    pub (crate) fn read_indexed_string(&mut self) -> Result<String, String>
    {
        let index = self.read_usize()?;
        Ok(self.get_indexed_string(index))
    }
    pub (crate) fn read_function(&mut self, generator : bool) -> Result<FuncSpec, String>
    {
        let argcount = self.read_usize()? as usize;
        let bodylen = self.read_usize()?;
        
        let startaddr = self.get_pc();
        self.add_pc(bodylen);
        
        Ok(FuncSpec { argcount, code : self.top_frame.code.clone(), startaddr, endaddr : startaddr + bodylen, fromobj : false, parentobj : 0, forcecontext : 0, generator })
    }
    
    pub (crate) fn read_lambda(&mut self) -> Result<(Vec<Value>, FuncSpec), String>
    {
        let capturecount = self.read_usize()?;
        
        if self.top_frame.stack.len() < capturecount
        {
            return Err(format!("internal error: not enough values on stack to satisfy requirements of read_lambda (need {}, have {})", capturecount, self.top_frame.stack.len()));
        }
        
        let mut captures = Vec::with_capacity(capturecount);
        for _i in 0..capturecount
        {
            let val = self.stack_pop_val().ok_or_else(|| minierr("internal error: read_lambda failed to collect capture value from stack"))?;
            captures.push(val);
        }
        
        let argcount = self.read_usize()? as usize;
        let bodylen = self.read_usize()?;
        
        let startaddr = self.get_pc();
        self.add_pc(bodylen);
        
        Ok((captures, FuncSpec { argcount, code : self.top_frame.code.clone(), startaddr, endaddr : startaddr + bodylen, fromobj : false, parentobj : 0, forcecontext : 0, generator : false }))
    }
    
    #[inline]
    pub (crate) fn drain_vars(&mut self, desired_count : u64)
    {
        self.top_frame.variables.truncate(desired_count as usize);
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