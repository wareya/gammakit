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
    if matches!(value, Value::SubFunc(_))
    {
        return plainerr!("error: tried to assign the result of the dismember operator (->) to a variable (you probably forgot the argument list)");
    }
    *var = value;
    Ok(())
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
            _ => Err(format!("error: tried to index into a non-array, non-dict, non-text value {:?} with index {:?}", var, index))
        }
    }
    else
    {
        assign_val(value, var)
    }
}
pub (crate) fn mutate_indexed<F : FnOnce(&mut Value) -> Result<(), String>>(mutator : F, var : &mut Value, indexes : &[HashableValue]) -> Result<(), String>
{
    if let (Some(index), Some(new_indexes)) = (indexes.get(0), indexes.get(1..))
    {
        match var
        {
            Value::Array(ref mut var) =>
            {
                let indexnum = match_or_err!(index, HashableValue::Number(indexnum) => indexnum, minierr("error: tried to use a non-number as an array index"))?;
                
                let mut newvar = var.get_mut(indexnum.round() as usize).ok_or_else(|| format!("error: tried to access non-extant index {} of an array", indexnum))?;
                mutate_indexed(mutator, &mut newvar, new_indexes)
            }
            Value::Dict(ref mut var) =>
            {
                let mut newvar = var.get_mut(index).ok_or_else(|| format!("error: tried to access non-extant index {:?} of a dict", index))?;
                mutate_indexed(mutator, &mut newvar, new_indexes)
            }
            Value::Text(ref mut text) =>
            {
                if !new_indexes.is_empty()
                {
                    return plainerr!("error: tried to index into the value at another index in a string (i.e. tried to do something like \"asdf\"[0][0])");
                }
                
                let indexnum = match_or_err!(index, HashableValue::Number(indexnum) => indexnum, minierr("error: tried to use a non-number as an index into a string"))?;
                
                let realindex = ((indexnum.round() as i64) % text.len() as i64) as usize;
                let mut codepoints = text.chars().collect::<Vec<char>>();
                let codepoint = codepoints.get_mut(realindex).ok_or_else(|| minierr("error: tried to assign to a character index that was past the end of a string"))?;
                let mut mutatee = Value::Text(vec!(*codepoint).iter().collect());
                mutator(&mut mutatee)?;
                
                let mutatee = match_or_err!(mutatee, Value::Text(mychar) => mychar, minierr("error: tried to assign non-string to an index into a string (assigning by codepoint is not supported yet)"))?;
                
                if mutatee.chars().count() == 1
                {
                    let mutatee = mutatee.chars().next().ok_or_else(|| minierr("internal error: failed to get first character of a string of length 1"))?;
                    // turn into array of codepoints, then modify
                    *codepoint = mutatee;
                    // turn array of codepoints back into string
                    let newstr : String = codepoints.iter().collect();
                    *text = newstr;
                    Ok(())
                }
                else
                {
                    Err(format!("error: tried to assign to an index into a string with a string that was not exactly one character long (was {} characters long)", mutatee.len()))
                }
            }
            _ => Err(format!("error: tried to index into a non-array, non-dict, non-text value {:?} with index {:?}", var, index))
        }
    }
    else
    {
        mutator(var)?;
        if matches!(var, Value::SubFunc(_))
        {
            return plainerr!("error: tried to assign the result of the dismember operator (->) to a variable (you probably forgot the argument list)");
        }
        Ok(())
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
            _ => Err(format!("error: tried to index into a non-array, non-dict, non-text value {:?} with index {:?}", var, index))
        }
    }
    else
    {
        Ok(var.clone())
    }
}

