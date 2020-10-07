#![allow(clippy::cast_lossless, clippy::map_entry, non_snake_case)]

use crate::interpreter::*;

#[inline]
fn stack_access_err<S : ToString>(text : S) -> String
{
    #[cfg(feature = "stack_access_debugging")]
    {
        return text.to_string();
    }
    panic!(text.to_string())
}
#[inline]
fn stack_access_err_err<A, S : ToString>(text : S) -> Result<A, String>
{
    #[cfg(feature = "stack_access_debugging")]
    {
        return Err(text.to_string());
    }
    panic!(text.to_string())
}
#[inline]
fn strange_err_plain<A, S : ToString>(text : S) -> Result<A, String>
{
    #[cfg(feature = "broken_compiler_debugging")]
    {
        return Err(text.to_string());
    }
    panic!(text.to_string())
}
#[inline]
fn strange_err<S : ToString>(text : S) -> String
{
    #[cfg(feature = "broken_compiler_debugging")]
    {
        return text.to_string();
    }
    panic!(text.to_string())
}

pub (crate) static mut OPTABLE : [OpFunc; 256] = [Interpreter::sim_INVALID as OpFunc; 256];
pub (crate) static mut REVERSE_OPTABLE : Option<BTreeMap<usize, u8>> = None;

pub (crate) fn build_opfunc_table()
{
    macro_rules! set { ( $x:ident, $y:ident ) => { unsafe { OPTABLE[$x as usize] = Interpreter::$y; } } }
    
    set!(NOP, sim_NOP);
    set!(PUSHFLT, sim_PUSHFLT);
    set!(PUSHSTR, sim_PUSHSTR);
    set!(PUSHNULL, sim_PUSHNULL);
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
    set!(BINSTATE, sim_BINSTATE);
    set!(BINSTATEADD, sim_BINSTATEADD);
    set!(BINSTATESUB, sim_BINSTATESUB);
    set!(BINSTATEMUL, sim_BINSTATEMUL);
    set!(BINSTATEDIV, sim_BINSTATEDIV);
    set!(UNSTATEINCR, sim_UNSTATEINCR);
    set!(UNSTATEDECR, sim_UNSTATEDECR);
    set!(SETBAREGLOBAL, sim_SETBAREGLOBAL);
    set!(BINOPAND, sim_BINOPAND);
    set!(BINOPOR, sim_BINOPOR);
    set!(BINOPEQ, sim_BINOPEQ);
    set!(BINOPNEQ, sim_BINOPNEQ);
    set!(BINOPGEQ, sim_BINOPGEQ);
    set!(BINOPLEQ, sim_BINOPLEQ);
    set!(BINOPG, sim_BINOPG);
    set!(BINOPL, sim_BINOPL);
    set!(BINOPADD, sim_BINOPADD);
    set!(BINOPSUB, sim_BINOPSUB);
    set!(BINOPMUL, sim_BINOPMUL);
    set!(BINOPDIV, sim_BINOPDIV);
    set!(BINOPMOD, sim_BINOPMOD);
    set!(UNOPNEG, sim_UNOPNEG);
    set!(UNOPNOT, sim_UNOPNOT);
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
    
    let mut my_table = BTreeMap::new();
    for i in 0..=255
    {
        unsafe
        {
            my_table.insert(OPTABLE[i as usize] as *const OpFunc as usize, i);
        }
    }
    unsafe
    {
        REVERSE_OPTABLE = Some(my_table);
    }
    
    println!("built opfunc table");
}

impl Interpreter
{
    pub (crate) fn sim_INVALID(&mut self) -> StepResult
    {
        self.sub_pc(1);
        #[cfg(feature = "compiler_invalid_execution_debugging")]
        {
            return Err(format!("internal error: no such operation 0x{:02X}", self.pull_single_from_code()));
        }
        panic!(format!("internal error: no such operation 0x{:02X}", self.pull_single_from_code()))
    }
    
