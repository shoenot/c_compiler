use super::*;
use std::{collections::hash_set::HashSet, i128};
use visitor_trait::*;

struct SwitchCollector;

impl Visitor for SwitchCollector {
    fn visit_statement(&mut self, statement: &mut Statement) -> Result<(), SemanticError> {
        match &mut statement.kind {
            StatementKind::Switch { cases, body, scrutinee, .. } => {
                let scrutinee_type = scrutinee.expression_type.as_ref().unwrap().clone();
                *cases = collect_switch_cases(body, &scrutinee_type)?;

                let mut seen: HashSet<i128> = HashSet::new();
                for expr in cases.iter().filter_map(|(opt, _)| opt.as_ref()) {
                    if let ExpressionKind::Constant(value) = &expr.kind {
                        let key = match value {
                            Const::Int(i) => *i as i128,
                            Const::Long(i) => *i as i128,
                            Const::UInt(i) => *i as i128,
                            Const::ULong(i) => *i as i128,
                        };
                        if !seen.insert(key) {
                            return Err(SemanticError::DuplicateCase(statement.span));
                        }
                    }
                }

                let default_count = cases.iter().filter(|(expr, _)| expr.is_none()).count();
                if default_count > 1 {
                    return Err(SemanticError::DuplicateDefault(statement.span));
                }

                if let StatementKind::Compound(block) = &body.kind {
                    check_block_for_decs(block.clone())?;
                }

                self.visit_statement(body)?;
            },
            _ => { walk_statement(self, statement)?; }
        }
        Ok(())
    }
}

fn collect_cases_in_block(items: &mut Vec<BlockItem>, scrutinee_type: &Type) -> Result<Vec<(Option<Expression>, String)>, SemanticError> {
    let mut cases = Vec::new();
    for item in items.iter_mut() {
        match item {
            BlockItem::S(stmt) => match &mut stmt.kind {
                StatementKind::Case { expr, lab } => {
                    match eval_constant(&expr.kind) {
                        Some(value) => {
                            let truncated = match scrutinee_type {
                                Type::Int => Const::Int(value as i32),
                                Type::Long => Const::Long(value as i64),
                                Type::UInt => Const::UInt(value as u32),
                                Type::ULong => Const::ULong(value as u64),
                                _ => unreachable!(),
                            };
                            **expr = ExpressionKind::Constant(truncated);
                            expr.expression_type = Some(scrutinee_type.clone());
                        },
                        None => return Err(SemanticError::NonConstantCase(expr.span)),
                    }
                    cases.push((Some(expr.clone()), lab.clone()));
                },
                StatementKind::Default { lab } => {
                    cases.push((None, lab.clone()));
                },
                StatementKind::Compound(bl) => {
                    cases.append(&mut collect_cases_in_block(&mut bl.items, scrutinee_type)?);
                },
                StatementKind::If(_, yes, no) => {
                    cases.append(&mut collect_switch_cases(yes.as_mut(), scrutinee_type)?);
                    if let Some(no) = no {
                        cases.append(&mut collect_switch_cases(no.as_mut(), scrutinee_type)?);
                    }
                },
                StatementKind::Label(_, body) => {
                    cases.append(&mut collect_switch_cases(body.as_mut(), scrutinee_type)?);
                },
                StatementKind::For { body, .. } |
                StatementKind::While { body, .. } |
                StatementKind::DoWhile { body, .. } => {
                    cases.append(&mut collect_switch_cases(body.as_mut(), scrutinee_type)?);
                },
                _ => {}, 
            },
            BlockItem::D(_) => {},
        }
    }
    Ok(cases)
}

fn collect_switch_cases(st: &mut Statement, scrutinee_type: &Type) -> Result<Vec<(Option<Expression>, String)>, SemanticError> {
    let mut cases = Vec::new();
    if let StatementKind::Compound(block) = &mut st.kind {
        cases.append(&mut collect_cases_in_block(&mut block.items, scrutinee_type)?);
    } else if let StatementKind::Case { expr, lab } = &mut st.kind {
        cases.push((Some(expr.clone()), lab.clone()));
    } else if let StatementKind::Default { lab } = &mut st.kind {
        cases.push((None, lab.clone()));
    }
    Ok(cases)
}

fn check_block_for_decs(block: Block) -> Result<(), SemanticError> {
    let items = &block.items;
    for window in items.windows(2) {
        if let [BlockItem::S(stmt), BlockItem::D(_)] = window {
            if let StatementKind::Case{..} | StatementKind::Default{..} = stmt.kind {
                return Err(SemanticError::DecInCase(stmt.span));
            }
        }
    }
    Ok(())
}

fn eval_constant(expr: &ExpressionKind) -> Option<i128> {
    match expr {
        // using lossy conversions here because it just needs to match the case
        ExpressionKind::Constant(c) =>  match c {
            Const::Int(i)  => Some(*i as i128),
            Const::Long(i) => Some(*i as i128),
            Const::UInt(i)  => Some(*i as i128), 
            Const::ULong(i) => Some(*i as i128),
        }
        ExpressionKind::Unary(op, expr) => {
            let val = eval_constant(expr)?;
            match op {
                UnaryOp::Negate => Some(-val),
                UnaryOp::Complement => Some(!val),
                UnaryOp::Not => Some((val == 0) as i128),
            }
        },
        ExpressionKind::Binary(op, left, right) => {
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
                BinaryOp::LessThan        => Some((l < r) as i128),
                BinaryOp::LessOrEqual     => Some((l <= r) as i128),
                BinaryOp::GreaterThan     => Some((l > r) as i128),
                BinaryOp::GreaterOrEqual  => Some((l >= r) as i128),
                BinaryOp::Equal           => Some((l == r) as i128),
                BinaryOp::NotEqual        => Some((l != r) as i128),
                BinaryOp::BitwiseAnd      => Some(l & r),
                BinaryOp::BitwiseXor      => Some(l ^ r),
                BinaryOp::BitwiseOr       => Some(l | r),
                BinaryOp::LogicalAnd      => Some((l != 0 && r != 0) as i128),
                BinaryOp::LogicalOr       => Some((l != 0 || r != 0) as i128),
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
