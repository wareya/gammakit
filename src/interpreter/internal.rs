use crate::interpreter::*;

impl Interpreter
{
    pub (super) fn stack_len(&mut self) -> usize
    {
        self.top_frame.len()
    }
    pub (super) fn stack_pop_val(&mut self) -> Option<Value>
    {
        self.top_frame.pop_val()
    }
    pub (super) fn stack_pop_var(&mut self) -> Option<Variable>
    {
        self.top_frame.pop_var()
    }
    pub (super) fn stack_pop(&mut self) -> Option<StackValue>
    {
        self.top_frame.pop()
    }
    pub (super) fn stack_push_val(&mut self, value : Value)
    {
        self.top_frame.push_val(value)
    }
    pub (super) fn stack_push_var(&mut self, variable : Variable)
    {
        self.top_frame.push_var(variable)
    }
    pub (super) fn stack_push(&mut self, stackvalue : StackValue)
    {
        self.top_frame.push(stackvalue)
    }
    
    fn call_arrow_function(&mut self, subfuncval : SubFuncVal, args : Vec<Value>, isexpr : bool) -> OpResult
    {
        if let Some(binding_wrapper) = self.get_arrow_binding(&subfuncval.name)
        {
            let binding = &mut *binding_wrapper.try_borrow_mut().or_else(|_| plainerr("error: tried to borrow internal function while it was borrowed elsewhere"))?;
            
            match subfuncval.source
            {
                StackValue::Val(val) =>
                {
                    let (_, ret) = binding(val, args)?.into_parts();
                    if isexpr
                    {
                        self.stack_push_val(ret);
                    }
                }
                StackValue::Var(source) =>
                {
                    let val = self.evaluate_or_store(&source, None)?.ok_or_else(|| minierr("internal error: evaluate_or_store returned None when just accessing a variable"))?;
                    let (var, ret) = binding(val, args)?.into_parts();
                    
                    if let Some(var) = var
                    {
                        self.evaluate_or_store(&source, Some(var))?;
                    }
                    
                    if isexpr
                    {
                        self.stack_push_val(ret);
                    }
                }
            };
        }
        else
        {
            return Err(format!("internal error: no such arrow function `{}`", subfuncval.name))
        }
        
        Ok(())
    }
    
    pub (super) fn handle_func_call_or_expr(&mut self, isexpr : bool) -> OpResult
    {
        let argcount_val = self.stack_pop_val().ok_or_else(|| minierr("internal error: not enough values on stack to run instruction FUNCEXPR/FUNCCALL"))?;
        
        let argcount = match_or_err!(argcount_val, Value::Number(argcount) => argcount, minierr("internal error: number on stack of arguments to function was not a number"))?;
        let argcount = argcount.round() as usize;
        
        if argcount > self.stack_len()
        {
            return plainerr("internal error: fewer values on stack than expected in FUNCEXPR/FUNCCALL");
        }
        
        let mut args = Vec::<Value>::new();
        for _i in 0..argcount
        {
            args.insert(0, self.stack_pop_val().ok_or_else(|| minierr("internal error: expected values, got variable on stack in FUNCEXPR/FUNCCALL"))?);
        }
        
        let funcdata = self.stack_pop_val().ok_or_else(|| minierr("internal error: not enough values on stack to run instruction FUNCEXPR/FUNCCALL"))?;
        
        match funcdata
        {
            Value::Func(funcdata) => self.call_function(*funcdata, args, isexpr)?,
            Value::SubFunc(subfuncval) => self.call_arrow_function(*subfuncval, args, isexpr)?,
            _ => return plainerr("internal error: value meant to hold function data in FUNCEXPR/FUNCCALL was not holding function data")
        }
        
        Ok(())
    }
}