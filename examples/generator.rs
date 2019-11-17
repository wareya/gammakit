extern crate gammakit;
use gammakit::*;

use std::fs::File;
use std::io::Write;

fn main() -> Result<(), String>
{
    let mut interpreter = Interpreter::new(Parser::new_from_default()?);
    interpreter.insert_default_bindings();

    let program = include_str!("generator.txt").to_string();
    
    interpreter.restart_into_string(&program)?;
    
    File::create("bytecode_dump_generator.bin").unwrap().write_all(&interpreter.dump_code()).unwrap();
    
    interpreter.step_until_error_or_exit().ok();
    if let Some(err) = &interpreter.last_error
    {
        panic!("{}", err);
    }
    
    Ok(())
}