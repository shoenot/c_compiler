use std::collections::HashMap;
use std::fmt;

use crate::parser::*;

#[derive(Debug)]
pub enum SemanticError {
    UseBeforeDeclaration(String),
    InvalidLValue,
    InvalidExpression,
    DoubleDeclaration,
    UndeclaredLabel(String),
    DuplicateLabel(String),
    LabelBeforeDeclaration(String),
    LabelWithoutStatement,
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SemanticError::UseBeforeDeclaration(n) => write!(f, "Used {} before it was declared", n),
            SemanticError::InvalidLValue => write!(f, "Invalid lvalue"),
            SemanticError::InvalidExpression => write!(f, "Invalid expression"),
            SemanticError::DoubleDeclaration => write!(f, "Variable declared"),
            SemanticError::UndeclaredLabel(n) => write!(f, "Undeclared label {}", n),
            SemanticError::DuplicateLabel(n) => write!(f, "Duplicate label {}", n),
            SemanticError::LabelWithoutStatement => write!(f, "Label without statement after it"),
            SemanticError::LabelBeforeDeclaration(n) => write!(f, "Using label {} before a declaration", n),
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
    
    fn labelgen(&mut self, name: &str) -> String {
        format!(".Luserlab.{}", name)
    }
}

fn resolve_program_vars(program: &mut Program,
    var_map: &mut HashMap<String, String>,
    label_map: &mut HashMap<String, (String, bool)>,
    counter: &mut Counter) -> Result<(), SemanticError> {
    for blockitem in &mut program.function.body.items {
        match blockitem {
            BlockItem::S(s) => resolve_statement(s, var_map, label_map, counter)?,
            BlockItem::D(d) => resolve_declaration(d, var_map, counter)?,
        }
    }
    Ok(())
}

fn process_label(name: &mut String,
    label_map: &mut HashMap<String, (String, bool)>,
    counter: &mut Counter) -> Result<(), SemanticError> {
    let labelname = name.clone();
    if let Some((mangled, status)) = label_map.get_mut(&labelname) {
        let mangled = mangled.clone();
        if *status {
            return Err(SemanticError::DuplicateLabel(mangled));
        } else {
            *status = true;
            *name = mangled;
            Ok(())
        }
    } else {
        let newlabel = counter.labelgen(&labelname);
        label_map.insert(labelname, (newlabel.clone(), true));
        *name = newlabel;
        Ok(())
    }
}

fn process_goto(name: &mut String,
    label_map: &mut HashMap<String, (String, bool)>,
    counter: &mut Counter) -> Result<(), SemanticError> {
    let labelname = name.clone();
    if label_map.contains_key(&labelname) {
        let (newname, _) = label_map.get(&labelname).unwrap();
        *name = String::from(newname);
        Ok(())
    } else {
        let newlabel = counter.labelgen(&labelname);
        label_map.insert(labelname, (newlabel.clone(), false));
        *name = newlabel;
        Ok(())
    }
}

fn resolve_statement(statement: &mut Statement,
    var_map: &mut HashMap<String, String>,
    label_map: &mut HashMap<String, (String, bool)>,
    counter: &mut Counter) -> Result<(), SemanticError> {
    match statement {
        Statement::Return(e) => resolve_expression(e, var_map, counter)?,
        Statement::Expression(e) => resolve_expression(e, var_map, counter)?,
        Statement::If(c,y,mn) => {
            resolve_expression(c, var_map, counter)?;
            resolve_statement(y, var_map, label_map, counter)?;
            if let Some(n) = mn {
                resolve_statement(n, var_map, label_map, counter)?;
            }
        },
        Statement::Null => return Ok(()),
        Statement::Label(name) => process_label(name, label_map, counter)?,
        Statement::Goto(name) => process_goto(name, label_map, counter)?,
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
                Expression::Var(_) => {},
                _ => {
                    eprintln!("Invalid L-value: {:?}", lhs);
                    return Err(SemanticError::InvalidLValue);
                }
            }
            resolve_expression(lhs.as_mut(), var_map, counter)?;
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
        Expression::PostfixIncrement(exp) | Expression::PrefixIncrement(exp) | 
        Expression::PostfixDecrement(exp) | Expression::PrefixDecrement(exp) => {
            match **exp {
                Expression::Var(_) => resolve_expression(exp.as_mut(), var_map, counter)?,
                _ => return Err(SemanticError::InvalidLValue),
            }
        },
        Expression::Constant(_) => return Ok(()),
    }
    Ok(())
}

pub fn duplicate_label_check(label_map: HashMap<String, (String, bool)>) -> Result<(), SemanticError> {
    for (key, (_, status)) in &label_map {
        if !status {
            return Err(SemanticError::UndeclaredLabel(key.clone()));
        }
    }
    Ok(())
}

pub fn check_label_before_dec(program: &Program) -> Result<(), SemanticError> {
    let items = &program.function.body;
    for i in 0..(items.len().saturating_sub(1)) {
        if let BlockItem::S(Statement::Label(name)) = &items[i] {
            if let BlockItem::D(_) = &items[i+1] {
                return Err(SemanticError::LabelBeforeDeclaration(name.clone()));
            }
        }
    }
    if let Some(BlockItem::S(Statement::Label(_))) = items.last() {
        return Err(SemanticError::LabelWithoutStatement);
    }
    Ok(())
}

pub fn semantic_analysis(program: &mut Program) -> Result<HashMap<String, String>, SemanticError>{
    let mut var_map = HashMap::new();
    let mut label_map = HashMap::new();
    let mut counter = Counter{count: 0};
    resolve_program_vars(program, &mut var_map, &mut label_map, &mut counter)?;
    duplicate_label_check(label_map)?;
    check_label_before_dec(program)?;
    Ok(var_map)
}

