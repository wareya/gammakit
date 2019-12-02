use crate::interpreter::*;

#[derive(Debug)]
pub enum ValueLoc<'a> {
    Static(Value),
    Immut(&'a Value),
    Mut(&'a mut Value)
}

impl<'a> ValueLoc<'a> {
    pub fn as_val(&self) -> Value
    {
        match self
        {
            ValueLoc::Static(v) => v.clone(),
            ValueLoc::Immut(v) => (*v).clone(),
            ValueLoc::Mut(v) => (*v).clone(),
        }
    }
    #[allow(clippy::wrong_self_convention)]
    pub fn to_val(self) -> Value
    {
        match self
        {
            ValueLoc::Static(v) => v,
            ValueLoc::Immut(v) => (*v).clone(),
            ValueLoc::Mut(v) => (*v).clone(),
        }
    }
    pub fn as_ref(&'a self) -> &'a Value
    {
        match self
        {
            ValueLoc::Static(v) => &v,
            ValueLoc::Immut(v) => v,
            ValueLoc::Mut(v) => v,
        }
    }
    pub fn as_mut(&mut self) -> Result<&mut Value, String>
    {
        match self
        {
            ValueLoc::Static(_) | ValueLoc::Immut(_) => plainerr("error: tried to assign to a read-only value"),
            ValueLoc::Mut(v) => Ok(*v),
        }
    }
    pub fn assign(&mut self, newval : Value) -> Result<(), String>
    {
        match self
        {
            ValueLoc::Static(_) | ValueLoc::Immut(_) => plainerr("error: tried to assign to a read-only value"),
            ValueLoc::Mut(v) => (**v = newval, Ok(())).1,
        }
    }
}

pub (crate) fn return_indexed<'a>(var : ValueLoc<'a>, indexes : &[HashableValue]) -> Result<ValueLoc<'a>, String>
{
    if indexes.is_empty()
    {
        return Ok(var);
    }
    let (index, new_indexes) = (&indexes[0], &indexes[1..]);
    match var
    {
        ValueLoc::<'a>::Mut(Value::Array(var)) =>
        {
            let indexnum = match_or_err!(index, HashableValue::Number(indexnum) => indexnum, minierr("error: tried to use a non-number as an array index"))?.round() as usize;
            
            let newvar = ValueLoc::Mut(var.get_mut(indexnum).ok_or_else(|| format!("error: tried to access non-extant index {} of an array", indexnum))?);
            return_indexed(newvar, new_indexes)
        }
        ValueLoc::<'a>::Immut(Value::Array(var)) =>
        {
            let indexnum = match_or_err!(index, HashableValue::Number(indexnum) => indexnum, minierr("error: tried to use a non-number as an array index"))?.round() as usize;
            
            let newvar = ValueLoc::Immut(var.get(indexnum).ok_or_else(|| format!("error: tried to access non-extant index {} of an array", indexnum))?);
            return_indexed(newvar, new_indexes)
        }
        ValueLoc::Static(Value::Array(mut var)) =>
        {
            let indexnum = match_or_err!(index, HashableValue::Number(indexnum) => indexnum, minierr("error: tried to use a non-number as an array index"))?.round() as usize;
            
            if indexnum >= var.len()
            {
                return Err(format!("error: tried to access non-extant index {} of an array", indexnum));
            }
            let newvar = ValueLoc::Static(var.swap_remove(indexnum));
            return_indexed(newvar, new_indexes)
        }
        ValueLoc::<'a>::Mut(Value::Dict(var)) =>
        {
            let newvar = ValueLoc::Mut(var.get_mut(index).ok_or_else(|| format!("error: tried to access non-extant index {:?} of a dict", index))?);
            return_indexed(newvar, new_indexes)
        }
        ValueLoc::<'a>::Immut(Value::Dict(var)) =>
        {
            let newvar = ValueLoc::Immut(var.get(index).ok_or_else(|| format!("error: tried to access non-extant index {:?} of a dict", index))?);
            return_indexed(newvar, new_indexes)
        }
        ValueLoc::Static(Value::Dict(mut var)) =>
        {
            let newvar = ValueLoc::Static(var.remove(index).ok_or_else(|| format!("error: tried to access non-extant index {:?} of a dict", index))?);
            return_indexed(newvar, new_indexes)
        }
        
        ValueLoc::<'a>::Mut(Value::Text(string)) =>
        {
            let indexnum = match_or_err!(index, HashableValue::Number(indexnum) => indexnum, minierr("error: tried to use a non-number as a string index"))?.round() as usize;
            
            if !new_indexes.is_empty()
            {
                return plainerr("error: tried to consecutively index into a string more than once (e.g. \"asdf\"[1][1])");
            }
            let newvar = ValueLoc::Static(Value::Text([string.chars().nth(indexnum).ok_or_else(|| format!("error: tried to access non-extant index {} of a string", indexnum))?].iter().collect()));
            Ok(newvar)
        }
        ValueLoc::<'a>::Immut(Value::Text(string)) =>
        {
            let indexnum = match_or_err!(index, HashableValue::Number(indexnum) => indexnum, minierr("error: tried to use a non-number as a string index"))?.round() as usize;
            
            if !new_indexes.is_empty()
            {
                return plainerr("error: tried to consecutively index into a string more than once (e.g. \"asdf\"[1][1])");
            }
            let newvar = ValueLoc::Static(Value::Text([string.chars().nth(indexnum).ok_or_else(|| format!("error: tried to access non-extant index {} of a string", indexnum))?].iter().collect()));
            Ok(newvar)
        }
        ValueLoc::Static(Value::Text(string)) =>
        {
            let indexnum = match_or_err!(index, HashableValue::Number(indexnum) => indexnum, minierr("error: tried to use a non-number as a string index"))?.round() as usize;
            
            if !new_indexes.is_empty()
            {
                return plainerr("error: tried to consecutively index into a string more than once (e.g. \"asdf\"[1][1])");
            }
            let newvar = ValueLoc::Static(Value::Text([string.chars().nth(indexnum).ok_or_else(|| format!("error: tried to access non-extant index {} of a string", indexnum))?].iter().collect()));
            Ok(newvar)
        }
        // TODO reintroduce string support
        _ => Err(format!("error: tried to index into a non-array, non-dict value {:?} with index {:?}", var, index))
    }
}

impl Interpreter
{
    pub(crate) fn evaluate_of_array(&mut self, arrayvar : ArrayVar) -> Result<ValueLoc<'_>, String>
    {
        match arrayvar.location
        {
            // FIXME borrow readonlyness 
            NonArrayVariable::Indirect(indirvar) =>
                return_indexed(self.evaluate_of_indirect(indirvar)?, &arrayvar.indexes),
            NonArrayVariable::Direct(dirvar) =>
                return_indexed(self.evaluate_of_direct(dirvar)?, &arrayvar.indexes),
            NonArrayVariable::Global(globalvar) =>
                return_indexed(self.evaluate_of_global(globalvar)?, &arrayvar.indexes),
            NonArrayVariable::ActualArray(array) =>
                return_indexed(ValueLoc::Static(Value::Array(array)), &arrayvar.indexes),
            NonArrayVariable::ActualDict(dict) =>
                return_indexed(ValueLoc::Static(Value::Dict(dict)), &arrayvar.indexes),
            NonArrayVariable::ActualText(string) =>
                return_indexed(ValueLoc::Static(Value::Text(*string)), &arrayvar.indexes),
        }
    }
    pub(crate) fn evaluate_of_indirect_simple(&self, ident : usize, name : usize) -> Result<Value, String>
    {
        if !self.global.instances.contains_key(&ident)
        {
            return Err(format!("error: tried to access variable `{}` from non-extant instance `{}`", self.get_indexed_string(name), ident));
        }
        let instance = self.global.instances.get(&ident).unwrap();
        
        if let Some(var) = instance.variables.get(&name)
        {
            Ok(var.clone())
        }
        else
        {
            // fallback to instance functions
            let objspec = self.global.objects.get(&instance.objtype).ok_or_else(|| "internal error: tried to access non-extant object type".to_string())?;
            
            let funcdat = objspec.functions.get(&name).ok_or_else(|| format!("error: tried to read non-extant variable `{}` in instance `{}`", self.get_indexed_string(name), ident))?;
            
            let mut mydata = funcdat.clone();
            mydata.forcecontext = ident;
            Ok(Value::new_funcval(None, mydata))
        }
    }
    pub(crate) fn evaluate_of_indirect(&mut self, indirvar : IndirectVar) -> Result<ValueLoc<'_>, String>
    {
        let ident = indirvar.ident;
        if !self.global.instances.contains_key(&ident)
        {
            return Err(format!("error: tried to access variable `{}` from non-extant instance `{}`", self.get_indexed_string(indirvar.name), ident));
        }
        let instance = self.global.instances.get_mut(&ident).unwrap();
        
        if let Some(var) = instance.variables.get_mut(&indirvar.name)
        {
            Ok(ValueLoc::Mut(var))
        }
        else
        {
            // fallback to instance functions
            let objspec = self.global.objects.get(&instance.objtype).ok_or_else(|| "internal error: tried to access non-extant object type".to_string())?;
            
            let funcdat = objspec.functions.get(&indirvar.name).ok_or_else(|| "error: tried to read non-extant instance variable".to_string())?;
            
            let mut mydata = funcdat.clone();
            mydata.forcecontext = ident;
            Ok(ValueLoc::Static(Value::new_funcval(None, mydata)))
        }
    }
    pub(crate) fn evaluate_of_global(&mut self, globalvar : usize) -> Result<ValueLoc<'_>, String>
    {
        if !self.global.variables.contains_key(&globalvar)
        {
            return Err(format!("error: tried to access global variable `{}` that doesn't exist", self.get_indexed_string(globalvar)));
        }
        Ok(ValueLoc::Mut(self.global.variables.get_mut(&globalvar).unwrap()))
    }
    pub(crate) fn evaluate_of_bareglobal(&mut self, bareglobalvar : usize) -> Result<ValueLoc<'_>, String>
    {
        Ok(ValueLoc::Immut(self.global.barevariables.get(&bareglobalvar).ok_or_else(|| format!("internal error: tried to access bare global variable `{}` that doesn't exist", self.get_indexed_string(bareglobalvar)))?))
    }
    pub(crate) fn evaluate_of_direct(&mut self, index : usize) -> Result<ValueLoc<'_>, String>
    {
        Ok(ValueLoc::Mut(self.top_frame.variables.get_mut(index).ok_or_else(|| "internal error: variable stack out-of-bounds access".to_string())?))
    }
    pub (crate) fn evaluate(&mut self, variable : Variable) -> Result<ValueLoc<'_>, String>
    {
        match variable
        {
            Variable::Array(arrayvar) => self.evaluate_of_array(arrayvar),
            Variable::Indirect(indirvar) => self.evaluate_of_indirect(indirvar),
            Variable::Global(globalvar) => self.evaluate_of_global(globalvar),
            Variable::BareGlobal(bareglobalvar) => self.evaluate_of_bareglobal(bareglobalvar),
            Variable::Direct(name) => self.evaluate_of_direct(name),
        }
    }
    pub (crate) fn evaluate_value(&mut self, variable : Variable) -> Result<Value, String>
    {
        self.evaluate(variable).map(|x| x.to_val())
    }
}
