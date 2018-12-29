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
    pub (super) fn build_funcspec_location(&self) -> FuncSpecLocation
    {
        let mut outer_frames = Vec::<FrameIdentity>::new();
        for frame in &self.frames
        {
            outer_frames.push(FrameIdentity::new(&frame));
        }
        FuncSpecLocation { outer_frames, top_frame : FrameIdentity::new(&self.top_frame) }
    }
    
    pub (super) fn handle_func_call_or_expr(&mut self, isexpr : bool) -> StepResult
    {
        if let Some(funcdata) = self.stack_pop()
        {
            if let Some(argcount_val) = self.stack_pop_val()
            {
                if let Value::Number(argcount) = argcount_val
                {
                    let mut args = Vec::<Value>::new();
                    for _i in 0..(argcount.round() as usize)
                    {
                        if let Some(arg) = self.stack_pop_val()
                        {
                            args.push(arg);
                        }
                        else
                        {
                            panic!("internal error: fewer variables on stack than expected in FUNCEXPR");
                        }
                    }
                    if let StackValue::Var(var) = funcdata
                    {
                        if let Some(funcdata_val) = self.evaluate_or_store(&var, None)?
                        {
                            if let Value::Func(funcdata) = funcdata_val
                            {
                                self.call_function(*funcdata, args, isexpr);
                                Ok(())
                            }
                            else
                            {
                                panic!("internal error: variable meant to hold function data in FUNCEXPR was not holding function data");
                            }
                        }
                        else
                        {
                            panic!("internal error: variable meant to hold function data in FUNCEXPR was invalid");
                        }
                    }
                    else if let StackValue::Val(Value::Func(funcdata)) = funcdata
                    {
                        self.call_function(*funcdata, args, isexpr);
                        Ok(())
                    }
                    else
                    {
                        panic!("internal error: variable meant to hold function data in FUNCEXPR was not holding function data");
                    }
                }
                else
                {
                    panic!("internal error: number on stack of arguments to function was not a number");
                }
            }
            else
            {
                panic!("internal error: not enough values on stack to run instruction FUNCEXPR");
            }
        }
        else
        {
            panic!("internal error: not enough values on stack to run instruction FUNCEXPR");
        }
    }
}