    pub (crate) fn sim_NOP(&mut self) -> StepResult
    {
        //plainerr("NOP")
        default_step_result()
    }
    pub (crate) fn sim_PUSHFLT(&mut self) -> StepResult
    {
        let value = self.read_float();
        self.stack_push_val(Value::Number(value));
        default_step_result()
    }
    pub (crate) fn sim_PUSHSTR(&mut self) -> StepResult
    {
        let text = self.read_indexed_string()?;
        self.stack_push_val(Value::Text(text));
        default_step_result()
    }
    pub (crate) fn sim_PUSHNULL(&mut self) -> StepResult
    {
        self.stack_push_val(Value::Null);
        default_step_result()
    }
    pub (crate) fn sim_PUSHVAR(&mut self) -> StepResult
    {
        let index = self.read_usize();
        self.stack_push_var(Variable::Direct(index));
        default_step_result()
    }
    pub (crate) fn sim_EVALUATEVAR(&mut self) -> StepResult
    {
        let index = self.read_usize();
        let val = self.top_frame.variables.get(index).ok_or_else(|| strange_err("internal error: variable stack out-of-bounds access"))?.clone();
        self.stack_push_val(val);
        default_step_result()
    }
    pub (crate) fn sim_PUSHINSTVAR(&mut self) -> StepResult
    {
        let instance_id = *self.top_frame.instancestack.last().ok_or_else(|| strange_err("internal error: tried to access instance variable when not executing within instance scope"))?;
        let index = self.read_usize();
        self.stack_push_var(Variable::from_indirection(instance_id, index));
        default_step_result()
    }
    pub (crate) fn sim_EVALUATEINSTVAR(&mut self) -> StepResult
    {
        let instance_id = *self.top_frame.instancestack.last().ok_or_else(|| strange_err("internal error: tried to access instance variable when not executing within instance scope"))?;
        let index = self.read_usize();
        
        self.stack_push_val(self.evaluate_of_indirect_simple(instance_id, index)?);
        default_step_result()
    }
    pub (crate) fn sim_PUSHBIND(&mut self) -> StepResult
    {
        let nameindex = self.read_usize();
        self.stack_push_val(Value::InternalFunc(InternalFuncVal{nameindex}));
        default_step_result()
    }
    pub (crate) fn sim_PUSHOBJ(&mut self) -> StepResult
    {
        let nameindex = self.read_usize();
        self.stack_push_val(Value::Object(nameindex));
        default_step_result()
    }
    pub (crate) fn sim_PUSHGLOBAL(&mut self) -> StepResult
    {
        let index = self.read_usize();
        self.stack_push_var(Variable::Global(index));
        default_step_result()
    }
    pub (crate) fn sim_PUSHGLOBALVAL(&mut self) -> StepResult
    {
        let index = self.read_usize();
        let val = self.global.variables.get(&index).ok_or_else(|| format!("error: tried to access global variable `{}` that doesn't exist", self.get_indexed_string(index)))?.clone();
        self.stack_push_val(val);
        default_step_result()
    }
    pub (crate) fn sim_PUSHGLOBALFUNC(&mut self) -> StepResult
    {
        let index = self.read_usize();
        let val = self.global.functions.get(&index).ok_or_else(|| format!("error: tried to access global function `{}` that doesn't exist", self.get_indexed_string(index)))?.clone();
        self.stack_push_val(val);
        default_step_result()
    }
    pub (crate) fn sim_PUSHBAREGLOBAL(&mut self) -> StepResult
    {
        let index = self.read_usize();
        self.stack_push_var(Variable::BareGlobal(index));
        default_step_result()
    }
    pub (crate) fn sim_EVALUATEBAREGLOBAL(&mut self) -> StepResult
    {
        let index = self.read_usize();
        let val = self.global.barevariables.get(&index).ok_or_else(|| format!("internal error: tried to access bare global variable `{}` that doesn't exist", self.get_indexed_string(index)))?.clone();
        self.stack_push_val(val);
        default_step_result()
    }
    
    pub (crate) fn sim_PUSHSELF(&mut self) -> StepResult
    {
        let instance_id = *self.top_frame.instancestack.last().ok_or_else(|| strange_err("internal error: tried to access `self` when not executing within instance scope"))?;
        self.stack_push_val(Value::Instance(instance_id));
        default_step_result()
    }
    pub (crate) fn sim_PUSHOTHER(&mut self) -> StepResult
    {
        let loc = self.top_frame.instancestack.len()-2;
        let instance_id = *self.top_frame.instancestack.get_mut(loc).ok_or_else(|| "error: tried to access `other` while not inside of at least two instance scopes".to_string())?;
        self.stack_push_val(Value::Instance(instance_id));
        default_step_result()
    }
    
