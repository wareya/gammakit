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
    
    pub (super) fn handle_func_call_or_expr(&mut self, isexpr : bool) -> OpResult
    {
        let funcdata = self.stack_pop().ok_or_else(|| minierr("internal error: not enough values on stack to run instruction FUNCEXPR/FUNCCALL"))?;
        
        let argcount_val = self.stack_pop_val().ok_or_else(|| minierr("internal error: not enough values on stack to run instruction FUNCEXPR/FUNCCALL"))?;
        
        let argcount = match_or_err!(argcount_val, Value::Number(argcount) => argcount, minierr("internal error: number on stack of arguments to function was not a number"))?;
        
        let mut args = VecDeque::<Value>::new();
        for _i in 0..(argcount.round() as usize)
        {
            let arg = self.stack_pop_val().ok_or_else(|| minierr("internal error: fewer variables on stack than expected in FUNCEXPR/FUNCCALL"))?;
            args.push_front(arg);
        }
        if let StackValue::Var(var) = funcdata
        {
            let funcdata_val = self.evaluate_or_store(&var, None)?.ok_or_else(|| minierr("internal error: variable meant to hold function data in FUNCEXPR/FUNCCALL was invalid"))?;
            
            let funcdata = match_or_err!(funcdata_val, Value::Func(funcdata) => funcdata, minierr("internal error: variable meant to hold function data in FUNCEXPR/FUNCCALL was not holding function data"))?;
            self.call_function(*funcdata, args, isexpr)?;
        }
        else
        {
            let funcdata = match_or_err!(funcdata, StackValue::Val(Value::Func(funcdata)) => funcdata, minierr("internal error: variable meant to hold function data in FUNCEXPR/FUNCCALL was not holding function data"))?;
            self.call_function(*funcdata, args, isexpr)?;
        }
        Ok(())
    }
    
    pub (super) fn handle_invocation(&mut self) -> OpResult
    {
        let var = self.stack_pop_var().ok_or_else(|| minierr("internal error: not enough variables on stack to run instruction INVOKE"))?;
        
        let val = self.evaluate_or_store(&var, None)?;
        
        if let Some(Value::Generator(generator_state)) = val
        {
            let frame = generator_state.frame.ok_or_else(|| minierr("error: tried to invoke a dead generator"))?;
            self.stack_push_var(var.clone());
            self.jump_to_generator_state(frame)?;
        }
        else
        {
            return Err(format!("error: tried to invoke a non-generator ({:?})", val));
        }
        
        Ok(())
    }
}