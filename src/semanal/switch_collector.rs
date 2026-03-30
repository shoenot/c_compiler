use super::*;
use std::collections::hash_set::HashSet;
use visitor_trait::*;

struct SwitchCollector;

impl Visitor for SwitchCollector {
    fn visit_statement(&mut self, statement: &mut Statement) -> Result<(), SemanticError> {
        match statement {
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

                let default_count = cases.iter().filter(|(expr, _)| expr.is_none()).count();
                if default_count > 1 {
                    return Err(SemanticError::DuplicateDefault);
                }

                if let Statement::Compound(block) = body.as_mut() {
                    check_block_for_decs(block)?;
                }

                self.visit_statement(body)?;
            },
            _ => { walk_statement(self, statement)?; }
        }
        Ok(())
    }
}

fn collect_cases_in_block(items: &mut Vec<BlockItem>) -> Result<Vec<(Option<Expression>, String)>, SemanticError> {
    let mut cases = Vec::new();
    for item in items.iter_mut() {
        match item {
            BlockItem::S(Statement::Case { expr, lab }) => {
                match eval_constant(&expr) {
                    Some(value) => *expr = Expression::Constant(value),
                    None => return Err(SemanticError::NonConstantCase),
                }
                cases.push((Some(expr.clone()), lab.clone()));
            },
            BlockItem::S(Statement::Default { lab }) => {
                cases.push((None, lab.clone()));
            },
            BlockItem::S(Statement::Compound(bl)) => {
                cases.append(&mut collect_cases_in_block(&mut bl.items)?);
            },
            BlockItem::S(Statement::If(_, yes, no)) => {
                cases.append(&mut collect_switch_cases(yes.as_mut())?);
                if let Some(no) = no {
                    cases.append(&mut collect_switch_cases(no.as_mut())?);
                }
            },
            BlockItem::S(Statement::Label(_, body)) => {
                cases.append(&mut collect_switch_cases(body.as_mut())?);
            },
            BlockItem::S(Statement::For { body, .. }) |
            BlockItem::S(Statement::While { body, .. }) |
            BlockItem::S(Statement::DoWhile { body, .. }) => {
                cases.append(&mut collect_switch_cases(body.as_mut())?);
            },
            BlockItem::S(_) | BlockItem::D(_) => {},
        }
    }
    Ok(cases)
}

fn collect_switch_cases(st: &mut Statement) -> Result<Vec<(Option<Expression>, String)>, SemanticError> {
    let mut cases = Vec::new();
    if let Statement::Compound(block) = st {
        cases.append(&mut collect_cases_in_block(&mut block.items)?);
    } else if let Statement::Case { expr, lab } = st {
        cases.push((Some(expr.clone()), lab.clone()));
    } else if let Statement::Default { lab } = st {
        cases.push((None, lab.clone()));
    }
    Ok(cases)
}

fn check_block_for_decs(block: &mut Block) -> Result<(), SemanticError> {
    let items = &block.items;
    for window in items.windows(2) {
        if let [BlockItem::S(Statement::Case { .. } | Statement::Default { .. }), BlockItem::D(_)] = window {
            return Err(SemanticError::DecInCase);
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

pub fn switch_collection_pass(program: &mut Program) -> Result<(), SemanticError> {
    let mut collector = SwitchCollector{};
    collector.visit_program(program)
}
