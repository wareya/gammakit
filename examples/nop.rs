#![allow(clippy::inconsistent_digit_grouping)]

extern crate gammakit;
use gammakit::*;

fn main() -> Result<(), String>
{
    use std::time::Instant;
    let mut interpreter = Interpreter::new(Parser::new_from_default()?);
    interpreter.insert_default_bindings();

    interpreter.restart_full_of_nops(10_000_000);
    
    let start_time = Instant::now();
    
    let steps = interpreter.step_until_error_or_exit().unwrap_or(0);
    if let Some(err) = &interpreter.last_error
    {
        panic!("{}", err);
    }
    
    let duration = Instant::now().duration_since(start_time);
    println!("simulation took {:?}", duration);
    println!("steps {:?}", steps);
    println!("{:?} steps per second", steps as f64 / (duration.as_micros() as f64 / 1000_000.0));
    println!("{:?} nanoseconds per step", duration.as_micros() as f64 * 1000.0 / steps as f64);
    interpreter.print_op_perf_log();
    
    if let Some(err) = &interpreter.last_error
    {
        panic!("{}", err);
    }
    
    Ok(())
}