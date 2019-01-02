#![allow(clippy::suspicious_else_formatting)]

use crate::interpreter::*;

impl Interpreter {
    fn handle_while_flow(&mut self, controller : &mut ControlData, put_controller_back : &mut bool) -> Result<(), Option<String>>
    {
        let expr_start = controller.controlpoints.get(0).ok_or_else(|| minierr("internal error: control logic could not find expr_start control point required to handle WHILE controller"))?;
        let loop_start = controller.controlpoints.get(1).ok_or_else(|| minierr("internal error: control logic could not find loop_start control point required to handle WHILE controller"))?;
        let loop_end   = controller.controlpoints.get(2).ok_or_else(|| minierr("internal error: control logic could not find loop_end control point required to handle WHILE controller"))?;
        
        // if we are at the end of the expression, test it, jump outside of the loop if it's false
        if self.get_pc() == *loop_start
        {
            let testval = self.stack_pop_val().ok_or_else(|| minierr("internal error: failed to find value on stack while handling WHILE controller"))?;
            if !value_truthy(&testval)
            {
                self.set_pc(*loop_end);
                self.drain_scopes(controller.scopes);
                *put_controller_back = false;
            }
        }
        // if we are at the end of the loop, go back to the expression
        else if self.get_pc() == *loop_end
        {
            self.set_pc(*expr_start);
            self.drain_scopes(controller.scopes);
        }
        Ok(())
    }
    fn handle_ifelse_flow(&mut self, controller : &mut ControlData, put_controller_back : &mut bool) -> Result<(), Option<String>>
    {
        let expr_end = controller.controlpoints.get(0).ok_or_else(|| minierr("internal error: control logic could not find expr_end control point required to handle IFELSE controller"))?;
        let if_end   = controller.controlpoints.get(1).ok_or_else(|| minierr("internal error: control logic could not find if_end control point required to handle IFELSE controller"))?;
        let else_end = controller.controlpoints.get(2).ok_or_else(|| minierr("internal error: control logic could not find else_end control point required to handle IFELSE controller"))?;
        
        // if we are at the end of the expression, test it, jump to the "else" block if it's false
        if self.get_pc() == *expr_end
        {
            let testval = self.stack_pop_val().ok_or_else(|| minierr("internal error: failed to find value on stack while handling IFELSE controller"))?;
            if !value_truthy(&testval)
            {
                self.set_pc(*if_end);
            }
        }
        // end of the main block, jump to the end of the "else" block
        else if self.get_pc() == *if_end
        {
            
            self.set_pc(*else_end);
            self.drain_scopes(controller.scopes);
            *put_controller_back = false;
        }
        // end of the "else" block, clean up
        else if self.get_pc() == *else_end
        {
            self.drain_scopes(controller.scopes);
            *put_controller_back = false;
        }
        Ok(())
    }
    fn handle_if_flow(&mut self, controller : &mut ControlData, put_controller_back : &mut bool) -> Result<(), Option<String>>
    {
        let expr_end = controller.controlpoints.get(0).ok_or_else(|| minierr("internal error: control logic could not find expr_end control point required to handle IF controller"))?;
        let if_end   = controller.controlpoints.get(1).ok_or_else(|| minierr("internal error: control logic could not find if_end control point required to handle IF controller"))?;
        
        // if we are at the end of the expression, test it, jump past the block if it's false
        if self.get_pc() == *expr_end
        {
            let testval = self.stack_pop_val().ok_or_else(|| minierr("internal error: failed to find value on stack while handling IF controller"))?;
            if !value_truthy(&testval)
            {
                self.set_pc(*if_end);
                self.drain_scopes(controller.scopes);
                *put_controller_back = false;
            }
        }
        Ok(())
    }
    fn handle_for_flow(&mut self, controller : &mut ControlData, put_controller_back : &mut bool) -> Result<(), Option<String>>
    {
        // the "init" block of a for loop is outside of the actual control region, because it is always run exactly once
        let expr_start = controller.controlpoints.get(0).ok_or_else(|| minierr("internal error: control logic could not find expr_start control point required to handle FOR controller"))?;
        let post_start = controller.controlpoints.get(1).ok_or_else(|| minierr("internal error: control logic could not find post_start control point required to handle FOR controller"))?;
        let loop_start = controller.controlpoints.get(2).ok_or_else(|| minierr("internal error: control logic could not find loop_start control point required to handle FOR controller"))?;
        let loop_end   = controller.controlpoints.get(3).ok_or_else(|| minierr("internal error: control logic could not find loop_end control point required to handle FOR controller"))?;
        
        if self.get_pc() == *post_start
        {
            if self.suppress_for_expr_end
            {
                self.suppress_for_expr_end = false;
            }
            // if we are at the end of the loop expression, test it
            else
            {
                let testval = self.stack_pop_val().ok_or_else(|| minierr("internal error: failed to find value on stack while handling FOR controller"))?;
                // jump past the block if it's false
                if !value_truthy(&testval)
                {
                    self.set_pc(*loop_end);
                    self.drain_scopes(controller.scopes);
                    *put_controller_back = false;
                }
                // otherwise jump to code (end of post expression)
                else
                {
                    self.set_pc(*loop_start);
                }
            }
        }
        else if self.get_pc() == *loop_start
        {
            // if we are at the end of the post expression, jump to the expression
            self.set_pc(*expr_start);
        }
        else if self.get_pc() == *loop_end
        {
            // if we are at the end of the code block, jump to the post expression
            self.set_pc(*post_start);
        }
        Ok(())
    }
    fn handle_with_flow(&mut self, controller : &mut ControlData, put_controller_back : &mut bool) -> Result<(), Option<String>>
    {
        let loop_start = controller.controlpoints.get(0).ok_or_else(|| minierr("internal error: control logic could not find loop_start control point required to handle WITH controller"))?;
        let loop_end   = controller.controlpoints.get(1).ok_or_else(|| minierr("internal error: control logic could not find loop_end control point required to handle WITH controller"))?;
        
        if self.get_pc() == *loop_end
        {
            if let Some(ref mut inst_list) = controller.other
            {
                if let Some(next_instance) = inst_list.remove(0)
                {
                    self.top_frame.instancestack.pop();
                    self.top_frame.instancestack.push(next_instance);
                    self.set_pc(*loop_start);
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