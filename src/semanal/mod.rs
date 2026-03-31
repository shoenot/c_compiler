use std::collections::HashMap;
use std::fmt;

use crate::parser::*;
use crate::types::*;
use crate::lexer::Span;

mod visitor_trait;

mod id_resolver;
pub use id_resolver::*;

mod switch_collector;
use switch_collector::*;

mod loop_labeler;
use loop_labeler::*;

mod type_checker;
pub use type_checker::*;

mod label_mangler;
use label_mangler::*;

#[derive(Debug)]
pub enum SemanticError {
    UseBeforeDeclaration(String, Span),
    InvalidLValue(Span),
    DoubleDeclaration(String, Span),
    NestedFunctionDefinition(String, Span),
    UndeclaredLabel(String, Span),
    DuplicateLabel(String, Span),
    BreakOutsideLoopOrSwitch(Span),
    CaseOutsideSwitch(Span),
    ContOutsideLoop(Span),
    NonConstantCase(Span),
    DuplicateCase(Span),
    DuplicateDefault(Span),
    DecInCase(Span),
    IncompatibleFuncDeclaration(String, Span),
    FuncCalledWithWrongNumArgs(String, Span),
    VarCalledAsFunc(String, Span),
    FuncUsedAsVar(String, Span),
    StaticAfterNonStatic(String, Span),
    NonConstantInitializer(String, Span),
    ConflictingStorageTypes(String, Span),
    ConflictingDefinitions(String, Span),
    LocalStaticVarNonConstantInit(String, Span),
    InitializerOnLocalExtern(String, Span),
    NonGlobalStaticFunc(String, Span),
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SemanticError::UseBeforeDeclaration(n, s) => write!(f, "Used '{}' before it was declared at {}", n, s),
            SemanticError::InvalidLValue(s) => write!(f, "Invalid lvalue at {}", s),
            SemanticError::DoubleDeclaration(n, s) => write!(f, "Duplicate declaration of '{}' at {}", n, s),
            SemanticError::NestedFunctionDefinition(n, s) => write!(f, "Nested declaration of '{}' at {}", n, s),
            SemanticError::UndeclaredLabel(n, s) => write!(f, "Undeclared label '{}' at {}", n, s),
            SemanticError::DuplicateLabel(n, s) => write!(f, "Duplicate label '{}' at {}", n, s),
            SemanticError::BreakOutsideLoopOrSwitch(s) => write!(f, "Break outside loop/switch at {}", s),
            SemanticError::CaseOutsideSwitch(s) => write!(f, "Case outside switch at {}", s),
            SemanticError::ContOutsideLoop(s) => write!(f, "Cont outside loop at {}", s),
            SemanticError::NonConstantCase(s) => write!(f, "Non constant case at {}", s),
            SemanticError::DuplicateCase(s) => write!(f, "Duplicate case at {}", s),
            SemanticError::DuplicateDefault(s) => write!(f, "Duplicate label at {}", s),
            SemanticError::DecInCase(s) => write!(f, "Dec in case at {}", s),
            SemanticError::IncompatibleFuncDeclaration(n, s) => write!(f, "Incompatible Function Declaration '{}' at {}", n, s),
            SemanticError::FuncCalledWithWrongNumArgs(n, s) => write!(f, "Function '{}' called with wrong number of args at {}", n, s),
            SemanticError::VarCalledAsFunc(n, s) => write!(f, "Variable '{}' called as a function at {}", n, s),
            SemanticError::FuncUsedAsVar(n, s) => write!(f, "Function '{}' used as a variable at {}", n, s),
            SemanticError::StaticAfterNonStatic(n, s) => write!(f, "Static function declaration '{}' follows non-static at {}", n, s),
            SemanticError::NonConstantInitializer(n, s) => write!(f, "Non constant initializer '{}' at {}", n, s),
            SemanticError::ConflictingStorageTypes(n, s) => write!(f, "Conflicting storage types '{}' at {}", n, s),
            SemanticError::ConflictingDefinitions(n, s) => write!(f, "Conflicting definitions '{}' at {}", n, s),
            SemanticError::LocalStaticVarNonConstantInit(n, s) => write!(f, "Local static variable with non-constant init '{}' at {}", n, s),
            SemanticError::InitializerOnLocalExtern(n, s) => write!(f, "Init on local external variable '{}' at {}", n, s),
            SemanticError::NonGlobalStaticFunc(n, s) => write!(f, "Non global static function '{}' at {}", n, s),
        }
    }
}

impl std::error::Error for SemanticError {}

pub fn semantic_analysis(program: &mut Program, symbols: &mut HashMap<String, Symbol>) 
    -> Result<HashMap<String, MapEntry>, SemanticError> {
    let map = identifier_resolution_pass(program)?;
    label_mangling_pass(program)?;
    loop_labeling_pass(program)?;
    switch_collection_pass(program)?;
    type_checking_pass(program, symbols)?;
    Ok(map)
}
