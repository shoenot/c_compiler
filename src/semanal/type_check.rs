use std::collections::HashMap;
use super::*;

pub fn type_checking_pass(program: &mut Program, symbols: &mut HashMap<String, Symbol>) -> Result<(), SemanticError> {
    for function in &mut program.functions {
        check_func_decl(function, symbols)?;
    }
    Ok(())
}

fn check_func_decl(function: &mut FuncDeclaration, symbols: &mut HashMap<String, Symbol>) 
    -> Result<(), SemanticError> {
    let func_type = Type::FuncType(function.params.len());
    let name = function.identifier.clone();
    let has_body = function.body.is_some();
    let mut alr_def = false;

    if let Some(old) = symbols.get(&function.identifier) {
        if old.datatype != func_type {
            return Err(SemanticError::IncompatibleFuncDeclaration(function.identifier.clone()));
        } 
        alr_def = old.linkage.unwrap();
        if alr_def && has_body {
            return Err(SemanticError::DoubleDeclaration(function.identifier.clone()));
        }
    }

    symbols.insert(name.clone(), Symbol::new_func(name.clone(), func_type, alr_def || has_body));

    if has_body {
        for parameter in &mut function.params {
            symbols.insert(parameter.clone(), Symbol::new_var(parameter.clone(), Type::Int));
        }
        check_block(function.body.as_mut().unwrap(), symbols)?;
    }
    Ok(())
}

fn check_var_decl(variable: &mut VarDeclaration, symbols: &mut HashMap<String, Symbol>) 
    -> Result<(), SemanticError> {
    let name = variable.identifier.clone();
    symbols.insert(name.clone(), Symbol::new_var(name, Type::Int));
    if let Some(init) = &mut variable.init {
        check_expression(init, symbols)?;
    }
    Ok(())
}

fn check_block(block: &mut Block, symbols: &mut HashMap<String, Symbol>)
    -> Result<(), SemanticError> {
    for item in &mut block.items {
        match item {
            BlockItem::D(Decl::VarDecl(v)) => check_var_decl(v, symbols)?,
            BlockItem::D(Decl::FuncDecl(f)) => check_func_decl(f, symbols)?,
            BlockItem::S(s) => check_statement(s, symbols)?,
        }
    }
    Ok(())
}

fn check_statement(statement: &mut Statement, symbols: &mut HashMap<String, Symbol>)
    -> Result<(), SemanticError> {
    match statement {
        Statement::Return(exp) | Statement::Expression(exp) => {
            check_expression(exp, symbols)?;
        },
        Statement::If(exp, y, mn) => {
            check_expression(exp, symbols)?;
            check_statement(y, symbols)?;
            if let Some(n) = mn {
                check_statement(n, symbols)?;
            }
        },
        Statement::While { cond, body, lab:_ } | Statement::DoWhile { body, cond, lab:_ } => {
            check_expression(cond, symbols)?;
            check_statement(body, symbols)?;
        },
        Statement::For { init, cond, post, body, lab:_ } => {
            match init {
                ForInit::InitDec(d) => check_var_decl(d, symbols)?,
                ForInit::InitExp(Some(e)) => check_expression(e, symbols)?,
                _ => {}
            }
            if let Some(c) = cond {
                check_expression(c, symbols)?;
            }
            if let Some(p) = post {
                check_expression(p, symbols)?;
            }
            check_statement(body, symbols)?;
        },
        Statement::Label(_, s) => check_statement(s, symbols)?,
        Statement::Switch { scrutinee, body, lab:_, cases } => {
            check_expression(scrutinee, symbols)?;
            check_statement(body, symbols)?;
            for c in cases {
                if let (Some(e), _) = c {
                    check_expression(e, symbols)?;
                }
            }
        },
        Statement::Case { expr, lab:_ } => {
            check_expression(expr, symbols)?;
        },
        _ => {}
    }
    Ok(())
}

fn check_expression(expression: &mut Expression, symbols: &mut HashMap<String, Symbol>)
    -> Result<(), SemanticError> {
    match expression {
        Expression::FunctionCall(identifier, args) => {
            if let Some(sym) = symbols.get(identifier) {
                if let Type::FuncType(n) = sym.datatype {
                    if n != args.len() {
                        return Err(SemanticError::FuncCalledWithWrongNumArgs(identifier.clone()));
                    } else {
                        for arg in args {
                            check_expression(arg, symbols)?;
                        }
                    }
                } else {
                    return Err(SemanticError::VarCalledAsFunc(identifier.clone()));
                }
            }
        },
        Expression::Var(identifier) => {
            if let Some(sym) = symbols.get(identifier) {
                if sym.datatype != Type::Int {
                    return Err(SemanticError::FuncUsedAsVar(identifier.clone()));
                } else if sym.linkage != None {
                    return Err(SemanticError::FuncUsedAsVar(identifier.clone()));
                }
            }
        },
        Expression::Assignment(exp1, exp2) => {
            if let Expression::FunctionCall(ident, _) = exp1.as_mut() {
                return Err(SemanticError::FuncUsedAsVar(ident.clone()));
            }
            if let Expression::FunctionCall(ident, _) = exp2.as_mut() {
                return Err(SemanticError::FuncUsedAsVar(ident.clone()));
            }
            check_expression(exp1, symbols)?;
            check_expression(exp2, symbols)?;
        },
        Expression::Unary(_, exp) => {
            check_expression(exp, symbols)?;
        },
        Expression::Binary(_, exp1, exp2) => {
            check_expression(exp1, symbols)?;
            check_expression(exp2, symbols)?;
        },
        Expression::Conditional(exp1, exp2, exp3) => {
            check_expression(exp1, symbols)?;
            check_expression(exp2, symbols)?;
            check_expression(exp3, symbols)?;
        },
        Expression::PrefixIncrement(exp) | Expression::PostfixIncrement(exp) |
        Expression::PrefixDecrement(exp) | Expression::PostfixDecrement(exp) => {
            check_expression(exp, symbols)?;
        },
        _ => {}
    }
    Ok(())
}
