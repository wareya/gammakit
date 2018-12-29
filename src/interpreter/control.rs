use crate::interpreter::*;

impl Interpreter {
    fn handle_while_flow(&mut self, controller : &mut ControlData, put_controller_back : &mut bool) -> Result<(), Option<String>>
    {
        // if we are at the end of the expression, test it, jump outside of the loop if it's false
        if self.get_pc() == controller.controlpoints[1]
        {
            if let Some(testval) = self.stack_pop_val()
            {
                if !value_truthy(&testval)
                {
                    self.set_pc(controller.controlpoints[2]);
                    self.drain_scopes(controller.scopes);
                    *put_controller_back = false;
                }
            }
            else
            {
                return plainerr("internal error: failed to find value on stack while handling WHILE controller");
            }
        }
        // if we are at the end of the loop, go back to the expression
        else if self.get_pc() == controller.controlpoints[2]
        {
            self.set_pc(controller.controlpoints[0]);
            self.drain_scopes(controller.scopes);
        }
        
        Ok(())
    }
    fn handle_ifelse_flow(&mut self, controller : &mut ControlData, put_controller_back : &mut bool) -> Result<(), Option<String>>
    {
        if self.get_pc() == controller.controlpoints[0]
        {
            // if we are at the end of the expression, test it, jump to the "else" block if it's false
            if let Some(testval) = self.stack_pop_val()
            {
                if !value_truthy(&testval)
                {
                    self.set_pc(controller.controlpoints[1]);
                }
            }
            else
            {
                return plainerr("internal error: failed to find value on stack while handling IFELSE controller");
            }
        }
        else if self.get_pc() == controller.controlpoints[1]
        {
            // end of the main block, jump to the end of the "else" block
            self.set_pc(controller.controlpoints[2]);
            self.drain_scopes(controller.scopes);
            *put_controller_back = false;
        }
        else if self.get_pc() == controller.controlpoints[2]
        {
            // end of the "else" block, clean up
            self.drain_scopes(controller.scopes);
            *put_controller_back = false;
        }
        
        Ok(())
    }
    fn handle_if_flow(&mut self, controller : &mut ControlData, put_controller_back : &mut bool) -> Result<(), Option<String>>
    {
        if self.get_pc() == controller.controlpoints[0]
        {
            // if we are at the end of the expression, test it, jump past the block if it's false
            if let Some(testval) = self.stack_pop_val()
            {
                if !value_truthy(&testval)
                {
                    self.set_pc(controller.controlpoints[1]);
                    self.drain_scopes(controller.scopes);
                    *put_controller_back = false;
                }
            }
            else
            {
                return plainerr("internal error: failed to find value on stack while handling IF controller");
            }
        }
        
        Ok(())
    }
    fn handle_for_flow(&mut self, controller : &mut ControlData, put_controller_back : &mut bool) -> Result<(), Option<String>>
    {
        if self.get_pc() == controller.controlpoints[1]
        {
            if self.suppress_for_expr_end
            {
                self.suppress_for_expr_end = false;
            }
            // if we are at the end of the loop expression, test it, jump past the block if it's false
            else if let Some(testval) = self.stack_pop_val()
            {
                if !value_truthy(&testval)
                {
                    self.set_pc(controller.controlpoints[3]);
                    self.drain_scopes(controller.scopes);
                    *put_controller_back = false;
                }
                // otherwise jump to code (end of post expression)
                else
                {
                    self.set_pc(controller.controlpoints[2]);
                }
            }
            else
            {
                return plainerr("internal error: failed to find value on stack while handling FOR controller");
            }
        }
        else if self.get_pc() == controller.controlpoints[2]
        {
            // if we are at the end of the post expression, jump to the expression
            self.set_pc(controller.controlpoints[0]);
        }
        else if self.get_pc() == controller.controlpoints[3]
        {
            // if we are at the end of the code block, jump to the post expression
            self.set_pc(controller.controlpoints[1]);
        }
        
        Ok(())
    }
    fn handle_with_flow(&mut self, controller : &mut ControlData, put_controller_back : &mut bool) -> Result<(), Option<String>>
    {
        if self.get_pc() == controller.controlpoints[1]
        {
            if let Some(ref mut inst_list) = controller.other
            {
                if let Some(next_instance) = inst_list.remove(0)
                {
                    self.top_frame.instancestack.pop();
                    self.top_frame.instancestack.push(next_instance);
                    self.set_pc(controller.controlpoints[0]);
                }
                else
                {
                    self.top_frame.instancestack.pop();
                    // FIXME do we have to drain scopes here or is it always consistent?
                    *put_controller_back = false;
                }
            }
        }
        
        Ok(())
    }
    pub (super) fn handle_flow_control(&mut self) -> Result<(), Option<String>>
    {
        if let Some(mut controller) = self.top_frame.controlstack.pop()
        {
            let mut put_controller_back = true;
            if controller.controlpoints.contains(&self.get_pc())
            {
                match controller.controltype
                {
                    WHILE  => self.handle_while_flow(&mut controller, &mut put_controller_back)?,
                    IFELSE => self.handle_ifelse_flow(&mut controller, &mut put_controller_back)?,
                    IF     => self.handle_if_flow(&mut controller, &mut put_controller_back)?,
                    FOR    => self.handle_for_flow(&mut controller, &mut put_controller_back)?,
                    WITH   => self.handle_with_flow(&mut controller, &mut put_controller_back)?,
                    _ =>
                    {
                        return Err(Some(format!("internal error: unknown controller type {:02X}", controller.controltype)));
                    }
                }
            }
            if put_controller_back
            {
                self.top_frame.controlstack.push(controller);
            }
        }
        
        Ok(())
    }
}