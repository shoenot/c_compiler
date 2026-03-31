use super::*;
use visitor_trait::*;

struct LoopLabeler {
    counter: usize,
    stack: Vec<String>,
    loopstack: Vec<String>,
    switchstack: Vec<String>,
}

impl LoopLabeler {
    fn genlabel(&mut self) -> String {
        let label = format!("loop.{}", self.counter);
        self.counter += 1;
        label
    }
}

impl Visitor for LoopLabeler {
    fn visit_statement(&mut self, statement: &mut Statement) -> Result<(), SemanticError> {
        match &mut statement.kind {
            StatementKind::DoWhile { body, cond:_, lab } |
            StatementKind::While { cond:_, body, lab } |
            StatementKind::For { init:_, cond:_, post:_, body, lab } => {
                let newlab = self.genlabel();
                *lab = newlab.clone();
                self.stack.push(newlab.clone());
                self.loopstack.push(newlab);
                self.visit_statement(body)?;
                self.loopstack.pop();
                self.stack.pop();
            },
            StatementKind::Break(lab) => {
                if let Some(newlab) = self.stack.last() {
                    *lab = newlab.into();
                } else {
                    return Err(SemanticError::BreakOutsideLoopOrSwitch(statement.span));
                }
            },
            StatementKind::Continue(lab) => {
                if let Some(newlab) = self.loopstack.last() {
                    *lab = newlab.into();
                } else {
                    return Err(SemanticError::ContOutsideLoop(statement.span));
                }
            },
            StatementKind::Compound(block) => {
                self.visit_block(block)?;
            },
            StatementKind::Label(_, body) => {
                self.visit_statement(body)?;
            }
            StatementKind::If(_, yes, no) => {
                self.visit_statement(yes)?;
                if let Some(no) = no {
                    self.visit_statement(no)?;
                }
            },
            StatementKind::Switch { scrutinee:_, body, lab, cases:_ } => {
                let newlab = self.genlabel();
                *lab = newlab.clone();
                self.stack.push(newlab.clone());
                self.switchstack.push(newlab);
                self.visit_statement(body)?;
                self.switchstack.pop();
                self.stack.pop();
            },
            StatementKind::Case { expr:_, lab } => {
                if self.switchstack.is_empty() {
                    return Err(SemanticError::CaseOutsideSwitch(statement.span));
                }
                *lab = self.genlabel();
            },
            StatementKind::Default { lab } => {
                if self.switchstack.is_empty() {
                    return Err(SemanticError::CaseOutsideSwitch(statement.span));
                }
                *lab = self.genlabel();
            },
            _ => walk_statement(self, statement)?,
        }
        Ok(())
    }
}

pub fn loop_labeling_pass(program: &mut Program) -> Result<(), SemanticError> {
    let mut labeler = LoopLabeler { 
        counter: 0,
        stack: Vec::new(), 
        loopstack: Vec::new(),
        switchstack: Vec::new(),
    };
    labeler.visit_program(program)
}

