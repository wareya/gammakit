use crate::interpreter::*;

impl Interpreter
{
    pub (crate) fn jump_to_function(&mut self, function : &FuncSpec, mut args : Vec<Value>, isexpr : bool, funcdata : &FuncVal) -> OpResult
    {
        if function.varnames.len() > args.len()
        {
            return plainerr("error: did not provide enough arguments to function");
        }
        if function.varnames.len() < args.len()
        {
            return plainerr("error: provided too many arguments to function");
        }
        
        let mut frameswapper = Frame::new_from_call(Rc::clone(&function.code), function.startaddr, function.endaddr, isexpr, function.impassable);
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
    pub (crate) fn call_function(&mut self, funcdata : FuncVal, args : Vec<Value>, isexpr : bool) -> OpResult
    {
        if funcdata.internal
        {
            let name = funcdata.name.ok_or_else(|| minierr("internal error: function variable describing internal function is lacking its function name"))?;
            
            let internal_func = self.get_internal_function(&name).ok_or_else(|| minierr("internal error: tried to look up non-extant internal function after it was already referenced in a value (this should be unreachable!)"))?;
            
            let (ret, moved_frame) = internal_func(self, args, isexpr)?;
            if isexpr && !self.internal_function_is_noreturn(&name)
            {
                if !moved_frame
                {
                    self.stack_push_val(ret);
                }
                else
                {
                    let frame = self.frames.last_mut().ok_or_else(|| format!("internal error: couldn't find old frame after calling function `{}` that moves the frame", name))?;
                    frame.push_val(ret);
                }
            }
        }
        else
        {
            let definition = funcdata.userdefdata.clone();
            let defdata = definition.ok_or_else(|| minierr("internal error: called a function that was not internal but didn't have definition data"))?;
            
            if !defdata.fromobj
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