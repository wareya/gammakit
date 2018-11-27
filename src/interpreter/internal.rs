use interpreter::*;

impl Interpreter
{
    pub(super) fn build_funcspec_location(&self) -> FuncSpecLocation
    {
        let mut outer_frames = Vec::<FrameIdentity>::new();
        for frame in &self.frames
        {
            outer_frames.push(FrameIdentity::new(&frame));
        }
        FuncSpecLocation { outer_frames, top_frame : FrameIdentity::new(&self.top_frame) }
    }
    
    pub(super) fn handle_func_call_or_expr(&mut self, global : &mut GlobalState, isexpr : bool)
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
    // if value is None, finds and returns appropriate value; otherwise, stores value and returns None
    pub(super) fn evaluate_or_store(&mut self, global : &mut GlobalState, variable : &Variable, value : Option<Value>) -> Option<Value>
    {
        macro_rules! assign_or_return_indexed {
            ( $value:expr, $var:expr, $indexes:expr, $isconst:expr ) =>
            {
                unsafe
                {
                    let mut ptr = $var as *mut Value;
                    
                    let num_indexes = $indexes.len();
                    let mut current_index = 0;
                    
                    for index in &$indexes
                    {
                        if let Value::Array(ref mut newvar) = *ptr
                        {
                            if let Value::Number(indexnum) = index
                            {
                                if let Some(newvar2) = newvar.get_mut(indexnum.round() as usize)
                                {
                                    ptr = newvar2 as *mut Value;
                                }
                                else
                                {
                                    panic!("error: tried to access non-extant index {} of an array", indexnum);
                                }
                            }
                            else
                            {
                                panic!("error: tried to use a non-number as an array index");
                            }
                        }
                        else if let Value::Dict(ref mut newvar) = *ptr
                        {
                            if let Value::Number(indexnum) = index
                            {
                                if let Some(newvar2) = newvar.get_mut(&HashableValue::Number(*indexnum))
                                {
                                    ptr = newvar2 as *mut Value;
                                }
                                else
                                {
                                    panic!("error: tried to access non-extant index {} of a dict", indexnum);
                                }
                            }
                            else if let Value::Text(indexstr) = index
                            {
                                if let Some(newvar2) = newvar.get_mut(&HashableValue::Text(indexstr.clone()))
                                {
                                    ptr = newvar2 as *mut Value;
                                }
                                else
                                {
                                    panic!("error: tried to access non-extant index {} of a dict", indexstr);
                                }
                            }
                            else
                            {
                                panic!("error: tried to use a non-number, non-string as a dict index");
                            }
                        }
                        else if let Value::Text(ref mut text) = *ptr
                        {
                            if current_index+1 != num_indexes
                            {
                                // FIXME should we just treat further indexes as 0? that's what they would do if they were indexes into the substring at that index anyway, so...
                                panic!("error: tried to index into the value at another index in a string (i.e. tried to do something like \"asdf\"[0][0])");
                            }
                            else
                            {
                                if let Value::Number(indexnum) = index
                                {
                                    let mut realindex = ((indexnum.round() as i64) % text.len() as i64) as usize;
                                    
                                    
                                    if let Some(value) = $value
                                    {
                                        if let Value::Text(mychar) = value
                                        {
                                            if mychar.len() == 1
                                            {
                                                let mut codepoints = text.chars().collect::<Vec<char>>();
                                                codepoints[realindex] = mychar.chars().next().unwrap();
                                                /*
                                                // turn array of codepoints back into string
                                                */
                                                let newstr : String = codepoints.iter().collect();
                                                *ptr = Value::Text(newstr);
                                                return None;
                                            }
                                            else
                                            {
                                                panic!("error: tried to assign to an index into a string with a string that was not exactly one character long (was {} characters long)", mychar.len());
                                            }
                                        }
                                        else
                                        {
                                            panic!("error: tried to assign non-string to an index into a string (assigning by codepoint is not supported yet)");
                                        }
                                    }
                                    else
                                    {
                                        let mychar = text.chars().collect::<Vec<char>>()[realindex];
                                        let mut newstr = String::new();
                                        newstr.push(mychar);
                                        return Some(Value::Text(newstr));
                                    }
                                }
                                else
                                {
                                    panic!("error: tried to use a non-number as an index into a string");
                                }
                            }
                        }
                        else
                        {
                            panic!("error: tried to index into a non-array, non-dict value");
                        }
                        current_index += 1;
                    }
                    
                    if let Some(value) = $value
                    {
                        if $isconst
                        {
                            panic!("error: tried to assign to non-variable or read-only value");
                        }
                        else
                        {
                            *ptr = value.clone();
                        }
                        
                        return None;
                    }
                    else
                    {
                        return Some((*ptr).clone());
                    }
                }
            }
        }
        macro_rules! check_frame_dirvar_arrayed {
            ( $frame:expr, $dirvar:expr, $value:expr, $indexes:expr ) =>
            {
                // FIXME: do I even want to search up instance stacks rather than just accessing the main one?
                for scope in $frame.scopes.iter_mut().rev()
                {
                    if let Some(var) = scope.get_mut(&$dirvar.name)
                    {
                        assign_or_return_indexed!($value, var, $indexes, false);
                    }
                }
                for id in $frame.instancestack.iter_mut().rev()
                {
                    if let Some(inst) = global.instances.get_mut(id)
                    {
                        if let Some(var) = inst.variables.get_mut(&$dirvar.name)
                        {
                            assign_or_return_indexed!($value, var, $indexes, false);
                        }
                        // no need to check for instance function names because they can't be indexed
                    }
                }
            }
        }
        
        macro_rules! assign_or_return {
            ( $value:expr, $var:expr ) =>
            {
                if let Some(value) = $value
                {
                    *$var = value.clone();
                    
                    return None;
                }
                else
                {
                    return Some($var.clone());
                }
            }
        }
        macro_rules! check_frame_dirvar {
            ( $frame:expr, $dirvar:expr, $value:expr ) =>
            {
                // FIXME: do I even want to search up instance stacks rather than just accessing the main one?
                for scope in $frame.scopes.iter_mut().rev()
                {
                    if let Some(var) = scope.get_mut(&$dirvar.name)
                    {
                        assign_or_return!($value, var);
                    }
                }
                for id in $frame.instancestack.iter_mut().rev()
                {
                    if let Some(inst) = global.instances.get_mut(id)
                    {
                        if let Some(var) = inst.variables.get_mut(&$dirvar.name)
                        {
                            assign_or_return!($value, var);
                        }
                        else if let Some(objspec) = global.objects.get(&inst.objtype)
                        {
                            if let Some(funcdat) = objspec.functions.get(&$dirvar.name)
                            {
                                if let Some(_value) = $value
                                {
                                    panic!("error: tried to assign to function `{}` in instance of object type `{}`", $dirvar.name, objspec.name);
                                }
                                else
                                {
                                    let mut mydata = funcdat.clone();
                                    mydata.forcecontext = inst.ident;
                                    return Some(Value::new_funcval(false, None, None, Some(mydata)));
                                }
                            }
                        }
                    }
                }
            }
        }
        match &variable
        {
            Variable::Array(ref arrayvar) =>
            {
                match &arrayvar.location
                {
                    NonArrayVariable::Indirect(ref indirvar) =>
                    {
                        // TODO: deduplicate with macros? (vs. non-array code below)
                        if let Some(instance) = global.instances.get_mut(&indirvar.ident)
                        {
                            if let Some(mut var) = instance.variables.get_mut(&indirvar.name)
                            {
                                assign_or_return_indexed!(value, var, arrayvar.indexes, false);
                            }
                            else
                            {
                                panic!("error: tried to read non-extant variable `{}` in instance `{}`", indirvar.name, indirvar.ident);
                            }
                        }
                        else
                        {
                            panic!("error: tried to access variable `{}` from non-extant instance `{}`", indirvar.name, indirvar.ident);
                        }
                    }
                    NonArrayVariable::Direct(ref dirvar) =>
                    {
                        check_frame_dirvar_arrayed!(self.top_frame, dirvar, value, arrayvar.indexes);
                        for frame in self.frames.iter_mut().rev()
                        {
                            check_frame_dirvar_arrayed!(frame, dirvar, value, arrayvar.indexes);
                        }
                        if let Some(_var) = global.objectnames.get(&dirvar.name)
                        {
                            panic!("error: tried to index into object name as though it was an array");
                        }
                        if let Some(_internal_func) = self.get_internal_function(&dirvar.name)
                        {
                            panic!("error: tried to index into internal function name as though it was an array");
                        }
                        panic!("error: unknown variable `{}`", dirvar.name);
                    }
                    NonArrayVariable::ActualArray(ref array) =>
                    {
                        assign_or_return_indexed!(value, &mut Value::Array(array.clone()), arrayvar.indexes, true);
                    }
                }
            }
            Variable::Indirect(ref indirvar) =>
            {
                // TODO: deduplicate with macros? (vs. array code above)
                if let Some(instance) = global.instances.get_mut(&indirvar.ident)
                {
                    if let Some(var) = instance.variables.get_mut(&indirvar.name)
                    {
                        assign_or_return!(value, var);
                    }
                    else if let Some(objspec) = global.objects.get(&instance.objtype)
                    {
                        if let Some(funcdat) = objspec.functions.get(&indirvar.name)
                        {
                            if let Some(_value) = value
                            {
                                panic!("error: tried to assign to function `{}` in instance of object type `{}`", indirvar.name, objspec.name);
                            }
                            else
                            {
                                let mut mydata = funcdat.clone();
                                mydata.forcecontext = indirvar.ident;
                                return Some(Value::new_funcval(false, None, None, Some(mydata)));
                            }
                        }
                        else
                        {
                            panic!("error: tried to read non-extant variable `{}` in instance `{}`", indirvar.name, indirvar.ident);
                        }
                    }
                    else
                    {
                        panic!("error: tried to read non-extant variable `{}` in instance `{}`", indirvar.name, indirvar.ident);
                    }
                }
                else
                {
                    panic!("error: tried to access variable `{}` from non-extant instance `{}`", indirvar.name, indirvar.ident);
                }
            }
            Variable::Direct(ref dirvar) =>
            {
                check_frame_dirvar!(self.top_frame, dirvar, value);
                for frame in self.frames.iter_mut().rev()
                {
                    check_frame_dirvar!(frame, dirvar, value);
                }
                if let Some(var) = global.objectnames.get(&dirvar.name)
                {
                    if let Some(_value) = value
                    {
                        panic!("error: tried to assign to read-only object name `{}`", dirvar.name);
                    }
                    else
                    {
                        return Some(Value::Number(*var as f64));
                    }
                }
                // TODO: Store actual function pointer instead?
                if let Some(_internal_func) = self.get_internal_function(&dirvar.name)
                {
                    return Some(Value::new_funcval(true, Some(dirvar.name.clone()), None, None ));
                }
                
                panic!("error: unknown identifier `{}`", dirvar.name);
            }
        }
    }
}