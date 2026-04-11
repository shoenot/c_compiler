use std::error::Error;
use std::fs;
use std::fmt;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::lexer::*;
use crate::parser::*;
use crate::semanal::*;
// use crate::poise::*;
// use crate::codegen::*;
// use crate::emit::*;
use crate::types::*;

pub mod gcc;
pub use gcc::*;

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

fn load_source(input_file: &Path) -> Result<String, std::io::Error> {
    let source = fs::read_to_string(input_file)?;
    Ok(source)
}

fn run_lexer(preprocessed: &Path, args: &crate::Args) -> Result<Vec<Token>, Box<dyn Error>> {
    let source = load_source(preprocessed)?;
    let mut tokenizer = Tokenizer::new(source);
    let tokens = tokenizer.tokenize()?;
    std::fs::remove_file(preprocessed)?;
    if args.lex {
        for item in tokens {
            println!("{:?}", item);
        };
        std::process::exit(0);
    } 
    Ok(tokens)
}

fn run_parser(tokens: Vec<Token>, args: &crate::Args) -> Result<Program, Box<dyn Error>> {
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;
    if args.parse {
        for decl in program.declarations {
            match decl {
                Decl::FuncDecl(f) => {
                    println!("Function: {}", f.identifier);
                    println!("Parameters: {:?}", f.params);
                    println!("Storage Specifiers: {:?}", f.storage);
                    if let Some(b) = f.body {
                        for item in b.items {
                            println!("{:?}", item);
                        }
                    }
                },
                Decl::VarDecl(v) => {
                    println!("Top Level Variable: {:?}", v);
                }
            }
        }
        std::process::exit(0);
    }

    Ok(program)
}

fn run_semanal(program: &mut Program, symbols: &mut SymbolTable, args: &crate::Args) -> Result<(), Box<dyn Error>> {
    let var_map = semantic_analysis(program, symbols)?;
    if args.validate {
        for decl in &program.declarations {
            match decl {
                Decl::FuncDecl(f) => {
                    println!("Function: {}", f.identifier);
                    println!("Parameters: {:?}", f.params);
                    println!("Storage Specifiers: {:?}", f.storage);
                    if let Some(b) = &f.body {
                        for item in &b.items {
                            println!("{:?}", item);
                        }
                    }
                },
                Decl::VarDecl(v) => {
                    println!("Top Level Variable: {:?}", v);
                }
            }
        }
        println!("--------------- VAR MAP ---------------");
        for item in var_map {
            println!("{:?}", item);
        }
        println!("--------------- SYMBOLS ---------------");
        for item in symbols {
            println!("{:?}", item);
        }
        std::process::exit(0);
    }
    Ok(())
}

// fn run_poise(program: Program, symbols: &mut SymbolTable, args: &crate::Args) -> PoiseProg {
//     let poise = gen_poise(&program, symbols);
//     if args.tacky {
//         let mut vars = Vec::new();
//         for item in poise.top_level_items  {
//             match item {
//                 TopLevelItem::F(f) => {
//                     println!("Function: {}", f.identifier);
//                     println!("Parameters: {:?}", f.params);
//                     println!("Global: {:?}", f.global);
//                     for instruction in f.body {
//                         println!("{:?}", instruction);
//                     }
//                     println!("\n");
//                 },
//                 TopLevelItem::V(v) => {
//                     vars.push(v);
//                 }
//             }
//         }
//         println!("------------- STATIC VARS -------------");
//         for item in vars {
//             println!("Top Level Variable: {:?}", item);
//         }
//         println!("\n");
//         println!("--------------- SYMBOLS ---------------");
//         for item in symbols {
//             println!("{:?}", item);
//         }
//         std::process::exit(0);
//     }
//     poise
// }
//
// fn run_codegen(program: PoiseProg, symbols:SymbolTable, asm_symbols: AsmSymbolTable, args: &crate::Args) -> (AsmProgram, AsmSymbolTable) {
//     let (asm, asm_symbols) = gen_program(program, symbols.clone(), asm_symbols);
//     if args.codegen {
//         let mut vars = Vec::new();
//         let mut floats = Vec::new();
//         for item in asm.top_level  {
//             match item {
//                 AsmTopLevel::F(f) => {
//                     println!("Function: {}", f.identifier);
//                     println!("Global: {:?}", f.global);
//                     for instruction in f.body {
//                         println!("{:?}", instruction);
//                     }
//                     println!("\n");
//                 },
//                 AsmTopLevel::V(v) => {
//                     vars.push(v);
//                 }
//                 AsmTopLevel::C(c) => {
//                     floats.push(c);
//                 }
//             }
//         }
//         println!("------------- STATIC VARS -------------");
//         for item in vars {
//             println!("Top Level Variable: {:?}", item);
//         }
//         println!("\n");
//         println!("--------------- FLOATS ----------------");
//         for item in floats {
//             println!("Float constant: {:?}", item);
//         }
//         println!("--------------- SYMBOLS ---------------");
//         for item in symbols {
//             println!("{:?}", item);
//         }
//         println!("------------- ASM SYMBOLS -------------");
//         for item in asm_symbols {
//             println!("{:?}", item);
//         }
//         std::process::exit(0);
//     }
//     (asm, asm_symbols)
// }
//
// fn run_emitter(asm_program: AsmProgram, symbols: &mut AsmSymbolTable, output_file: &PathBuf) -> Result<PathBuf, Box<dyn Error>> {
//     fs::write(&output_file, emit_program(asm_program, symbols)?)?;
//     Ok(output_file.to_path_buf())
// }

pub fn run_compiler(input_file: &Path, args: crate::Args) -> Result<PathBuf, Box<dyn Error>> {
    let preprocessed = input_file;
    let mut symbols = HashMap::new();
    // let asm_symbols = HashMap::new();

    let lexed = run_lexer(preprocessed, &args)?;
    let mut parsed = run_parser(lexed, &args)?;
    run_semanal(&mut parsed, &mut symbols, &args)?;
    // let poise = run_poise(parsed, &mut symbols, &args);
    // let (asm, mut asm_symbols) = run_codegen(poise, symbols, asm_symbols, &args);

    let mut output_file = input_file.to_path_buf();
    output_file.set_extension("s");
    // run_emitter(asm, &mut asm_symbols, &output_file)?;

    Ok(output_file.to_path_buf())
}

