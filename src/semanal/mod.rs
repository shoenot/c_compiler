use std::collections::HashMap;
use std::fmt;

use crate::parser::*;

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
    UseBeforeDeclaration(String),
    InvalidLValue,
    DoubleDeclaration(String),
    NestedFunctionDefinition(String),
    UndeclaredLabel(String),
    DuplicateLabel(String),
    BreakOutsideLoopOrSwitch,
    CaseOutsideSwitch,
    ContOutsideLoop,
    NonConstantCase,
    DuplicateCase,
    DuplicateDefault,
    DecInCase,
    IncompatibleFuncDeclaration(String),
    FuncCalledWithWrongNumArgs(String),
    VarCalledAsFunc(String),
    FuncUsedAsVar(String),
    StaticAfterNonStatic(String),
    NonConstantInitializer(String),
    ConflictingStorageTypes(String),
    ConflictingDefinitions(String),
    LocalStaticVarNonConstantInit(String),
    InitializerOnLocalExtern(String),
    NonGlobalStaticFunc(String),
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SemanticError::UseBeforeDeclaration(n) => write!(f, "Used {} before it was declared", n),
            SemanticError::InvalidLValue => write!(f, "Invalid lvalue"),
            SemanticError::DoubleDeclaration(n) => write!(f, "Duplicate declaration of {}", n),
            SemanticError::NestedFunctionDefinition(n) => write!(f, "Nested declaration of {}", n),
            SemanticError::UndeclaredLabel(n) => write!(f, "Undeclared label {}", n),
            SemanticError::DuplicateLabel(n) => write!(f, "Duplicate label {}", n),
            SemanticError::BreakOutsideLoopOrSwitch => write!(f, "Break outside loop/switch"),
            SemanticError::CaseOutsideSwitch => write!(f, "Case outside switch"),
            SemanticError::ContOutsideLoop => write!(f, "Cont outside loop"),
            SemanticError::NonConstantCase => write!(f, "Non constant case"),
            SemanticError::DuplicateCase => write!(f, "Duplicate case"),
            SemanticError::DuplicateDefault => write!(f, "Duplicate label"),
            SemanticError::DecInCase => write!(f, "Dec in case"),
            SemanticError::IncompatibleFuncDeclaration(n) => write!(f, "Incompatible Function Declaration {}", n),
            SemanticError::FuncCalledWithWrongNumArgs(n) => write!(f, "Function {} called with wrong number of args", n),
            SemanticError::VarCalledAsFunc(n) => write!(f, "Variable {} called as a function", n),
            SemanticError::FuncUsedAsVar(n) => write!(f, "Function {} used as a variable", n),
            SemanticError::StaticAfterNonStatic(n) => write!(f, "Static function declaration {} follows non-static", n),
            SemanticError::NonConstantInitializer(n) => write!(f, "Non constant initializer {}", n),
            SemanticError::ConflictingStorageTypes(n) => write!(f, "Conflicting storage types {}", n),
            SemanticError::ConflictingDefinitions(n) => write!(f, "Conflicting definitions {}", n),
            SemanticError::LocalStaticVarNonConstantInit(n) => write!(f, "Local static variable with non-constant init {}", n),
            SemanticError::InitializerOnLocalExtern(n) => write!(f, "Init on local external variable {}", n),
            SemanticError::NonGlobalStaticFunc(n) => write!(f, "Non global static function {}", n),
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
