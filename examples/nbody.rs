#![allow(clippy::inconsistent_digit_grouping)]

extern crate gammakit;
use gammakit::*;

use std::fs::File;
use std::io::Write;

fn main() -> Result<(), String>
{
    use std::time::Instant;
    let mut interpreter = Interpreter::new(Parser::new_from_default()?);
    interpreter.insert_default_bindings();

    let program = include_str!("nbody.txt").to_string();
    
    let start_time = Instant::now();
    interpreter.restart_into_string(&program)?;
    let duration = Instant::now().duration_since(start_time);
    println!("Compilation took {:?}", duration);
    
    File::create("bytecode_dump_nbodies.bin").unwrap().write_all(&interpreter.dump_code()).unwrap();
    
    let start_time = Instant::now();
    
    let steps = interpreter.step_cached_until_error_or_exit().unwrap_or(0);
    if let Some(err) = &interpreter.last_error
    {
        panic!("{}", err);
    }
    
    let duration = Instant::now().duration_since(start_time);
    println!("simulation took {:?}", duration);
    println!("steps {:?}", steps);
    println!("{:?} steps per second", steps as f64 / (duration.as_millis() as f64 / 1000.0));
    println!("{:?} nanoseconds per step", duration.as_millis() as f64 * 1000_000.0 / steps as f64);
    
    #[cfg(feature = "track_op_performance")]
    interpreter.print_op_perf_log();
    
    if let Some(err) = &interpreter.last_error
    {
        panic!("{}", err);
    }
    
    Ok(())
}