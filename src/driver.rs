use std::process::Command;
use std::path::PathBuf;
use std::error::Error;
use std::fmt;
use std::fs::read_to_string;
use crate::lexer::Tokenizer;

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

fn load_source(input_file: PathBuf) -> Result<String, std::io::Error> {
    let source = read_to_string(input_file)?;
    Ok(source)
}

pub fn run_preprocessor(input_file: PathBuf) -> Result<PathBuf, DriverError> {
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

pub fn run_compiler(preprocessed: PathBuf, args: crate::Args) -> Result<(), Box<dyn Error>> {
    let source = load_source(preprocessed.clone())?;
    let mut tokenizer = Tokenizer::new(source);
    let tokens = tokenizer.tokenize();
    println!("{:?}", tokens);
    if args.lex {
        std::fs::remove_file(preprocessed).ok().unwrap();
        return Ok(());
    } else {
        std::fs::remove_file(preprocessed).ok().unwrap();
        todo!()
    }
}

pub fn run_assembler(input_file: PathBuf) -> Result<(), DriverError> {
    let mut output_file = input_file.clone();
    output_file.set_extension("i");
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

