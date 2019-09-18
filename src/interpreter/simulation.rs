#![allow(clippy::cast_lossless, clippy::map_entry, non_snake_case)]

use crate::interpreter::*;

#[inline]
fn stack_access_err<S : ToString>(text : S) -> String
{
    if cfg!(stack_access_debugging)
    {
        text.to_string()
    }
    else
    {
        panic!(text.to_string())
    }
}
#[inline]
fn strange_err_plain<T, S : ToString>(text : S) -> Result<T, String>
{
    if cfg!(broken_compiler_debugging)
    {
        Err(text.to_string())
    }
    else
    {
        panic!(text.to_string())
    }
}
#[inline]
fn strange_err<S : ToString>(text : S) -> String
{
    if cfg!(broken_compiler_debugging)
    {
        text.to_string()
    }
    else
    {
        panic!(text.to_string())
    }
}

static mut OPTABLE : [OpFunc; 256] = [Interpreter::sim_INVALID as OpFunc; 256];

impl Interpreter
{
    pub (crate) fn build_opfunc_table()
    {
        macro_rules! set { ( $x:ident, $y:ident ) => { unsafe { OPTABLE[$x as usize] = Interpreter::$y; } } }
        
        set!(NOP, sim_NOP);
        set!(PUSHFLT, sim_PUSHFLT);
        set!(PUSHSTR, sim_PUSHSTR);
        set!(PUSHVAR, sim_PUSHVAR);
        set!(EVALUATEVAR, sim_EVALUATEVAR);
        set!(PUSHBAREGLOBAL, sim_PUSHBAREGLOBAL);
        set!(EVALUATEBAREGLOBAL, sim_EVALUATEBAREGLOBAL);
        set!(PUSHINSTVAR, sim_PUSHINSTVAR);
        set!(EVALUATEINSTVAR, sim_EVALUATEINSTVAR);
        set!(PUSHOBJ, sim_PUSHOBJ);
        set!(PUSHBIND, sim_PUSHBIND);
        set!(PUSHGLOBAL, sim_PUSHGLOBAL);
        set!(PUSHGLOBALFUNC, sim_PUSHGLOBALFUNC);
        set!(PUSHGLOBALVAL, sim_PUSHGLOBALVAL);
        set!(PUSHSELF, sim_PUSHSELF);
        set!(PUSHOTHER, sim_PUSHOTHER);
        set!(NEWVAR, sim_NEWVAR);
        set!(BINSTATE, sim_BINSTATE);
        set!(UNSTATE, sim_UNSTATE);
        set!(SETBAREGLOBAL, sim_SETBAREGLOBAL);
        set!(BINOP, sim_BINOP);
        set!(UNOP, sim_UNOP);
        set!(SHORTCIRCUITIFTRUE, sim_SHORTCIRCUITIFTRUE);
        set!(SHORTCIRCUITIFFALSE, sim_SHORTCIRCUITIFFALSE);
        set!(INDIRECTION, sim_INDIRECTION);
        set!(EVALUATEINDIRECTION, sim_EVALUATEINDIRECTION);
        set!(DISMEMBER, sim_DISMEMBER);
        set!(FUNCCALL, sim_FUNCCALL);
        set!(FUNCEXPR, sim_FUNCEXPR);
        set!(INVOKE, sim_INVOKE);
        set!(INVOKECALL, sim_INVOKECALL);
        set!(INVOKEEXPR, sim_INVOKEEXPR);
        set!(FUNCDEF, sim_FUNCDEF);
        set!(LAMBDA, sim_LAMBDA);
        set!(GENERATORDEF, sim_GENERATORDEF);
        set!(COLLECTARRAY, sim_COLLECTARRAY);
        set!(COLLECTDICT, sim_COLLECTDICT);
        set!(COLLECTSET, sim_COLLECTSET);
        set!(ARRAYEXPR, sim_ARRAYEXPR);
        set!(EVALUATEARRAYEXPR, sim_EVALUATEARRAYEXPR);
        set!(BREAK, sim_BREAK);
        set!(CONTINUE, sim_CONTINUE);
        set!(IF, sim_IF);
        set!(WHILE, sim_WHILE);
        set!(FOR, sim_FOR);
        set!(FOREACH, sim_FOREACH);
        set!(SWITCH, sim_SWITCH);
        set!(SWITCHCASE, sim_SWITCHCASE);
        set!(SWITCHDEFAULT, sim_SWITCHDEFAULT);
        set!(SWITCHEXIT, sim_SWITCHEXIT);
        set!(UNSCOPE, sim_UNSCOPE);
        set!(WITH, sim_WITH);
        set!(WITHAS, sim_WITHAS);
            
        set!(WHILETEST, sim_WHILETEST);
        set!(WHILELOOP, sim_WHILELOOP);
        set!(WITHLOOP, sim_WITHLOOP);
        set!(FOREACHLOOP, sim_FOREACHLOOP);
        set!(FOREACHHEAD, sim_FOREACHHEAD);
            
        set!(JUMPRELATIVE, sim_JUMPRELATIVE);
            
        set!(EXIT, sim_EXIT);
        set!(RETURN, sim_RETURN);
        set!(YIELD, sim_YIELD);
    }
    
