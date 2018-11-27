use std::collections::VecDeque;
use std::collections::HashMap;
use std::rc::Rc;
use std::rc::Weak;

use super::strings::*;
use super::ast::*;
use super::parser::*;
use super::bytecode::*;
use super::compiler::*;
use super::regexholder::RegexHolder;

mod bindings;
mod internal;
mod simulation;
mod manipulation;
mod jumping;
mod types;

use self::types::*;

// global interpreter data
pub struct GlobalState {
    instance_id: usize,// init 100000000
    object_id: usize,  // init 300000000
    instances: HashMap<usize, Instance>,
    instances_by_type: HashMap<usize, Vec<usize>>,
    objectnames: HashMap<String, usize>,
    objects: HashMap<usize, ObjSpec>,
    regex_holder: RegexHolder,
    parser: Parser
}

impl GlobalState {
    pub fn new(parser : Parser) -> GlobalState
    {
        GlobalState { instance_id : 100000000, object_id : 300000000, instances : HashMap::new(), instances_by_type : HashMap::new(), objectnames : HashMap::new(), objects : HashMap::new() , regex_holder : RegexHolder::new(), parser }
    }
}

// interpreter state
pub struct Interpreter {
    top_frame: Frame,
    frames: Vec<Frame>,
    doexit: bool,
    suppress_for_expr_end: bool
}

impl Interpreter {
    pub fn new(code : Vec<u8>) -> Interpreter
    {
        Interpreter { top_frame : Frame::new_root(Rc::new(code)), frames : vec!() , doexit : false, suppress_for_expr_end : false }
    }
    #[cfg_attr(feature = "cargo-clippy", allow(cyclomatic_complexity))]
    fn handle_flow_control(&mut self)
    {
        if let Some(mut controller) = self.top_frame.controlstack.pop()
        {
            let mut must_put_back = true;
            if controller.controlpoints.contains(&self.get_pc())
            {
                if controller.controltype == WHILE
                {
                    // if we are at the end of the expression, test it, jump outside of the loop if it's false
                    if self.get_pc() == controller.controlpoints[1]
                    {
                        if let Ok(testval) = self.stack_pop_any()
                        {
                            if !value_truthy(&testval)
                            {
                                self.set_pc(controller.controlpoints[2]);
                                self.drain_scopes(controller.scopes);
                                must_put_back = false;
                            }
                        }
                        else
                        {
                            panic!("internal error: not enough values on stack while handling WHILE controller");
                        }
                    }
                    // if we are at the end of the loop, go back to the expression
                    else if self.get_pc() == controller.controlpoints[2]
                    {
                        self.set_pc(controller.controlpoints[0]);
                        self.drain_scopes(controller.scopes);
                    }
                }
                else if controller.controltype == IFELSE
                {
                    if self.get_pc() == controller.controlpoints[0]
                    {
                        // if we are at the end of the expression, test it, jump to the "else" block if it's false
                        if let Ok(testval) = self.stack_pop_any()
                        {
                            if !value_truthy(&testval)
                            {
                                self.set_pc(controller.controlpoints[1]);
                            }
                        }
                        else
                        {
                            panic!("internal error: not enough values on stack while handling IFELSE controller");
                        }
                    }
                    else if self.get_pc() == controller.controlpoints[1]
                    {
                        // end of the main block, jump to the end of the "else" block
                        self.set_pc(controller.controlpoints[2]);
                        self.drain_scopes(controller.scopes);
                        must_put_back = false;
                    }
                    else if self.get_pc() == controller.controlpoints[2]
                    {
                        // end of the "else" block, clean up
                        self.drain_scopes(controller.scopes);
                        must_put_back = false;
                    }
                }
                else if controller.controltype == IF
                {
                    if self.get_pc() == controller.controlpoints[0]
                    {
                        // if we are at the end of the expression, test it, jump past the block if it's false
                        if let Ok(testval) = self.stack_pop_any()
                        {
                            if !value_truthy(&testval)
                            {
                                self.set_pc(controller.controlpoints[1]);
                                self.drain_scopes(controller.scopes);
                                must_put_back = false;
                            }
                        }
                        else
                        {
                            panic!("internal error: not enough values on stack while handling IF controller");
                        }
                    }
                }
                else if controller.controltype == FOR
                {
                    if self.get_pc() == controller.controlpoints[1]
                    {
                        if self.suppress_for_expr_end
                        {
                            self.suppress_for_expr_end = false;
                        }
                        // if we are at the end of the loop expression, test it, jump past the block if it's false
                        else if let Ok(testval) = self.stack_pop_any()
                        {
                            if !value_truthy(&testval)
                            {
                                self.set_pc(controller.controlpoints[3]);
                                self.drain_scopes(controller.scopes);
                                must_put_back = false;
                            }
                            // otherwise jump to code (end of post expression)
                            else
                            {
                                self.set_pc(controller.controlpoints[2]);
                            }
                        }
                        else
                        {
                            panic!("internal error: not enough values on stack while handling FOR controller");
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
                }
                else if controller.controltype == WITH
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
                                must_put_back = false;
                            }
                        }
                    }
                }
                else
                {
                    panic!("internal error: unknown controller type {:02X}", controller.controltype);
                }
            }
            if must_put_back
            {
                self.top_frame.controlstack.push(controller);
            }
        }
    }
    pub fn step(&mut self, global : &mut GlobalState) -> bool // TODO: return whether there was an error or not
    {
        let code = self.get_code();
        
        if self.top_frame.pc >= code.len()
        {
            println!("internal error: ran past end of code");
            return false;
        }
        let op = self.pull_single_from_code();
        
        if let Some(opfunc) = self.get_opfunc(op)
        {
            opfunc(self, global);
            self.handle_flow_control();
            return !self.doexit;
        }
        else
        {
            println!("internal error: unknown operation 0x{:02X}", op);
            println!("line: {}", self.top_frame.currline);
            return false;
        }
    }
}
