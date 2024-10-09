use std::{error::Error, io, process};
use tx_processor::process_file_and_output;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        Err("Not enough args")?;
    }

    let path = &args[1];
    process_file_and_output(path)
}
