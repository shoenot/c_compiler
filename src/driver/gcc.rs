// These functions call on GCC because my compiler doesn't do preprocessing/assembling (yet?)
use std::process::Command;
use super::*;
use std::path::{Path, PathBuf};

pub fn run_preprocessor(input_file: &Path) -> Result<PathBuf, DriverError> {
    let mut output_file = input_file.to_path_buf();
    output_file.set_extension("i");
    match Command::new("gcc")
        .args(["-E", "-P", &input_file.to_str().unwrap(), "-o", &output_file.to_str().unwrap()])
        .output() {
            Ok(output) => {
                if output.status.success() {
                    let stdout_str = String::from_utf8_lossy(&output.stdout).into_owned();
                    println!("{}", stdout_str);
                    Ok(output_file)
                } else {
                    let msg = String::from_utf8_lossy(&output.stderr).into_owned();
                    Err(DriverError::PreprocessorError(msg))
                }
            },
            Err(e) => Err(DriverError::PreprocessorError(e.to_string())),
        }
}

pub fn run_assembler(input_file: &Path, args: crate::Args) -> Result<PathBuf, DriverError> {
    let mut output_file = input_file.to_path_buf();
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
