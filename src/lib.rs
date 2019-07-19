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

#![allow(clippy::suspicious_else_formatting)]
#![allow(clippy::redundant_closure)]

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
mod bookkeeping;

pub use crate::{parser::*, compiler::*, interpreter::*};

impl Parser {
    /// optional helper function to simplify the process of compiling from text to bytecode
    pub fn give_me_bytecode(&mut self, text: &str) -> Result<Code, String>
    {
        let program_lines : Vec<String> = text.lines().map(|x| x.to_string()).collect();
        
        let tokens = self.tokenize(&program_lines, false)?;
        
        let ast = self.parse_program(&tokens, &program_lines, false)?.ok_or_else(|| "failed to parse program".to_string())?;
        
        Ok(compile_bytecode(&ast)?)
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Read;
    use std::io::Write;
    
    use super::*;
    
    #[test]
    fn test_everything() -> Result<(), String>
    {
        let mut parser = Parser::new_from_default()?;

        let mut program = String::new();
        File::open("program.txt").or_else(|_| Err("failed to open program".to_string()))?.read_to_string(&mut program).or_else(|_| Err("failed to read program into memory".to_string()))?;
        
        let code = parser.give_me_bytecode(&program)?;
        
        File::create("bytecode_dump_main.bin").unwrap().write_all(code.get(..).unwrap()).unwrap();
        
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
    
    #[test]
    fn test_nbodies() -> Result<(), String>
    {
        use std::time::Instant;
        let mut parser = Parser::new_from_default()?;

        let mut program = String::new();
        File::open("nbody.txt").or_else(|_| Err("failed to open program".to_string()))?.read_to_string(&mut program).or_else(|_| Err("failed to read program into memory".to_string()))?;
        
        let code = parser.give_me_bytecode(&program)?;
        
        File::create("bytecode_dump_nbodies.bin").unwrap().write_all(code.get(..).unwrap()).unwrap();
        
        let mut interpreter = Interpreter::new(&code, Some(parser));
        interpreter.insert_default_bindings();
        
        let start_time = Instant::now();
        
        let mut steps = 0;
        while interpreter.step().is_ok()
        {
            steps += 1;
        }
        
        let duration = Instant::now().duration_since(start_time);
        println!("simulation took {:?}", duration);
        println!("steps {:?}", steps);
        println!("{:?} steps per second", steps as f64 / (duration.as_millis() as f64 / 1000.0));
        println!("{:?} seconds per step", duration.as_millis() as f64 / 1000.0 / steps as f64);
        let mut op_map = interpreter.op_map.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>();
        op_map.sort_by(|a, b| a.1.cmp(&b.1));
        let mut total = 0.0;
        for (op, time) in op_map
        {
            let time = (time/1000000) as f64 / 1000.0;
            total  += time;
            println!("{:02X}:{}", op, time);
        }
        println!("total: {}", total);
        println!("({} steps per second)", steps as f64 / total);
        
        if let Some(err) = &interpreter.last_error
        {
            panic!("{}", err);
        }
        
        Ok(())
    }
    
    #[test]
    fn test_nopspeed() -> Result<(), String>
    {
        use std::time::Instant;
        use std::collections::BTreeMap;
        use std::rc::Rc;
        use bookkeeping::*;

        let mut code = Code{code : Rc::new(vec!(0; 10_000_000)), debug : Rc::new(BTreeMap::new()), bookkeeping : Bookkeeping::new()};
        
        let end = code.code.len()-1;
        Rc::get_mut(&mut code.code).unwrap()[end] = bytecode::EXIT;
        
        let mut interpreter = Interpreter::new(&code, None);
        interpreter.insert_default_bindings();
        
        let start_time = Instant::now();
        
        let mut steps = 0;
        while interpreter.step().is_ok()
        {
            steps += 1;
        }
        
        let duration = Instant::now().duration_since(start_time);
        println!("simulation took {:?}", duration);
        println!("steps {:?}", steps);
        println!("{:?} steps per second", steps as f64 / (duration.as_millis() as f64 / 1000.0));
        println!("{:?} seconds per step", duration.as_millis() as f64 / 1000.0 / steps as f64);
        let mut op_map = interpreter.op_map.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>();
        op_map.sort_by(|a, b| a.1.cmp(&b.1));
        let mut total = 0.0;
        for (op, time) in op_map
        {
            let time = (time/1000000) as f64 / 1000.0;
            total  += time;
            println!("{:02X}:{}", op, time);
        }
        println!("total: {}", total);
        
        if let Some(err) = &interpreter.last_error
        {
            panic!("{}", err);
        }
        
        Ok(())
    }
}
