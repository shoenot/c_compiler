use super::*;
use visitor_trait::*;

struct ContRewrite {
    tgt_loop_lab: String,
    new_goto_lab: String,
}

impl Visitor for ContRewrite {
    fn visit_statement(&mut self, statement: &mut Statement) -> Result<(), SemanticError> {
        if let StatementKind::Continue(lab) = &statement.kind {
            if lab == &self.tgt_loop_lab {
                statement.kind = StatementKind::Goto(self.new_goto_lab.clone());
                return Ok(())
            }
        }

        walk_statement(self, statement)
    }
}

pub struct ForDesugarer;

impl Visitor for ForDesugarer {
    fn visit_statement(&mut self, statement: &mut Statement) -> Result<(), SemanticError> {
        walk_statement(self, statement)?;

        if let StatementKind::For { init, cond, post, body, lab } = &mut statement.kind {

            let loop_label = lab.clone();
            let post_label = format!("{}_post", loop_label);

            let mut rewriter = ContRewrite {
                tgt_loop_lab: loop_label.clone(),
                new_goto_lab: post_label.clone(),
            };
            rewriter.visit_statement(body)?;

            let post_stmt = if let Some(post_expr) = post.take() {
                Statement::new(StatementKind::Expression(post_expr), statement.span)
            } else {
                Statement::new(StatementKind::Null, statement.span)
            };

            let labeled_post = Statement::new(StatementKind::Label(post_label, Box::new(post_stmt)), statement.span);

            let newbod = Statement::new(StatementKind::Compound(Block { items: 
                vec![BlockItem::S(*body.clone()), BlockItem::S(labeled_post)] 
            }), statement.span);

            let newcond = cond.take().unwrap_or(Expression::new(ExpressionKind::Constant(Const::Int(1)), None, statement.span));

            let newloop = Statement::new(StatementKind::While { cond: newcond, 
                                                                body: Box::new(newbod), 
                                                                lab: loop_label }, 
                                                                statement.span);
            let mut outer = Vec::new();
            match init {
                ForInit::InitDec(decl) => {
                    outer.push(BlockItem::D(Decl::VarDecl(decl.clone())));
                },
                ForInit::InitExp(Some(expr)) => {
                    outer.push(BlockItem::S(Statement::new(StatementKind::Expression(expr.clone()), statement.span)));
                },
                ForInit::InitExp(None) => {},
            }
            outer.push(BlockItem::S(newloop));

            statement.kind = StatementKind::Compound(Block { items: outer });
        }

        Ok(())
    }
}

pub fn for_desugaring_pass(program: &mut Program) -> Result<(), SemanticError> {
    let mut desugarer = ForDesugarer;
    desugarer.visit_program(program)
}
