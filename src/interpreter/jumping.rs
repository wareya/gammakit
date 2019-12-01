use crate::interpreter::*;

impl Interpreter
{
    pub (crate) fn jump_to_function(&mut self, function : &FuncSpec, mut args : Vec<Value>, isexpr : bool, funcdata : &FuncVal) -> OpResult
    {
        if function.generator
        {
            return plainerr("internal error: can't use jump_to_function on a generator");
        }
        if function.argcount != args.len()
        {
            return plainerr("error: provided wrong number of arguments to function");
        }
        
        self.push_new_frame(Frame::new_from_call(&function.code, function.startaddr, function.endaddr, isexpr, false))?;
        
        self.top_frame_mut().variables.push(Value::Func(Box::new(funcdata.clone())));
        
        // copy lambda's universe, if there is one
        if let Some(ref universe) = funcdata.predefined
        {
            self.top_frame_mut().variables.extend(universe.clone());
        }
        self.set_pc(function.startaddr);
        
        self.top_frame_mut().variables.append(&mut args);
        
        Ok(())
    }
    pub (crate) fn push_new_frame(&mut self, new_frame : Frame) -> OpResult
    {
        self.frames.push(new_frame);
        //self.top_frame = self.frames.last_mut().unwrap();
        
        Ok(())
    }
    pub (crate) fn call_internal_function(&mut self, funcdata : InternalFuncVal, args : Vec<Value>, isexpr : bool) -> OpResult
    {
        let name = funcdata.nameindex;
        
        // some internal functions (e.g. instance_create()) open a new user-function frame
        // if they do, we need to add the return value to the old frame instead of the current frame
        let frames_len_before = self.frames.len();
        let ret = 
        if let Some(binding) = self.get_trivial_binding(name)
        {
            binding(self, args)?
        }
        else if let Some(binding) = self.get_trivial_simple_binding(name)
        {
            binding(args)?
        }
        else if let Some(binding_wrapper) = self.get_binding(name)
        {
            let binding = &mut *binding_wrapper.try_borrow_mut().or_else(|_| plainerr("error: tried to borrow internal function while it was borrowed elsewhere"))?;
            binding(self, args)?
        }
        else if let Some(binding_wrapper) = self.get_simple_binding(name)
        {
            let binding = &mut *binding_wrapper.try_borrow_mut().or_else(|_| plainerr("error: tried to borrow internal function while it was borrowed elsewhere"))?;
            binding(args)?
        }
        else
        {
            return plainerr("internal error: tried to look up non-extant internal function after it was already referenced in a value (this should be unreachable!)");
        };
        let frames_len = self.frames.len();
        if isexpr
        {
            match frames_len - frames_len_before
            {
                0 => self.stack_push_val(ret),
                1 =>
                {
                    let frame = self.frames.get_mut(frames_len-2).ok_or_else(|| format!("internal error: couldn't find old frame after calling function `{}` that moves the frame", name))?;
                    frame.push_val(ret);
                }
                _ => return plainerr("internal error: internal function affected the frame stack in some way other than doing nothing or adding a single frame")
            }
        }
        Ok(())
    }
    pub (crate) fn call_function(&mut self, funcdata : Box<FuncVal>, mut args : Vec<Value>, isexpr : bool) -> OpResult
    {
        let defdata = &funcdata.userdefdata;
        
        if defdata.generator
        {
            if isexpr
            {
                if defdata.argcount != args.len()
                {
                    return plainerr("error: provided wrong number of arguments to function");
                }
                let mut new_frame = Frame::new_from_call(&defdata.code, defdata.startaddr, defdata.endaddr, true, true);
                
                new_frame.variables.push(Value::Func(funcdata.clone()));
                
                for _ in 0..defdata.argcount
                {
                    new_frame.variables.push(args.pop().unwrap());
                }
                
                self.stack_push_val(Value::Generator(Box::new(GeneratorState{frame: Some(new_frame)})));
                return Ok(());
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
                self.top_frame_mut().instancestack.push(defdata.forcecontext);
                return Ok(());
            }
        }
        else
        {
            // FIXME ?
            if let Some(instance) = self.top_frame().instancestack.last().cloned()
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
                self.top_frame_mut().instancestack.push(instance);
                return Ok(());
            }
        }
        Err(minierr("FIXME unwritten error adfkgalwef"))
    }
}