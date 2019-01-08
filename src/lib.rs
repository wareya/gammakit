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

/// 1) Load a grammar into a string (e.g. grammarsimple.txt)
/// 2) Call Parser::new() to get a new, uninitialized parser
/// 3) Initialize the parser with parser.
///
///

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Read;
    use std::vec::Vec;
    use std::rc::Rc;
    
    use super::*;
    
    #[test]
    fn test_everything() -> Result<(), String>
    {
        let mut file = File::open("grammarsimple.txt").or_else(|_| Err("failed to open grammar".to_string()))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).or_else(|_| Err("failed to read grammar into memory".to_string()))?;
        
        let mut parser = Parser::new();
        parser.init(&contents)?;

        let mut file2 = File::open("program.txt").or_else(|_| Err("failed to open program".to_string()))?;
        let mut contents2 = String::new();
        file2.read_to_string(&mut contents2).or_else(|_| Err("failed to read program into memory".to_string()))?;
        
        let program_lines : Vec<String> = contents2.lines().map(|x| x.to_string()).collect();
        
        let tokens = parser.tokenize(&program_lines, false)?;
        
        let ast = parser.parse_program(&tokens, &program_lines, false)?.ok_or_else(|| "failed to parse program".to_string())?;
        
        let code = Rc::new(compile_bytecode(&ast)?);
        
        let mut interpreter = Interpreter::new(&code, Some(parser));
        interpreter.insert_default_internal_functions();
        
        while interpreter.step().is_ok(){}
        
        if let Some(err) = &interpreter.last_error
        {
            println!("{}", err);
        }
        
        // test clearing interpreter state and restarting
        
        interpreter.clear_global_state().unwrap_or(());
        interpreter.restart(&code).unwrap_or(());
        
        while interpreter.step().is_ok(){}
        
        if let Some(err) = &interpreter.last_error
        {
            println!("{}", err);
        }
        
        Ok(())
    }
}
