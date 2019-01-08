use crate::interpreter::*;

impl Interpreter
{
    pub (crate) fn jump_to_function(&mut self, function : &FuncSpec, mut args : Vec<Value>, isexpr : bool, funcdata : &FuncVal) -> OpResult
    {
        if function.generator
        {
            return plainerr("internal error: can't use jump_to_function on a generator");
        }
        if function.varnames.len() > args.len()
        {
            return plainerr("error: did not provide enough arguments to function");
        }
        if function.varnames.len() < args.len()
        {
            return plainerr("error: provided too many arguments to function");
        }
        
        let mut frameswapper = Frame::new_from_call(Rc::clone(&function.code), function.startaddr, function.endaddr, isexpr, function.impassable, false);
        std::mem::swap(&mut frameswapper, &mut self.top_frame);
        self.frames.push(frameswapper);
        
        // copy lambda's universe, if there is one
        if let Some(ref universe) = funcdata.predefined
        {
            self.top_frame.scopes = vec!(universe.clone());
        }
        self.set_pc(function.startaddr);
        
        let scope = self.top_frame.scopes.last_mut().ok_or_else(|| minierr("internal error: no scope in top frame despite just making it in jump_to_function (this error should be unreachable!)"))?;
        
        if let Some(ref name) = funcdata.name
        {
            scope.insert(name.clone(), Value::Func(Box::new(funcdata.clone())));
        }
        for varname in &function.varnames
        {
            let arg = args.pop().ok_or_else(|| minierr("internal error: list of arguments to provide to function was shorter than list of argument names (this error should be unreachable!)"))?;
            scope.insert(varname.clone(), arg);
        }
        
        Ok(())
    }
    pub (crate) fn jump_to_generator_state(&mut self, mut new_frame : Frame) -> OpResult
    {
        std::mem::swap(&mut new_frame, &mut self.top_frame);
        self.frames.push(new_frame);
        
        Ok(())
    }
    pub (crate) fn call_function(&mut self, funcdata : FuncVal, args : Vec<Value>, isexpr : bool) -> OpResult
    {
        if funcdata.internal
        {
            let name = funcdata.name.ok_or_else(|| minierr("internal error: function variable describing internal function is lacking its function name"))?;
            
            let internal_func = self.get_internal_function(&name).ok_or_else(|| minierr("internal error: tried to look up non-extant internal function after it was already referenced in a value (this should be unreachable!)"))?;
            
            // some internal functions (e.g. instance_create()) open a new user-function frame
            // if they do, we need to add the return value to the old frame instead of the current frame
            let frames_len_before = self.frames.len();
            let ret = internal_func(self, args)?;
            if isexpr
            {
                match self.frames.len() - frames_len_before
                {
                    0 => self.stack_push_val(ret),
                    1 =>
                    {
                        let frame = self.frames.last_mut().ok_or_else(|| format!("internal error: couldn't find old frame after calling function `{}` that moves the frame", name))?;
                        frame.push_val(ret);
                    }
                    _ => return plainerr("internal error: internal function affected the frame stack in some way other than doing nothing or adding a single frame")
                }
            }
        }
        else
        {
            let defdata = funcdata.userdefdata.as_ref().ok_or_else(|| minierr("internal error: called a function that was not internal but didn't have definition data"))?;
            
            if defdata.generator
            {
                if isexpr
                {
                    // mostly copy pasted from jump_to_function... but didn't want to make jump_to_function even more complicated.
                    // make a new function for constructing a function's first frame?
                    if defdata.varnames.len() > args.len()
                    {
                        return plainerr("error: did not provide enough arguments to function");
                    }
                    if defdata.varnames.len() < args.len()
                    {
                        return plainerr("error: provided too many arguments to function");
                    }
                    let mut new_frame = Frame::new_from_call(Rc::clone(&defdata.code), defdata.startaddr, defdata.endaddr, true, true, true);
                    
                    let scope = new_frame.scopes.last_mut().ok_or_else(|| minierr("internal error: no scope in top frame despite just making it in jump_to_function (this error should be unreachable!)"))?;
                    if let Some(ref name) = funcdata.name
                    {
                        scope.insert(name.clone(), Value::Func(Box::new(funcdata.clone())));
                    }
                    let mut args = args.clone();
                    for varname in &defdata.varnames
                    {
                        let arg = args.pop().ok_or_else(|| minierr("internal error: list of arguments to provide to function was shorter than list of argument names (this error should be unreachable!)"))?;
                        scope.insert(varname.clone(), arg);
                    }
                    
                    self.stack_push_val(Value::Generator(GeneratorState{frame: Some(new_frame)}));
                }
            }
            else if !defdata.fromobj
            {
                self.jump_to_function(&defdata, args, isexpr, &funcdata)?;
                return Ok(());
            }
            else if defdata.forcecontext != 0
            {
                if let Some(inst) = self.global.instances.get(&defdata.forcecontext)
                {
                    // FIXME ?
                    if !self.global.objects.contains_key(&inst.objtype)
                    {
                        return Err(format!("error: tried to access data from object type {} that no longer exists", inst.objtype));
                    }
                    if defdata.parentobj != inst.objtype
                    {
                        return Err(format!("error: tried to call function from object type {} in the context of an instance of object type {}", defdata.parentobj, inst.objtype));
                    }
                    self.jump_to_function(&defdata, args, isexpr, &funcdata)?;
                    self.top_frame.instancestack.push(defdata.forcecontext);
                    return Ok(());
                }
            }
            else
            {
                // FIXME ?
                if let Some(instance) = self.top_frame.instancestack.last().cloned()
                {
                    let inst = self.global.instances.get(&instance).ok_or_else(|| minierr("internal error: tried to look for a variable inside of an instance that no longer exists (this might not be an error state!)"))?;
                    
                    if !self.global.objects.contains_key(&inst.objtype)
                    {
                        return Err(format!("error: tried to access data from object type {} that no longer exists", inst.objtype));
                    }
                    if defdata.parentobj != inst.objtype
                    {
                        return Err(format!("error: tried to call function from object type {} in the context of an instance of object type {}", defdata.parentobj, inst.objtype));
                    }
                    self.jump_to_function(&defdata, args, isexpr, &funcdata)?; // opens a new frame, changing top_frame to a clean slate
                    self.top_frame.instancestack.push(instance);
                    return Ok(());
                }
            }
        }
        Ok(())
    }
}