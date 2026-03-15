mod driver;
mod lexer;
mod parser;
mod codegen;
mod emit;
use clap::Parser;
use std::{path::PathBuf, process};
use crate::driver::*;

#[derive(Parser, Debug)]
struct Args {
    input_file: PathBuf,
    #[arg(short)]
    S: bool,
    #[arg(long)]
    lex: bool,
    #[arg(long)]
    parsed: bool,
    #[arg(long)]
    codegen: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let input_file = args.input_file.clone();
    
    let preprocessed = match run_preprocessor(&input_file) {
        Ok(pb) => pb,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1)
        },
    };

    if let Err(e) = run_compiler(&preprocessed, args) {
        eprintln!("{}", e);
        return Err(e);
    }

    match run_assembler(&input_file) {
        Ok(pb) => pb,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1)
        },
    };

    Ok(())
}
