use std::collections::HashMap;
use super::*;
use visitor_trait::*;

#[derive(Debug, Clone, PartialEq)]
pub enum IdentAttrs {
    FuncAttr{defined: bool, global: bool},
    StaticAttr{init: InitialValue, global: bool},
    LocalAttr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InitialValue {
    Tentative,
    Initial(i32),
    NoInitializer,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub ident: String,
    pub datatype: Type,
    pub linkage: Option<bool>,
    pub stack_size: Option<i32>,
    pub attrs: IdentAttrs,
}

impl Symbol {
    fn new_func(ident: String, ftype: Type, linkage: bool, defined: bool, global: bool) -> Symbol {
        Symbol {
            ident: ident,
            datatype: ftype,
            linkage: Some(linkage),
            stack_size: None,
            attrs: IdentAttrs::FuncAttr { defined, global }
        }
    }

    fn new_static_var(ident: String, vtype: Type, init: InitialValue, global: bool) -> Symbol {
        Symbol { ident, datatype: vtype, linkage: None, stack_size: None, 
            attrs: IdentAttrs::StaticAttr { init, global } }
    }
    
    fn new_var(ident: String, vtype: Type) -> Symbol {
        Symbol { ident, datatype: vtype, linkage: None, stack_size: None, attrs: IdentAttrs::LocalAttr }
    }
}


struct TypeChecker<'a> {
    symbols: &'a mut HashMap<String, Symbol>
}

impl<'a> Visitor for TypeChecker<'a> {
    fn visit_func_decl(&mut self, function: &mut FuncDeclaration) -> Result<(), SemanticError> {
        let func_type = Type::FuncType(function.params.len());
        let has_body = function.body.is_some();
        let mut alr_def = false;

        if let Some(old) = self.symbols.get(&function.identifier) {
            if old.datatype != func_type {
                return Err(SemanticError::IncompatibleFuncDeclaration(function.identifier.clone()));
            }
            alr_def = old.linkage.unwrap();
            if alr_def && has_body {
                return Err(SemanticError::DoubleDeclaration(function.identifier.clone()));
            }
        }

        self.symbols.insert(function.identifier.clone(), 
            Symbol::new_func(function.identifier.clone(), func_type, alr_def || has_body));

        if has_body {
            for parameter in &function.params {
                self.symbols.insert(parameter.clone(), Symbol::new_var(parameter.clone(), Type::Int));
            }
        }

        walk_func_decl(self, function)?;
        Ok(())
    }

    fn visit_var_decl(&mut self, variable: &mut VarDeclaration) -> Result<(), SemanticError> {
        self.symbols.insert(variable.identifier.clone(), 
            Symbol::new_var(variable.identifier.clone(), Type::Int));
        walk_var_decl(self, variable)?;
        Ok(())
    }

    fn visit_expression(&mut self, expression: &mut Expression) -> Result<(), SemanticError> {
        match expression {
            Expression::FunctionCall(identifier, args) => {
                if let Some(sym) = self.symbols.get(identifier) {
                    if let Type::FuncType(n) = sym.datatype {
                        if n != args.len() {
                            return Err(SemanticError::FuncCalledWithWrongNumArgs(identifier.clone()));
                        }
                    } else {
                        return Err(SemanticError::VarCalledAsFunc(identifier.clone()));
                    }
                }
            },
            Expression::Var(identifier) => {
                if let Some(sym) = self.symbols.get(identifier) {
                    if sym.datatype != Type::Int || sym.linkage.is_some() {
                        return Err(SemanticError::FuncUsedAsVar(identifier.clone()));
                    }
                }
            },
            Expression::Assignment(exp1, _) => {
                if let Expression::FunctionCall(ident, _) = exp1.as_ref() {
                    return Err(SemanticError::FuncUsedAsVar(ident.clone()));
                }
            },
            _ => {}
        }
        walk_expression(self, expression)?;
        Ok(())
    }
}

pub fn type_checking_pass(program: &mut Program, symbols: &mut HashMap<String, Symbol>) -> Result<(), SemanticError> {
    let mut checker = TypeChecker { symbols };
    checker.visit_program(program)
}
