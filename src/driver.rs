use std::process::Command;
use std::path::PathBuf;
use std::error::Error;
use std::fs;
use std::{fmt};
use crate::codegen::{AsmProgram, gen_program};
use crate::lexer::{Token, Tokenizer};
use crate::parser::{Parser, Program, pretty_print};
use crate::poise::{PoiseProg, gen_poise};
use crate::emit::emit_program;

#[derive(Debug)]
pub enum DriverError {
    PreprocessorError(String),
    AssemblerError(String)
}

impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DriverError::PreprocessorError(msg) => write!(f, "Preprocessor Error: {}", msg),
            DriverError::AssemblerError(msg) => write!(f, "Assembler Error: {}", msg),
        }
    }
}

impl Error for DriverError {}

fn load_source(input_file: &PathBuf) -> Result<String, std::io::Error> {
    let source = fs::read_to_string(input_file)?;
    Ok(source)
}

pub fn run_preprocessor(input_file: &PathBuf) -> Result<PathBuf, DriverError> {
    let mut output_file = input_file.clone();
    output_file.set_extension("i");
    match Command::new("gcc")
        .args(["-E", "-P", &input_file.to_str().unwrap(), "-o", &output_file.to_str().unwrap()])
        .output() {
            Ok(output) => {
                if output.status.success() {
                    let stdout_str = String::from_utf8_lossy(&output.stdout).into_owned();
                    println!("{}", stdout_str);
                    return Ok(output_file);
                } else {
                    let msg = String::from_utf8_lossy(&output.stderr).into_owned();
                    return Err(DriverError::PreprocessorError(msg));
                }
            },
            Err(e) => return Err(DriverError::PreprocessorError(e.to_string())),
        }
}

fn run_lexer(preprocessed: &PathBuf) -> Result<Vec<Token>, Box<dyn Error>> {
    let source = load_source(preprocessed)?;
    let mut tokenizer = Tokenizer::new(source);
    let tokens = tokenizer.tokenize()?;
    Ok(tokens)
}

fn run_parser(tokens: Vec<Token>) -> Result<Program, Box<dyn Error>> {
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;
    Ok(program)
}

fn run_poiser(program: Program) -> PoiseProg {
    gen_poise(program)
}

fn run_codegen(program: PoiseProg) -> AsmProgram {
    gen_program(program)
}

fn run_emitter(asm_program: AsmProgram, output_file: &PathBuf) -> Result<(), Box<dyn Error>> {
    fs::write(&output_file, emit_program(asm_program)?)?;
    Ok(())
}

pub fn run_assembler(input_file: &PathBuf) -> Result<(), DriverError> {
    let mut output_file = input_file.clone();
    output_file.set_extension("");
    match Command::new("gcc")
        .args([&input_file.to_str().unwrap(), "-o", &output_file.to_str().unwrap()])
        .output() {
            Ok(output) => {
                if output.status.success() {
                    let stdout_str = String::from_utf8_lossy(&output.stdout).into_owned();
                    println!("{}", stdout_str);
                    Ok(())
                } else {
                    let msg = String::from_utf8_lossy(&output.stderr).into_owned();
                    Err(DriverError::AssemblerError(msg))
                }
            },
            Err(e) => Err(DriverError::AssemblerError(e.to_string()))
        }
}

pub fn run_compiler(input_file: &PathBuf, args: crate::Args) -> Result<(), Box<dyn Error>> {
    let preprocessed = input_file.clone();
    let lexed = run_lexer(&preprocessed)?;
    std::fs::remove_file(preprocessed)?;
    if args.lex {
        println!("{:#?}", lexed);
        std::process::exit(0);
    } 

    let parsed = run_parser(lexed)?;
    if args.parse {
        pretty_print(parsed);
        std::process::exit(0);
    }

    let poise = run_poiser(parsed);
    if args.tacky {
        println!("{:#?}", poise);
        std::process::exit(0);
    }

    let asm = run_codegen(poise);
    if args.codegen {
        println!("{:#?}", asm);
        std::process::exit(0);
    }
    
    let mut output_file = input_file.clone();
    output_file.set_extension("");
    run_emitter(asm, &output_file)?;
    Ok(())
}
