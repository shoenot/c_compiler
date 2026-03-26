// Var and label resolution pass
use std::collections::hash_map::HashMap;
use crate::parser::*;
use super::SemanticError;

struct Counter {
    count: usize,
    current_block: usize,
}

impl Counter {
    fn namegen(&mut self, name: &str) -> String {
        let new = format!("{}.{}_{}", name, self.count, self.current_block);
        self.count += 1;
        new
    }
    
    fn labelgen(&mut self, name: &str) -> String {
        format!("userlab.{}", name)
    }
}

fn resolve_block(block: &mut Block,
    var_map: &mut HashMap<String, (String, usize)>,
    label_map: &mut HashMap<String, (String, bool)>,
    counter: &mut Counter) -> Result<(), SemanticError> {
    for blockitem in &mut block.items {
        match blockitem {
            BlockItem::S(s) => resolve_statement(s, var_map, label_map, counter)?,
            BlockItem::D(Decl::VarDecl(d)) => resolve_var_declaration(d, var_map, counter)?,
        }
    }
    Ok(())
}

fn resolve_program_vars(program: &mut Program,
    var_map: &mut HashMap<String, (String, usize)>,
    label_map: &mut HashMap<String, (String, bool)>,
    counter: &mut Counter) -> Result<(), SemanticError> {
    resolve_block(&mut program.function.body, var_map, label_map, counter)?;
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
    var_map: &mut HashMap<String, (String, usize)>,
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
        Statement::Label(name, st) => {
            process_label(name, label_map, counter)?;
            resolve_statement(st, var_map, label_map, counter)?;
        },
        Statement::Goto(name) => process_goto(name, label_map, counter)?,
        Statement::Compound(block) => {
            let mut new_map = var_map.clone();
            counter.current_block += 1;
            resolve_block(block, &mut new_map, label_map, counter)?;
            counter.current_block -= 1;
        },
        Statement::While { cond, body, lab:_ } |
        Statement::DoWhile { body, cond, lab:_ } => {
            resolve_expression(cond, var_map, counter)?;
            resolve_statement(body, var_map, label_map, counter)?;
        },
        Statement::For { init, cond, post, body, lab:_ } => {
            let mut new_map = var_map.clone();
            counter.current_block += 1;
            resolve_for_init(init, &mut new_map, counter)?;
            if let Some(cond) = cond { resolve_expression(cond, &mut new_map, counter)?; }
            if let Some(post) = post { resolve_expression(post, &mut new_map, counter)?; }
            resolve_statement(body, &mut new_map, label_map, counter)?;
            counter.current_block -= 1;
        },
        Statement::Switch { scrutinee, body, .. } => {
            resolve_expression(scrutinee, var_map, counter)?;
            resolve_statement(body, var_map, label_map, counter)?;
        },
        Statement::Case{expr, ..} => {
            match eval_constant(expr) {
                Some(value) => *expr = Expression::Constant(value),
                None => return Err(SemanticError::NonConstantCase),
            }
        },
        Statement::Null | Statement::Break(_) | 
        Statement::Continue(_) | Statement::Default{..} => {},
    }
    Ok(())
}

fn resolve_var_declaration(declaration: &mut VarDeclaration,
    var_map: &mut HashMap<String, (String, usize)>,
    counter: &mut Counter) -> Result<(), SemanticError> {

    if let Some((_, blk)) = var_map.get(&declaration.identifier) {
        if *blk == counter.current_block {
            return Err(SemanticError::DoubleDeclaration)
        }
    }

    let newname = counter.namegen(&declaration.identifier);
    var_map.insert(declaration.identifier.clone(), (newname.clone(), counter.current_block));
    declaration.identifier = newname;

    match &mut declaration.init {
        None => return Ok(()),
        Some(e) => {
            resolve_expression(e, var_map, counter)?;
            Ok(())
        },
    }
}

fn resolve_expression(expression: &mut Expression,
    var_map: &mut HashMap<String, (String, usize)>,
    counter: &mut Counter) -> Result<(), SemanticError> {
    match expression {
        Expression::Var(x) => {
            if let Some((name, _)) = var_map.get(x) {
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

fn resolve_for_init(init: &mut ForInit, var_map: &mut HashMap<String, (String, usize)>, counter: &mut Counter) -> Result<(), SemanticError> {
    if let ForInit::InitDec(dec) = init {
        resolve_var_declaration(dec, var_map, counter)?;
    } else if let ForInit::InitExp(Some(exp)) = init {
        resolve_expression(exp, var_map, counter)?;
    }
    Ok(())
}

fn check_undeclared_label(label_map: HashMap<String, (String, bool)>) -> Result<(), SemanticError> {
    for (key, (_, status)) in &label_map {
        if !status {
            return Err(SemanticError::UndeclaredLabel(key.clone()));
        }
    }
    Ok(())
}

fn eval_constant(expr: &Expression) -> Option<i32> {
    match expr {
        Expression::Constant(n) => Some(*n),
        Expression::Unary(op, expr) => {
            let val = eval_constant(expr)?;
            match op {
                UnaryOp::Negate => Some(-val),
                UnaryOp::Complement => Some(!val),
                UnaryOp::Not => Some((val == 0) as i32),
            }
        },
        Expression::Binary(op, left, right) => {
            let l = eval_constant(left)?;
            let r = eval_constant(right)?;
            match op {
                BinaryOp::Add             => Some(l + r),
                BinaryOp::Subtract        => Some(l - r),
                BinaryOp::Multiply        => Some(l * r),
                BinaryOp::Divide          => if r == 0 { None } else { Some(l / r) },
                BinaryOp::Remainder       => if r == 0 { None } else { Some(l % r) },
                BinaryOp::LeftShift       => Some(l << r),
                BinaryOp::RightShift      => Some(l >> r),
                BinaryOp::LessThan        => Some((l < r) as i32),
                BinaryOp::LessOrEqual     => Some((l <= r) as i32),
                BinaryOp::GreaterThan     => Some((l > r) as i32),
                BinaryOp::GreaterOrEqual  => Some((l >= r) as i32),
                BinaryOp::Equal           => Some((l == r) as i32),
                BinaryOp::NotEqual        => Some((l != r) as i32),
                BinaryOp::BitwiseAnd      => Some(l & r),
                BinaryOp::BitwiseXor      => Some(l ^ r),
                BinaryOp::BitwiseOr       => Some(l | r),
                BinaryOp::LogicalAnd      => Some((l != 0 && r != 0) as i32),
                BinaryOp::LogicalOr       => Some((l != 0 || r != 0) as i32),
                _ => None,
            }
        },
        _ => None,
    }
}


pub fn variable_resolution_pass(program: &mut Program) -> Result<HashMap<String, (String, usize)>, SemanticError>{
    let mut var_map = HashMap::new();
    let mut label_map = HashMap::new();
    let mut counter = Counter{count: 0, current_block: 1};
    resolve_program_vars(program, &mut var_map, &mut label_map, &mut counter)?;
    check_undeclared_label(label_map)?;
    Ok(var_map)
} 