    #[inline]
    pub (crate) fn run_next_op(&mut self) -> Result<u8, String>
    {
        let op = self.pull_single_from_code().or_else(|_|
        {
            if cfg!(code_bounds_debugging)
            {
                Err(minierr("internal error: simulation stepped while outside of the range of the frame it was in"))
            }
            else
            {
                panic!("internal error: simulation stepped while outside of the range of the frame it was in");
            }
        })?;
        unsafe { OPTABLE[op as usize](self) }?;
        Ok(op)
    }
    
    pub (crate) fn sim_INVALID(&mut self) -> OpResult
    {
        self.sub_pc(1);
        #[cfg(feature = "compiler_invalid_execution_debugging")]
        {
            return Err(format!("internal error: no such operation 0x{:02X}", self.pull_single_from_code()?));
        }
        panic!(format!("internal error: no such operation 0x{:02X}", self.pull_single_from_code()?))
    }
    
    pub (crate) fn sim_NOP(&mut self) -> OpResult
    {
        //plainerr("NOP")
        Ok(())
    }
    pub (crate) fn sim_PUSHFLT(&mut self) -> OpResult
    {
        let value = self.read_float()?;
        self.stack_push_val(Value::Number(value));
        Ok(())
    }
    pub (crate) fn sim_PUSHSTR(&mut self) -> OpResult
    {
        let text = self.read_string()?;
        self.stack_push_val(Value::Text(text));
        Ok(())
    }
    pub (crate) fn sim_PUSHVAR(&mut self) -> OpResult
    {
        let index = self.read_usize()?;
        self.stack_push_var(Variable::Direct(index));
        Ok(())
    }
    pub (crate) fn sim_EVALUATEVAR(&mut self) -> OpResult
    {
        let index = self.read_usize()?;
        let val = self.top_frame.variables.get(index).ok_or_else(|| strange_err("internal error: variable stack out-of-bounds access"))?.clone();
        self.stack_push_val(val);
        Ok(())
    }
    pub (crate) fn sim_PUSHINSTVAR(&mut self) -> OpResult
    {
        let instance_id = *self.top_frame.instancestack.last().ok_or_else(|| strange_err("internal error: tried to access instance variable when not executing within instance scope"))?;
        let index = self.read_usize()?;
        self.stack_push_var(Variable::from_indirection(instance_id, index));
        Ok(())
    }
    pub (crate) fn sim_EVALUATEINSTVAR(&mut self) -> OpResult
    {
        let instance_id = *self.top_frame.instancestack.last().ok_or_else(|| strange_err("internal error: tried to access instance variable when not executing within instance scope"))?;
        let index = self.read_usize()?;
        
        self.stack_push_val(self.evaluate_of_indirect_simple(instance_id, index)?);
        Ok(())
    }
    pub (crate) fn sim_PUSHBIND(&mut self) -> OpResult
    {
        let nameindex = self.read_usize()?;
        self.stack_push_val(Value::InternalFunc(InternalFuncVal{nameindex}));
        Ok(())
    }
    pub (crate) fn sim_PUSHOBJ(&mut self) -> OpResult
    {
        let nameindex = self.read_usize()?;
        self.stack_push_val(Value::Object(nameindex));
        Ok(())
    }
    pub (crate) fn sim_PUSHGLOBAL(&mut self) -> OpResult
    {
        let index = self.read_usize()?;
        self.stack_push_var(Variable::Global(index));
        Ok(())
    }
    pub (crate) fn sim_PUSHGLOBALVAL(&mut self) -> OpResult
    {
        let index = self.read_usize()?;
        let val = self.global.variables.get(&index).ok_or_else(|| format!("error: tried to access global variable `{}` that doesn't exist", self.get_indexed_string(index)))?.clone();
        self.stack_push_val(val);
        Ok(())
    }
    pub (crate) fn sim_PUSHGLOBALFUNC(&mut self) -> OpResult
    {
        let index = self.read_usize()?;
        let val = self.global.functions.get(&index).ok_or_else(|| format!("error: tried to access global function `{}` that doesn't exist", self.get_indexed_string(index)))?.clone();
        self.stack_push_val(val);
        Ok(())
    }
    pub (crate) fn sim_PUSHBAREGLOBAL(&mut self) -> OpResult
    {
        let index = self.read_usize()?;
        self.stack_push_var(Variable::BareGlobal(index));
        Ok(())
    }
    pub (crate) fn sim_EVALUATEBAREGLOBAL(&mut self) -> OpResult
    {
        let index = self.read_usize()?;
        let val = self.global.barevariables.get(&index).ok_or_else(|| format!("internal error: tried to access bare global variable `{}` that doesn't exist", self.get_indexed_string(index)))?.clone();
        self.stack_push_val(val);
        Ok(())
    }
    
