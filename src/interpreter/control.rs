#![allow(clippy::suspicious_else_formatting)]

use crate::interpreter::*;

impl Interpreter {
    fn handle_while_flow(&mut self, data : &WhileData, put_controller_back : &mut bool) -> Result<(), String>
    {
        // if we are at the end of the expression, test it, jump outside of the loop if it's false
        if self.get_pc() == data.loop_start
        {
            let testval = self.stack_pop_val().ok_or_else(|| minierr("internal error: failed to find value on stack while handling WHILE controller"))?;
            if !value_truthy(&testval)
            {
                self.set_pc(data.loop_end);
                self.drain_scopes(data.scopes);
                *put_controller_back = false;
            }
        }
        // if we are at the end of the loop, go back to the expression
        else if self.get_pc() == data.loop_end
        {
            self.set_pc(data.expr_start);
            self.drain_scopes(data.scopes);
        }
        Ok(())
    }
    fn handle_ifelse_flow(&mut self, data : &IfElseData, put_controller_back : &mut bool) -> Result<(), String>
    {
        // if we are at the end of the expression, test it, jump to the "else" block if it's false
        if self.get_pc() == data.expr_end
        {
            let testval = self.stack_pop_val().ok_or_else(|| minierr("internal error: failed to find value on stack while handling IFELSE controller"))?;
            if !value_truthy(&testval)
            {
                self.set_pc(data.if_end);
            }
        }
        // end of the main block, jump to the end of the "else" block
        else if self.get_pc() == data.if_end
        {
            
            self.set_pc(data.else_end);
            self.drain_scopes(data.scopes);
            *put_controller_back = false;
        }
        // end of the "else" block, clean up
        else if self.get_pc() == data.else_end
        {
            self.drain_scopes(data.scopes);
            *put_controller_back = false;
        }
        Ok(())
    }
    fn handle_if_flow(&mut self, data : &IfData, put_controller_back : &mut bool) -> Result<(), String>
    {
        // if we are at the end of the expression, test it, jump past the block if it's false
        if self.get_pc() == data.expr_end
        {
            let testval = self.stack_pop_val().ok_or_else(|| minierr("internal error: failed to find value on stack while handling IF controller"))?;
            if !value_truthy(&testval)
            {
                self.set_pc(data.if_end);
                self.drain_scopes(data.scopes);
                *put_controller_back = false;
            }
        }
        Ok(())
    }
    fn handle_with_flow(&mut self, data : &mut WithData, put_controller_back : &mut bool) -> Result<(), String>
    {
        if self.get_pc() == data.loop_end
        {
            if let Some(next_instance) = data.instances.remove(0)
            {
                if let Value::Number(next_instance) = next_instance
                {
                    self.top_frame.instancestack.pop();
                    self.top_frame.instancestack.push(next_instance as usize);
                    self.set_pc(data.loop_start);
                }
                else
                {
                    return plainerr("internal error: values fed to with controller's 'other' data must be a list of only numbers");
                }
            }
            else
            {
                self.top_frame.instancestack.pop();
                // FIXME do we have to drain scopes here or is it always consistent?
                *put_controller_back = false;
            }
        }
        Ok(())
    }
    fn handle_foreach_flow(&mut self, data : &mut ForEachData, put_controller_back : &mut bool) -> Result<(), String>
    {
        if self.get_pc() == data.loop_end
        {
            if let Some(value) = data.values.remove(0)
            {
                self.drain_scopes(data.scopes);
                
                let scope = self.top_frame.scopes.last_mut().ok_or_else(|| minierr("internal error: there are no scopes in the top frame"))?;
                scope.insert(data.name.clone(), value);
                
                self.set_pc(data.loop_start);
            }
            else
            {
                self.top_frame.instancestack.pop();
                // FIXME do we have to drain scopes here or is it always consistent?
                *put_controller_back = false;
            }
        }
        Ok(())
    }
    pub (super) fn handle_flow_control(&mut self) -> Result<(), String>
    {
        if let Some(mut controller) = self.top_frame.controlstack.pop()
        {
            let mut put_controller_back = true;
            
            match controller
            {
                Controller::If(ref controller)          => self.handle_if_flow(&controller, &mut put_controller_back)?,
                Controller::IfElse(ref controller)      => self.handle_ifelse_flow(&controller, &mut put_controller_back)?,
                Controller::While(ref controller)       => self.handle_while_flow(&controller, &mut put_controller_back)?,
                Controller::With(ref mut controller)    => self.handle_with_flow(controller, &mut put_controller_back)?,
                Controller::ForEach(ref mut controller) => self.handle_foreach_flow(controller, &mut put_controller_back)?
            }
            
            if put_controller_back
            {
                self.top_frame.controlstack.push(controller);
            }
        }
        Ok(())
    }
}