fn access_frame<'a, 'b>(global : &'a GlobalState, frame : &'b Frame, name : usize, seen_instance : &mut bool) -> Option<Result<Result<&'b ValRef, &'a ValRef>, Value>>
{
    for scope in frame.scopes.iter().rev().filter(|x| !x.is_empty())
    {
        if let Some(var) = scope.get(&name)
        {
            return Some(Ok(Ok(var)));
        }
    }
    if !*seen_instance
    {
        if let Some(id) = frame.instancestack.last()
        {
            *seen_instance = true;
            if let Some(inst) = global.instances.get(id)
            {
                if let Some(var) = inst.variables.get(&name)
                {
                    return Some(Ok(Err(var)));
                }
                else if let Some(objspec) = global.objects.get(&inst.objtype)
                {
                    if let Some(funcdat) = objspec.functions.get(&name)
                    {
                        let mut mydata = funcdat.clone();
                        mydata.forcecontext = inst.ident;
                        return Some(Err(Value::new_funcval(false, Some(name), None, Some(mydata))));
                    }
                }
            }
        }
    }
    None
}

impl Interpreter
{
    fn evaluate_of_array
        <T,
         F : FnOnce(&ValRef) -> Result<T, String>,
         F2 : FnOnce(Value) -> Result<T, String>
        >
        (&self, arrayvar : ArrayVar, varhandler : F, valhandler : F2) -> Result<T, String>
    {
        match arrayvar.location
        {
            NonArrayVariable::Indirect(indirvar) => varhandler(&ValRef::from_ref(self.evaluate_of_indirect(indirvar, |x| Ok(x.refclone()), |x| Ok(ValRef::from_val(x)))?.extract_ref()?, arrayvar.indexes, false)),
            NonArrayVariable::Direct(dirvar) => varhandler(&ValRef::from_ref(self.evaluate_of_direct(dirvar, |x| Ok(x.refclone()), |x| Ok(ValRef::from_val(x)))?.extract_ref()?, arrayvar.indexes, false)),
            NonArrayVariable::Global(globalvar) => varhandler(&ValRef::from_ref(self.evaluate_of_global(globalvar, |x| Ok(x.refclone()))?.extract_ref()?, arrayvar.indexes, false)),
            NonArrayVariable::ActualArray(array) => valhandler(return_indexed(&Value::Array(array), &arrayvar.indexes)?),
            NonArrayVariable::ActualDict(dict) => valhandler(return_indexed(&Value::Dict(dict), &arrayvar.indexes)?),
            NonArrayVariable::ActualText(string) => valhandler(return_indexed(&Value::Text(*string), &arrayvar.indexes)?),
        }
    }
    fn evaluate_of_indirect
        <T,
         F : FnOnce(&ValRef) -> Result<T, String>,
         F2 : FnOnce(Value) -> Result<T, String>
        >
        (&self, indirvar : IndirectVar, varhandler : F, valhandler : F2) -> Result<T, String>
    {
        let ident = indirvar.ident;
        let instance = self.global.instances.get(&ident).ok_or_else(|| format!("error: tried to access variable `{}` from non-extant instance `{}`", self.get_indexed_string(indirvar.name), ident))?;
        
        if let Some(var) = instance.variables.get(&indirvar.name)
        {
            varhandler(var)
        }
        else
        {
            let objspec = self.global.objects.get(&instance.objtype).ok_or_else(|| format!("error: tried to read non-extant variable `{}` in instance `{}`", self.get_indexed_string(indirvar.name), ident))?;
            
            let funcdat = objspec.functions.get(&indirvar.name).ok_or_else(|| format!("error: tried to read non-extant variable `{}` in instance `{}`", self.get_indexed_string(indirvar.name), ident))?;
            
            let mut mydata = funcdat.clone();
            mydata.forcecontext = ident;
            valhandler(Value::new_funcval(false, Some(indirvar.name), None, Some(mydata)))
        }
    }
    fn evaluate_of_global
        <T,
         F : FnOnce(&ValRef) -> Result<T, String>
        >
        (&self, globalvar : usize, varhandler : F) -> Result<T, String>
    {
        let var = self.global.variables.get(&globalvar).ok_or_else(|| format!("error: tried to access global variable `{}` that doesn't exist", self.get_indexed_string(globalvar)))?;
        varhandler(var)
    }
    pub(crate) fn evaluate_of_direct
        <T,
         F : FnOnce(&ValRef) -> Result<T, String>,
         F2 : FnOnce(Value) -> Result<T, String>
        >
        (&self, name : usize, varhandler : F, valhandler : F2) -> Result<T, String>
    {
        let mut seen_instance = false;
        if let Some(my_ref) = access_frame(&self.global, &self.top_frame, name, &mut seen_instance)
        {
            return match my_ref
            {
                Ok(Ok(scopedvar)) => varhandler(scopedvar),
                Ok(Err(globalvar)) => varhandler(globalvar),
                Err(globalval) => valhandler(globalval),
            };
        }
        if !self.top_frame.impassable
        {
            for frame in self.frames.iter().rev()
            {
                if let Some(my_ref) = access_frame(&self.global, &frame, name, &mut seen_instance)
                {
                    return match my_ref
                    {
                        Ok(Ok(scopedvar)) => varhandler(scopedvar),
                        Ok(Err(globalvar)) => varhandler(globalvar),
                        Err(globalval) => valhandler(globalval),
                    };
                }
                if frame.impassable { break; }
            }
        }
        
        if let Some(var) = self.global.barevariables.get(&name)
        {
            return varhandler(var);
        }
        if let Some(var) = self.global.objectnames.get(&name)
        {
            return valhandler(Value::Object(*var));
        }
        if let Some(var) = self.global.functions.get(&name)
        {
            return valhandler(var.clone());
        }
        if self.get_binding(name).is_some() || self.get_simple_binding(name).is_some()
        {
            return valhandler(Value::new_funcval(true, Some(name), None, None));
        }
        
        Err(format!("error: unknown identifier `{}`", self.get_indexed_string(name)))
    }
    #[inline]
    pub (crate) fn evaluate_self
        <T,
         F2 : FnOnce(Value) -> Result<T, String>
        >
        (&self, valhandler : F2) -> Result<T, String>
    {
        let id = self.top_frame.instancestack.last().ok_or_else(|| "error: tried to access `self` while not inside of instance scope".to_string())?;
        valhandler(Value::Instance(*id))
    }
    #[inline]
    pub (crate) fn evaluate_other
        <T,
         F2 : FnOnce(Value) -> Result<T, String>
        >
        (&self, valhandler : F2) -> Result<T, String>
    {
        let id = self.top_frame.instancestack.get(self.top_frame.instancestack.len()-2).ok_or_else(|| "error: tried to access `other` while not inside of at least two instance scopes".to_string())?;
        valhandler(Value::Instance(*id))
    }
    pub (crate) fn evaluate
        <T,
         F : FnOnce(&ValRef) -> Result<T, String>,
         F2 : FnOnce(Value) -> Result<T, String>
        >
        (&self, variable : Variable, varhandler : F, valhandler : F2) -> Result<T, String>
    {
        match variable
        {
            Variable::Array(arrayvar) => self.evaluate_of_array(arrayvar, varhandler, valhandler),
            Variable::Indirect(indirvar) => self.evaluate_of_indirect(indirvar, varhandler, valhandler),
            Variable::Global(globalvar) => self.evaluate_of_global(globalvar, varhandler),
            Variable::Direct(name) => self.evaluate_of_direct(name, varhandler, valhandler),
            Variable::Selfref => self.evaluate_self(valhandler),
            Variable::Other => self.evaluate_other(valhandler),
        }
    }
    pub (crate) fn evaluate_value(&self, variable : Variable) -> Result<Value, String>
    {
        self.evaluate(variable, |x| x.to_val(), |x| Ok(x))
    }
    pub (crate) fn evaluate_and_mutate<F : FnOnce(&mut Value) -> Result<(), String>>(&self, variable : Variable, mutator : F) -> Result<(), String>
    {
        self.evaluate(variable, |x| x.mutate(mutator), |_| Err("error: tried to mutate a literal value".to_string()))
    }
}