    pub (crate) fn sim_PUSHSELF(&mut self) -> OpResult
    {
        let instance_id = *self.top_frame.instancestack.last().ok_or_else(|| strange_err("internal error: tried to access `self` when not executing within instance scope"))?;
        self.stack_push_val(Value::Instance(instance_id));
        Ok(())
    }
    pub (crate) fn sim_PUSHOTHER(&mut self) -> OpResult
    {
        let loc = self.top_frame.instancestack.len()-2;
        let instance_id = *self.top_frame.instancestack.get_mut(loc).ok_or_else(|| "error: tried to access `other` while not inside of at least two instance scopes".to_string())?;
        self.stack_push_val(Value::Instance(instance_id));
        Ok(())
    }
    
    pub (crate) fn sim_NEWVAR(&mut self) -> OpResult
    {
        self.top_frame.variables.push(Value::Number(0.0));
        
        Ok(())
    }
    
    pub (crate) fn sim_INDIRECTION(&mut self) -> OpResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return Err(format!("internal error: INDIRECTION instruction requires 1 values on the stack but only found {}", self.stack_len()));
            }
        }
        let name = self.read_usize()?;
        let source = self.stack_pop().ok_or_else(|| stack_access_err("internal error: failed to get source from stack in INDIRECTION operation"))?;
        
        //eprintln!("performing indirection on {:?}", source);
        match source
        {
            StackValue::Val(Value::Instance(ident)) =>
                self.stack_push_var(Variable::from_indirection(ident, name)),
            // FIXME eliminate this
            StackValue::Var(var) =>
            {
                match self.evaluate(var)?.as_ref()
                {
                    Value::Instance(id) =>
                    {
                        let id = *id;
                        self.stack_push_var(Variable::from_indirection(id, name))
                    }
                    _ => Err("error: tried to use indirection on a non-instance or non-global value".to_string())?
                }
            }
            _ => return plainerr("error: tried to use indirection on a type that doesn't support it (only instances, dictionaries, and 'special' values are allowed)")
        }
        
        Ok(())
    }
    pub (crate) fn sim_EVALUATEINDIRECTION(&mut self) -> OpResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return Err(format!("internal error: EVALUATEINDIRECTION instruction requires 1 values on the stack but only found {}", self.stack_len()));
            }
        }
        let name = self.read_usize()?;
        let source = self.stack_pop().ok_or_else(|| stack_access_err("internal error: failed to get source from stack in EVALUATEINDIRECTION operation"))?;
        
        //eprintln!("performing indirection on {:?}", source);
        match source
        {
            StackValue::Val(Value::Instance(ident)) =>
                self.stack_push_val(self.evaluate_of_indirect_simple(ident, name)?),
            // FIXME eliminate this
            StackValue::Var(var) =>
            {
                match self.evaluate(var)?.as_ref()
                {
                    Value::Instance(id) =>
                    {
                        let id = *id;
                        self.stack_push_val(self.evaluate_of_indirect_simple(id, name)?)
                    }
                    q => Err(format!("error: tried to use eval indirection on a non-instance or non-global value ({:?})", q))?
                }
            }
            _ => return plainerr("error: tried to use eval indirection on a type that doesn't support it (only instances, dictionaries, and 'special' values are allowed)")
        }
        
        Ok(())
    }
    pub (crate) fn sim_DISMEMBER(&mut self) -> OpResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return Err(format!("internal error: DISMEMBER instruction requires 1 values on the stack but only found {}", self.stack_len()));
            }
        }
        let name = self.read_usize()?;
        let source = self.stack_pop().ok_or_else(|| stack_access_err("internal error: failed to get source from stack in DISMEMBER operation"))?;
        
        self.stack_push_val(Value::SubFunc(Box::new(SubFuncVal{source, name})));
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
        let var = self.stack_pop_var().ok_or_else(|| stack_access_err("internal error: not enough variables on stack to run instruction INVOKE"))?;
        let val = self.evaluate_value(var.clone())?;
        
        if let Value::Generator(generator_state) = val
        {
            let frame = generator_state.frame.ok_or_else(|| minierr("error: tried to invoke a dead generator"))?;
            self.stack_push_var(var);
            self.push_new_frame(frame)?;
        }
        else
        {
            return Err(format!("error: tried to invoke a non-generator ({:?})", val));
        }
        
        Ok(())
    }
    pub (crate) fn sim_INVOKECALL(&mut self) -> OpResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 3
            {
                return Err(format!("internal error: INVOKECALL instruction requires 3 values on the stack but found {}", self.stack_len()));
            }
        }
        let generator = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: stack argument 1 to INVOKECALL must be a value"))?;
        let _yielded = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: stack argument 2 to INVOKECALL must be a value"))?;
        let var = self.stack_pop_var().ok_or_else(|| stack_access_err("internal error: stack argument 3 to INVOKECALL must be a variable"))?;
        
        self.evaluate(var)?.assign(generator)?;
        
        Ok(())
    }
    pub (crate) fn sim_INVOKEEXPR(&mut self) -> OpResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 3
            {
                return Err(format!("internal error: INVOKEEXPR instruction requires 3 values on the stack but found {}", self.stack_len()));
            }
        }
        let generator = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: stack argument 1 to INVOKEEXPR must be a value"))?;
        let yielded = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: stack argument 2 to INVOKEEXPR must be a value"))?;
        let var = self.stack_pop_var().ok_or_else(|| stack_access_err("internal error: stack argument 3 to INVOKEEXPR must be a variable"))?;
        
        self.evaluate(var)?.assign(generator)?;
        
        self.stack_push_val(yielded);
        
        Ok(())
    }
    
    pub (crate) fn sim_UNSCOPE(&mut self) -> OpResult
    {
        let immediate = self.read_usize()?;
        
        self.drain_vars(immediate as u64);
        Ok(())
    }
    pub (crate) fn sim_BREAK(&mut self) -> OpResult
    {
        self.pop_controlstack_until_loop();
        
        let controller = self.top_frame.controlstack.last().ok_or_else(|| minierr("error: break instruction not inside of loop"))?;
        
        let (variables, destination) =
        match controller
        {
            Controller::While(data) => (data.variables, data.loop_end),
            _ => return plainerr("FIXME: unimplemented BREAK out from non-for/while loop")
        };
        
        self.drain_vars(variables);
        self.set_pc(destination);
        self.top_frame.controlstack.pop();
        
        Ok(())
    }
    pub (crate) fn sim_CONTINUE(&mut self) -> OpResult
    {
        self.pop_controlstack_until_loop();
        
        let controller = self.top_frame.controlstack.last().ok_or_else(|| minierr("error: continue instruction not inside of loop"))?;
        
        let (variables, destination) =
        match controller
        {
            Controller::While(data) => (data.variables, data.expr_start),
            _ => return plainerr("FIXME: unimplemented CONTINUE out from non-for/while loop")
        };
        
        self.drain_vars(variables);
        self.set_pc(destination);
        
        Ok(())
    }
    pub (crate) fn sim_IF(&mut self) -> OpResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return plainerr("internal error: IF instruction requires 1 values on the stack but found 0");
            }
        }
        let testval = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: failed to find value on stack while handling IF controller"))?;
        let codelen = self.read_usize()?;
        if !value_truthy(&testval)
        {
            self.add_pc(codelen);
        }
        
        Ok(())
    }
    pub (crate) fn sim_WHILE(&mut self) -> OpResult
    {
        let exprlen = self.read_usize()?;
        let codelen = self.read_usize()?;
        let current_pc = self.get_pc();
        self.top_frame.controlstack.push(Controller::While(WhileData{
            variables : self.top_frame.variables.len() as u64,
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
            variables : self.top_frame.variables.len() as u64,
            expr_start : current_pc,
            loop_start : current_pc+postlen+exprlen,
            loop_end : current_pc+postlen+exprlen+codelen
        }));
        self.add_pc(postlen);
        Ok(())
    }
    pub (crate) fn sim_FOREACH(&mut self) -> OpResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return Err(format!("internal error: FOREACH instruction requires 1 values on the stack but found {}", self.stack_len()));
            }
        }
        
        let mut val = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: foreach loop was fed a variable of some sort, instead of a value, for what to loop over"))?;
        
        let list : ForEachValues = match val
        {
            Value::Array(ref mut list) => ForEachValues::List(list.drain(..).rev().collect()),
            Value::Dict(ref mut dict)  => ForEachValues::List(dict.drain().map(|(k, v)| Value::Array(Box::new(vec!(hashval_to_val(k), v)))).collect()),
            Value::Set(ref mut set)    => ForEachValues::List(set.drain().map(hashval_to_val).collect()),
            Value::Generator(_) => ForEachValues::Gen(GeneratorState{frame : None}),
            _ => return plainerr("error: value fed to for-each loop must be an array, dictionary, set, or generatorstate")
        };
        
        let codelen = self.read_usize()?;
        let current_pc = self.get_pc();
        self.top_frame.controlstack.push(Controller::ForEach(ForEachData{
            variables : self.top_frame.variables.len() as u64,
            loop_start : current_pc,
            loop_end : current_pc+codelen,
            values : list
        }));
        
        if let Value::Generator(genstate) = val
        {
            let frame = genstate.frame.ok_or_else(|| minierr("error: tried to invoke a dead generator in a foreach loop"))?;
            self.push_new_frame(frame)?;
        }
        
        Ok(())
    }
    pub (crate) fn sim_WITH(&mut self) -> OpResult
    {
        let object_id = self.read_usize()?;
        let codelen = self.read_usize()?;
        let current_pc = self.get_pc();
        
        let instance_id_list : Vec<usize> = self.global.instances_by_type.get(&object_id).ok_or_else(|| minierr("error: tried to use non-existant object type in with expression"))?.iter().cloned().collect();
        if let Some(first) = instance_id_list.first()
        {
            self.top_frame.instancestack.push(*first);
            self.top_frame.controlstack.push(Controller::With(WithData{
                variables : self.top_frame.variables.len() as u64,
                loop_start : current_pc,
                loop_end : current_pc + codelen,
                instances : instance_id_list.get(1..).unwrap().iter().rev().map(|id| Value::Number(*id as f64)).collect()
            }));
        }
        else
        {
            // silently skip block if there are no instances of this object type
            self.add_pc(codelen as usize);
        }
        
        Ok(())
    }
    pub (crate) fn sim_WITHAS(&mut self) -> OpResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return plainerr("internal error: WITHAS instruction requires 1 values on the stack but found 0");
            }
        }
        let other_id = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: withas expression was a variable instead of a value"))?;
        let instance_id = match_or_err!(other_id, Value::Instance(x) => x, minierr("error: tried to use with() with a value that was not an object id or instance id"))?;
        let codelen = self.read_usize()?;
        let current_pc = self.get_pc();
        
        if !self.global.instances.contains_key(&instance_id)
        {
            return plainerr("error: tried to use non-extant instance as argument of with()");
        }
        
        self.top_frame.instancestack.push(instance_id);
        
        self.top_frame.controlstack.push(Controller::With(WithData{
            variables : self.top_frame.variables.len() as u64,
            loop_start : current_pc,
            loop_end : current_pc + codelen,
            instances : Vec::new()
        }));
        
        Ok(())
    }
    pub (crate) fn sim_SWITCH(&mut self) -> OpResult
    {
        //eprintln!("hit sim_switch");
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return plainerr("internal error: SWITCH instruction requires 1 values on the stack but found 0");
            }
        }
        let value = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: switch expression was a variable instead of a value"))?;
        
        let num_cases = self.read_u16()?;
        let current_pc = self.get_pc();
        
        let mut case_block_addresses = vec!();
        for _ in 0..num_cases
        {
            case_block_addresses.push(current_pc + self.read_usize()?);
        }
        let exit = current_pc + self.read_usize()?;
        
        self.top_frame.controlstack.push(Controller::Switch(SwitchData{
            variables : self.top_frame.variables.len() as u64,
            blocks : case_block_addresses,
            exit,
            value
        }));
        
        //eprintln!("end of sim_switch");
        //eprintln!("{:?}", self.top_frame.controlstack.last().unwrap());
        
        Ok(())
    }
    pub (crate) fn sim_SWITCHCASE(&mut self) -> OpResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return plainerr("internal error: SWITCHCASE instruction requires 1 values on the stack but found 0");
            }
        }
        let value = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: switch case expression was a variable instead of a value"))?;
        
        let which_case = self.read_u16()?;
        
        let switchdata : &SwitchData = match_or_err!(self.top_frame.controlstack.last(), Some(Controller::Switch(ref x)) => x, strange_err("internal error: SWITCHCASE instruction outside of switch statement"))?;
        let dest = *switchdata.blocks.get(which_case as usize).ok_or_else(|| strange_err("internal error: which_case in SWITCHCASE was too large"))?;
        
        if ops::value_equal(&value, &switchdata.value)?
        {
            //eprintln!("jumping to {} thanks to switch", dest);
            self.set_pc(dest);
        }
        
        Ok(())
    }
    pub (crate) fn sim_SWITCHDEFAULT(&mut self) -> OpResult
    {
        let which_case = self.read_u16()?;
        let switchdata : &SwitchData = match_or_err!(self.top_frame.controlstack.last(), Some(Controller::Switch(ref x)) => x, strange_err("internal error: SWITCHDEFAULT instruction outside of switch statement"))?;
        let dest = *switchdata.blocks.get(which_case as usize).ok_or_else(|| strange_err("internal error: which_case in SWITCHDEFAULT was too large"))?;
        self.set_pc(dest);
        
        Ok(())
    }
    pub (crate) fn sim_SWITCHEXIT(&mut self) -> OpResult
    {
        let switchdata = match_or_err!(self.top_frame.controlstack.pop(), Some(Controller::Switch(x)) => x, strange_err("internal error: SWITCHDEFAULT instruction outside of switch statement"))?;
        self.set_pc(switchdata.exit);
        
        Ok(())
    }
    pub (crate) fn sim_FUNCDEF(&mut self) -> OpResult
    {
        let myfuncspec = self.read_function(false)?;
        self.top_frame.variables.push(Value::new_funcval(None, myfuncspec));
        Ok(())
    }
    pub (crate) fn sim_GENERATORDEF(&mut self) -> OpResult
    {
        let myfuncspec = self.read_function(true)?;
        self.top_frame.variables.push(Value::new_funcval(None, myfuncspec));
        Ok(())
    }
    
    pub (crate) fn sim_BINSTATE(&mut self) -> OpResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 2
            {
                return Err(format!("internal error: BINSTATE instruction requires 2 values on the stack but found {}", self.stack_len()));
            }
        }
        
        let immediate = self.pull_single_from_code()?;
        
        let var = self.stack_pop_var().ok_or_else(|| stack_access_err("internal error: primary argument to BINSTATE could not be found or was not a variable"))?;
        
        let value = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: not enough values on stack to run instruction BINSTATE (this error should be inaccessible)"))?;
        
        if immediate == 0x00
        {
            self.evaluate(var)?.assign(value)?;
        }
        else
        {
            do_binstate_function(immediate, self.evaluate(var)?, &value)?;
        }
        Ok(())
    }
    pub (crate) fn sim_UNSTATE(&mut self) -> OpResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return Err(format!("internal error: UNSTATE instruction requires 2 values on the stack but found {}", self.stack_len()));
            }
        }
        
        let immediate = self.pull_single_from_code()?;
        
        let var = self.stack_pop_var().ok_or_else(|| stack_access_err("internal error: argument to UNSTATE could not be found or was not a variable"))?;
        let mut val = self.evaluate(var)?;
        do_unstate_function(immediate, val)?;
        Ok(())
    }
    pub (crate) fn sim_SETBAREGLOBAL(&mut self) -> OpResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return Err(format!("internal error: SETBAREGLOBAL instruction requires 1 values on the stack but found 0"));
            }
        }
        
        let nameindex = self.read_usize()?;
        
        let value = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: not enough values on stack to run instruction SETBAREGLOBAL (this error should be inaccessible)"))?;
        
        self.global.barevariables.insert(nameindex, value);
        
        Ok(())
    }
    
    pub (crate) fn sim_BINOP(&mut self) -> OpResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 2
            {
                return Err(format!("internal error: BINOP instruction requires 2 values on the stack but found {}", self.stack_len()));
            }
        }
        
        let immediate = self.pull_single_from_code()?;
        
        let right = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: not enough values on stack to run instruction BINOP (this error should be inaccessible!)"))?;
        let left = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: not enough values on stack to run instruction BINOP (this error should be inaccessible!)"))?;
        
        self.stack_push_val(do_binop_function(immediate, &left, &right)?);
        Ok(())
    }
    
    fn handle_short_circuit(&mut self, truthiness : bool) -> OpResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return Err(format!("internal error: short circuit instruction requires 1 values on the stack but found {}", self.stack_len()))
            }
        }
        let val = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: left operand of binary logical operator was a variable instead of a value"))?;
        
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
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return Err(format!("internal error: UNOP instruction requires 1 values on the stack but found {}", self.stack_len()))
            }
        }
        
        let immediate = self.pull_single_from_code()?;
        
        let value = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: not enough values on stack to run instruction UNOP (this error should be inaccessible!)"))?;
        let new_value = do_unop_function(immediate, &value)?;
        self.stack_push_val(new_value);
        Ok(())
    }
    pub (crate) fn sim_LAMBDA(&mut self) -> OpResult
    {
        let (captures, myfuncspec) = self.read_lambda()?;
        self.stack_push_val(Value::new_funcval(Some(captures), myfuncspec));
        Ok(())
    }
    pub (crate) fn sim_COLLECTARRAY(&mut self) -> OpResult
    {
        let numvals = self.read_u16()? as usize;
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < numvals
            {
                return Err(format!("internal error: not enough values on stack for COLLECTARRAY instruction to build array (need {}, have {})", numvals, self.stack_len()));
            }
        }
        let mut myarray = Vec::with_capacity(numvals);
        for _ in 0..numvals
        {
            let val = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: COLLECTARRAY instruction failed to collect values from stack (this error should be unreachable!)"))?;
            myarray.push(val);
        }
        myarray.reverse();
        self.stack_push_val(Value::Array(Box::new(myarray)));
        Ok(())
    }
    pub (crate) fn sim_COLLECTDICT(&mut self) -> OpResult
    {
        let numvals = self.read_u16()? as usize;
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < numvals*2
            {
                return Err(format!("internal error: not enough values on stack for COLLECTDICT instruction to build dict (need {}, have {})", numvals*2, self.stack_len()));
            }
        }
        
        let mut mydict = HashMap::<HashableValue, Value>::new();
        for _ in 0..numvals
        {
            let val = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: COLLECTDICT instruction failed to collect values from stack"))?;
            let key = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: COLLECTDICT instruction failed to collect values from stack"))?;
            let hashval = val_to_hashval(key)?;
            // stack popping goes in reverse order (most-recently added items first) so we just don't insert items that are already there
            if !mydict.contains_key(&hashval)
            {
                mydict.insert(hashval, val);
            }
        }
        self.stack_push_val(Value::Dict(Box::new(mydict)));
        Ok(())
    }
    pub (crate) fn sim_COLLECTSET(&mut self) -> OpResult
    {
        let numvals = self.read_u16()? as usize;
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < numvals
            {
                return Err(format!("internal error: not enough values on stack for COLLECTSET instruction to build dict (need {}, have {})", numvals, self.stack_len()));
            }
        }
        
        let mut myset = HashSet::<HashableValue>::new();
        for _ in 0..numvals
        {
            let val = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: COLLECTSET instruction failed to collect values from stack"))?;
            myset.insert(val_to_hashval(val)?);
        }
        self.stack_push_val(Value::Set(Box::new(myset)));
        Ok(())
    }
    pub (crate) fn sim_ARRAYEXPR(&mut self) -> OpResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 2
            {
                return Err(format!("internal error: ARRAYEXPR instruction requires 2 values on the stack but found {}", self.stack_len()));
            }
        }
        let index = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: TODO write error askdgfauiowef"))?;
        let index = val_to_hashval(index)?;
        let array = self.stack_pop().ok_or_else(|| stack_access_err("internal error: TODO write error cvbhsrtgaerffd"))?;
        match array
        {
            StackValue::Var(Variable::Array(mut arrayvar)) =>
            {
                arrayvar.indexes.push(index);
                self.stack_push_var(Variable::Array(arrayvar));
            }
            StackValue::Var(Variable::Direct(dirvar)) =>
                self.stack_push_var(Variable::Array(ArrayVar::new(NonArrayVariable::Direct(dirvar), vec!(index)))),
            StackValue::Var(Variable::Indirect(indirvar)) =>
                self.stack_push_var(Variable::Array(ArrayVar::new(NonArrayVariable::Indirect(indirvar), vec!(index)))),
            StackValue::Var(Variable::Global(globalvar)) =>
                self.stack_push_var(Variable::Array(ArrayVar::new(NonArrayVariable::Global(globalvar), vec!(index)))),
            StackValue::Val(Value::Array(array)) =>
                self.stack_push_var(Variable::Array(ArrayVar::new(NonArrayVariable::ActualArray(array), vec!(index)))),
            StackValue::Val(Value::Dict(dict)) =>
                self.stack_push_var(Variable::Array(ArrayVar::new(NonArrayVariable::ActualDict(dict), vec!(index)))),
            StackValue::Val(Value::Text(string)) =>
                self.stack_push_var(Variable::Array(ArrayVar::new(NonArrayVariable::ActualText(Box::new(string)), vec!(index)))),
            _ =>
                return plainerr("error: tried to use array indexing on a non-indexable value"),
        }
        Ok(())
    }
    pub (crate) fn sim_EVALUATEARRAYEXPR(&mut self) -> OpResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 2
            {
                return Err(format!("internal error: ARRAYEXPR instruction requires 2 values on the stack but found {}", self.stack_len()));
            }
        }
        let index = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: TODO write error askdgfauiowef"))?;
        let index = val_to_hashval(index)?;
        let array = self.stack_pop().ok_or_else(|| stack_access_err("internal error: TODO write error cvbhsrtgaerffd"))?;
        use super::variableaccess::return_indexed;
        match array
        {
            StackValue::Var(Variable::Array(mut arrayvar)) =>
            {
                arrayvar.indexes.push(index);
                let val = self.evaluate_of_array(arrayvar)?.to_val();
                self.stack_push_val(val);
            }
            StackValue::Var(Variable::Direct(dirvar)) =>
            {
                let val = return_indexed(self.evaluate_of_direct(dirvar)?, &[index])?.to_val();
                self.stack_push_val(val);
            }
            StackValue::Var(Variable::Indirect(indirvar)) =>
            {
                let val = return_indexed(self.evaluate_of_indirect(indirvar)?, &[index])?.to_val();
                self.stack_push_val(val);
            }
            StackValue::Var(Variable::Global(globalvar)) =>
            {
                let val = return_indexed(self.evaluate_of_global(globalvar)?, &[index])?.to_val();
                self.stack_push_val(val);
            }
            StackValue::Val(Value::Array(mut array)) =>
            {
                let indexnum = match_or_err!(index, HashableValue::Number(indexnum) => indexnum, minierr("error: tried to use a non-number as an array index"))?.round() as usize;
                
                if indexnum >= array.len()
                {
                    return Err(format!("error: tried to access non-extant index {} of an array", indexnum));
                }
                self.stack_push_val(array.swap_remove(indexnum));
            }
            StackValue::Val(Value::Dict(mut dict)) =>
            {
                self.stack_push_val(dict.remove(&index).ok_or_else(|| format!("error: tried to access non-extant index {:?} of a dict", index))?);
            }
            // TODO reintroduce string support
            StackValue::Val(val) =>
            {
                self.stack_push_val(return_indexed(ValueLoc::Static(val), &[index])?.to_val());
            }
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
            let todrain = data.variables;
            let testval = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: failed to find value on stack while handling WHILE controller"))?;
            if !value_truthy(&testval)
            {
                self.set_pc(dest);
                self.drain_vars(todrain);
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
            let todrain = data.variables;
            self.set_pc(dest);
            self.drain_vars(todrain);
            return Ok(());
        }
        plainerr("internal error: WHILELOOP instruction when immediate controller is not a while controller")
    }
    pub (crate) fn sim_WITHLOOP(&mut self) -> OpResult
    {
        self.top_frame.instancestack.pop();
        
        if let Some(Controller::With(ref mut data)) = self.top_frame.controlstack.last_mut()
        {
            if let Some(next_instance) = data.instances.pop()
            {
                if let Value::Number(next_instance) = next_instance
                {
                    self.top_frame.instancestack.push(next_instance as usize);
                    let dest = data.loop_start;
                    self.set_pc(dest);
                }
                else
                {
                    return strange_err_plain("internal error: values fed to with controller's 'other' data must be a list of only numbers");
                }
            }
            else
            {
                self.top_frame.controlstack.pop();
            }
            return Ok(());
        }
        strange_err_plain("internal error: WITHLOOP instruction when immediate controller is not a with controller")
    }
    pub (crate) fn sim_FOREACHLOOP(&mut self) -> OpResult
    {
        if let Some(Controller::ForEach(ref mut data)) = self.top_frame.controlstack.last_mut()
        {
            let todrain = data.variables;
            let dest = data.loop_start;
            
            if let ForEachValues::Gen(ref mut gen) = data.values
            {
                let mut holder = GeneratorState{frame : None};
                std::mem::swap(&mut holder, gen);
                let frame = holder.frame;
                
                if let Some(frame) = frame
                {
                    self.set_pc(dest);
                    self.drain_vars(todrain);
                    self.push_new_frame(frame)?;
                }
            }
            else
            {
                self.set_pc(dest);
                self.drain_vars(todrain);
            }
            return Ok(());
        }
        strange_err_plain("internal error: FOREACHLOOP instruction when immediate controller is not a foreach controller")
    }
    pub (crate) fn sim_FOREACHHEAD(&mut self) -> OpResult
    {
        if let Some(Controller::ForEach(ref mut data)) = self.top_frame.controlstack.last_mut()
        {
            let dest = data.loop_end;
            if let Some(value) = match data.values
                {
                    ForEachValues::List(ref mut values) => values.pop(),
                    ForEachValues::Gen(ref mut gen) =>
                    {
                        if let Some(StackValue::Val(Value::Generator(mut holder))) = self.top_frame.stack.pop()
                        {
                            std::mem::swap(&mut *holder, gen);
                        }
                        else
                        {
                            return strange_err_plain("internal error: failed to recover generator state in foreach loop over generator");
                        }
                        self.stack_pop_val()
                    }
                }
            {
                self.top_frame.variables.push(value);
            }
            else
            {
                self.set_pc(dest);
                self.top_frame.controlstack.pop();
            }
            return Ok(());
        }
        strange_err_plain("internal error: FOREACHHEAD instruction when immediate controller is not a foreach controller")
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
                self.stack_push_val(Value::Generator(Box::new(GeneratorState{frame : None})));
                if !frame_was_expr
                {
                    return strange_err_plain("internal error: generators must always return into an expression");
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
        if let Some(old_frame) = self.frames.pop()
        {
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
                self.stack_push_val(Value::Generator(Box::new(GeneratorState{frame : None})));
                if !frame_was_expr
                {
                    return strange_err_plain("internal error: generators must always return into an expression");
                }
            }
        }
        else
        {
            self.doexit = true;
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
        let new_gen_state = Box::new(GeneratorState{frame : Some(old_frame)});
        
        if frame_was_expr
        {
            let val = inner_frame_stack_last.ok_or_else(|| minierr("error: YIELD instruction needed a value remaining on the inner frame's stack, but there were none"))?;
            self.stack_push(val);
            self.stack_push_val(Value::Generator(new_gen_state));
        }
        else
        {
            return strange_err_plain("internal error: generators must always return into an expression");
        }
        Ok(())
    }
}