    pub (crate) fn sim_INDIRECTION(&mut self) -> StepResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return Err(format!("internal error: INDIRECTION instruction requires 1 values on the stack but only found {}", self.stack_len()));
            }
        }
        let name = self.read_usize();
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
                    _ => return Err("error: tried to use indirection on a non-instance or non-global value".to_string())
                }
            }
            _ => return plainerr("error: tried to use indirection on a type that doesn't support it (only instances, dictionaries, and 'special' values are allowed)")
        }
        
        default_step_result()
    }
    pub (crate) fn sim_EVALUATEINDIRECTION(&mut self) -> StepResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return Err(format!("internal error: EVALUATEINDIRECTION instruction requires 1 values on the stack but only found {}", self.stack_len()));
            }
        }
        let name = self.read_usize();
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
                    q => return Err(format!("error: tried to use eval indirection on a non-instance or non-global value ({:?})", q))
                }
            }
            _ => return plainerr("error: tried to use eval indirection on a type that doesn't support it (only instances, dictionaries, and 'special' values are allowed)")
        }
        
        default_step_result()
    }
    pub (crate) fn sim_DISMEMBER(&mut self) -> StepResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return Err(format!("internal error: DISMEMBER instruction requires 1 values on the stack but only found {}", self.stack_len()));
            }
        }
        let name = self.read_usize();
        let source = self.stack_pop().ok_or_else(|| stack_access_err("internal error: failed to get source from stack in DISMEMBER operation"))?;
        
        self.stack_push_val(Value::SubFunc(Box::new(SubFuncVal{source, name})));
        default_step_result()
    }
    pub (crate) fn sim_FUNCCALL(&mut self) -> StepResult
    {
        self.handle_func_call_or_expr(false)?;
        default_step_result()
    }
    pub (crate) fn sim_FUNCEXPR(&mut self) -> StepResult
    {
        self.handle_func_call_or_expr(true)?;
        default_step_result()
    }
    pub (crate) fn sim_INVOKE(&mut self) -> StepResult
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
        
        default_step_result()
    }
    pub (crate) fn sim_INVOKECALL(&mut self) -> StepResult
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
        
        default_step_result()
    }
    pub (crate) fn sim_INVOKEEXPR(&mut self) -> StepResult
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
        
        default_step_result()
    }
    
    pub (crate) fn sim_BREAK(&mut self) -> StepResult
    {
        self.pop_controlstack_until_loop();
        
        let controller = self.top_frame.controlstack.last().ok_or_else(|| minierr("error: break instruction not inside of loop"))?;
        
        let destination =
        match controller
        {
            Controller::While(data) => data.loop_end,
            _ => return plainerr("FIXME: unimplemented BREAK out from non-for/while loop")
        };
        
        self.set_pc(destination);
        self.top_frame.controlstack.pop();
        
        default_step_result()
    }
    pub (crate) fn sim_CONTINUE(&mut self) -> StepResult
    {
        self.pop_controlstack_until_loop();
        
        let controller = self.top_frame.controlstack.last().ok_or_else(|| minierr("error: continue instruction not inside of loop"))?;
        
        let destination =
        match controller
        {
            Controller::While(data) => data.expr_start,
            _ => return plainerr("FIXME: unimplemented CONTINUE out from non-for/while loop")
        };
        
        self.set_pc(destination);
        
        default_step_result()
    }
    pub (crate) fn sim_IF(&mut self) -> StepResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return plainerr("internal error: IF instruction requires 1 values on the stack but found 0");
            }
        }
        let testval = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: failed to find value on stack while handling IF controller"))?;
        let codelen = self.read_usize();
        if !value_truthy(self, &testval)
        {
            self.add_pc(codelen);
        }
        
        default_step_result()
    }
    pub (crate) fn sim_WHILE(&mut self) -> StepResult
    {
        let exprlen = self.read_usize();
        let codelen = self.read_usize();
        let current_pc = self.get_pc();
        self.top_frame.controlstack.push(Controller::While(WhileData{
            expr_start : current_pc,
            loop_start : current_pc+exprlen,
            loop_end : current_pc+exprlen+codelen
        }));
        default_step_result()
    }
    pub (crate) fn sim_FOR(&mut self) -> StepResult
    {
        let postlen = self.read_usize();
        let exprlen = self.read_usize();
        let codelen = self.read_usize();
        let current_pc = self.get_pc();
        self.top_frame.controlstack.push(Controller::While(WhileData{
            expr_start : current_pc,
            loop_start : current_pc+postlen+exprlen,
            loop_end : current_pc+postlen+exprlen+codelen
        }));
        self.add_pc(postlen);
        default_step_result()
    }
    pub (crate) fn sim_FOREACH(&mut self) -> StepResult
    {
        let mut val = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: foreach loop was fed a variable of some sort, instead of a value, for what to loop over"))?;
        
        let list : ForEachValues = match val
        {
            Value::Array(ref mut list) => ForEachValues::List(list.drain(..).rev().collect()),
            Value::Dict(ref mut dict)  => ForEachValues::List(dict.drain().map(|(k, v)| Value::Array(vec!(hashval_to_val(k), v))).collect()),
            Value::Set(ref mut set)    => ForEachValues::List(set.drain().map(hashval_to_val).collect()),
            Value::Generator(_) => ForEachValues::Gen(GeneratorState{frame : None}),
            _ => return plainerr("error: value fed to for-each loop must be an array, dictionary, set, or generatorstate")
        };
        
        let varindex = self.read_usize();
        let codelen = self.read_usize();
        let current_pc = self.get_pc();
        self.top_frame.controlstack.push(Controller::ForEach(ForEachData{
            varindex,
            loop_start : current_pc,
            loop_end : current_pc+codelen,
            values : list
        }));
        
        if let Value::Generator(genstate) = val
        {
            let frame = genstate.frame.ok_or_else(|| minierr("error: tried to invoke a dead generator in a foreach loop"))?;
            self.push_new_frame(frame)?;
        }
        
        default_step_result()
    }
    pub (crate) fn sim_WITH(&mut self) -> StepResult
    {
        let object_id = self.read_usize();
        let codelen = self.read_usize();
        let current_pc = self.get_pc();
        
        let instance_id_list : Vec<usize> = self.global.instances_by_type.get(&object_id).ok_or_else(|| minierr("error: tried to use non-existant object type in with expression"))?.iter().cloned().collect();
        if let Some(first) = instance_id_list.first()
        {
            self.top_frame.instancestack.push(*first);
            self.top_frame.controlstack.push(Controller::With(WithData{
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
        
        default_step_result()
    }
    pub (crate) fn sim_WITHAS(&mut self) -> StepResult
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
        let codelen = self.read_usize();
        let current_pc = self.get_pc();
        
        if !self.global.instances.contains_key(&instance_id)
        {
            return plainerr("error: tried to use non-extant instance as argument of with()");
        }
        
        self.top_frame.instancestack.push(instance_id);
        
        self.top_frame.controlstack.push(Controller::With(WithData{
            loop_start : current_pc,
            loop_end : current_pc + codelen,
            instances : Vec::new()
        }));
        
        default_step_result()
    }
    pub (crate) fn sim_SWITCH(&mut self) -> StepResult
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
        
        let num_cases = self.read_usize();
        let current_pc = self.get_pc();
        
        let mut case_block_addresses = Vec::with_capacity(num_cases as usize);
        for _ in 0..num_cases
        {
            case_block_addresses.push(current_pc + self.read_usize());
        }
        let exit = current_pc + self.read_usize();
        
        self.top_frame.controlstack.push(Controller::Switch(SwitchData{
            blocks : case_block_addresses,
            exit,
            value
        }));
        
        //eprintln!("end of sim_switch");
        //eprintln!("{:?}", self.top_frame.controlstack.last().unwrap());
        
        default_step_result()
    }
    pub (crate) fn sim_SWITCHCASE(&mut self) -> StepResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return plainerr("internal error: SWITCHCASE instruction requires 1 values on the stack but found 0");
            }
        }
        let value = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: switch case expression was a variable instead of a value"))?;
        
        let which_case = self.read_usize();
        
        let switchdata : &SwitchData = match_or_err!(self.top_frame.controlstack.last(), Some(Controller::Switch(ref x)) => x, strange_err("internal error: SWITCHCASE instruction outside of switch statement"))?;
        let dest = *switchdata.blocks.get(which_case as usize).ok_or_else(|| strange_err("internal error: which_case in SWITCHCASE was too large"))?;
        
        if ops::value_equal(&value, &switchdata.value)?
        {
            //eprintln!("jumping to {} thanks to switch", dest);
            self.set_pc(dest);
        }
        
        default_step_result()
    }
    pub (crate) fn sim_SWITCHDEFAULT(&mut self) -> StepResult
    {
        let which_case = self.read_usize();
        let switchdata : &SwitchData = match_or_err!(self.top_frame.controlstack.last(), Some(Controller::Switch(ref x)) => x, strange_err("internal error: SWITCHDEFAULT instruction outside of switch statement"))?;
        let dest = *switchdata.blocks.get(which_case as usize).ok_or_else(|| strange_err("internal error: which_case in SWITCHDEFAULT was too large"))?;
        self.set_pc(dest);
        
        default_step_result()
    }
    pub (crate) fn sim_SWITCHEXIT(&mut self) -> StepResult
    {
        let switchdata = match_or_err!(self.top_frame.controlstack.pop(), Some(Controller::Switch(x)) => x, strange_err("internal error: SWITCHDEFAULT instruction outside of switch statement"))?;
        self.set_pc(switchdata.exit);
        
        default_step_result()
    }
    pub (crate) fn sim_FUNCDEF(&mut self) -> StepResult
    {
        let myfuncspec = self.read_function(false)?;
        let which_id = self.read_usize();
        self.top_frame.variables[which_id] = Value::new_funcval(None, myfuncspec);
        default_step_result()
    }
    pub (crate) fn sim_GENERATORDEF(&mut self) -> StepResult
    {
        let myfuncspec = self.read_function(true)?;
        let which_id = self.read_usize();
        self.top_frame.variables[which_id] = Value::new_funcval(None, myfuncspec);
        default_step_result()
    }
    
    #[inline]
    fn binstate_prep(&mut self) -> Result<(ValueLoc, Value), String>
    {
        if self.stack_len() < 2
        {
            return Err(stack_access_err(format!("internal error: BINSTATE instruction requires 2 values on the stack but found {}", self.stack_len())));
        }
        
        let var = self.stack_pop_var().ok_or_else(|| stack_access_err("internal error: primary argument to BINSTATE could not be found or was not a variable"))?;
        let value = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: not enough values on stack to run instruction BINSTATE (this error should be inaccessible)"))?;
        
        Ok((self.evaluate(var)?, value))
    }
    
    pub (crate) fn sim_BINSTATE(&mut self) -> StepResult
    {
        let (mut var, value) = self.binstate_prep()?;
        var.assign(value)?;
        default_step_result()
    }
    pub (crate) fn sim_BINSTATEADD(&mut self) -> StepResult
    {
        let (var, value) = self.binstate_prep()?;
        inplace_value_op_add(var, &value)?;
        default_step_result()
    }
    pub (crate) fn sim_BINSTATESUB(&mut self) -> StepResult
    {
        let (var, value) = self.binstate_prep()?;
        inplace_value_op_subtract(var, &value)?;
        default_step_result()
    }
    pub (crate) fn sim_BINSTATEMUL(&mut self) -> StepResult
    {
        let (var, value) = self.binstate_prep()?;
        inplace_value_op_multiply(var, &value)?;
        default_step_result()
    }
    pub (crate) fn sim_BINSTATEDIV(&mut self) -> StepResult
    {
        let (var, value) = self.binstate_prep()?;
        inplace_value_op_divide(var, &value)?;
        default_step_result()
    }
    pub (crate) fn sim_UNSTATEINCR(&mut self) -> StepResult
    {
        if self.stack_len() < 1
        {
            return Err(stack_access_err("internal error: UNSTATEINCR instruction requires 2 values on the stack but found 0"));
        }
        let var = self.stack_pop_var().ok_or_else(|| stack_access_err("internal error: argument to UNSTATEINCR could not be found or was not a variable"))?;
        let val = self.evaluate(var)?;
        do_inplace_value_op_increment(val)?;
        default_step_result()
    }
    pub (crate) fn sim_UNSTATEDECR(&mut self) -> StepResult
    {
        if self.stack_len() < 1
        {
            return Err(stack_access_err("internal error: UNSTATEINCR instruction requires 2 values on the stack but found 0"));
        }
        let var = self.stack_pop_var().ok_or_else(|| stack_access_err("internal error: argument to UNSTATEDECR could not be found or was not a variable"))?;
        let val = self.evaluate(var)?;
        do_inplace_value_op_decrement(val)?;
        default_step_result()
    }
    pub (crate) fn sim_SETBAREGLOBAL(&mut self) -> StepResult
    {
        if self.stack_len() < 1
        {
            return stack_access_err_err("internal error: SETBAREGLOBAL instruction requires 1 values on the stack but found 0");
        }
        
        let nameindex = self.read_usize();
        
        let value = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: not enough values on stack to run instruction SETBAREGLOBAL (this error should be inaccessible)"))?;
        
        self.global.barevariables.insert(nameindex, value);
        
        default_step_result()
    }
    
    #[inline]
    fn binop_prep(&mut self) -> Result<(Value, Value), String>
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 2
            {
                return Err(format!("internal error: BINOP instruction requires 2 values on the stack but found {}", self.stack_len()));
            }
        }
        let right = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: not enough values on stack to run instruction BINOP (this error should be inaccessible!)"))?;
        let left = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: not enough values on stack to run instruction BINOP (this error should be inaccessible!)"))?;
        
        Ok((left, right))
    }
    
    pub (crate) fn sim_BINOPAND(&mut self) -> StepResult
    {
        let (left, right) = self.binop_prep()?;
        self.stack_push_val(value_op_and(&left, &right)?);
        default_step_result()
    }
    pub (crate) fn sim_BINOPOR(&mut self) -> StepResult
    {
        let (left, right) = self.binop_prep()?;
        self.stack_push_val(value_op_or(&left, &right)?);
        default_step_result()
    }
    pub (crate) fn sim_BINOPEQ(&mut self) -> StepResult
    {
        let (left, right) = self.binop_prep()?;
        self.stack_push_val(value_op_equal(&left, &right)?);
        default_step_result()
    }
    pub (crate) fn sim_BINOPNEQ(&mut self) -> StepResult
    {
        let (left, right) = self.binop_prep()?;
        self.stack_push_val(value_op_not_equal(&left, &right)?);
        default_step_result()
    }
    pub (crate) fn sim_BINOPGEQ(&mut self) -> StepResult
    {
        let (left, right) = self.binop_prep()?;
        self.stack_push_val(value_op_greater_or_equal(&left, &right)?);
        default_step_result()
    }
    pub (crate) fn sim_BINOPLEQ(&mut self) -> StepResult
    {
        let (left, right) = self.binop_prep()?;
        self.stack_push_val(value_op_less_or_equal(&left, &right)?);
        default_step_result()
    }
    pub (crate) fn sim_BINOPG(&mut self) -> StepResult
    {
        let (left, right) = self.binop_prep()?;
        self.stack_push_val(value_op_greater(&left, &right)?);
        default_step_result()
    }
    pub (crate) fn sim_BINOPL(&mut self) -> StepResult
    {
        let (left, right) = self.binop_prep()?;
        self.stack_push_val(value_op_less(&left, &right)?);
        default_step_result()
    }
    pub (crate) fn sim_BINOPADD(&mut self) -> StepResult
    {
        let (left, right) = self.binop_prep()?;
        self.stack_push_val(value_op_add(&left, &right)?);
        default_step_result()
    }
    pub (crate) fn sim_BINOPSUB(&mut self) -> StepResult
    {
        let (left, right) = self.binop_prep()?;
        self.stack_push_val(value_op_subtract(&left, &right)?);
        default_step_result()
    }
    pub (crate) fn sim_BINOPMUL(&mut self) -> StepResult
    {
        let (left, right) = self.binop_prep()?;
        self.stack_push_val(value_op_multiply(&left, &right)?);
        default_step_result()
    }
    pub (crate) fn sim_BINOPDIV(&mut self) -> StepResult
    {
        let (left, right) = self.binop_prep()?;
        self.stack_push_val(value_op_divide(&left, &right)?);
        default_step_result()
    }
    pub (crate) fn sim_BINOPMOD(&mut self) -> StepResult
    {
        let (left, right) = self.binop_prep()?;
        self.stack_push_val(value_op_modulo(&left, &right)?);
        default_step_result()
    }
    
    fn handle_short_circuit(&mut self, truthiness : bool) -> StepResult
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return Err(format!("internal error: short circuit instruction requires 1 values on the stack but found {}", self.stack_len()))
            }
        }
        let val = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: left operand of binary logical operator was a variable instead of a value"))?;
        
        let rel = self.read_usize();
        
        let truthy = value_truthy(self, &val);
        
        if truthy as bool == truthiness
        {
            self.add_pc(rel as usize);
            self.stack_push_val(Value::Number(bool_floaty(truthy)));
        }
        else
        {
            self.stack_push_val(val);
        }
        default_step_result()
    }
    pub (crate) fn sim_SHORTCIRCUITIFTRUE(&mut self) -> StepResult
    {
        self.handle_short_circuit(true)
    }
    
    pub (crate) fn sim_SHORTCIRCUITIFFALSE(&mut self) -> StepResult
    {
        self.handle_short_circuit(false)
    }
    
    #[inline]
    fn unop_prep(&mut self) -> Result<Value, String>
    {
        #[cfg(feature = "stack_len_debugging")]
        {
            if self.stack_len() < 1
            {
                return Err(format!("internal error: UNOP instruction requires 1 values on the stack but found {}", self.stack_len()))
            }
        }
        self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: not enough values on stack to run instruction UNOP (this error should be inaccessible!)"))
    }
    
    pub (crate) fn sim_UNOPNEG(&mut self) -> StepResult
    {
        let value = self.unop_prep()?;
        self.stack_push_val(do_value_op_negative(&value)?);
        default_step_result()
    }
    pub (crate) fn sim_UNOPNOT(&mut self) -> StepResult
    {
        let value = self.unop_prep()?;
        self.stack_push_val(do_value_op_not(&value)?);
        default_step_result()
    }
    pub (crate) fn sim_LAMBDA(&mut self) -> StepResult
    {
        let (captures, myfuncspec) = self.read_lambda()?;
        self.stack_push_val(Value::new_funcval(Some(captures), myfuncspec));
        default_step_result()
    }
    pub (crate) fn sim_COLLECTARRAY(&mut self) -> StepResult
    {
        let numvals = self.read_usize() as usize;
        let mut myarray = vec!(Value::Null; numvals);
        for i in numvals-1..=0
        {
            let val = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: COLLECTARRAY instruction failed to collect values from stack (this error should be unreachable!)"))?;
            myarray[i] = val;
        }
        self.stack_push_val(Value::Array(myarray));
        default_step_result()
    }
    pub (crate) fn sim_COLLECTDICT(&mut self) -> StepResult
    {
        let numvals = self.read_usize() as usize;
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
        default_step_result()
    }
    pub (crate) fn sim_COLLECTSET(&mut self) -> StepResult
    {
        let numvals = self.read_usize() as usize;
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
        default_step_result()
    }
    pub (crate) fn sim_ARRAYEXPR(&mut self) -> StepResult
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
                self.stack_push_var(Variable::Array(ArrayVar::new(NonArrayVariable::ActualArray(Box::new(array)), vec!(index)))),
            StackValue::Val(Value::Dict(dict)) =>
                self.stack_push_var(Variable::Array(ArrayVar::new(NonArrayVariable::ActualDict(dict), vec!(index)))),
            StackValue::Val(Value::Text(string)) =>
                self.stack_push_var(Variable::Array(ArrayVar::new(NonArrayVariable::ActualText(Box::new(string)), vec!(index)))),
            _ =>
                return plainerr("error: tried to use array indexing on a non-indexable value"),
        }
        default_step_result()
    }
    pub (crate) fn sim_EVALUATEARRAYEXPR(&mut self) -> StepResult
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
            StackValue::Val(val) =>
            {
                self.stack_push_val(return_indexed(ValueLoc::Static(val), &[index])?.to_val());
            }
            _ =>
                return plainerr("error: tried to use array indexing on a non-indexable value"),
        }
        default_step_result()
    }
    pub (crate) fn sim_WHILETEST(&mut self) -> StepResult
    {
        if let Some(Controller::While(ref data)) = self.top_frame.controlstack.last()
        {
            let dest = data.loop_end;
            let testval = self.stack_pop_val().ok_or_else(|| stack_access_err("internal error: failed to find value on stack while handling WHILE controller"))?;
            if !value_truthy(self, &testval)
            {
                self.set_pc(dest);
                self.top_frame.controlstack.pop();
            }
            return default_step_result();
        }
        plainerr("internal error: WHILELOOP instruction when immediate controller is not a while controller")
    }
    pub (crate) fn sim_WHILELOOP(&mut self) -> StepResult
    {
        if let Some(Controller::While(ref data)) = self.top_frame.controlstack.last()
        {
            let dest = data.expr_start;
            self.set_pc(dest);
            return default_step_result();
        }
        plainerr("internal error: WHILELOOP instruction when immediate controller is not a while controller")
    }
    pub (crate) fn sim_WITHLOOP(&mut self) -> StepResult
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
            return default_step_result();
        }
        strange_err_plain("internal error: WITHLOOP instruction when immediate controller is not a with controller")
    }
    pub (crate) fn sim_FOREACHLOOP(&mut self) -> StepResult
    {
        if let Some(Controller::ForEach(ref mut data)) = self.top_frame.controlstack.last_mut()
        {
            let dest = data.loop_start;
            
            if let ForEachValues::Gen(ref mut gen) = data.values
            {
                let mut holder = GeneratorState{frame : None};
                std::mem::swap(&mut holder, gen);
                let frame = holder.frame;
                
                if let Some(frame) = frame
                {
                    self.set_pc(dest);
                    self.push_new_frame(frame)?;
                }
            }
            else
            {
                self.set_pc(dest);
            }
            return default_step_result();
        }
        strange_err_plain("internal error: FOREACHLOOP instruction when immediate controller is not a foreach controller")
    }
    pub (crate) fn sim_FOREACHHEAD(&mut self) -> StepResult
    {
        if let Some(Controller::ForEach(ref mut data)) = self.top_frame.controlstack.last_mut()
        {
            let varindex = data.varindex;
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
                self.top_frame.variables[varindex] = value;
            }
            else
            {
                self.set_pc(dest);
                self.top_frame.controlstack.pop();
            }
            return default_step_result();
        }
        strange_err_plain("internal error: FOREACHHEAD instruction when immediate controller is not a foreach controller")
    }
    pub (crate) fn sim_JUMPRELATIVE(&mut self) -> StepResult
    {
        let rel = self.read_usize();
        self.add_pc(rel);
        default_step_result()
    }
    pub (crate) fn sim_EXIT(&mut self) -> StepResult // an exit is a return with no value
    {
        if let Some(outer_top_frame) = self.frames.pop()
        {
            let was_generator = self.top_frame.generator;
            let frame_was_expr = self.top_frame.isexpr;
            self.top_frame = outer_top_frame;
            // exit implies no remaining value on the stack. if the outside expects a value, push it
            if frame_was_expr
            {
                self.stack_push_val(Value::default());
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
            return Err("GRACEFUL_EXIT".to_string())
        }
        default_step_result()
    }
    pub (crate) fn sim_RETURN(&mut self) -> StepResult
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
            return Err("GRACEFUL_EXIT".to_string())
        }
        default_step_result()
    }
    pub (crate) fn sim_YIELD(&mut self) -> StepResult
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
        default_step_result()
    }
}