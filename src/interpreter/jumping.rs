use crate::interpreter::*;

impl Interpreter
{
    pub(super) fn jump_to_function(&mut self, function : &FuncSpec, mut args : Vec<Value>, isexpr : bool, funcdata : &FuncVal)
    {
        if function.varnames.len() > args.len()
        {
            panic!("error: did not provide enough arguments to function");
        }
        if function.varnames.len() < args.len()
        {
            panic!("error: provided too many arguments to function");
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
        
        if let Some(scope) = self.top_frame.scopes.last_mut()
        {
            if let Some(ref name) = funcdata.name
            {
                scope.insert(name.clone(), Value::Func(Box::new(funcdata.clone())));
            }
            for varname in &function.varnames
            {
                if let Some(arg) = args.pop()
                {
                    scope.insert(varname.clone(), arg);
                }
                else
                {
                    panic!("internal error: list of arguments to provide to function was shorter than list of argument names (this error should be unreachable!)");
                }
            }
        }
        else
        {
            panic!("internal error: no scope in top frame despite just making it in jump_to_function (this error should be unreachable!)");
        }
    }
    pub(super) fn call_function(&mut self, global : &mut GlobalState, funcdata : FuncVal, args : Vec<Value>, isexpr : bool)
    {
        if funcdata.internal
        {
            if let Some(name) = funcdata.name
            {
                if let Some(internal_func) = self.get_internal_function(&name)
                {
                    let (ret, moved_frame) = internal_func(self, global, args, isexpr);
                    if isexpr && !self.internal_function_is_noreturn(&name)
                    {
                        let frames_len = self.frames.len(); // for the panic down there (non-lexical borrow lifetimes pls happen soon)
                        if !moved_frame
                        {
                            self.top_frame.stack.push(ret);
                        }
                        else if let Some(frame) = self.frames.last_mut()
                        {
                            frame.stack.push(ret);
                        }
                        else
                        {
                            panic!("internal error: couldn't find old frame after calling function `{}` that moves the frame; framestack has length {}", name, frames_len);
                        }
                    }
                }
                else
                {
                    panic!("internal error: tried to look up non-extant internal function after it was already referenced in a value (this should be unreachable!)");
                }
            }
            else
            {
                panic!("internal error: function variable describing internal function is lacking its function name");
            }
        }
        else
        {
            let definition = funcdata.userdefdata.clone();
            if let Some(defdata) = definition
            {
                if !defdata.fromobj
                {
                    self.jump_to_function(&defdata, args, isexpr, &funcdata);
                    return;
                }
                else if defdata.forcecontext != 0
                {
                    if let Some(inst) = global.instances.get(&defdata.forcecontext)
                    {
                        // FIXME ?
                        if !global.objects.contains_key(&inst.objtype)
                        {
                            panic!("error: tried to access data from object type {} that no longer exists", inst.objtype);
                        }
                        if defdata.parentobj != inst.objtype
                        {
                            panic!("error: tried to call function from object type {} in the context of an instance of object type {}", defdata.parentobj, inst.objtype);
                        }
                        self.jump_to_function(&defdata, args, isexpr, &funcdata);
                        self.top_frame.instancestack.push(defdata.forcecontext);
                        return;
                    }
                }
                else
                {
                    // FIXME ?
                    let inst_copy : Vec<usize> = self.top_frame.instancestack.iter().cloned().rev().collect();
                    for instance in inst_copy
                    {
                        if let Some(inst) = global.instances.get(&instance)
                        {
                            if !global.objects.contains_key(&inst.objtype)
                            {
                                panic!("error: tried to access data from object type {} that no longer exists", inst.objtype);
                            }
                            if defdata.parentobj != inst.objtype
                            {
                                panic!("error: tried to call function from object type {} in the context of an instance of object type {}", defdata.parentobj, inst.objtype);
                            }
                            self.jump_to_function(&defdata, args, isexpr, &funcdata);
                            self.top_frame.instancestack.push(instance);
                            return;
                        }
                        else
                        {
                            panic!("TODO error aidsfgojaedfouajiefjfbdgnwru");
                        }
                    }
                }
            }
            else
            {
                panic!("internal error: called a function that was not internal but didn't have definition data");
            }
        }
    }
}