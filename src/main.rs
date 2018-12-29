extern crate regex;

use std::fs::File;
use std::io::Read;
use std::vec::Vec;

mod strings;
mod regexholder;
mod ast;
#[macro_use]
mod matches;
mod parser;
mod bytecode;
mod grammar;
mod compiler;
mod disassembler;
mod interpreter;

use crate::parser::*;
use crate::compiler::*;
use crate::disassembler::*;
use crate::interpreter::*;

fn main() -> std::io::Result<()>
{
    let mut file = File::open("grammarsimple.txt")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    
    let mut parser = Parser::new();
    if parser.init(&contents).is_ok()
    {
        let mut file2 = File::open("program.txt")?;
        let mut contents2 = String::new();
        file2.read_to_string(&mut contents2)?;
        
        let program_lines : Vec<String> = contents2.lines().map(|x| x.to_string()).collect();
        
        if let Ok(tokens) = parser.tokenize(&program_lines, false)
        {
            if let Ok(Some(ref ast)) = parser.parse_program(&tokens, &program_lines, false)
            {
                let code = compile_bytecode(ast);
                
                if let Ok(code) = code
                {
                    if false
                    {
                        if let Ok(disassembly) = disassemble_bytecode(&code, 0, 0)
                        {
                            for line in disassembly
                            {
                                println!("{}", line);
                            }
                        }
                    }
                    
                    let mut interpreter = Interpreter::new(code, Some(parser));
                    interpreter.insert_default_internal_functions();
                    
                    while interpreter.step().is_ok(){}
                }
                else if let Err(Some(err)) = code
                {
                    println!("{}", err);
                }
            }
        }
    }
    
    Ok(())
}