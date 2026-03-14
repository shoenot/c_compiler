mod driver;
mod lexer;
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
    parse: bool,
    #[arg(long)]
    codegen: bool,
}

fn main() {
    let args = Args::parse();
    
    let preprocessed = match run_preprocessor(args.input_file.clone()) {
        Ok(pb) => pb,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1)
        },
    };

    if let Err(e) = run_compiler(preprocessed, args) {
        eprintln!("{}", e)
    }
}
