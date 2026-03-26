use std::collections::HashMap;
use std::fmt;

use crate::parser::*;

mod resvars;
use resvars::variable_resolution_pass;

mod labels;
use labels::label_generation_pass;

#[derive(Debug)]
pub enum SemanticError {
    UseBeforeDeclaration(String),
    InvalidLValue,
    DoubleDeclaration,
    UndeclaredLabel(String),
    DuplicateLabel(String),
    BreakOutsideLoopOrSwitch,
    CaseOutsideSwitch,
    ContOutsideLoop,
    NonConstantCase,
    DuplicateCase,
    DuplicateDefault,
    DecInCase,
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SemanticError::UseBeforeDeclaration(n) => write!(f, "Used {} before it was declared", n),
            SemanticError::InvalidLValue => write!(f, "Invalid lvalue"),
            SemanticError::DoubleDeclaration => write!(f, "Variable declared"),
            SemanticError::UndeclaredLabel(n) => write!(f, "Undeclared label {}", n),
            SemanticError::DuplicateLabel(n) => write!(f, "Duplicate label {}", n),
            SemanticError::BreakOutsideLoopOrSwitch => write!(f, "Break outside loop/switch"),
            SemanticError::CaseOutsideSwitch => write!(f, "Case outside switch"),
            SemanticError::ContOutsideLoop => write!(f, "Cont outside loop"),
            SemanticError::NonConstantCase => write!(f, "Non constant case"),
            SemanticError::DuplicateCase => write!(f, "Duplicate case"),
            SemanticError::DuplicateDefault => write!(f, "Duplicate label"),
            SemanticError::DecInCase => write!(f, "Dec in case"),
        }
    }
}

impl std::error::Error for SemanticError {}

pub fn semantic_analysis(program: &mut Program) -> Result<HashMap<String, (String, usize)>, SemanticError> {
    let map = variable_resolution_pass(program)?;
    label_generation_pass(program)?;
    Ok(map)
}
