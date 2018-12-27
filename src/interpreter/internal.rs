use crate::interpreter::*;

impl Interpreter
{
    pub (crate) fn build_funcspec_location(&self) -> FuncSpecLocation
    {
        let mut outer_frames = Vec::<FrameIdentity>::new();
        for frame in &self.frames
        {
            outer_frames.push(FrameIdentity::new(&frame));
        }
        FuncSpecLocation { outer_frames, top_frame : FrameIdentity::new(&self.top_frame) }
    }
    
    pub (crate) fn handle_func_call_or_expr(&mut self, global : &mut GlobalState, isexpr : bool)
    {
        if let Some(funcdata) = self.top_frame.stack.pop()
        {
            if let Some(argcount_val) = self.top_frame.stack.pop()
            {
                if let Value::Number(argcount) = argcount_val
                {
                    let mut args = Vec::<Value>::new();
                    for _i in 0..(argcount.round() as usize)
                    {
                        if let Some(arg) = self.top_frame.stack.pop()
                        {
                            args.push(arg);
                        }
                        else
                        {
                            panic!("internal error: fewer variables on stack than expected in FUNCEXPR");
                        }
                    }
                    if let Value::Var(var) = funcdata
                    {
                        if let Some(funcdata_val) = self.evaluate_or_store(global, &var, None)
                        {
                            if let Value::Func(funcdata) = funcdata_val
                            {
                                self.call_function(global, *funcdata, args, isexpr)
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
                    else if let Value::Func(funcdata) = funcdata
                    {
                        self.call_function(global, *funcdata, args, isexpr)
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