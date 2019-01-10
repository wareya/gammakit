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

/// Gammakit is a high-level scripting language meant for games.
///
/// WARNING: Gammakit is the bespoke programming language of a toy game engine. It is not for general use.
///
/// There are no API stability guarantees of any kind, and anything can change at any time if it makes sense for the game engine.
///
/// If, for some reason, you decide to use gammkit, make a hard fork of it. You're gonna want to change stuff anyway.
///
/// 1) Load a grammar into a string (e.g. grammarsimple.txt)
/// 2) Call Parser::new_from_default() to get a new parser initialized with the default parser
/// 3) Load a program into a vector of Strings. (This will probably be changed to a mere single string later.)
/// 4) Tokenize the program with parser.tokenize(&lines, false)
/// 5) Parse the token list with parser.parse_programs(&tokens, &lines, false)
/// 6) Compile to bytecode with compile_bytecode(&ast)
/// 7) Create an interpreter with Interpreter::new(&Rc::new(code), Some(parser)) or similar
/// 8) Optional: insert the default binding functions with interpreter.insert_default_internal_functions()
/// 9) Run interpreter.step() until it returns Err. Err(None) indicates graceful exit, Err(Some(String))) indicates an error.

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
        let mut parser = Parser::new_from_default()?;

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
