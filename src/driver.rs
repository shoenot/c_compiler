use std::process::Command;
use std::path::PathBuf;
use std::error::Error;
use std::fs;
use std::{fmt};
use std::collections::HashMap;
use crate::codegen::{AsmProgram, gen_program};
use crate::lexer::{Token, Tokenizer};
use crate::parser::{Parser, Program};
use crate::semanal::{semantic_analysis, Symbol, MapEntry};
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

fn run_semanal(program: &mut Program, symbols: &mut HashMap<String, Symbol>) -> Result<HashMap<String, MapEntry>, Box<dyn Error>> {
    let var_map = semantic_analysis(program, symbols)?;
    Ok(var_map)
}

fn run_poise(program: Program) -> PoiseProg {
    gen_poise(&program)
}

fn run_codegen(program: PoiseProg) -> AsmProgram {
    gen_program(program)
}


fn run_emitter(asm_program: AsmProgram, symbols: &mut HashMap<String, Symbol>, output_file: &PathBuf) -> Result<PathBuf, Box<dyn Error>> {
    fs::write(&output_file, emit_program(asm_program, symbols)?)?;
    Ok(output_file.to_path_buf())
}

pub fn run_compiler(input_file: &PathBuf, args: crate::Args) -> Result<PathBuf, Box<dyn Error>> {
    let preprocessed = input_file.clone();
    let lexed = run_lexer(&preprocessed)?;
    std::fs::remove_file(preprocessed)?;
    if args.lex {
        for item in lexed {
            println!("{:?}", item);
        };
        std::process::exit(0);
    } 

    let mut symbols = HashMap::new();

    let mut parsed = run_parser(lexed)?;
    if args.parse {
        for decl in parsed.declarations {
            println!("{:#?}", decl);
        }
        std::process::exit(0);
    }

    let var_map = run_semanal(&mut parsed, &mut symbols)?;
    if args.validate {
        for decl in parsed.declarations {
            println!("{:#?}", decl);
        }
        println!("{:#?}", var_map);
        std::process::exit(0);
    }

    let poise = run_poise(parsed);
    if args.tacky {
        for function in poise.functions {
            for item in function.body {
                println!("{:?}", item);
            }
        }
        std::process::exit(0);
    }

    let asm = run_codegen(poise);
    if args.codegen {
        for function in asm.functions {
            for item in function.body {
                println!("{:?}", item);
            }
        }
        std::process::exit(0);
    }
    
    let mut output_file = input_file.clone();
    output_file.set_extension("s");
    run_emitter(asm, &mut symbols, &output_file)?;
    Ok(output_file.to_path_buf())
}

pub fn run_assembler(input_file: &PathBuf, args: crate::Args) -> Result<PathBuf, DriverError> {
    let mut output_file = input_file.clone();
    output_file.set_extension("");
    let mut gcc_args = vec![];
    if args.c {
        gcc_args.push("-c");
        output_file.set_extension("o");
    }
    gcc_args.push(input_file.to_str().unwrap());
    gcc_args.append(&mut vec!["-o", &output_file.to_str().unwrap()]);
    match Command::new("gcc")
        .args(gcc_args)
        .output() {
            Ok(output) => {
                if output.status.success() {
                    let stdout_str = String::from_utf8_lossy(&output.stdout).into_owned();
                    println!("{}", stdout_str);
                    Ok(output_file.to_path_buf())
                } else {
                    let msg = String::from_utf8_lossy(&output.stderr).into_owned();
                    Err(DriverError::AssemblerError(msg))
                }
            },
            Err(e) => Err(DriverError::AssemblerError(e.to_string()))
        }
}

