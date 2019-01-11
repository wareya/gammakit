//! Gammakit is a high-level scripting language meant for games.
//!
//! WARNING: Gammakit is the bespoke programming language of a toy game engine. It is not for general use.
//!
//! There are no API stability guarantees of any kind, and anything can change at any time if it makes sense for the game engine.
//!
//! If, for some reason, you decide to use gammkit, make a hard fork of it. You're gonna want to change stuff anyway.
//!
//! General use:
//!
//! 1) Call Parser::new_from_default() to get a new parser initialized with the default parser
//! 2) Compile program text to bytecode with parser.give_me_bytecode(text) (a helper function)
//! 3) Create an interpreter with Interpreter::new(&code, Some(parser)) or similar
//! 4) Optional: insert the default binding functions with interpreter.insert_default_bindings()
//! 5) Run interpreter.step() until it returns Err. Err(None) indicates graceful exit, Err(Some(String))) indicates an error.

#[macro_use]
mod matches;
mod strings;
mod regexholder;
mod ast;
mod parser;
mod bytecode;
mod grammar;
mod compiler;
mod interpreter;

pub use crate::{parser::*, compiler::*, interpreter::*};

use std::rc::Rc;

impl Parser {
    /// optional helper function to simplify the process of compiling from text to bytecode
    pub fn give_me_bytecode(&mut self, text: &str) -> Result<Rc<Vec<u8>>, String>
    {
        let program_lines : Vec<String> = text.lines().map(|x| x.to_string()).collect();
        
        let tokens = self.tokenize(&program_lines, false)?;
        
        let ast = self.parse_program(&tokens, &program_lines, false)?.ok_or_else(|| "failed to parse program".to_string())?;
        
        Ok(Rc::new(compile_bytecode(&ast)?))
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Read;
    
    use super::*;
    
    #[test]
    fn test_everything() -> Result<(), String>
    {
        let mut parser = Parser::new_from_default()?;

        let mut program = String::new();
        File::open("program.txt").or_else(|_| Err("failed to open program".to_string()))?.read_to_string(&mut program).or_else(|_| Err("failed to read program into memory".to_string()))?;
        
        let code = parser.give_me_bytecode(&program)?;
        
        let mut interpreter = Interpreter::new(&code, Some(parser));
        interpreter.insert_default_bindings();
        
        while interpreter.step().is_ok(){}
        
        if let Some(err) = &interpreter.last_error
        {
            panic!("{}", err);
        }
        
        // test clearing interpreter state and restarting
        
        interpreter.clear_global_state().unwrap_or(());
        interpreter.restart(&code).unwrap_or(());
        
        while interpreter.step().is_ok(){}
        
        if let Some(err) = &interpreter.last_error
        {
            panic!("{}", err);
        }
        
        Ok(())
    }
}
