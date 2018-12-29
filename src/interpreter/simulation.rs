#![allow(clippy::cast_lossless)]
#![allow(clippy::map_entry)]

use crate::interpreter::*;

impl Interpreter
{
    pub (crate) fn get_opfunc(&mut self, op : u8) -> Option<Box<Fn(&mut Interpreter)>>
    {
        macro_rules! enbox {
            ( $x:ident ) =>
            {
                Some(Box::new(Interpreter::$x))
            }
        }
        match op
        {
            NOP => enbox!(sim_NOP),
            PUSHFLT => enbox!(sim_PUSHFLT),
            PUSHSHORT => enbox!(sim_PUSHSHORT),
            PUSHSTR => enbox!(sim_PUSHSTR),
            PUSHNAME => enbox!(sim_PUSHNAME),
            PUSHVAR => enbox!(sim_PUSHVAR),
            DECLVAR => enbox!(sim_DECLVAR),
            DECLFAR => enbox!(sim_DECLFAR),
            BINSTATE => enbox!(sim_BINSTATE),
            BINOP => enbox!(sim_BINOP),
            UNOP => enbox!(sim_UNOP),
            INDIRECTION => enbox!(sim_INDIRECTION),
            EVALUATION => enbox!(sim_EVALUATION),
            FUNCCALL => enbox!(sim_FUNCCALL),
            FUNCEXPR => enbox!(sim_FUNCEXPR),
            FUNCDEF => enbox!(sim_FUNCDEF),
            LAMBDA => enbox!(sim_LAMBDA),
            OBJDEF => enbox!(sim_OBJDEF),
            COLLECTARRAY => enbox!(sim_COLLECTARRAY),
            COLLECTDICT => enbox!(sim_COLLECTDICT),
            ARRAYEXPR => enbox!(sim_ARRAYEXPR),
            BREAK => enbox!(sim_BREAK),
            CONTINUE => enbox!(sim_CONTINUE),
            IF => enbox!(sim_IF),
            IFELSE => enbox!(sim_IFELSE),
            WHILE => enbox!(sim_WHILE),
            FOR => enbox!(sim_FOR),
            SCOPE => enbox!(sim_SCOPE),
            UNSCOPE => enbox!(sim_UNSCOPE),
            WITH => enbox!(sim_WITH),
            EXIT => enbox!(sim_EXIT),
            RETURN => enbox!(sim_RETURN),
            LINENUM => enbox!(sim_LINENUM),
            _ => None
        }
    }
    
