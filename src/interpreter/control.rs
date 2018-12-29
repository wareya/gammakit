use crate::interpreter::*;

impl Interpreter {
    fn handle_while_flow(&mut self, controller : &mut ControlData, put_controller_back : &mut bool) -> Result<(), Option<String>>
    {
        if let (Some(expr_start), Some(loop_start), Some(loop_end)) = (controller.controlpoints.get(0), controller.controlpoints.get(1), controller.controlpoints.get(2))
        {
            // if we are at the end of the expression, test it, jump outside of the loop if it's false
            if self.get_pc() == *loop_start
            {
                if let Some(testval) = self.stack_pop_val()
                {
                    if !value_truthy(&testval)
                    {
                        self.set_pc(*loop_end);
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
            else if self.get_pc() == *loop_end
            {
                self.set_pc(*expr_start);
                self.drain_scopes(controller.scopes);
            }
            Ok(())
        }
        else
        {
            plainerr("internal error: control logic could not find the 3 control points required to handle WHILE controller")
        }
    }
    fn handle_ifelse_flow(&mut self, controller : &mut ControlData, put_controller_back : &mut bool) -> Result<(), Option<String>>
    {
        if let (Some(expr_end), Some(if_end), Some(else_end)) = (controller.controlpoints.get(0), controller.controlpoints.get(1), controller.controlpoints.get(2))
        {
            if self.get_pc() == *expr_end
            {
                // if we are at the end of the expression, test it, jump to the "else" block if it's false
                if let Some(testval) = self.stack_pop_val()
                {
                    if !value_truthy(&testval)
                    {
                        self.set_pc(*if_end);
                    }
                }
                else
                {
                    return plainerr("internal error: failed to find value on stack while handling IFELSE controller");
                }
            }
            else if self.get_pc() == *if_end
            {
                // end of the main block, jump to the end of the "else" block
                self.set_pc(*else_end);
                self.drain_scopes(controller.scopes);
                *put_controller_back = false;
            }
            else if self.get_pc() == *else_end
            {
                // end of the "else" block, clean up
                self.drain_scopes(controller.scopes);
                *put_controller_back = false;
            }
            Ok(())
        }
        else
        {
            plainerr("internal error: control logic could not find the 3 control points required to handle IFELSE controller")
        }
    }
    fn handle_if_flow(&mut self, controller : &mut ControlData, put_controller_back : &mut bool) -> Result<(), Option<String>>
    {
        if let (Some(expr_end), Some(if_end)) = (controller.controlpoints.get(0), controller.controlpoints.get(1))
        {
            if self.get_pc() == *expr_end
            {
                // if we are at the end of the expression, test it, jump past the block if it's false
                if let Some(testval) = self.stack_pop_val()
                {
                    if !value_truthy(&testval)
                    {
                        self.set_pc(*if_end);
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
        else
        {
            plainerr("internal error: control logic could not find the 2 control points required to handle IF controller")
        }
    }
    fn handle_for_flow(&mut self, controller : &mut ControlData, put_controller_back : &mut bool) -> Result<(), Option<String>>
    {
        // the "init" block of a for loop is outside of the actual control region, because it is always run exactly once
        if let (Some(expr_start), Some(post_start), Some(loop_start), Some(loop_end)) = (controller.controlpoints.get(0), controller.controlpoints.get(1), controller.controlpoints.get(2), controller.controlpoints.get(3))
        {
            if self.get_pc() == *post_start
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
                else
                {
                    return plainerr("internal error: failed to find value on stack while handling FOR controller");
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
        else
        {
            plainerr("internal error: control logic could not find the 4 control points required to handle FOR controller")
        }
    }
    fn handle_with_flow(&mut self, controller : &mut ControlData, put_controller_back : &mut bool) -> Result<(), Option<String>>
    {
        if let (Some(loop_start), Some(loop_end)) = (controller.controlpoints.get(0), controller.controlpoints.get(1))
        {
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
        else
        {
            plainerr("internal error: control logic could not find the 2 control points required to handle WITH controller")
        }
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