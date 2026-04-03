use super::*;
use std::collections::hash_set::HashSet;
use visitor_trait::*;

struct SwitchCollector;

impl Visitor for SwitchCollector {
    fn visit_statement(&mut self, statement: &mut Statement) -> Result<(), SemanticError> {
        match &mut statement.kind {
            StatementKind::Switch { cases, body, .. } => {
                *cases = collect_switch_cases(body)?;

                let mut seen = HashSet::new();
                for expr in cases.iter().filter_map(|(opt, _)| opt.as_ref()) {
                    if let ExpressionKind::Constant(value) = &expr.kind {
                        if !seen.insert(value) {
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

fn collect_cases_in_block(items: &mut Vec<BlockItem>) -> Result<Vec<(Option<Expression>, String)>, SemanticError> {
    let mut cases = Vec::new();
    for item in items.iter_mut() {
        match item {
            BlockItem::S(stmt) => match &mut stmt.kind {
                StatementKind::Case { expr, lab } => {
                    match eval_constant(expr) {
                        Some(value) => **expr = ExpressionKind::Constant(Const::Long(value)),
                        None => return Err(SemanticError::NonConstantCase(expr.span)),
                    }
                    cases.push((Some(expr.clone()), lab.clone()));
                },
                StatementKind::Default { lab } => {
                    cases.push((None, lab.clone()));
                },
                StatementKind::Compound(bl) => {
                    cases.append(&mut collect_cases_in_block(&mut bl.items)?);
                },
                StatementKind::If(_, yes, no) => {
                    cases.append(&mut collect_switch_cases(yes.as_mut())?);
                    if let Some(no) = no {
                        cases.append(&mut collect_switch_cases(no.as_mut())?);
                    }
                },
                StatementKind::Label(_, body) => {
                    cases.append(&mut collect_switch_cases(body.as_mut())?);
                },
                StatementKind::For { body, .. } |
                StatementKind::While { body, .. } |
                StatementKind::DoWhile { body, .. } => {
                    cases.append(&mut collect_switch_cases(body.as_mut())?);
                },
                _ => {}, 
            },
            BlockItem::D(_) => {},
        }
    }
    Ok(cases)
}

fn collect_switch_cases(st: &mut Statement) -> Result<Vec<(Option<Expression>, String)>, SemanticError> {
    let mut cases = Vec::new();
    if let StatementKind::Compound(block) = &mut st.kind {
        cases.append(&mut collect_cases_in_block(&mut block.items)?);
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

fn eval_constant(expr: &ExpressionKind) -> Option<i64> {
    match expr {
        ExpressionKind::Constant(c) =>  match c {
            Const::Int(i)  => Some(*i as i64), 
            Const::Long(i) => Some(*i as i64),
        }
        ExpressionKind::Unary(op, expr) => {
            let val = eval_constant(expr)?;
            match op {
                UnaryOp::Negate => Some(-val),
                UnaryOp::Complement => Some(!val),
                UnaryOp::Not => Some((val == 0) as i64),
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
                BinaryOp::LessThan        => Some((l < r) as i64),
                BinaryOp::LessOrEqual     => Some((l <= r) as i64),
                BinaryOp::GreaterThan     => Some((l > r) as i64),
                BinaryOp::GreaterOrEqual  => Some((l >= r) as i64),
                BinaryOp::Equal           => Some((l == r) as i64),
                BinaryOp::NotEqual        => Some((l != r) as i64),
                BinaryOp::BitwiseAnd      => Some(l & r),
                BinaryOp::BitwiseXor      => Some(l ^ r),
                BinaryOp::BitwiseOr       => Some(l | r),
                BinaryOp::LogicalAnd      => Some((l != 0 && r != 0) as i64),
                BinaryOp::LogicalOr       => Some((l != 0 || r != 0) as i64),
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
