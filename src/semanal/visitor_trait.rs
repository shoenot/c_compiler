use super::*;

pub trait Visitor {
    fn visit_program(&mut self, program: &mut Program) -> Result<(), SemanticError> {
        walk_program(self, program)
    }
    
    fn visit_declaration(&mut self, declaration: &mut Decl) -> Result<(), SemanticError> {
        walk_declaration(self, declaration)
    }

    fn visit_var_decl(&mut self, var: &mut VarDeclaration) -> Result<(), SemanticError> {
        walk_var_decl(self, var)
    }

    fn visit_func_decl(&mut self, func: &mut FuncDeclaration) -> Result<(), SemanticError> {
        walk_func_decl(self, func)
    }

    fn visit_block(&mut self, block: &mut Block) -> Result<(), SemanticError> {
        walk_block(self, block)
    }

    fn visit_statement(&mut self, statement: &mut Statement) -> Result<(), SemanticError> {
        walk_statement(self, statement)
    }

    fn visit_expression(&mut self, expression: &mut Expression) -> Result<(), SemanticError> {
        walk_expression(self, expression)
    }
}

pub fn walk_program<V: Visitor + ?Sized>(v: &mut V, program: &mut Program) -> Result<(), SemanticError> {
    for decl in &mut program.declarations {
        v.visit_declaration(decl)?;
    }
    Ok(())
}

pub fn walk_declaration<V: Visitor + ?Sized>(v: &mut V, declaration: &mut Decl) -> Result<(), SemanticError> {
    match declaration {
        Decl::VarDecl(d) => v.visit_var_decl(d)?,
        Decl::FuncDecl(f) => v.visit_func_decl(f)?,
    }
    Ok(())
}

pub fn walk_block<V: Visitor + ?Sized>(v: &mut V, block: &mut Block) -> Result<(), SemanticError> {
    for item in &mut block.items {
        match item {
            BlockItem::D(d) => v.visit_declaration(d)?,
            BlockItem::S(s) => v.visit_statement(s)?,
        }
    }
    Ok(())
}

pub fn walk_var_decl<V: Visitor + ?Sized>(v: &mut V, var: &mut VarDeclaration) -> Result<(), SemanticError> {
    if let Some(exp) = &mut var.init {
        v.visit_expression(exp)?;
    }
    Ok(())
}

pub fn walk_func_decl<V: Visitor + ?Sized>(v: &mut V, func: &mut FuncDeclaration) -> Result<(), SemanticError> {
    if let Some(blk) = &mut func.body {
        v.visit_block(blk)?;
    }
    Ok(())
}

pub fn walk_statement<V: Visitor + ?Sized>(v: &mut V, statement: &mut Statement) -> Result<(), SemanticError> {
    match &mut statement.kind {
        StatementKind::Return(exp) | StatementKind::Expression(exp) |
        StatementKind::Case { expr: exp, lab:_ } => {
            v.visit_expression(exp)?;
        },
        StatementKind::If(exp, y, mn) => {
            v.visit_expression(exp)?;
            v.visit_statement(y)?;
            if let Some(n) = mn {
                v.visit_statement(n)?;
            }
        },
        StatementKind::While { cond, body, lab:_ } | StatementKind::DoWhile { body, cond, lab:_ } => {
            v.visit_expression(cond)?;
            v.visit_statement(body)?;
        },
        StatementKind::For { init, cond, post, body, lab:_ } => {
            match init {
                ForInit::InitDec(d) => v.visit_var_decl(d)?,
                ForInit::InitExp(Some(e)) => v.visit_expression(e)?,
                _ => {}
            }
            if let Some(c) = cond {
                v.visit_expression(c)?;
            }
            if let Some(p) = post {
                v.visit_expression(p)?;
            }
            v.visit_statement(body)?;
        },
        StatementKind::Label(_, s) => v.visit_statement(s)?,
        StatementKind::Switch { scrutinee, body, lab:_, cases } => {
            v.visit_expression(scrutinee)?;
            v.visit_statement(body)?;
            for c in cases {
                if let (Some(e), _) = c {
                    v.visit_expression(e)?;
                }
            }
        },
        StatementKind::Compound(blk) => v.visit_block(blk)?,
        _ => {}
    }
    Ok(())
}

pub fn walk_expression<V: Visitor + ?Sized>(v: &mut V, expression: &mut Expression) -> Result<(), SemanticError> {
    match &mut expression.kind {
        ExpressionKind::Assignment(exp1, exp2) |
        ExpressionKind::Binary(_, exp1, exp2) => {
            v.visit_expression(exp1.as_mut())?;
            v.visit_expression(exp2.as_mut())?;
        },
        ExpressionKind::Conditional(exp1, exp2, exp3) => {
            v.visit_expression(exp1.as_mut())?;
            v.visit_expression(exp2.as_mut())?;
            v.visit_expression(exp3.as_mut())?;
        },
        ExpressionKind::Unary(_, exp) |
        ExpressionKind::PostfixIncrement(exp) | ExpressionKind::PrefixIncrement(exp) | 
        ExpressionKind::PostfixDecrement(exp) | ExpressionKind::PrefixDecrement(exp) => {
            v.visit_expression(exp.as_mut())?;
        },
        ExpressionKind::FunctionCall(_, args) => {
            for exp in args {
                v.visit_expression(exp)?;
            }
        },
        ExpressionKind::Var(_) |
        ExpressionKind::Constant(_) => {},
        ExpressionKind::Cast(_, exp) => v.visit_expression(exp.as_mut())?,
        ExpressionKind::Dereference(exp) => v.visit_expression(exp)?,
        ExpressionKind::AddrOf(exp) => v.visit_expression(exp)?,
    }
    Ok(())
}
