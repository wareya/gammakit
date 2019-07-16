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

macro_rules! plainerr { ( $x:expr ) => { Err($x.to_string()) } }

fn assign_val(value : Value, var : &mut Value) -> Result<(), String>
{
    match value
    {
        Value::Special(_) => plainerr!("error: tried to assign a special value to a variable"),
        Value::SubFunc(_) => plainerr!("error: tried to assign the result of the dismember operator (->) to a variable (you probably forgot the argument list)"),
        value =>
        {
            *var = value;
            Ok(())
        }
    }
}

fn assign_or_return_valref(value : Option<Value>, var : ValRef) -> Result<Option<Value>, String>
{
    match value
    {
        Some(value) =>
        {
            var.assign(value)?;
            Ok(None)
        }
        _ => var.to_val().map(|x| Some(x))
    }
}

pub (crate) fn assign_indexed(value : Value, var : &mut Value, indexes : &[HashableValue]) -> Result<(), String>
{
    if let (Some(index), Some(new_indexes)) = (indexes.get(0), indexes.get(1..))
    {
        match var
        {
            Value::Array(ref mut var) =>
            {
                let indexnum = match_or_err!(index, HashableValue::Number(indexnum) => indexnum, minierr("error: tried to use a non-number as an array index"))?;
                
                let mut newvar = var.get_mut(indexnum.round() as usize).ok_or_else(|| format!("error: tried to access non-extant index {} of an array", indexnum))?;
                assign_indexed(value, &mut newvar, new_indexes)
            }
            Value::Dict(ref mut var) =>
            {
                let mut newvar = var.get_mut(index).ok_or_else(|| format!("error: tried to access non-extant index {:?} of a dict", index))?;
                assign_indexed(value, &mut newvar, new_indexes)
            }
            Value::Text(ref mut text) =>
            {
                if !new_indexes.is_empty()
                {
                    return plainerr!("error: tried to index into the value at another index in a string (i.e. tried to do something like \"asdf\"[0][0])");
                }
                
                let indexnum = match_or_err!(index, HashableValue::Number(indexnum) => indexnum, minierr("error: tried to use a non-number as an index into a string"))?;
                
                let realindex = ((indexnum.round() as i64) % text.len() as i64) as usize;
                
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
                    Ok(())
                }
                else
                {
                    Err(format!("error: tried to assign to an index into a string with a string that was not exactly one character long (was {} characters long)", mychar.len()))
                }
            }
            _ => plainerr!("error: tried to index into a non-array, non-dict value")
        }
    }
    else
    {
        assign_val(value, var)
    }
}
pub (crate) fn return_indexed(var : &Value, indexes : &[HashableValue]) -> Result<Value, String>
{
    if let (Some(index), Some(new_indexes)) = (indexes.get(0), indexes.get(1..))
    {
        match var
        {
            Value::Array(ref var) =>
            {
                let indexnum = match_or_err!(index, HashableValue::Number(indexnum) => indexnum, minierr("error: tried to use a non-number as an array index"))?;
                
                let newvar = var.get(indexnum.round() as usize).ok_or_else(|| format!("error: tried to access non-extant index {} of an array", indexnum))?;
                return_indexed(&newvar, new_indexes)
            }
            Value::Dict(ref var) =>
            {
                let newvar = var.get(index).ok_or_else(|| format!("error: tried to access non-extant index {:?} of a dict", index))?;
                return_indexed(&newvar, new_indexes)
            }
            Value::Text(ref text) =>
            {
                if !new_indexes.is_empty()
                {
                    return plainerr!("error: tried to index into the value at another index in a string (i.e. tried to do something like \"asdf\"[0][0])");
                }
                
                let indexnum = match_or_err!(index, HashableValue::Number(indexnum) => indexnum, minierr("error: tried to use a non-number as an index into a string"))?;
                
                let realindex = ((indexnum.round() as i64) % text.len() as i64) as usize;
                
                let codepoints = text.chars().collect::<Vec<char>>();
                let codepoint = codepoints.get(realindex).ok_or_else(|| minierr("error: tried to evaluate a character from an index that was past the end of a string"))?;
                
                let mut newstr = String::new();
                newstr.push(*codepoint);
                Ok(Value::Text(newstr))
            }
            _ => plainerr!("error: tried to index into a non-array, non-dict value")
        }
    }
    else
    {
        Ok(var.clone())
    }
}

