use std::collections::HashMap;
use std::fmt;

use crate::parser::*;

#[derive(Debug)]
pub enum SemanticError {
    UseBeforeDeclaration(String),
    InvalidLValue,
    InvalidExpression,
    DoubleDeclaration,
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SemanticError::UseBeforeDeclaration(n) => write!(f, "Used {} before it was declared", n),
            SemanticError::InvalidLValue => write!(f, "Invalid lvalue"),
            SemanticError::InvalidExpression => write!(f, "Invalid expression"),
            SemanticError::DoubleDeclaration => write!(f, "Variable declared"),
        }
    }
}

impl std::error::Error for SemanticError {}

struct Counter {
    count: usize,
}

impl Counter {
    fn namegen(&mut self, name: &str) -> String {
        let new = format!("{}.{}", name, self.count);
        self.count += 1;
        new
    }
}

fn resolve_program_vars(program: &mut Program,
    var_map: &mut HashMap<String, String>,
    counter: &mut Counter) -> Result<(), SemanticError> {
    for blockitem in &mut program.function.body {
        match blockitem {
            BlockItem::S(s) => resolve_statement(s, var_map, counter)?,
            BlockItem::D(d) => resolve_declaration(d, var_map, counter)?,
        }
    }
    Ok(())
}

fn resolve_statement(statement: &mut Statement,
    var_map: &mut HashMap<String, String>,
    counter: &mut Counter) -> Result<(), SemanticError> {
    match statement {
        Statement::Return(e) => resolve_expression(e, var_map, counter)?,
        Statement::Expression(e) => resolve_expression(e, var_map, counter)?,
        Statement::If(c,y,mn) => {
            resolve_expression(c, var_map, counter)?;
            resolve_statement(y, var_map, counter)?;
            if let Some(n) = mn {
                resolve_statement(n, var_map, counter)?;
            }
        },
        Statement::Null => return Ok(()),
    }
    Ok(())
}

fn resolve_declaration(declaration: &mut Declaration,
    var_map: &mut HashMap<String, String>,
    counter: &mut Counter) -> Result<(), SemanticError> {

    if var_map.contains_key(&declaration.identifier) {
        return Err(SemanticError::DoubleDeclaration)
    } else {
        let newname = counter.namegen(&declaration.identifier);
        var_map.insert(declaration.identifier.clone(), newname.clone());
        declaration.identifier = newname;
    }

    match &mut declaration.init {
        None => return Ok(()),
        Some(e) => {
            resolve_expression(e, var_map, counter)?;
            Ok(())
        },
    }
}

fn resolve_expression(expression: &mut Expression,
    var_map: &mut HashMap<String, String>,
    counter: &mut Counter) -> Result<(), SemanticError> {
    match expression {
        Expression::Var(x) => {
            if let Some(name) = var_map.get(x) {
                *x = name.into();
            } else {
                return Err(SemanticError::UseBeforeDeclaration(x.clone()));
            }
        },
        Expression::Assignment(lhs, rhs) => {
            match lhs.as_mut() {
                Expression::Var(x) => {
                    if let Some(name) = var_map.get(x.as_str()) {
                        *x = name.clone();
                    } else {
                        return Err(SemanticError::UseBeforeDeclaration(x.to_string()));
                    }
                },
                _ => {
                    eprintln!("Invalid L-value: {:?}", lhs);
                    return Err(SemanticError::InvalidLValue);
                }
            }
            resolve_expression(rhs.as_mut(), var_map, counter)?;
        },
        Expression::Unary(_, exp) => resolve_expression(exp.as_mut(), var_map, counter)?,
        Expression::Binary(_, exp1, exp2) => {
            resolve_expression(exp1.as_mut(), var_map, counter)?;
            resolve_expression(exp2.as_mut(), var_map, counter)?;
        },
        Expression::Conditional(exp1, exp2, exp3) => {
            resolve_expression(exp1.as_mut(), var_map, counter)?;
            resolve_expression(exp2.as_mut(), var_map, counter)?;
            resolve_expression(exp3.as_mut(), var_map, counter)?;
        },
        Expression::Constant(_) => return Ok(()),
    }
    Ok(())
}

pub fn semantic_analysis(program: &mut Program) -> Result<HashMap<String, String>, SemanticError>{
    let mut var_map = HashMap::new();
    let mut counter = Counter{count: 0};
    resolve_program_vars(program, &mut var_map, &mut counter)?;
    Ok(var_map)
}
