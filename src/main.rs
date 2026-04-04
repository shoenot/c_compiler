mod driver;
mod lexer;
mod parser;
mod semanal;
mod poise;
mod codegen;
mod emit;
mod types;
use clap::Parser;
use std::{path::PathBuf, process};
use crate::driver::*;

#[derive(Parser, Debug, Clone)]
struct Args {
    input_file: PathBuf,
    #[arg(short)]
    s: bool,
    #[arg(long)]
    lex: bool,
    #[arg(long)]
    parse: bool,
    #[arg(long)]
    validate: bool,
    #[arg(long)]
    tacky: bool,
    #[arg(long)]
    codegen: bool,
    #[arg(short)]
    c: bool
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

    let compiled = match run_compiler(&preprocessed, args.clone()) {
        Ok(pb) => pb,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1)
        },
    };

    match run_assembler(&compiled, args) {
        Ok(pb) => pb,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1)
        },
    };

    Ok(())
}