fn access_frame(global : &mut GlobalState, frame : &mut Frame, dirvar : &DirectVar, seen_instance : &mut bool) -> Option<ValRef>
{
    for scope in frame.scopes.iter_mut().rev()
    {
        if let Some(var) = scope.get(&dirvar.name)
        {
            return Some(var.refclone());
        }
    }
    if !*seen_instance
    {
        if let Some(id) = frame.instancestack.last()
        {
            *seen_instance = true;
            if let Some(inst) = global.instances.get(id)
            {
                if let Some(var) = inst.variables.get(&dirvar.name)
                {
                    return Some(var.refclone());
                }
                else if let Some(objspec) = global.objects.get(&inst.objtype)
                {
                    if let Some(funcdat) = objspec.functions.get(&dirvar.name)
                    {
                        let mut mydata = funcdat.clone();
                        mydata.forcecontext = inst.ident;
                        return Some(ValRef::from_val(Value::new_funcval(false, Some(dirvar.name.clone()), None, Some(mydata))));
                    }
                }
            }
        }
    }
    None
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
                        if let Some(value) = value
                        {
                            let instance = self.global.instances.get_mut(&ident).ok_or_else(|| format!("error: tried to access variable `{}` from non-extant instance `{}`", indirvar.name, ident))?;
                            let var = instance.variables.get_mut(&indirvar.name).ok_or_else(|| format!("error: tried to read non-extant variable `{}` in instance `{}`", indirvar.name, ident))?;
                            assign_indexed(value, &mut var.borrow_mut(), &arrayvar.indexes)?;
                            Ok(None)
                        }
                        else
                        {
                            let instance = self.global.instances.get(&ident).ok_or_else(|| format!("error: tried to access variable `{}` from non-extant instance `{}`", indirvar.name, ident))?;
                            let var = instance.variables.get(&indirvar.name).ok_or_else(|| format!("error: tried to read non-extant variable `{}` in instance `{}`", indirvar.name, ident))?;
                            Ok(Some(return_indexed(&var.borrow(), &arrayvar.indexes)?))
                        }
                    }
                    IndirectSource::Global =>
                    {
                        if let Some(value) = value
                        {
                            let var = self.global.variables.get_mut(&indirvar.name).ok_or_else(|| format!("error: tried to access global variable `{}` that doesn't exist", indirvar.name))?;
                            assign_indexed(value, &mut var.borrow_mut(), &arrayvar.indexes)?;
                            Ok(None)
                        }
                        else
                        {
                            let var = self.global.variables.get(&indirvar.name).ok_or_else(|| format!("error: tried to access global variable `{}` that doesn't exist", indirvar.name))?;
                            Ok(Some(return_indexed(&var.borrow(), &arrayvar.indexes)?))
                        }
                    }
                }
            }
            NonArrayVariable::Direct(ref dirvar) =>
            {
                let mut seen_instance = false;
                let mut my_ref = access_frame(&mut self.global, &mut self.top_frame, dirvar, &mut seen_instance);
                if my_ref.is_none() && !self.top_frame.impassable
                {
                    for mut frame in self.frames.iter_mut().rev()
                    {
                        my_ref = access_frame(&mut self.global, &mut frame, dirvar, &mut seen_instance);
                        if frame.impassable || my_ref.is_some()
                        {
                            break;
                        }
                    }
                }
                if self.global.objectnames.contains_key(&dirvar.name)
                {
                    return plainerr!("error: tried to index into object name as though it was an array");
                }
                if self.global.functions.contains_key(&dirvar.name)
                {
                    return plainerr!("error: tried to index into global function name as though it was an array");
                }
                if self.get_binding(&dirvar.name).is_some() || self.get_simple_binding(&dirvar.name).is_some()
                {
                    return plainerr!("error: tried to index into internal function name as though it was an array");
                }
                match my_ref
                {
                    Some(my_ref) =>
                    {
                        if let Some(value) = value
                        {
                            assign_indexed(value, &mut my_ref.borrow_mut(), &arrayvar.indexes)?;
                            Ok(None)
                        }
                        else
                        {
                            Ok(Some(return_indexed(&my_ref.borrow(), &arrayvar.indexes)?))
                        }
                    }
                    None => Err(format!("error: unknown variable `{}`", dirvar.name))
                }
            }
            NonArrayVariable::ActualArray(array) =>
            {
                if value.is_none()
                {
                    return Ok(Some(return_indexed(&Value::Array(array.clone()), &arrayvar.indexes)?));
                }
                plainerr!("error: tried to assign to an index of a non-variable array value")
            }
            NonArrayVariable::ActualDict(dict) =>
            {
                if value.is_none()
                {
                    return Ok(Some(return_indexed(&Value::Dict(dict.clone()), &arrayvar.indexes)?));
                }
                plainerr!("error: tried to assign to an index of a non-variable dict value")
            }
            NonArrayVariable::ActualText(string) =>
            {
                if value.is_none()
                {
                    return Ok(Some(return_indexed(&Value::Text(string.clone()), &arrayvar.indexes)?));
                }
                plainerr!("error: tried to assign to an index of a non-variable string value")
            }
        }
    }
    fn evaluate_or_store_of_indirect(&mut self, indirvar : &IndirectVar, value : Option<Value>) -> Result<Option<Value>, String>
    {
        match indirvar.source
        {
            IndirectSource::Ident(ident) =>
            {
                let instance = self.global.instances.get(&ident).ok_or_else(|| format!("error: tried to access variable `{}` from non-extant instance `{}`", indirvar.name, ident))?;
                
                if let Some(var) = instance.variables.get(&indirvar.name)
                {
                    assign_or_return_valref(value, var.refclone())
                }
                else
                {
                    let objspec = self.global.objects.get(&instance.objtype).ok_or_else(|| format!("error: tried to read non-extant variable `{}` in instance `{}`", indirvar.name, ident))?;
                    
                    let funcdat = objspec.functions.get(&indirvar.name).ok_or_else(|| format!("error: tried to read non-extant variable `{}` in instance `{}`", indirvar.name, ident))?;
                    
                    if value.is_none()
                    {
                        let mut mydata = funcdat.clone();
                        mydata.forcecontext = ident;
                        return Ok(Some(Value::new_funcval(false, Some(indirvar.name.clone()), None, Some(mydata))));
                    }
                    Err(format!("error: tried to assign to function `{}` in instance of object type `{}`", indirvar.name, objspec.name))
                }
            }
            IndirectSource::Global =>
            {
                let var = self.global.variables.get(&indirvar.name).ok_or_else(|| format!("error: tried to access global variable `{}` that doesn't exist", indirvar.name))?;
                assign_or_return_valref(value, var.refclone())
            }
        }
    }
    fn evaluate_or_store_of_direct(&mut self, dirvar : &DirectVar, value : Option<Value>) -> Result<Option<Value>, String>
    {
        let mut seen_instance = false;
        if let Some(my_ref) = access_frame(&mut self.global, &mut self.top_frame, dirvar, &mut seen_instance)
        {
            return assign_or_return_valref(value, my_ref);
        }
        if !self.top_frame.impassable
        {
            for mut frame in self.frames.iter_mut().rev()
            {
                if let Some(my_ref) = access_frame(&mut self.global, &mut frame, dirvar, &mut seen_instance)
                {
                    return assign_or_return_valref(value, my_ref);
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
            return Err(format!("error: tried to assign to read-only object name `{}`", dirvar.name));
        }
        if let Some(var) = self.global.functions.get(&dirvar.name)
        {
            if value.is_none()
            {
                return Ok(Some(var.clone()));
            }
            return Err(format!("error: tried to assign to global function `{}` (no such identifier exists in any other scope, you should declare it with 'var' to override this logic)", dirvar.name));
        }
        if self.get_binding(&dirvar.name).is_some() || self.get_simple_binding(&dirvar.name).is_some()
        {
            if value.is_none()
            {
                return Ok(Some(Value::new_funcval(true, Some(dirvar.name.clone()), None, None)));
            }
            return plainerr!("error: tried to assign to internal function name");
        }
        
        Err(format!("error: unknown identifier `{}`", dirvar.name))
    }
    pub (crate) fn evaluate_self(&mut self, value : Option<Value>) -> Result<Option<Value>, String>
    {
        if value.is_some()
        {
            return plainerr!("error: cannot assign to variable called \"self\" (special read-only name)");
        }
        if let Some(id) = self.top_frame.instancestack.last()
        {
            return Ok(Some(Value::Instance(*id)));
        }
        return plainerr!("error: tried to access `self` while not inside of instance scope");
    }
    pub (crate) fn evaluate_global(&mut self, value : Option<Value>) -> Result<Option<Value>, String>
    {
        if value.is_some()
        {
            return plainerr!("error: cannot assign to variable called \"global\" (special read-only name)");
        }
        return Ok(Some(Value::Special(Special::Global)));
    }
    pub (crate) fn evaluate_other(&mut self, value : Option<Value>) -> Result<Option<Value>, String>
    {
        if value.is_some()
        {
            return plainerr!("error: cannot assign to variable called \"other\" (special read-only name)");
        }
        if let Some(id) = self.top_frame.instancestack.get(self.top_frame.instancestack.len()-2)
        {
            return Ok(Some(Value::Instance(*id)));
        }
        return plainerr!("error: tried to access `other` while not inside of at least two instance scopes");
    }
    // if value is None, finds and returns appropriate value; otherwise, stores value and returns None
    pub (crate) fn evaluate_or_store(&mut self, variable : &Variable, value : Option<Value>) -> Result<Option<Value>, String>
    {
        let ret = match &variable
        {
            Variable::Array(ref arrayvar) => self.evaluate_or_store_of_array(arrayvar, value),
            Variable::Indirect(ref indirvar) => self.evaluate_or_store_of_indirect(indirvar, value),
            Variable::Direct(ref dirvar) => self.evaluate_or_store_of_direct(dirvar, value),
            Variable::Selfref => self.evaluate_self(value),
            Variable::Global => self.evaluate_global(value),
            Variable::Other => self.evaluate_other(value),
        };
        return ret;
    }
}