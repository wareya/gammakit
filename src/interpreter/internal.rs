use crate::interpreter::*;

impl Interpreter
{
    pub (super) fn stack_len(&mut self) -> usize
    {
        self.top_frame_mut().len()
    }
    pub (super) fn stack_pop_val(&mut self) -> Option<Value>
    {
        self.top_frame_mut().pop_val()
    }
    pub (super) fn stack_pop_var(&mut self) -> Option<Variable>
    {
        self.top_frame_mut().pop_var()
    }
    pub (super) fn stack_pop(&mut self) -> Option<StackValue>
    {
        self.top_frame_mut().pop()
    }
    pub (super) fn stack_pop_as_val(&mut self) -> Option<Value>
    {
        match self.top_frame_mut().pop()
        {
            Some(StackValue::Var(x)) => self.evaluate_value(x).ok(),
            Some(StackValue::Val(x)) => Some(x),
            _ => None
        }
    }
    pub (super) fn stack_push_val(&mut self, value : Value)
    {
        self.top_frame_mut().push_val(value)
    }
    pub (super) fn stack_push_var(&mut self, variable : Variable)
    {
        self.top_frame_mut().push_var(variable)
    }
    pub (super) fn stack_push(&mut self, stackvalue : StackValue)
    {
        self.top_frame_mut().push(stackvalue)
    }
    
    fn call_arrow_function(&mut self, subfuncval : SubFuncVal, args : Vec<Value>, isexpr : bool) -> OpResult
    {
        if let Some(binding) = self.get_trivial_arrow_binding(subfuncval.name)
        {
            match subfuncval.source
            {
                StackValue::Val(val) =>
                {
                    let ret = binding(ValueLoc::Static(val), args)?;
                    if isexpr
                    {
                        self.stack_push_val(ret);
                    }
                }
                StackValue::Var(source) =>
                {
                    let val = self.evaluate(source)?;
                    let ret = binding(val, args)?;
                    if isexpr
                    {
                        self.stack_push_val(ret);
                    }
                }
            };
        }
        else if let Some(binding_wrapper) = self.get_arrow_binding(subfuncval.name)
        {
            let binding = &mut *binding_wrapper.try_borrow_mut().or_else(|_| plainerr("error: tried to borrow internal function while it was borrowed elsewhere"))?;
            
            match subfuncval.source
            {
                StackValue::Val(val) =>
                {
                    let ret = binding(ValueLoc::Static(val), args)?;
                    if isexpr
                    {
                        self.stack_push_val(ret);
                    }
                }
                StackValue::Var(source) =>
                {
                    let val = self.evaluate(source)?;
                    let ret = binding(val, args)?;
                    if isexpr
                    {
                        self.stack_push_val(ret);
                    }
                }
            };
        }
        else
        {
            return Err(format!("error: no such arrow function `{}`", subfuncval.name))
        }
        
        Ok(())
    }
    
    pub (super) fn handle_func_call_or_expr(&mut self, isexpr : bool) -> OpResult
    {
        let argcount = self.read_usize()?;
        
        //eprintln!("{} args", argcount);
        
        if cfg!(stack_len_debugging) && argcount+1 > self.stack_len()
        {
            return plainerr("internal error: fewer values on stack than expected in FUNCEXPR/FUNCCALL");
        }
        
        let mut args = Vec::with_capacity(argcount);
        for _ in 0..argcount
        {
            args.push(self.stack_pop_val().ok_or_else(|| minierr("internal error: expected values, got variable on stack in FUNCEXPR/FUNCCALL"))?);
        }
        args.reverse();
        
        // FIXME gives bad error message when can't access variable
        let funcdata = self.stack_pop_as_val().ok_or_else(|| minierr("internal error: not enough values on stack to run instruction FUNCEXPR/FUNCCALL (after args)"))?;
        
        match funcdata
        {
            Value::Func(funcdata) => self.call_function(funcdata, args, isexpr)?,
            Value::InternalFunc(funcdata) => self.call_internal_function(funcdata, args, isexpr)?,
            Value::SubFunc(subfuncval) => self.call_arrow_function(*subfuncval, args, isexpr)?,
            _ => return Err(format!("internal error: value meant to hold function data in FUNCEXPR/FUNCCALL was not holding function data; {:?}", funcdata))
        }
        
        Ok(())
    }
}