    #[allow(non_snake_case)] 
    pub (crate) fn sim_NOP(&mut self)
    {
        
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_PUSHFLT(&mut self)
    {
        let value = unpack_f64(&self.pull_from_code(8));
        self.stack_push_val(Value::Number(value));
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_PUSHSHORT(&mut self)
    {
        let value = unpack_u16(&self.pull_from_code(2));
        self.stack_push_val(Value::Number(value as f64));
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_PUSHSTR(&mut self)
    {
        let text = self.read_string();
        self.stack_push_val(Value::Text(text));
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_PUSHNAME(&mut self)
    {
        let text = self.read_string();
        self.stack_push_var(Variable::Direct(DirectVar{name:text}));
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_PUSHVAR(&mut self)
    {
        let name = self.read_string();
        let dirvar = Variable::Direct(DirectVar{name : name.clone()}); // FIXME suboptimal but helps error message
        if let Some(val) = self.evaluate_or_store(&dirvar, None)
        {
            self.stack_push_val(val);
        }
        else
        {
            panic!("error: tried to evaluate non-extant variable `{}`", name);
        }
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_DECLVAR(&mut self)
    {
        if self.stack_len() < 1
        {
            panic!("internal error: DECLVAR instruction requires 1 values on the stack but only found 0");
        }
        if let Some(name) = self.stack_pop_name()
        {
            if let Some(scope) = self.top_frame.scopes.last_mut()
            {
                if scope.contains_key(&name)
                {
                    panic!("error: redeclared identifier {}", name);
                }
                scope.insert(name, Value::Number(0.0));
            }
            else
            {
                panic!("internal error: there are no scopes in the top frame");
            }
        }
        else
        {
            panic!("internal error: tried to declare a variable with a name of invalid type");
        }
    }
    
    #[allow(non_snake_case)]
    pub (crate) fn sim_DECLFAR(&mut self)
    {
        if self.stack_len() < 1
        {
            panic!("internal error: DECLFAR instruction requires 1 values on the stack but only found 0");
        }
        if let Some(name) = self.stack_pop_name()
        {
            if let Some(instance_id) = self.top_frame.instancestack.last()
            {
                if let Some(instance) = self.global.instances.get_mut(instance_id)
                {
                    if !instance.variables.contains_key(&name)
                    {
                        instance.variables.insert(name, Value::Number(0.0));
                    }
                    else
                    {
                        panic!("error: redeclared identifier {}", name);
                    }
                }
                else
                {
                    panic!("error: tried to declare instance variable but instance of current scope ({}) no longer exists", instance_id);
                }
            }
            else
            {
                panic!("error: tried to declare instance variable when not executing within instance scope");
            }
        }
        else
        {
            panic!("internal error: tried to declare instance variable with non-var-name type name");
        }
    }
    
    #[allow(non_snake_case)]
    pub (crate) fn sim_INDIRECTION(&mut self)
    {
        if self.stack_len() < 2
        {
            panic!("internal error: INDIRECTION instruction requires 2 values on the stack but only found {}", self.stack_len());
        }
        if let Some(right) = self.stack_pop_name()
        {
            if let Some(left) = self.stack_pop_number()
            {
                let id = left.round() as usize;
                
                if self.global.instances.contains_key(&id)
                {
                    self.stack_push_var(Variable::Indirect(IndirectVar{ident : id, name : right}));
                }
                else
                {
                    panic!("error: tried to perform indirection on instance {} that doesn't exist", id)
                }
            }
            else
            {
                panic!("error: tried to use indirection on a type that can't be an identifier (only numbers can be identifiers)")
            }
        }
        else
        {
            panic!("error: FIXME ADFGJAWEIFASDFJGERG")
        }
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_EVALUATION(&mut self)
    {
        if let Some(var) = self.stack_pop_var()
        {
            match var
            {
                Variable::Indirect(_) |
                Variable::Array(_) =>
                {
                    if let Some(value) = self.evaluate_or_store(&var, None)
                    {
                        self.stack_push_val(value);
                    }
                    else
                    {
                        panic!("internal error: evaluate_or_store returned None when just storing a variable");
                    }
                }
                Variable::Direct(var) =>
                {
                    panic!("internal error: tried to evaluate direct variable `{}`\n(note: the evaluation instruction is for indirect (id.y) variables and array (arr[0]) variables; bytecode metaprogramming for dynamic direct variable access is unsupported)", var.name);
                }
            }
        }
        else
        {
            panic!("internal error: failed to find a variable in the stack in EVALUATION");
        }
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_FUNCCALL(&mut self)
    {
        self.handle_func_call_or_expr(false);
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_FUNCEXPR(&mut self)
    {
        self.handle_func_call_or_expr(true);
    }
    
    #[allow(non_snake_case)]
    pub (crate) fn sim_SCOPE(&mut self)
    {
        self.top_frame.scopes.push(HashMap::new());
        let here = self.get_pc();
        self.top_frame.scopestarts.push(here);
        if self.top_frame.scopes.len() >= 0x10000
        {
            panic!("error: scope recursion limit of 0x10000 reached at line {}\n(note: use more functions!)", self.top_frame.currline);
        }
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_UNSCOPE(&mut self)
    {
        let immediate = unpack_u16(&self.pull_from_code(2)) as usize;
        
        self.drain_scopes((immediate+1) as u16);
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_BREAK(&mut self)
    {
        self.pop_controlstack_until_loop();
        
        if self.top_frame.controlstack.is_empty()
        {
            panic!("error: break instruction not inside of loop");
        }
        
        let controller = self.top_frame.controlstack.last().unwrap().clone();
        
        if controller.controltype == WHILE
        {
            self.set_pc(controller.controlpoints[2]);
            self.drain_scopes(controller.scopes);
            self.top_frame.controlstack.pop();
        }
        else if controller.controltype == FOR
        {
            self.set_pc(controller.controlpoints[3]);
            self.drain_scopes(controller.scopes);
            self.top_frame.controlstack.pop();
        }
        else
        {
            panic!("FIXME: unimplemented BREAK out from 0x{:02X} loop", controller.controltype);
        }
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_CONTINUE(&mut self)
    {
        self.pop_controlstack_until_loop();
        
        if self.top_frame.controlstack.is_empty()
        {
            panic!("error: continue instruction not inside of loop");
        }
        
        let controller = self.top_frame.controlstack.last().unwrap().clone();
        
        if controller.controltype == WHILE
        {
            self.set_pc(controller.controlpoints[0]);
            self.drain_scopes(controller.scopes);
        }
        else if controller.controltype == FOR
        {
            self.set_pc(controller.controlpoints[1]);
            self.suppress_for_expr_end = true;
            self.drain_scopes(controller.scopes);
        }
        else
        {
            panic!("FIXME: unimplemented CONTINUE out from 0x{:02X} loop", controller.controltype);
        }
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_IF(&mut self)
    {
        let exprlen = unpack_u64(&self.pull_from_code(8)) as usize;
        let codelen = unpack_u64(&self.pull_from_code(8)) as usize;
        let current_pc = self.get_pc();
        let scopelen = self.top_frame.scopes.len() as u16;
        self.top_frame.controlstack.push(ControlData{controltype : IF, controlpoints : vec!(current_pc+exprlen, current_pc+exprlen+codelen), scopes : scopelen, other : None});
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_IFELSE(&mut self)
    {
        let exprlen = unpack_u64(&self.pull_from_code(8)) as usize;
        let codelen1 = unpack_u64(&self.pull_from_code(8)) as usize;
        let codelen2 = unpack_u64(&self.pull_from_code(8)) as usize;
        let current_pc = self.get_pc();
        let scopelen = self.top_frame.scopes.len() as u16;
        self.top_frame.controlstack.push(ControlData{controltype : IFELSE, controlpoints : vec!(current_pc+exprlen, current_pc+exprlen+codelen1, current_pc+exprlen+codelen1+codelen2), scopes : scopelen, other : None});
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_WHILE(&mut self)
    {
        let exprlen = unpack_u64(&self.pull_from_code(8)) as usize;
        let codelen = unpack_u64(&self.pull_from_code(8)) as usize;
        let current_pc = self.get_pc();
        let scopelen = self.top_frame.scopes.len() as u16;
        self.top_frame.controlstack.push(ControlData{controltype : WHILE, controlpoints : vec!(current_pc, current_pc+exprlen, current_pc+exprlen+codelen), scopes : scopelen, other : None});
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_FOR(&mut self)
    {
        let exprlen = unpack_u64(&self.pull_from_code(8)) as usize;
        let postlen = unpack_u64(&self.pull_from_code(8)) as usize;
        let codelen = unpack_u64(&self.pull_from_code(8)) as usize;
        let current_pc = self.get_pc();
        let scopelen = self.top_frame.scopes.len() as u16;
        self.top_frame.controlstack.push(ControlData{controltype : FOR, controlpoints : vec!(current_pc, current_pc+exprlen, current_pc+exprlen+postlen, current_pc+exprlen+postlen+codelen), scopes : scopelen, other : None});
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_WITH(&mut self)
    {
        if self.stack_len() < 1
        {
            panic!("internal error: WITH instruction requires 1 values on the stack but found 0");
        }
        // NOTE: for with(), the self.top_frame.scopes.len() >= 0xFFFF error case is handled by SCOPE instruction
        if let Some(expr) = self.stack_pop_number()
        {
            let other_id = expr.round() as usize;
            
            let codelen = unpack_u64(&self.pull_from_code(8));
            
            let current_pc = self.get_pc();
            
            if self.global.instances.contains_key(&other_id)
            {
                self.top_frame.instancestack.push(other_id);
                
                self.top_frame.controlstack.push(ControlData{controltype : WITH, controlpoints : vec!(current_pc, current_pc + codelen as usize), scopes : self.top_frame.scopes.len() as u16, other : Some(VecDeque::new())});
            }
            else if let Some(instance_id_list) = self.global.instances_by_type.get(&other_id)
            {
                if let Some(first) = instance_id_list.first()
                {
                    self.top_frame.instancestack.push(*first);
                    let mut copylist : VecDeque<usize> = instance_id_list.iter().cloned().collect();
                    copylist.pop_front();
                    self.top_frame.controlstack.push(ControlData{controltype : WITH, controlpoints : vec!(current_pc, current_pc + codelen as usize), scopes : self.top_frame.scopes.len() as u16, other : Some(copylist)});
                }
                else
                {
                    // silently skip block if there are no instances of this object type
                    self.add_pc(codelen as usize);
                }
            }
            else
            {
                panic!("error: tried to use non-existant instance in with expression");
            }
        }
        else
        {
            panic!("error: tried to use with() on a non-numeric expression (instance ids and object ids are numeric)");
        }
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_FUNCDEF(&mut self)
    {
        let (funcname, myfuncspec) = self.read_function();
        if let Some(scope) = self.top_frame.scopes.last_mut()
        {
            if scope.contains_key(&funcname)
            {
                panic!("error: redeclared identifier {}", funcname)
            }
            //scope.insert(funcname.clone(), Value::Func(Box::new(FuncVal { internal : false, name : Some(funcname), predefined : None, userdefdata : Some(myfuncspec) })));
            scope.insert(funcname.clone(), Value::new_funcval(false, Some(funcname), None, Some(myfuncspec)));
        }
        else
        {
            panic!("internal error: there are no scopes in the top frame");
        }
    }
    
    #[allow(non_snake_case)]
    pub (crate) fn sim_BINSTATE(&mut self)
    {
        if self.stack_len() < 2
        {
            panic!("internal error: BINSTATE instruction requires 2 values on the stack but found {}", self.stack_len());
        }
        
        let immediate = self.pull_single_from_code();
        
        if let Some(value) = self.stack_pop_val()
        {
            if let Some(var) = self.stack_pop_var()
            {
                if immediate == 0x00
                {
                    self.evaluate_or_store(&var, Some(value));
                }
                else if let Some(opfunc) = get_binop_function(immediate)
                {
                    if let Some(var_initial_value) = self.evaluate_or_store(&var, None)
                    {
                        match opfunc(&var_initial_value, &value)
                        {
                            Ok(var_new_value) =>
                            {
                                self.evaluate_or_store(&var, Some(var_new_value));
                            }
                            Err(text) =>
                            {
                                //panic!("error: disallowed binary statement\n({})\n(line {})", text, self.top_frame.currline);
                                panic!("error: disallowed binary statement\n({})", text);
                            }
                        }
                    }
                    else
                    {
                        panic!("internal error: evaluate_or_store returned None when just accessing value");
                    }
                }
                else
                {
                    panic!("internal error: unknown binary operation 0x{:02X}", immediate);
                }
            }
            else
            {
                panic!("internal error: primary argument to BINSTATE could not be found or was not a variable");
            }
        }
        else
        {
            panic!("internal error: not enough values on stack to run instruction BINSTATE (this error should be inaccessible)");
        }
    }
    
    #[allow(non_snake_case)]
    pub (crate) fn sim_BINOP(&mut self)
    {
        if self.stack_len() < 2
        {
            panic!("internal error: BINOP instruction requires 2 values on the stack but found {}", self.stack_len());
        }
        
        let immediate = self.pull_single_from_code();
        
        if let Some(right) = self.stack_pop_val()
        {
            if let Some(left) = self.stack_pop_val()
            {
                if let Some(opfunc) = get_binop_function(immediate)
                {
                    match opfunc(&left, &right)
                    {
                        Ok(new_value) =>
                        {
                            self.stack_push_val(new_value);
                        }
                        Err(text) =>
                        {
                            panic!("error: disallowed binary expression\n({})\n(value 1: {})\n(value 2: {})", text, format_val(&left).unwrap(), format_val(&right).unwrap());
                        }
                    }
                }
                else
                {
                    panic!("internal error: unknown binary operation 0x{:02X}", immediate);
                }
            }
            else
            {
                panic!("internal error: not enough values on stack to run instruction BINOP (this error should be inaccessible!)");
            }
        }
        else
        {
            panic!("internal error: not enough values on stack to run instruction BINOP (this error should be inaccessible!)");
        }
    }
    
    #[allow(non_snake_case)]
    pub (crate) fn sim_UNOP(&mut self)
    {
        if self.stack_len() < 1
        {
            panic!("internal error: UNOP instruction requires 1 values on the stack but found {}", self.stack_len());
        }
        
        let immediate = self.pull_single_from_code();
        
        if let Some(value) = self.stack_pop_val()
        {
            if let Some(opfunc) = get_unop_function(immediate)
            {
                match opfunc(&value)
                {
                    Ok(new_value) =>
                    {
                        self.stack_push_val(new_value);
                    }
                    Err(text) =>
                    {
                        panic!("error: disallowed unary expression\n({})", text);
                    }
                }
            }
            else
            {
                panic!("internal error: unknown binary operation 0x{:02X}", immediate);
            }
        }
        else
        {
            panic!("internal error: not enough values on stack to run instruction UNOP (this error should be inaccessible!)");
        }
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_LAMBDA(&mut self)
    {
        let (captures, myfuncspec) = self.read_lambda();
        self.stack_push_val(Value::new_funcval(false, Some("lambda_self".to_string()), Some(captures), Some(myfuncspec)));
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_OBJDEF(&mut self)
    {
        let name = self.read_string();
        if self.global.objectnames.contains_key(&name)
        {
            panic!("error: redeclared object {}", name);
        }
        
        let object_id = self.global.object_id;
        let numfuncs = unpack_u16(&self.pull_from_code(2));
        
        let mut funcs = HashMap::<String, FuncSpec>::new();
        for _ in 0..numfuncs
        {
            let (funcname, mut myfuncspec) = self.read_function();
            myfuncspec.fromobj = true;
            myfuncspec.parentobj = object_id;
            if funcs.contains_key(&funcname)
            {
                panic!("error: redeclared function {} in object {}", funcname, name);
            }
            funcs.insert(funcname, myfuncspec);
        }
        
        self.global.objectnames.insert(name.clone(), object_id);
        self.global.objects.insert(object_id, ObjSpec { ident : object_id, name, functions : funcs });
        self.global.instances_by_type.insert(object_id, Vec::new());
        
        self.global.object_id += 1;
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_COLLECTARRAY(&mut self)
    {
        let numvals = unpack_u16(&self.pull_from_code(2)) as usize;
        if self.stack_len() < numvals
        {
            panic!("internal error: not enough values on stack for COLLECTARRAY instruction to build array (need {}, have {})", numvals, self.stack_len());
        }
        let mut myarray = VecDeque::<Value>::new();
        for _i in 0..numvals
        {
            if let Some(val) = self.stack_pop_val()
            {
                myarray.push_front(val);
            }
            else
            {
                panic!("internal error: COLLECTARRAY instruction failed to collect values from stack (this error should be unreachable!)");
            }
        }
        self.stack_push_val(Value::Array(myarray));
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_COLLECTDICT(&mut self)
    {
        let numvals = unpack_u16(&self.pull_from_code(2)) as usize;
        if self.stack_len() < numvals*2
        {
            panic!("internal error: not enough values on stack for COLLECTDICT instruction to build dict (need {}, have {})", numvals*2, self.stack_len());
        }
        
        let mut names = VecDeque::<HashableValue>::new();
        let mut values = VecDeque::<Value>::new();
        
        for _i in 0..numvals
        {
            if let Some(val) = self.stack_pop_val()
            {
                if let Some(key) = self.stack_pop_val()
                {
                    values.push_front(val);
                    match key
                    {
                        Value::Number(number) =>
                        {
                            names.push_front(HashableValue::Number(number));
                        }
                        Value::Text(text) =>
                        {
                            names.push_front(HashableValue::Text(text));
                        }
                        _ =>
                        {
                            panic!("error: dictionary key must be a string or number; was {:?}; line {}", key, self.top_frame.currline);
                        }
                    }
                }
                else
                {
                    panic!("internal error: COLLECTDICT instruction failed to collect values from stack");
                }
            }
            else
            {
                panic!("internal error: COLLECTDICT instruction failed to collect values from stack");
            }
        }
        let mut mydict = HashMap::<HashableValue, Value>::new();
        for (name, value) in names.into_iter().zip(values.into_iter())
        {
            mydict.insert(name, value);
        }
        self.stack_push_val(Value::Dict(mydict));
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_ARRAYEXPR(&mut self)
    {
        if self.stack_len() < 2
        {
            panic!("internal error: ARRAYEXPR instruction requires 2 values on the stack but found {}", self.stack_len());
        }
        if let Some(index) = self.stack_pop_val()
        {
            if let Some(array) = self.stack_pop()
            {
                match array
                {
                    StackValue::Var(Variable::Array(mut arrayvar)) =>
                    {
                        arrayvar.indexes.push(index);
                        self.stack_push_var(Variable::Array(arrayvar));
                    }
                    StackValue::Var(Variable::Direct(dirvar)) =>
                    {
                        self.stack_push_var(Variable::Array(ArrayVar { location : NonArrayVariable::Direct(dirvar), indexes : vec!(index).into_iter().collect() } ));
                    }
                    StackValue::Var(Variable::Indirect(indirvar)) =>
                    {
                        self.stack_push_var(Variable::Array(ArrayVar { location : NonArrayVariable::Indirect(indirvar), indexes : vec!(index).into_iter().collect() } ));
                    }
                    StackValue::Val(Value::Array(array)) =>
                    {
                        self.stack_push_var(Variable::Array(ArrayVar { location : NonArrayVariable::ActualArray(array), indexes : vec!(index).into_iter().collect() } ));
                    }
                    _ =>
                    {
                        //panic!("error: tried to use array indexing on a non-indexable value\n{}", array);
                        panic!("error: tried to use array indexing on a non-indexable value");
                    }
                }
            }
            else
            {
                panic!("internal error: TODO write error askdgfauiowef");
            }
        }
        else
        {
            panic!("internal error: TODO write error askdgfauiowef");
        }
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_EXIT(&mut self) // an exit is a return with no value
    {
        if let Some(top_frame) = self.frames.pop()
        {
            let frame_was_expr = self.top_frame.isexpr;
            self.top_frame = top_frame;
            // exit implies no pushed variable. if the outside expects a value, push it
            if frame_was_expr
            {
                self.stack_push_val(Value::Number(0.0));
            }
        }
        else
        {
            self.doexit = true;
        }
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_RETURN(&mut self)
    {
        if let Some(old_frame) = self.frames.pop()
        {
            let inner_frame_stack_last = self.stack_pop();
            let frame_was_expr = self.top_frame.isexpr;
            self.top_frame = old_frame;
            // exit implies no pushed variable. if the outside expects a value, push it
            if frame_was_expr
            {
                if let Some(val) = inner_frame_stack_last
                {
                    self.stack_push(val);
                }
                else
                {
                    panic!("error: RETURN instruction needed a value remaining on the inner frame's stack, but there were none");
                }
            }
        }
        else
        {
            panic!("error: attempted to return from global code; use exit() instead");
        }
    }
    #[allow(non_snake_case)]
    pub (crate) fn sim_LINENUM(&mut self)
    {
        self.top_frame.currline = unpack_u64(&self.pull_from_code(8)) as usize;
    }
}