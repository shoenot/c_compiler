mod driver;
mod lexer;
use clap::Parser;
use std::path::PathBuf;
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
    
    if let Err(e) = run_preprocessor(args.input_file) {
        println!("{}", e)
    }
}
