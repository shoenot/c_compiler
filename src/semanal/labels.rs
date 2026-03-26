// Loop and switch label generation pass
use crate::parser::*;
use super::SemanticError;
use std::collections::hash_set::HashSet;

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

pub fn label_generation_pass(program: &mut Program) -> Result<(), SemanticError> {
    loop_labeling_pass(program)?;
    switch_collection_pass(program)?;
    Ok(())
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

fn switch_collection_pass(program: &mut Program) -> Result<(), SemanticError> {
    block_collector(&mut program.function.body.items)?;
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

                if let Statement::Compound(block) = body.as_ref() {
                    check_block_for_decs(&block)?;
                } 
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

fn check_block_for_decs(block: &Block) -> Result<(), SemanticError> {
    let items = &block.items;
    for window in items.windows(2) {
        if let [BlockItem::S(Statement::Case { .. } | Statement::Default { .. }), BlockItem::D(_)] = window {
            return Err(SemanticError::DecInCase);
        }
    }
    Ok(())
}
