use crate::interpreter::*;

// "Indirect" variables store an instance id and a variable name.
// mychar.player.inputs evaluates from left to right:
// (characterid).player.inputs
// (playerid).inputs
// This is done because gammakit doesn't have any kind of concept of "references" in the sense of a binding to a variable somewhere else or in the GC.
// The only "references" that exist anywhere in gammakit are instance ids, which are manually managed with instance_kill.
// When you assign to an "indirect" variable, the language needs to hold on to what instance that variable belongs to.

// Arrays and dictionaries are handled differently. Because they're moved by value, not id, they can't be partially evaluated.
// Arrays-of-arrays are stored, literally, as arrays of arrays. Not as arrays of references or pointers.
// So the entire list of indexes (e.g. myarray["stats"][35][23], for a dictionary of arrays of arrays) needs to be stored.
// Each index can be evaluated individually. These are then stored in a list.
// When the expression is accessed, the language searches for the variable name in the current scope,
//  then uses assign_or_return_indexed to get at and work with the relevant value.

// Before you ask: Things like x += y work by evaluating x and storing the evaluation temporarily, so variableaccess.rs only handles evaluation and storage.

fn plainerr(mystr : &'static str) -> Result<Option<Value>, String>
{
    Err(mystr.to_string())
}

fn assign_or_return(value : Option<Value>, var : &mut Value) -> Result<Option<Value>, String>
{
    if let Some(value) = value
    {
        *var = value;
        Ok(None)
    }
    else
    {
        Ok(Some(var.clone()))
    }
}

fn assign_or_return_indexed(value : Option<Value>, var : &mut Value, indexes : &[Value]) -> Result<Option<Value>, String>
{
    if let (Some(index), Some(new_indexes)) = (indexes.get(0), indexes.get(1..))
    {
        match var
        {
            Value::Array(ref mut var) =>
            {
                let indexnum = match_or_err!(index, Value::Number(indexnum) => indexnum, minierr("error: tried to use a non-number as an array index"))?;
                
                let mut newvar = var.get_mut(indexnum.round() as usize).ok_or_else(|| format!("error: tried to access non-extant index {} of an array", indexnum))?;
                assign_or_return_indexed(value, &mut newvar, new_indexes)
            }
            Value::Dict(ref mut var) =>
            {
                if let Value::Number(indexnum) = index
                {
                    let mut newvar = var.get_mut(&HashableValue::Number(*indexnum)).ok_or_else(|| format!("error: tried to access non-extant index {} of a dict", indexnum))?;
                    assign_or_return_indexed(value, &mut newvar, new_indexes)
                }
                else if let Value::Text(indexstr) = index
                {
                    let mut newvar = var.get_mut(&HashableValue::Text(indexstr.clone())).ok_or_else(|| format!("error: tried to access non-extant index {} of a dict", indexstr))?;
                    assign_or_return_indexed(value, &mut newvar, new_indexes)
                }
                else
                {
                    plainerr("error: tried to use a non-number, non-string as a dict index")
                }
            }
            Value::Text(ref mut text) =>
            {
                if !new_indexes.is_empty()
                {
                    return plainerr("error: tried to index into the value at another index in a string (i.e. tried to do something like \"asdf\"[0][0])");
                }
                
                let indexnum = match_or_err!(index, Value::Number(indexnum) => indexnum, minierr("error: tried to use a non-number as an index into a string"))?;
                
                let realindex = ((indexnum.round() as i64) % text.len() as i64) as usize;
                
                if let Some(value) = value
                {
                    let mychar = match_or_err!(value, Value::Text(mychar) => mychar, minierr("error: tried to assign non-string to an index into a string (assigning by codepoint is not supported yet)"))?;
                    
                    if mychar.chars().count() == 1
                    {
                        let mychar = mychar.chars().next().ok_or_else(|| minierr("internal error: failed to get first character of a string of length 1"))?;
                        // turn into array of codepoints, then modify
                        let mut codepoints = text.chars().collect::<Vec<char>>();
                        let codepoint = codepoints.get_mut(realindex).ok_or_else(|| minierr("error: tried to assign to a character index that was past the end of a string"))?;
                        *codepoint = mychar;
                        // turn array of codepoints back into string
                        let newstr : String = codepoints.iter().collect();
                        *text = newstr;
                        Ok(None)
                    }
                    else
                    {
                        Err(format!("error: tried to assign to an index into a string with a string that was not exactly one character long (was {} characters long)", mychar.len()))
                    }
                }
                else
                {
                    let codepoints = text.chars().collect::<Vec<char>>();
                    let codepoint = codepoints.get(realindex).ok_or_else(|| minierr("error: tried to evaluate a character from an index that was past the end of a string"))?;
                    
                    let mut newstr = String::new();
                    newstr.push(*codepoint);
                    Ok(Some(Value::Text(newstr)))
                }
            }
            _ =>
            {
                plainerr("error: tried to index into a non-array, non-dict value")
            }
        }
    }
    else
    {
        assign_or_return(value, var)
    }
}

fn check_frame_dirvar_indexed(global : &mut GlobalState, frame : &mut Frame, dirvar : &DirectVar) -> bool
{
    // FIXME: do I even want to search up instance stacks rather than just accessing the main one?
    for scope in frame.scopes.iter_mut().rev()
    {
        if scope.contains_key(&dirvar.name)
        {
            return true;
        }
    }
    for id in frame.instancestack.iter_mut().rev()
    {
        if let Some(inst) = global.instances.get_mut(id)
        {
            if inst.variables.contains_key(&dirvar.name)
            {
                return true;
            }
            // no need to check for instance function names because they can't be indexed
        }
    }
    false
}
fn access_frame_dirvar_indexed(global : &mut GlobalState, frame : &mut Frame, dirvar : &DirectVar, value : Option<Value>, indexes : &[Value]) -> Result<Option<Value>, String>
{
    // FIXME: do I even want to search up instance stacks rather than just accessing the main one?
    for scope in frame.scopes.iter_mut().rev()
    {
        if let Some(var) = scope.get_mut(&dirvar.name)
        {
            return assign_or_return_indexed(value, var, indexes);
        }
    }
    for id in frame.instancestack.iter_mut().rev()
    {
        if let Some(inst) = global.instances.get_mut(id)
        {
            if let Some(var) = inst.variables.get_mut(&dirvar.name)
            {
                return assign_or_return_indexed(value, var, indexes);
            }
            // no need to check for instance function names because they can't be indexed - it will either skip them and look for something else, or fail with a generic error
            // FIXME is this good behavior?
        }
    }
    plainerr("internal error: tried to assign (via index) to a variable that could not be found")
}
fn check_frame_dirvar(global : &mut GlobalState, frame : &mut Frame, dirvar : &DirectVar) -> bool
{
    for scope in frame.scopes.iter_mut().rev()
    {
        if scope.contains_key(&dirvar.name)
        {
            return true;
        }
    }
    // FIXME: do I even want to search up instance stacks rather than just accessing the main one?
    for id in frame.instancestack.iter_mut().rev()
    {
        if let Some(inst) = global.instances.get_mut(id)
        {
            if inst.variables.contains_key(&dirvar.name)
            {
                return true;
            }
            else if let Some(objspec) = global.objects.get(&inst.objtype)
            {
                if objspec.functions.contains_key(&dirvar.name)
                {
                    return true;
                }
            }
        }
    }
    false
}
fn access_frame_dirvar(global : &mut GlobalState, frame : &mut Frame, dirvar : &DirectVar, value : Option<Value>) -> Result<Option<Value>, String>
{
    for scope in frame.scopes.iter_mut().rev()
    {
        if let Some(var) = scope.get_mut(&dirvar.name)
        {
            return assign_or_return(value, var);
        }
    }
    // FIXME: do I even want to search up instance stacks rather than just accessing the main one?
    for id in frame.instancestack.iter_mut().rev()
    {
        if let Some(inst) = global.instances.get_mut(id)
        {
            if let Some(var) = inst.variables.get_mut(&dirvar.name)
            {
                return assign_or_return(value, var);
            }
            else if let Some(objspec) = global.objects.get(&inst.objtype)
            {
                if let Some(funcdat) = objspec.functions.get(&dirvar.name)
                {
                    if value.is_none()
                    {
                        let mut mydata = funcdat.clone();
                        mydata.forcecontext = inst.ident;
                        return Ok(Some(Value::new_funcval(false, Some(dirvar.name.clone()), None, Some(mydata))));
                        // FIXME is this good behavior?
                    }
                    else
                    {
                        return Err(format!("error: tried to assign to function `{}` in instance of object type `{}`", dirvar.name, objspec.name));
                    }
                }
            }
        }
    }
    plainerr("internal error: tried to assign to a variable that could not be found")
}
impl Interpreter
{
    fn evaluate_or_store_of_array(&mut self, arrayvar : &ArrayVar, value : Option<Value>) -> Result<Option<Value>, String>
    {
        match &arrayvar.location
        {
            NonArrayVariable::Indirect(ref indirvar) =>
            {
                match indirvar.source
                {
                    IndirectSource::Ident(ident) =>
                    {
                        let instance = self.global.instances.get_mut(&ident).ok_or_else(|| format!("error: tried to access variable `{}` from non-extant instance `{}`", indirvar.name, ident))?;
                        
                        let var = instance.variables.get_mut(&indirvar.name).ok_or_else(|| format!("error: tried to read non-extant variable `{}` in instance `{}`", indirvar.name, ident))?;
                        
                        assign_or_return_indexed(value, var, &arrayvar.indexes)
                    }
                    IndirectSource::Global =>
                    {
                        let var = self.global.variables.get_mut(&indirvar.name).ok_or_else(|| format!("error: tried to access global variable `{}` that doesn't exist", indirvar.name))?;
                        assign_or_return_indexed(value, var, &arrayvar.indexes)
                    }
                }
            }
            NonArrayVariable::Direct(ref dirvar) =>
            {
                if check_frame_dirvar_indexed(&mut self.global, &mut self.top_frame, dirvar)
                {
                    return access_frame_dirvar_indexed(&mut self.global, &mut self.top_frame, dirvar, value, &arrayvar.indexes);
                }
                if !self.top_frame.impassable
                {
                    for mut frame in self.frames.iter_mut().rev()
                    {
                        if check_frame_dirvar_indexed(&mut self.global, &mut frame, dirvar)
                        {
                            return access_frame_dirvar_indexed(&mut self.global, &mut frame, dirvar, value, &arrayvar.indexes);
                        }
                        if frame.impassable { break; }
                    }
                }
                if self.global.objectnames.contains_key(&dirvar.name)
                {
                    return plainerr("error: tried to index into object name as though it was an array");
                }
                if self.global.functions.contains_key(&dirvar.name)
                {
                    return plainerr("error: tried to index into global function name as though it was an array");
                }
                if self.get_internal_function(&dirvar.name).is_some()
                {
                    return plainerr("error: tried to index into internal function name as though it was an array");
                }
                Err(format!("error: unknown variable `{}`", dirvar.name))
            }
            NonArrayVariable::ActualArray(ref array) =>
            {
                if value.is_none()
                {
                    assign_or_return_indexed(None, &mut Value::Array(array.clone()), &arrayvar.indexes)
                }
                else
                {
                    plainerr("error: tried to assign to a non-variable array value")
                }
            }
        }
    }
    fn evaluate_or_store_of_indirect(&mut self, indirvar : &IndirectVar, value : Option<Value>) -> Result<Option<Value>, String>
    {
        match indirvar.source
        {
            IndirectSource::Ident(ident) =>
            {
                let instance = self.global.instances.get_mut(&ident).ok_or_else(|| format!("error: tried to access variable `{}` from non-extant instance `{}`", indirvar.name, ident))?;
                
                if let Some(var) = instance.variables.get_mut(&indirvar.name)
                {
                    assign_or_return(value, var)
                }
                else
                {
                    let objspec = self.global.objects.get(&instance.objtype).ok_or_else(|| format!("error: tried to read non-extant variable `{}` in instance `{}`", indirvar.name, ident))?;
                    
                    let funcdat = objspec.functions.get(&indirvar.name).ok_or_else(|| format!("error: tried to read non-extant variable `{}` in instance `{}`", indirvar.name, ident))?;
                    
                    if value.is_none()
                    {
                        let mut mydata = funcdat.clone();
                        mydata.forcecontext = ident;
                        Ok(Some(Value::new_funcval(false, Some(indirvar.name.clone()), None, Some(mydata))))
                    }
                    else
                    {
                        Err(format!("error: tried to assign to function `{}` in instance of object type `{}`", indirvar.name, objspec.name))
                    }
                }
            }
            IndirectSource::Global =>
            {
                let var = self.global.variables.get_mut(&indirvar.name).ok_or_else(|| format!("error: tried to access global variable `{}` that doesn't exist", indirvar.name))?;
                assign_or_return(value, var)
            }
        }
    }
    fn evaluate_or_store_of_direct(&mut self, dirvar : &DirectVar, value : Option<Value>) -> Result<Option<Value>, String>
    {
        if dirvar.name == "global"
        {
            if value.is_none()
            {
                return Ok(Some(Value::Special(Special::Global)));
            }
            else
            {
                return Err(minierr("error: cannot assign to variable called \"global\" (special read-only name)"));
            }
        }
        if check_frame_dirvar(&mut self.global, &mut self.top_frame, dirvar)
        {
            return access_frame_dirvar(&mut self.global, &mut self.top_frame, dirvar, value);
        }
        if !self.top_frame.impassable
        {
            for mut frame in self.frames.iter_mut().rev()
            {
                if check_frame_dirvar(&mut self.global, &mut frame, dirvar)
                {
                    return access_frame_dirvar(&mut self.global, &mut frame, dirvar, value);
                }
                if frame.impassable { break; }
            }
        }
        if let Some(var) = self.global.objectnames.get(&dirvar.name)
        {
            if value.is_none()
            {
                return Ok(Some(Value::Object(*var)));
            }
            else
            {
                return Err(format!("error: tried to assign to read-only object name `{}`", dirvar.name));
            }
        }
        if let Some(var) = self.global.functions.get(&dirvar.name)
        {
            if value.is_none()
            {
                return Ok(Some(var.clone()));
            }
            else
            {
                return Err(format!("error: tried to assign to global function `{}` (no such identifier exists in any other scope, you should declare it with 'var' to override this logic)", dirvar.name));
            }
        }
        if self.get_internal_function(&dirvar.name).is_some()
        {
            if value.is_none()
            {
                return Ok(Some(Value::new_funcval(true, Some(dirvar.name.clone()), None, None)));
            }
            else
            {
                return plainerr("error: tried to assign to internal function name");
            }
        }
        
        Err(format!("error: unknown identifier `{}`", dirvar.name))
    }
    // if value is None, finds and returns appropriate value; otherwise, stores value and returns None
    pub (crate) fn evaluate_or_store(&mut self, variable : &Variable, value : Option<Value>) -> Result<Option<Value>, String>
    {
        match &variable
        {
            Variable::Array(ref arrayvar) =>
            {
                self.evaluate_or_store_of_array(arrayvar, value)
            }
            Variable::Indirect(ref indirvar) =>
            {
                self.evaluate_or_store_of_indirect(indirvar, value)
            }
            Variable::Direct(ref dirvar) =>
            {
                self.evaluate_or_store_of_direct(dirvar, value)
            }
        }
    }
}