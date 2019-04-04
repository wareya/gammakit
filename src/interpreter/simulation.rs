#![allow(clippy::cast_lossless, clippy::map_entry, non_snake_case)]

use crate::interpreter::*;

impl Interpreter
{
    pub (crate) fn get_opfunc(&mut self, op : u8) -> Option<Box<Fn(&mut Interpreter) -> OpResult>>
    {
        macro_rules! enbox { ( $x:ident ) => { Some(Box::new(Interpreter::$x)) } }
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
            DECLGLOBALVAR => enbox!(sim_DECLGLOBALVAR),
            BINSTATE => enbox!(sim_BINSTATE),
            UNSTATE => enbox!(sim_UNSTATE),
            BINOP => enbox!(sim_BINOP),
            UNOP => enbox!(sim_UNOP),
            SHORTCIRCUITIFTRUE => enbox!(sim_SHORTCIRCUITIFTRUE),
            SHORTCIRCUITIFFALSE => enbox!(sim_SHORTCIRCUITIFFALSE),
            INDIRECTION => enbox!(sim_INDIRECTION),
            DISMEMBER => enbox!(sim_DISMEMBER),
            EVALUATION => enbox!(sim_EVALUATION),
            FUNCCALL => enbox!(sim_FUNCCALL),
            FUNCEXPR => enbox!(sim_FUNCEXPR),
            INVOKE => enbox!(sim_INVOKE),
            INVOKECALL => enbox!(sim_INVOKECALL),
            INVOKEEXPR => enbox!(sim_INVOKEEXPR),
            FUNCDEF => enbox!(sim_FUNCDEF),
            LAMBDA => enbox!(sim_LAMBDA),
            OBJDEF => enbox!(sim_OBJDEF),
            GLOBALFUNCDEF => enbox!(sim_GLOBALFUNCDEF),
            SUBFUNCDEF => enbox!(sim_SUBFUNCDEF),
            GENERATORDEF => enbox!(sim_GENERATORDEF),
            COLLECTARRAY => enbox!(sim_COLLECTARRAY),
            COLLECTDICT => enbox!(sim_COLLECTDICT),
            COLLECTSET => enbox!(sim_COLLECTSET),
            ARRAYEXPR => enbox!(sim_ARRAYEXPR),
            BREAK => enbox!(sim_BREAK),
            CONTINUE => enbox!(sim_CONTINUE),
            IF => enbox!(sim_IF),
            IFELSE => enbox!(sim_IFELSE),
            WHILE => enbox!(sim_WHILE),
            FOR => enbox!(sim_FOR),
            FOREACH => enbox!(sim_FOREACH),
            SWITCH => enbox!(sim_SWITCH),
            SWITCHCASE => enbox!(sim_SWITCHCASE),
            SWITCHDEFAULT => enbox!(sim_SWITCHDEFAULT),
            SWITCHEXIT => enbox!(sim_SWITCHEXIT),
            SCOPE => enbox!(sim_SCOPE),
            UNSCOPE => enbox!(sim_UNSCOPE),
            WITH => enbox!(sim_WITH),
            
            WHILETEST => enbox!(sim_WHILETEST),
            WHILELOOP => enbox!(sim_WHILELOOP),
            WITHLOOP => enbox!(sim_WITHLOOP),
            FOREACHLOOP => enbox!(sim_FOREACHLOOP),
            
            JUMPRELATIVE => enbox!(sim_JUMPRELATIVE),
            
            EXIT => enbox!(sim_EXIT),
            RETURN => enbox!(sim_RETURN),
            YIELD => enbox!(sim_YIELD),
            LINENUM => enbox!(sim_LINENUM),
            _ => None
        }
    }
    
    pub (crate) fn sim_NOP(&mut self) -> OpResult
    {
        Ok(())
    }
    pub (crate) fn sim_PUSHFLT(&mut self) -> OpResult
    {
        let value = unpack_f64(&self.pull_from_code(8)?)?;
        self.stack_push_val(Value::Number(value));
        Ok(())
    }
    pub (crate) fn sim_PUSHSHORT(&mut self) -> OpResult
    {
        let value = self.read_u16()?;
        self.stack_push_val(Value::Number(value as f64));
        Ok(())
    }
    pub (crate) fn sim_PUSHSTR(&mut self) -> OpResult
    {
        let text = self.read_string()?;
        self.stack_push_val(Value::Text(text));
        Ok(())
    }
    pub (crate) fn sim_PUSHNAME(&mut self) -> OpResult
    {
        let text = self.read_string()?;
        self.stack_push_var(Variable::Direct(DirectVar{name:text}));
        Ok(())
    }
    pub (crate) fn sim_PUSHVAR(&mut self) -> OpResult
    {
        let name = self.read_string()?;
        let val = self.evaluate_or_store(&Variable::Direct(DirectVar{name}), None)?.ok_or_else(|| minierr("error: tried to evaluate non-extant variable"))?;
        self.stack_push_val(val);
        Ok(())
    }
    
    pub (crate) fn sim_DECLVAR(&mut self) -> OpResult
    {
        if self.stack_len() < 1
        {
            return plainerr("internal error: DECLVAR instruction requires 1 values on the stack but only found 0");
        }
        let name = self.stack_pop_name().ok_or_else(|| minierr("internal error: tried to declare a variable with a name of invalid type"))?;
        let scope = self.top_frame.scopes.last_mut().ok_or_else(|| minierr("internal error: there are no scopes in the top frame"))?;
        
        if scope.contains_key(&name)
        {
            return Err(format!("error: redeclared identifier {}", name))
        }
        scope.insert(name, Value::Number(0.0));
        
        Ok(())
    }
    pub (crate) fn sim_DECLFAR(&mut self) -> OpResult
    {
        if self.stack_len() < 1
        {
            return plainerr("internal error: DECLFAR instruction requires 1 values on the stack but only found 0");
        }
        let name = self.stack_pop_name().ok_or_else(|| minierr("internal error: tried to declare instance variable with non-var-name type name"))?;
        let instance_id = self.top_frame.instancestack.last().ok_or_else(|| minierr("error: tried to declare instance variable when not executing within instance scope"))?;
        let instance = self.global.instances.get_mut(instance_id).ok_or_else(|| format!("error: tried to declare instance variable but instance of current scope ({}) no longer exists", instance_id))?;
        if instance.variables.contains_key(&name)
        {
            return Err(format!("error: redeclared identifier {}", name));
        }
        instance.variables.insert(name, Value::Number(0.0));
        Ok(())
    }
    pub (crate) fn sim_DECLGLOBALVAR(&mut self) -> OpResult
    {
        if self.stack_len() < 1
        {
            return plainerr("internal error: DECLGLOBALVAR instruction requires 1 values on the stack but only found 0");
        }
        let name = self.stack_pop_name().ok_or_else(|| minierr("internal error: tried to declare a global variable with a name of invalid type"))?;
        
        if self.global.variables.contains_key(&name)
        {
            return Err(format!("error: redeclared global variable identifier {}", name))
        }
        self.global.variables.insert(name, Value::Number(0.0));
        
        Ok(())
    }
    
    pub (crate) fn sim_INDIRECTION(&mut self) -> OpResult
    {
        if self.stack_len() < 2
        {
            return Err(format!("internal error: INDIRECTION instruction requires 2 values on the stack but only found {}", self.stack_len()));
        }
        let name = self.stack_pop_name().ok_or_else(|| minierr("internal error: tried to perform INDIRECTION operation with a right-hand side that wasn't a name"))?;
        
        let source = self.stack_pop().ok_or_else(|| minierr("internal error: failed to get source from stack in INDIRECTION operation"))?;
        match source
        {
            StackValue::Val(Value::Instance(ident)) =>
            {
                if !self.global.instances.contains_key(&ident)
                {
                    return Err(format!("error: tried to perform indirection on instance {} that doesn't exist", ident));
                }
                self.stack_push_var(IndirectVar::from_ident(ident, name));
            }
            StackValue::Val(Value::Special(Special::Global)) =>
                self.stack_push_var(IndirectVar::from_global(name)),
            StackValue::Val(Value::Dict(dict)) =>
                self.stack_push_var(Variable::Array(ArrayVar { location : NonArrayVariable::ActualDict(dict), indexes : vec!(Value::Text(name)) } )),
            
            StackValue::Var(Variable::Array(mut arrayvar)) =>
            {
                arrayvar.indexes.push(Value::Text(name));
                self.stack_push_var(Variable::Array(arrayvar));
            }
            StackValue::Var(var) =>
            {
                let value = self.evaluate_or_store(&var, None)?.ok_or_else(|| minierr("internal error: evaluate_or_store returned None when just accessing a variable"))?;
                match value
                {
                    Value::Instance(ident) =>
                    {
                        if !self.global.instances.contains_key(&ident)
                        {
                            return Err(format!("error: tried to perform indirection on instance {} that doesn't exist", ident));
                        }
                        self.stack_push_var(IndirectVar::from_ident(ident, name));
                    }
                    Value::Special(Special::Global) =>
                        self.stack_push_var(IndirectVar::from_global(name)),
                    _ =>
                    {
                        match var
                        {
                            Variable::Array(mut arrayvar) =>
                            {
                                arrayvar.indexes.push(Value::Text(name));
                                self.stack_push_var(Variable::Array(arrayvar));
                            }
                            Variable::Direct(dirvar) =>
                                self.stack_push_var(Variable::Array(ArrayVar { location : NonArrayVariable::Direct(dirvar), indexes : vec!(Value::Text(name)) } )),
                            Variable::Indirect(indirvar) =>
                                self.stack_push_var(Variable::Array(ArrayVar { location : NonArrayVariable::Indirect(indirvar), indexes : vec!(Value::Text(name)) } )),
                        }
                    }
                }
                
            }
            
            _ => return plainerr("error: tried to use indirection on a type that doesn't support it (only instances, dictionaries, and 'special' values are allowed)")
        }
        
        Ok(())
    }
    pub (crate) fn sim_DISMEMBER(&mut self) -> OpResult
    {
        if self.stack_len() < 2
        {
            return Err(format!("internal error: DISMEMBER instruction requires 2 values on the stack but only found {}", self.stack_len()));
        }
        let name = self.stack_pop_name().ok_or_else(|| minierr("internal error: tried to perform DISMEMBER operation with a right-hand side that wasn't a name"))?;
        // FIXME support indirection into fake member functions
        let source = self.stack_pop().ok_or_else(|| minierr("internal error: failed to get source from stack in DISMEMBER operation"))?;
        
        self.stack_push_val(Value::SubFunc(Box::new(SubFuncVal{source, name})));
        Ok(())
    }
    pub (crate) fn sim_EVALUATION(&mut self) -> OpResult
    {
        if self.stack_len() < 1
        {
            return Err(format!("internal error: EVALUATION instruction requires 1 values on the stack but only found {}", self.stack_len()));
        }
        let var = self.stack_pop_var().ok_or_else(|| minierr("internal error: failed to find a variable on the stack in EVALUATION"))?;
        let value = self.evaluate_or_store(&var, None)?.ok_or_else(|| minierr("internal error: evaluate_or_store returned None when just accessing a variable"))?;
        self.stack_push_val(value);
        Ok(())
    }
    pub (crate) fn sim_FUNCCALL(&mut self) -> OpResult
    {
        self.handle_func_call_or_expr(false)
    }
    pub (crate) fn sim_FUNCEXPR(&mut self) -> OpResult
    {
        self.handle_func_call_or_expr(true)
    }
    pub (crate) fn sim_INVOKE(&mut self) -> OpResult
    {
        self.handle_invocation()
    }
    pub (crate) fn sim_INVOKECALL(&mut self) -> OpResult
    {
        if self.stack_len() < 3
        {
            return Err(format!("internal error: INVOKECALL instruction requires 3 values on the stack but found {}", self.stack_len()));
        }
        let generator = self.stack_pop_val().ok_or_else(|| minierr("internal error: stack argument 1 to INVOKECALL must be a value"))?;
        let _yielded = self.stack_pop_val().ok_or_else(|| minierr("internal error: stack argument 2 to INVOKECALL must be a value"))?;
        let var = self.stack_pop_var().ok_or_else(|| minierr("internal error: stack argument 3 to INVOKECALL must be a variable"))?;
        
        self.evaluate_or_store(&var, Some(generator))?;
        
        Ok(())
    }
    pub (crate) fn sim_INVOKEEXPR(&mut self) -> OpResult
    {
        if self.stack_len() < 3
        {
            return Err(format!("internal error: INVOKECALL instruction requires 3 values on the stack but found {}", self.stack_len()));
        }
        let generator = self.stack_pop_val().ok_or_else(|| minierr("internal error: stack argument 1 to INVOKECALL must be a value"))?;
        let yielded = self.stack_pop_val().ok_or_else(|| minierr("internal error: stack argument 2 to INVOKECALL must be a value"))?;
        let var = self.stack_pop_var().ok_or_else(|| minierr("internal error: stack argument 3 to INVOKECALL must be a variable"))?;
        
        self.evaluate_or_store(&var, Some(generator))?;
        
        self.stack_push_val(yielded);
        
        Ok(())
    }
    
    pub (crate) fn sim_SCOPE(&mut self) -> OpResult
    {
        self.top_frame.scopes.push(HashMap::new());
        let here = self.get_pc();
        self.top_frame.scopestarts.push(here);
        if self.top_frame.scopes.len() >= 0x10000
        {
            return Err(format!("error: scope recursion limit of 0x10000 reached at line {}\n(note: use more functions!)", self.top_frame.currline));
        }
        Ok(())
    }
    pub (crate) fn sim_UNSCOPE(&mut self) -> OpResult
    {
        let immediate = self.read_u16()? as usize;
        
        self.drain_scopes((immediate+1) as u16);
        Ok(())
    }
    pub (crate) fn sim_BREAK(&mut self) -> OpResult
    {
        self.pop_controlstack_until_loop();
        
        let controller = self.top_frame.controlstack.last().ok_or_else(|| minierr("error: break instruction not inside of loop"))?;
        
        let (scopes, destination) =
        match controller
        {
            Controller::While(data) => (data.scopes, data.loop_end),
            _ => return plainerr("FIXME: unimplemented BREAK out from non-for/while loop")
        };
        
        self.drain_scopes(scopes);
        self.set_pc(destination);
        self.top_frame.controlstack.pop();
        
        Ok(())
    }
    pub (crate) fn sim_CONTINUE(&mut self) -> OpResult
    {
        self.pop_controlstack_until_loop();
        
        let controller = self.top_frame.controlstack.last().ok_or_else(|| minierr("error: continue instruction not inside of loop"))?;
        
        let (scopes, destination) =
        match controller
        {
            Controller::While(data) => (data.scopes, data.expr_start),
            _ => return plainerr("FIXME: unimplemented CONTINUE out from non-for/while loop")
        };
        
        self.drain_scopes(scopes);
        self.set_pc(destination);
        
        Ok(())
    }
    pub (crate) fn sim_IF(&mut self) -> OpResult
    {
        if self.stack_len() < 1
        {
            return plainerr("internal error: IF instruction requires 1 values on the stack but found 0");
        }
        let testval = self.stack_pop_val().ok_or_else(|| minierr("internal error: failed to find value on stack while handling IF controller"))?;
        let codelen = self.read_usize()?;
        if !value_truthy(&testval)
        {
            self.add_pc(codelen);
        }
        
        Ok(())
    }
    pub (crate) fn sim_IFELSE(&mut self) -> OpResult
    {
        if self.stack_len() < 1
        {
            return plainerr("internal error: IFELSE instruction requires 1 values on the stack but found 0");
        }
        let testval = self.stack_pop_val().ok_or_else(|| minierr("internal error: failed to find value on stack while handling IF controller"))?;
        let codelen1 = self.read_usize()?;
        if !value_truthy(&testval)
        {
            self.add_pc(codelen1);
        }
        Ok(())
    }
    pub (crate) fn sim_WHILE(&mut self) -> OpResult
    {
        let exprlen = self.read_usize()?;
        let codelen = self.read_usize()?;
        let current_pc = self.get_pc();
        self.top_frame.controlstack.push(Controller::While(WhileData{
            scopes : self.top_frame.scopes.len() as u16,
            expr_start : current_pc,
            loop_start : current_pc+exprlen,
            loop_end : current_pc+exprlen+codelen
        }));
        Ok(())
    }
    pub (crate) fn sim_FOR(&mut self) -> OpResult
    {
        let postlen = self.read_usize()?;
        let exprlen = self.read_usize()?;
        let codelen = self.read_usize()?;
        let current_pc = self.get_pc();
        self.top_frame.controlstack.push(Controller::While(WhileData{
            scopes : self.top_frame.scopes.len() as u16,
            expr_start : current_pc,
            loop_start : current_pc+postlen+exprlen,
            loop_end : current_pc+postlen+exprlen+codelen
        }));
        self.add_pc(postlen);
        Ok(())
    }
    pub (crate) fn sim_FOREACH(&mut self) -> OpResult
    {
        if self.stack_len() < 2
        {
            return Err(format!("internal error: FOREACH instruction requires 2 values on the stack but found {}", self.stack_len()));
        }
        
        let val = self.stack_pop_val().ok_or_else(|| minierr("internal error: foreach loop was fed a variable of some sort, instead of a value, for what to loop over"))?;
        let name = self.stack_pop_name().ok_or_else(|| minierr("internal error: foreach loop was fed a non-name stack value"))?;
        
        let mut list : VecDeque<Value> = match val
        {
            Value::Array(mut list) => list.drain(..).collect(),
            Value::Dict(mut dict) => dict.drain().map(|(k, v)| Value::Array(vec!(hashval_to_val(k), v))).collect(),
            Value::Set(mut set) => set.drain().map(hashval_to_val).collect(),
            // TODO: support foreach over generators
            _ => return plainerr("error: value fed to for-each loop must be an array or dictionary")
        };
        
        let codelen = self.read_usize()?;
        
        if let Some(first_val) = list.pop_front()
        {
            let current_pc = self.get_pc();
            self.top_frame.controlstack.push(Controller::ForEach(ForEachData{
                scopes : self.top_frame.scopes.len() as u16,
                loop_start : current_pc,
                loop_end : current_pc+codelen,
                name : name.clone(),
                values : list
            }));
            
            let scope = self.top_frame.scopes.last_mut().ok_or_else(|| minierr("internal error: there are no scopes in the top frame"))?;
            if scope.contains_key(&name)
            {
                return Err(format!("error: redeclared identifier {}", name))
            }
            scope.insert(name, first_val);
        }
        else
        {
            self.add_pc(codelen);
        }
        
        Ok(())
    }
    pub (crate) fn sim_WITH(&mut self) -> OpResult
    {
        if self.stack_len() < 1
        {
            return plainerr("internal error: WITH instruction requires 1 values on the stack but found 0");
        }
        // NOTE: for with(), the self.top_frame.scopes.len() >= 0xFFFF error case is handled by SCOPE instruction
        let other_id = self.stack_pop_val().ok_or_else(|| minierr("internal error: tried to use with() on a non-value expression"))?;
        
        let codelen = self.read_usize()?;
        
        let current_pc = self.get_pc();
        
        if let Value::Object(object_id) = other_id
        {
            let instance_id_list : Vec<usize> = self.global.instances_by_type.get(&object_id).ok_or_else(|| minierr("error: tried to use non-existant instance in with expression"))?.iter().cloned().collect();
            if let Some(first) = instance_id_list.first()
            {
                self.top_frame.instancestack.push(*first);
                self.top_frame.controlstack.push(Controller::With(WithData{
                    scopes : self.top_frame.scopes.len() as u16,
                    loop_start : current_pc,
                    loop_end : current_pc + codelen,
                    instances : instance_id_list.get(1..).ok_or_else(|| minierr("internal error: inaccessible error in sim_WITH"))?.iter().map(|id| Value::Number(*id as f64)).collect()
                }));
            }
            else
            {
                // silently skip block if there are no instances of this object type
                self.add_pc(codelen as usize);
            }
        }
        else 
        {
            let instance_id = match_or_err!(other_id, Value::Instance(x) => x, minierr("error: tried to use with() with a value that was not an object id or instance id"))?;
            if !self.global.instances.contains_key(&instance_id)
            {
                return plainerr("error: tried to use non-extant instance as argument of with()");
            }
            
            self.top_frame.instancestack.push(instance_id);
            
            self.top_frame.controlstack.push(Controller::With(WithData{
                scopes : self.top_frame.scopes.len() as u16,
                loop_start : current_pc,
                loop_end : current_pc + codelen,
                instances : VecDeque::new()
            }));
        }
        Ok(())
    }
    pub (crate) fn sim_SWITCH(&mut self) -> OpResult
    {
        if self.stack_len() < 1
        {
            return plainerr("internal error: SWITCH instruction requires 1 values on the stack but found 0");
        }
        let value = self.stack_pop_val().ok_or_else(|| minierr("internal error: switch expression was a variable instead of a value"))?;
        
        let num_cases = self.read_u16()?;
        let current_pc = self.get_pc();
        
        let mut case_block_addresses = vec!();
        for _ in 0..num_cases
        {
            case_block_addresses.push(current_pc + self.read_usize()?);
        }
        let exit = current_pc + self.read_usize()?;
        
        self.top_frame.controlstack.push(Controller::Switch(SwitchData{
            scopes : self.top_frame.scopes.len() as u16,
            blocks : case_block_addresses,
            exit,
            value
        }));
        
        Ok(())
    }
    pub (crate) fn sim_SWITCHCASE(&mut self) -> OpResult
    {
        if self.stack_len() < 1
        {
            return plainerr("internal error: SWITCHCASE instruction requires 1 values on the stack but found 0");
        }
        let value = self.stack_pop_val().ok_or_else(|| minierr("internal error: switch case expression was a variable instead of a value"))?;
        
        let which_case = self.read_u16()?;
        
        let switchdata : &SwitchData = match_or_err!(self.top_frame.controlstack.last(), Some(Controller::Switch(ref x)) => x, minierr("internal error: SWITCHCASE instruction outside of switch statement"))?;
        
        if ops::value_equal(&value, &switchdata.value)?
        {
            self.set_pc(*switchdata.blocks.get(which_case as usize).ok_or_else(|| minierr("internal error: which_case in SWITCHCASE was too large"))?);
        }
        
        Ok(())
    }
    pub (crate) fn sim_SWITCHDEFAULT(&mut self) -> OpResult
    {
        let which_case = self.read_u16()?;
        let switchdata : &SwitchData = match_or_err!(self.top_frame.controlstack.last(), Some(Controller::Switch(ref x)) => x, minierr("internal error: SWITCHDEFAULT instruction outside of switch statement"))?;
        self.set_pc(*switchdata.blocks.get(which_case as usize).ok_or_else(|| minierr("internal error: which_case in SWITCHDEFAULT was too large"))?);
        
        Ok(())
    }
    pub (crate) fn sim_SWITCHEXIT(&mut self) -> OpResult
    {
        let switchdata = match_or_err!(self.top_frame.controlstack.pop(), Some(Controller::Switch(x)) => x, minierr("internal error: SWITCHDEFAULT instruction outside of switch statement"))?;
        self.set_pc(switchdata.exit);
        
        Ok(())
    }
    pub (crate) fn sim_FUNCDEF(&mut self) -> OpResult
    {
        let (funcname, myfuncspec) = self.read_function(false, false)?;
        let scope = self.top_frame.scopes.last_mut().ok_or_else(|| minierr("internal error: there are no scopes in the top frame"))?;
        
        if scope.contains_key(&funcname)
        {
            return Err(format!("error: redeclared identifier {}", funcname));
        }
        scope.insert(funcname.clone(), Value::new_funcval(false, Some(funcname), None, Some(myfuncspec)));
        Ok(())
    }
    pub (crate) fn sim_GLOBALFUNCDEF(&mut self) -> OpResult
    {
        let (funcname, myfuncspec) = self.read_function(false, false)?;
        
        if self.global.functions.contains_key(&funcname)
        {
            return Err(format!("error: redeclared global function {}", funcname));
        }
        self.global.functions.insert(funcname.clone(), Value::new_funcval(false, Some(funcname), None, Some(myfuncspec)));
        Ok(())
    }
    pub (crate) fn sim_SUBFUNCDEF(&mut self) -> OpResult
    {
        let (funcname, myfuncspec) = self.read_function(true, false)?;
        let scope = self.top_frame.scopes.last_mut().ok_or_else(|| minierr("internal error: there are no scopes in the top frame"))?;
        
        if scope.contains_key(&funcname)
        {
            return Err(format!("error: redeclared identifier {}", funcname));
        }
        scope.insert(funcname.clone(), Value::new_funcval(false, Some(funcname), None, Some(myfuncspec)));
        Ok(())
    }
    pub (crate) fn sim_GENERATORDEF(&mut self) -> OpResult
    {
        let (funcname, myfuncspec) = self.read_function(false, true)?;
        let scope = self.top_frame.scopes.last_mut().ok_or_else(|| minierr("internal error: there are no scopes in the top frame"))?;
        
        if scope.contains_key(&funcname)
        {
            return Err(format!("error: redeclared identifier {}", funcname));
        }
        scope.insert(funcname.clone(), Value::new_funcval(false, Some(funcname), None, Some(myfuncspec)));
        Ok(())
    }
    
    pub (crate) fn sim_BINSTATE(&mut self) -> OpResult
    {
        if self.stack_len() < 2
        {
            return Err(format!("internal error: BINSTATE instruction requires 2 values on the stack but found {}", self.stack_len()));
        }
        
        let immediate = self.pull_single_from_code()?;
        
        let value = self.stack_pop_val().ok_or_else(|| minierr("internal error: not enough values on stack to run instruction BINSTATE (this error should be inaccessible)"))?;
        
        let var = self.stack_pop_var().ok_or_else(|| minierr("internal error: primary argument to BINSTATE could not be found or was not a variable"))?;
        
        if immediate == 0x00
        {
            self.evaluate_or_store(&var, Some(value))?;
        }
        else
        {
            let opfunc = get_binop_function(immediate).ok_or_else(|| format!("internal error: unknown binary operation 0x{:02X}", immediate))?;
            
            let var_initial_value = self.evaluate_or_store(&var, None)?.ok_or_else(|| minierr("internal error: evaluate_or_store returned None when just accessing value"))?;
            
            let var_new_value = opfunc(&var_initial_value, &value).or_else(|text| Err(format!("error: disallowed binary statement\n({})", text)))?;
            self.evaluate_or_store(&var, Some(var_new_value))?;
        }
        Ok(())
    }
    pub (crate) fn sim_UNSTATE(&mut self) -> OpResult
    {
        if self.stack_len() < 1
        {
            return Err(format!("internal error: UNSTATE instruction requires 2 values on the stack but found {}", self.stack_len()));
        }
        
        let immediate = self.pull_single_from_code()?;
        
        let var = self.stack_pop_var().ok_or_else(|| minierr("internal error: argument to UNSTATE could not be found or was not a variable"))?;
        
        let opfunc = get_unstate_function(immediate).ok_or_else(|| format!("internal error: unknown unary statement operation 0x{:02X}", immediate))?;
        let mut value = self.evaluate_or_store(&var, None)?.ok_or_else(|| minierr("internal error: evaluate_or_store returned None when just accessing value"))?;
        value = opfunc(&value).or_else(|text| Err(format!("error: disallowed unary statement operation\n({})", text)))?;
        
        self.evaluate_or_store(&var, Some(value))?;
        Ok(())
    }
    
    pub (crate) fn sim_BINOP(&mut self) -> OpResult
    {
        if self.stack_len() < 2
        {
            return Err(format!("internal error: BINOP instruction requires 2 values on the stack but found {}", self.stack_len()));
        }
        
        let immediate = self.pull_single_from_code()?;
        
        let right = self.stack_pop_val().ok_or_else(|| minierr("internal error: not enough values on stack to run instruction BINOP (this error should be inaccessible!)"))?;
        let left = self.stack_pop_val().ok_or_else(|| minierr("internal error: not enough values on stack to run instruction BINOP (this error should be inaccessible!)"))?;
        
        let opfunc = get_binop_function(immediate).ok_or_else(|| format!("internal error: unknown binary operation 0x{:02X}", immediate))?;
        
        let new_value = opfunc(&left, &right).or_else(|text|
        {
            let left_fmt = format_val(&left).ok_or_else(|| minierr("internal error: failed to format left value for printing error when creating error for invalid binary expression"))?;
            let right_fmt = format_val(&right).ok_or_else(|| minierr("internal error: failed to format right value for printing error when creating error for invalid binary expression"))?;
            Err(format!("error: disallowed binary expression\n({})\n(value 1: {})\n(value 2: {})", text, left_fmt, right_fmt))
        })?;
        self.stack_push_val(new_value);
        Ok(())
    }
    
    fn handle_short_circuit(&mut self, truthiness : bool) -> OpResult
    {
        if self.stack_len() < 1
        {
            return Err(format!("internal error: short circuit instruction requires 1 values on the stack but found {}", self.stack_len()))
        }
        let val = self.stack_pop_val().ok_or_else(|| minierr("internal error: left operand of binary logical operator was a value instead of a variable"))?;
        
        let rel = self.read_usize()?;
        
        let truthy = value_truthy(&val);
        
        if truthy as bool == truthiness
        {
            self.add_pc(rel as usize);
            self.stack_push_val(Value::Number(bool_floaty(truthy)));
        }
        else
        {
            self.stack_push_val(val);
        }
        Ok(())
    }
    pub (crate) fn sim_SHORTCIRCUITIFTRUE(&mut self) -> OpResult
    {
        self.handle_short_circuit(true)
    }
    
    pub (crate) fn sim_SHORTCIRCUITIFFALSE(&mut self) -> OpResult
    {
        self.handle_short_circuit(false)
    }
    
    pub (crate) fn sim_UNOP(&mut self) -> OpResult
    {
        if self.stack_len() < 1
        {
            return Err(format!("internal error: UNOP instruction requires 1 values on the stack but found {}", self.stack_len()))
        }
        
        let immediate = self.pull_single_from_code()?;
        
        let value = self.stack_pop_val().ok_or_else(|| minierr("internal error: not enough values on stack to run instruction UNOP (this error should be inaccessible!)"))?;
        let opfunc = get_unop_function(immediate).ok_or_else(|| format!("internal error: unknown unary operation 0x{:02X}", immediate))?;
        
        let new_value = opfunc(&value).or_else(|text| Err(format!("error: disallowed unary expression\n({})", text)))?;
        self.stack_push_val(new_value);
        Ok(())
    }
    pub (crate) fn sim_LAMBDA(&mut self) -> OpResult
    {
        let (captures, myfuncspec) = self.read_lambda()?;
        self.stack_push_val(Value::new_funcval(false, Some("lambda_self".to_string()), Some(captures), Some(myfuncspec)));
        Ok(())
    }
    pub (crate) fn sim_OBJDEF(&mut self) -> OpResult
    {
        let name = self.read_string()?;
        if self.global.objectnames.contains_key(&name)
        {
            return Err(format!("error: redeclared object {}", name));
        }
        
        let object_id = self.global.object_id;
        let numfuncs = self.read_u16()?;
        
        let mut funcs = HashMap::<String, FuncSpec>::new();
        for _ in 0..numfuncs
        {
            let (funcname, mut myfuncspec) = self.read_function(false, false)?;
            myfuncspec.fromobj = true;
            myfuncspec.parentobj = object_id;
            if funcs.contains_key(&funcname)
            {
                return Err(format!("error: redeclared function {} in object {}", funcname, name));
            }
            funcs.insert(funcname, myfuncspec);
        }
        
        self.global.objectnames.insert(name.clone(), object_id);
        self.global.objects.insert(object_id, ObjSpec { ident : object_id, name, functions : funcs });
        self.global.instances_by_type.insert(object_id, BTreeSet::new());
        
        self.global.object_id += 1;
        Ok(())
    }
    pub (crate) fn sim_COLLECTARRAY(&mut self) -> OpResult
    {
        let numvals = self.read_u16()? as usize;
        if self.stack_len() < numvals
        {
            return Err(format!("internal error: not enough values on stack for COLLECTARRAY instruction to build array (need {}, have {})", numvals, self.stack_len()));
        }
        let mut myarray = Vec::new();
        for _ in 0..numvals
        {
            let val = self.stack_pop_val().ok_or_else(|| minierr("internal error: COLLECTARRAY instruction failed to collect values from stack (this error should be unreachable!)"))?;
            myarray.insert(0, val);
        }
        self.stack_push_val(Value::Array(myarray));
        Ok(())
    }
    pub (crate) fn sim_COLLECTDICT(&mut self) -> OpResult
    {
        let numvals = self.read_u16()? as usize;
        if self.stack_len() < numvals*2
        {
            return Err(format!("internal error: not enough values on stack for COLLECTDICT instruction to build dict (need {}, have {})", numvals*2, self.stack_len()));
        }
        
        let mut mydict = HashMap::<HashableValue, Value>::new();
        for _ in 0..numvals
        {
            let val = self.stack_pop_val().ok_or_else(|| minierr("internal error: COLLECTDICT instruction failed to collect values from stack"))?;
            let key = self.stack_pop_val().ok_or_else(|| minierr("internal error: COLLECTDICT instruction failed to collect values from stack"))?;
            let hashval = val_to_hashval(key)?;
            // stack popping goes in reverse order (most-recently added items first) so we just don't insert items that are already there
            if !mydict.contains_key(&hashval)
            {
                mydict.insert(hashval, val);
            }
        }
        self.stack_push_val(Value::Dict(mydict));
        Ok(())
    }
    pub (crate) fn sim_COLLECTSET(&mut self) -> OpResult
    {
        let numvals = self.read_u16()? as usize;
        if self.stack_len() < numvals
        {
            return Err(format!("internal error: not enough values on stack for COLLECTSET instruction to build dict (need {}, have {})", numvals, self.stack_len()));
        }
        
        let mut myset = HashSet::<HashableValue>::new();
        for _ in 0..numvals
        {
            let val = self.stack_pop_val().ok_or_else(|| minierr("internal error: COLLECTSET instruction failed to collect values from stack"))?;
            myset.insert(val_to_hashval(val)?);
        }
        self.stack_push_val(Value::Set(myset));
        Ok(())
    }
    pub (crate) fn sim_ARRAYEXPR(&mut self) -> OpResult
    {
        if self.stack_len() < 2
        {
            return Err(format!("internal error: ARRAYEXPR instruction requires 2 values on the stack but found {}", self.stack_len()));
        }
        let index = self.stack_pop_val().ok_or_else(|| minierr("internal error: TODO write error askdgfauiowef"))?;
        let array = self.stack_pop().ok_or_else(|| minierr("internal error: TODO write error cvbhsrtgaerffd"))?;
        match array
        {
            StackValue::Var(Variable::Array(mut arrayvar)) =>
            {
                arrayvar.indexes.push(index);
                self.stack_push_var(Variable::Array(arrayvar));
            }
            StackValue::Var(Variable::Direct(dirvar)) =>
                self.stack_push_var(Variable::Array(ArrayVar { location : NonArrayVariable::Direct(dirvar), indexes : vec!(index) } )),
            StackValue::Var(Variable::Indirect(indirvar)) =>
                self.stack_push_var(Variable::Array(ArrayVar { location : NonArrayVariable::Indirect(indirvar), indexes : vec!(index) } )),
            StackValue::Val(Value::Array(array)) =>
                self.stack_push_var(Variable::Array(ArrayVar { location : NonArrayVariable::ActualArray(array), indexes : vec!(index) } )),
            StackValue::Val(Value::Dict(dict)) =>
                self.stack_push_var(Variable::Array(ArrayVar { location : NonArrayVariable::ActualDict(dict), indexes : vec!(index) } )),
            StackValue::Val(Value::Text(string)) =>
                self.stack_push_var(Variable::Array(ArrayVar { location : NonArrayVariable::ActualText(string), indexes : vec!(index) } )),
            _ =>
                return plainerr("error: tried to use array indexing on a non-indexable value"),
        }
        Ok(())
    }
    pub (crate) fn sim_WHILETEST(&mut self) -> OpResult
    {
        if let Some(Controller::While(ref data)) = self.top_frame.controlstack.last()
        {
            let dest = data.loop_end;
            let todrain = data.scopes;
            let testval = self.stack_pop_val().ok_or_else(|| minierr("internal error: failed to find value on stack while handling WHILE controller"))?;
            if !value_truthy(&testval)
            {
                self.set_pc(dest);
                self.drain_scopes(todrain);
                self.top_frame.controlstack.pop();
            }
            return Ok(());
        }
        plainerr("internal error: WHILELOOP instruction when immediate controller is not a while controller")
    }
    pub (crate) fn sim_WHILELOOP(&mut self) -> OpResult
    {
        if let Some(Controller::While(ref data)) = self.top_frame.controlstack.last()
        {
            let dest = data.expr_start;
            let todrain = data.scopes;
            self.set_pc(dest);
            self.drain_scopes(todrain);
            return Ok(());
        }
        plainerr("internal error: WHILELOOP instruction when immediate controller is not a while controller")
    }
    pub (crate) fn sim_WITHLOOP(&mut self) -> OpResult
    {
        self.top_frame.instancestack.pop();
        
        if let Some(Controller::With(ref mut data)) = self.top_frame.controlstack.last_mut()
        {
            if let Some(next_instance) = data.instances.remove(0)
            {
                if let Value::Number(next_instance) = next_instance
                {
                    self.top_frame.instancestack.push(next_instance as usize);
                    let dest = data.loop_start;
                    self.set_pc(dest);
                }
                else
                {
                    return plainerr("internal error: values fed to with controller's 'other' data must be a list of only numbers");
                }
            }
            else
            {
                self.top_frame.controlstack.pop();
            }
            return Ok(());
        }
        plainerr("internal error: WITHLOOP instruction when immediate controller is not a with controller")
    }
    pub (crate) fn sim_FOREACHLOOP(&mut self) -> OpResult
    {
        if let Some(Controller::ForEach(ref mut data)) = self.top_frame.controlstack.last_mut()
        {
            if let Some(value) = data.values.remove(0)
            {
                let todrain = data.scopes;
                let name = data.name.clone();
                let dest = data.loop_start;
                
                self.drain_scopes(todrain);
                
                let scope = self.top_frame.scopes.last_mut().ok_or_else(|| minierr("internal error: there are no scopes in the top frame"))?;
                scope.insert(name, value);
                
                self.set_pc(dest);
            }
            else
            {
                self.top_frame.controlstack.pop();
            }
            return Ok(());
        }
        plainerr("internal error: FOREACHLOOP instruction when immediate controller is not a foreach controller")
    }
    pub (crate) fn sim_JUMPRELATIVE(&mut self) -> OpResult
    {
        let rel = self.read_usize()?;
        self.add_pc(rel);
        Ok(())
    }
    pub (crate) fn sim_EXIT(&mut self) -> OpResult // an exit is a return with no value
    {
        if let Some(outer_top_frame) = self.frames.pop()
        {
            let was_generator = self.top_frame.generator;
            let frame_was_expr = self.top_frame.isexpr;
            self.top_frame = outer_top_frame;
            // exit implies no remaining value on the stack. if the outside expects a value, push it
            if frame_was_expr
            {
                self.stack_push_val(Value::Number(0.0));
            }
            if was_generator
            {
                self.stack_push_val(Value::Generator(GeneratorState{frame : None}));
                if !frame_was_expr
                {
                    return plainerr("internal error: generators must always return into an expression");
                }
            }
        }
        else
        {
            self.doexit = true;
        }
        Ok(())
    }
    pub (crate) fn sim_RETURN(&mut self) -> OpResult
    {
        let was_generator = self.top_frame.generator;
        let old_frame = self.frames.pop().ok_or_else(|| minierr("error: attempted to return from global code; use exit() instead"))?;
        
        let inner_frame_stack_last = self.stack_pop();
        let frame_was_expr = self.top_frame.isexpr;
        self.top_frame = old_frame;
        
        if frame_was_expr
        {
            let val = inner_frame_stack_last.ok_or_else(|| minierr("error: RETURN instruction needed a value remaining on the inner frame's stack, but there were none"))?;
            self.stack_push(val);
        }
        if was_generator
        {
            self.stack_push_val(Value::Generator(GeneratorState{frame : None}));
            if !frame_was_expr
            {
                return plainerr("internal error: generators must always return into an expression");
            }
        }
        Ok(())
    }
    pub (crate) fn sim_YIELD(&mut self) -> OpResult
    {
        if !self.top_frame.generator
        {
            return plainerr("error: tried to yield in non-generator function; use return instead");
        }
        
        let frame_was_expr = self.top_frame.isexpr;
        let mut old_frame = self.frames.pop().ok_or_else(|| minierr("error: attempted to return from global code; use exit() instead"))?;
        
        let inner_frame_stack_last = self.stack_pop();
        std::mem::swap(&mut self.top_frame, &mut old_frame);
        let new_gen_state = GeneratorState{frame : Some(old_frame)};
        
        if frame_was_expr
        {
            let val = inner_frame_stack_last.ok_or_else(|| minierr("error: YIELD instruction needed a value remaining on the inner frame's stack, but there were none"))?;
            self.stack_push(val);
            self.stack_push_val(Value::Generator(new_gen_state));
        }
        else
        {
            return plainerr("internal error: generators must always return into an expression");
        }
        Ok(())
    }
    pub (crate) fn sim_LINENUM(&mut self) -> OpResult
    {
        self.top_frame.currline = self.read_usize()? as usize;
        Ok(())
    }
}