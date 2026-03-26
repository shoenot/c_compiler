use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::parser::*;

#[derive(Debug)]
pub enum SemanticError {
    UseBeforeDeclaration(String),
    InvalidLValue,
    DoubleDeclaration,
    UndeclaredLabel(String),
    DuplicateLabel(String),
    LabelBeforeDeclaration(String),
    LabelWithoutStatement,
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
            SemanticError::LabelWithoutStatement => write!(f, "Label without statement after it"),
            SemanticError::LabelBeforeDeclaration(n) => write!(f, "Using label {} before a declaration", n),
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
    loop_labeling_pass(program)?;
    switch_collection_pass(program)?;
    Ok(map)
}

// Variable and label resolution pass
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
            BlockItem::D(d) => resolve_declaration(d, var_map, counter)?,
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

fn resolve_declaration(declaration: &mut Declaration,
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
        resolve_declaration(dec, var_map, counter)?;
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

// // Label semantic analysis pass 
// fn check_label_before_dec(items: &Vec<BlockItem>) -> Result<(), SemanticError> {
//     // Check if label is before a declaration 
//     for i in 0..items.len() {
//         if let BlockItem::S(Statement::Compound(block)) = &items[i] {
//             check_label_before_dec(&block.items)?;
//         }
//
//         if i + 1 < items.len() {
//             if let BlockItem::S(Statement::Label(name, _)) = &items[i] {
//                 if let BlockItem::D(_) = &items[i+1] {
//                     return Err(SemanticError::LabelBeforeDeclaration(name.clone()));
//                 }
//             } 
//         }
//     }
//
//     // Check if label is the last statement in a block
//     if let Some(BlockItem::S(Statement::Label(_, _))) = items.last() {
//         return Err(SemanticError::LabelWithoutStatement);
//     }
//     Ok(())
// }
//
// fn label_semantic_analysis_pass(program: &mut Program) -> Result<(), SemanticError> {
//     let items = &program.function.body.items;
//     check_label_before_dec(items)
// }

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


fn variable_resolution_pass(program: &mut Program) -> Result<HashMap<String, (String, usize)>, SemanticError>{
    let mut var_map = HashMap::new();
    let mut label_map = HashMap::new();
    let mut counter = Counter{count: 0, current_block: 1};
    resolve_program_vars(program, &mut var_map, &mut label_map, &mut counter)?;
    check_undeclared_label(label_map)?;
    Ok(var_map)
} 

// Loop labeling pass
struct LoopCounter {
    counter: usize,
    stack: Vec<String>,
    loopstack: Vec<String>,
    switchstack: Vec<String>,
}

impl LoopCounter {
    fn genlabel(&mut self) -> String {
        let label = format!("loop.{}", self.counter);
        self.counter += 1;
        label
    }
}

fn loop_labeling_pass(program: &mut Program) -> Result<(), SemanticError> {
    let mut count = LoopCounter{ 
        counter: 0, 
        stack: Vec::new(), 
        loopstack: Vec::new(),
        switchstack: Vec::new(),
    };
    assign_loop_labels(&mut program.function.body.items, &mut count)?;
    Ok(())
}

fn assign_label(st: &mut Statement, count: &mut LoopCounter) -> Result<(), SemanticError> {
    match st {
        Statement::DoWhile { body, cond:_, lab } |
        Statement::While { cond:_, body, lab } |
        Statement::For { init:_, cond:_, post:_, body, lab } => {
            let newlab = count.genlabel();
            *lab = newlab.clone();
            count.stack.push(newlab.clone());
            count.loopstack.push(newlab);
            assign_label(body, count)?;
            count.loopstack.pop();
            count.stack.pop();
        },
        Statement::Break(lab) => {
            if let Some(newlab) = count.stack.last() {
                *lab = newlab.into();
            } else {
                return Err(SemanticError::BreakOutsideLoopOrSwitch);
            }
        },
        Statement::Continue(lab) => {
            if let Some(newlab) = count.loopstack.last() {
                *lab = newlab.into();
            } else {
                return Err(SemanticError::ContOutsideLoop);
            }
        },
        Statement::Compound(block) => {
            assign_loop_labels(&mut block.items, count)?;
        },
        Statement::Label(_, body) => {
            assign_label(body, count)?;
        }
        Statement::If(_, yes, no) => {
            assign_label(yes, count)?;
            if let Some(no) = no {
                assign_label(no, count)?;
            }
        },
        Statement::Switch { scrutinee:_, body, lab, cases:_ } => {
            let newlab = count.genlabel();
            *lab = newlab.clone();
            count.stack.push(newlab.clone());
            count.switchstack.push(newlab);
            assign_label(body, count)?;
            count.switchstack.pop();
            count.stack.pop();
        },
        Statement::Case { expr:_, lab } => {
            if count.switchstack.is_empty() {
                return Err(SemanticError::CaseOutsideSwitch);
            }
            *lab = count.genlabel();
        },
        Statement::Default { lab } => {
            if count.switchstack.is_empty() {
                return Err(SemanticError::CaseOutsideSwitch);
            }
            *lab = count.genlabel();
        },
        _ => {},
    }
    Ok(())
}

fn assign_loop_labels(items: &mut Vec<BlockItem>, count: &mut LoopCounter) -> Result<(), SemanticError> {
    for item in items {
        if let BlockItem::S(st) = item {
            assign_label(st, count)?;
        }
    }
    Ok(())
}

fn collect_cases_in_block(items: &Vec<BlockItem>) -> Result<Vec<(Option<Expression>, String)>, SemanticError> {
    let mut cases = Vec::new();
    for item in items {
        match item {
            BlockItem::S(Statement::Case { expr, lab }) => {
                cases.push((Some(expr.clone()), lab.clone()));
            },
            BlockItem::S(Statement::Default { lab }) => {
                cases.push((None, lab.clone()));
            },
            BlockItem::S(Statement::Compound(bl)) => {
                cases.append(&mut collect_cases_in_block(&bl.items)?);
            },
            BlockItem::S(Statement::If(_, yes, no)) => {
                cases.append(&mut collect_switch_cases(yes)?);
                if let Some(no) = no {
                    cases.append(&mut collect_switch_cases(no)?);
                }
            },
            BlockItem::S(Statement::Label(_, body)) => {
                cases.append(&mut collect_switch_cases(body)?);
            },
            BlockItem::S(Statement::For {body,..}) |
            BlockItem::S(Statement::While {body,..}) |
            BlockItem::S(Statement::DoWhile {body,..}) => {
                if let Statement::Compound(bl) = body.as_ref() {
                    cases.append(&mut collect_cases_in_block(&bl.items)?);
                } else {
                    cases.append(&mut collect_switch_cases(body)?);
                }
            },
            BlockItem::S(_) | BlockItem::D(_) => {},
        }
    }
    Ok(cases)
}

fn collect_switch_cases(st: &Statement) -> Result<Vec<(Option<Expression>, String)>, SemanticError> {
    let mut cases = Vec::new();
    if let Statement::Compound(block) = st {
        cases.append(&mut collect_cases_in_block(&block.items)?);
    } else if let Statement::Case { expr, lab } = st {
        cases.push((Some(expr.clone()), lab.clone()));
    } else if let Statement::Default { lab } = st {
        cases.push((None, lab.clone()));
    }
    Ok(cases)
}

fn statement_collector(st: &mut Statement) -> Result<(), SemanticError> {
        match st {
            Statement::Switch { cases, body, .. } => {
                *cases = collect_switch_cases(body)?;   

                let mut seen = HashSet::new();
                for (expr, _) in cases.iter() {
                    if let Some(Expression::Constant(value)) = expr {
                        if !seen.insert(value) {
                            return Err(SemanticError::DuplicateCase);
                        }
                    }
                }

                let default_count = cases.iter().filter(|(expr,_)| expr.is_none()).count();
                if default_count > 1 { return Err(SemanticError::DuplicateDefault); }

                // if let Statement::Compound(block) = body.as_ref() {
                //     check_block_for_decs(&block)?;
                // } 
                statement_collector(body)?;
            },
            Statement::If(_, yes, no) => {
                statement_collector(yes)?;
                if let Some(no) = no {
                    statement_collector(no)?;
                }
            },
            Statement::Label(_, body) => {
                statement_collector(body)?;
            },
            Statement::While { body, .. } => {
                statement_collector(body)?;
            },
            Statement::DoWhile { body, .. } => {
                statement_collector(body)?;
            },
            Statement::For { body, .. } => {
                statement_collector(body)?;
            },
            Statement::Compound(bl) => {
                block_collector(&mut bl.items)?;
            }
            _ => {}
        }
    Ok(())
}

fn block_collector(items: &mut Vec<BlockItem>) -> Result<(), SemanticError> {
    for item in items {
        if let BlockItem::S(st) = item {
            statement_collector(st)?;
        }
    }
    Ok(())
}

// fn check_block_for_decs(block: &Block) -> Result<(), SemanticError> {
//     for item in &block.items {
//         match item {
//             BlockItem::D(Declaration { init:Some(_),.. }) => return Err(SemanticError::DecInCase),
//             _ => {}
//         }
//     }
//     Ok(())
// }

fn switch_collection_pass(program: &mut Program) -> Result<(), SemanticError> {
    block_collector(&mut program.function.body.items)?;
    Ok(())
}
