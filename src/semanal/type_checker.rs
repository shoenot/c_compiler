use std::iter::Zip;

use super::*;
use visitor_trait::*;
use crate::types::Type;

struct TypeChecker<'a> {
    symbols: &'a mut SymbolTable,
    scope_depth: usize,
    current_function_type: Option<Type>,
}

impl Symbol {
    fn new_func(ident: String, ftype: Type, defined: bool, global: bool) -> Symbol {
        Symbol { ident, datatype: ftype, attrs: IdentAttrs::FuncAttr { defined, global } }
    }

    fn new_static_var(ident: String, vtype: Type, init: InitialValue, global: bool) -> Symbol {
        Symbol { ident, datatype: vtype,  attrs: IdentAttrs::StaticAttr { init, global } }
    }

    fn new_var(ident: String, vtype: Type) -> Symbol {
        Symbol { ident, datatype: vtype, attrs: IdentAttrs::LocalAttr }
    }
}

fn set_type(expr: &mut Expression, expression_type: Type) {
    expr.expression_type = Some(expression_type);
}

fn get_common_type(type1: Type, type2: Type) -> Type {
    if type1 == type2 {
        type1
    } else {
        Type::Long
    }
}

fn convert_type(expr: &mut Expression, datatype: Type) {
    if *expr.expression_type.as_mut().unwrap() != datatype {
        expr.kind = ExpressionKind::Cast(datatype.clone(), Box::new(expr.clone()));
        set_type(expr, datatype);
    }
}

pub fn is_static<T: HasStorage>(decl: &T) -> bool {
    decl.storage_class() == Some(StorageClass::Static)
}

pub fn is_extern<T: HasStorage>(decl: &T) -> bool {
    decl.storage_class() == Some(StorageClass::Extern)
}

impl<'a> TypeChecker<'a> {
    fn check_global_var(&mut self, decl: &mut VarDeclaration) -> Result<(), SemanticError> {
        let mut initial_value = match &decl.init {
            Some(expr) => {
                if let ExpressionKind::Constant(i) = expr.kind {
                    InitialValue::Initial(i)
                } else {
                    return Err(SemanticError::NonConstantInitializer(decl.identifier.clone(), decl.span));
                }
            }
            None => if is_extern(decl) {
                InitialValue::NoInitializer
            } else {
                InitialValue::Tentative
            },
        };

        let mut global = !is_static(decl);

        if let Some(old) = self.symbols.get(&decl.identifier) {
            if let IdentAttrs::StaticAttr { init: old_init, global: old_global } = &old.attrs {
                if old.datatype != Type::Int {
                    return Err(SemanticError::FuncUsedAsVar(decl.identifier.clone(), decl.span));
                }

                if is_extern(decl) {
                    global = *old_global;
                } else if global != *old_global {
                    return Err(SemanticError::ConflictingStorageTypes(decl.identifier.clone(), decl.span));
                }

                if matches!(old_init, InitialValue::Initial(..)) {
                    if matches!(initial_value, InitialValue::Initial(..)) {
                        return Err(SemanticError::ConflictingDefinitions(decl.identifier.clone(), decl.span));
                    } else {
                        initial_value = old_init.clone();
                    }
                } else if !matches!(initial_value, InitialValue::Initial(..)) && matches!(old_init, InitialValue::Tentative) {
                    initial_value = InitialValue::Tentative;
                }
            } else {
                return Err(SemanticError::FuncUsedAsVar(decl.identifier.clone(), decl.span));
            }
        }
        self.symbols.insert(decl.identifier.clone(),
            Symbol::new_static_var(decl.identifier.clone(), Type::Int, initial_value, global));
        Ok(())
    }
}

impl<'a> Visitor for TypeChecker<'a> {
    fn visit_func_decl(&mut self, function: &mut FuncDeclaration) -> Result<(), SemanticError> {
        
        let has_body = function.body.is_some();
        let mut alr_def = false;
        let mut global = match function.storage {
            Some(StorageClass::Static) => false,
            _ => true,
        };

        if let Some(old) = self.symbols.get(&function.identifier) {
            if let IdentAttrs::FuncAttr { defined: olddef, global: oldglobal } = old.attrs {
                if old.datatype != func_type {
                    return Err(SemanticError::IncompatibleFuncDeclaration(function.identifier.clone(), function.span));
                }
                alr_def = olddef;
                if alr_def && has_body {
                    return Err(SemanticError::DoubleDeclaration(function.identifier.clone(), function.span));
                }

                if oldglobal && function.storage == Some(StorageClass::Static) {
                    return Err(SemanticError::StaticAfterNonStatic(function.identifier.clone(), function.span));
                }
                global = oldglobal;
            } else {
                return Err(SemanticError::IncompatibleFuncDeclaration(function.identifier.clone(), function.span));
            }
        }

        self.symbols.insert(function.identifier.clone(),
            Symbol::new_func(function.identifier.clone(), func_type, alr_def || has_body, global));

        if has_body {
            for parameter in &function.params {
                self.symbols.insert(parameter.clone(), Symbol::new_var(parameter.clone(), Type::Int));
            }
            self.scope_depth += 1;
            walk_func_decl(self, function)?;
            self.scope_depth -= 1;
        }

        Ok(())
    }

    fn visit_var_decl(&mut self, decl: &mut VarDeclaration) -> Result<(), SemanticError> {
        if self.scope_depth == 0 {
            return self.check_global_var(decl);
        }

        if is_extern(decl) {
            if decl.init != None {
                return Err(SemanticError::InitializerOnLocalExtern(decl.identifier.clone(), decl.span));
            }
            if let Some(old) = self.symbols.get(&decl.identifier) {
                if old.datatype != Type::Int {
                    return Err(SemanticError::FuncUsedAsVar(decl.identifier.clone(), decl.span));
                }
            } else {
                self.symbols.insert(decl.identifier.clone(),
                    Symbol::new_static_var(decl.identifier.clone(), Type::Int, InitialValue::NoInitializer, true));
            }
        } else if is_static(decl) {
            let initial_value = match &decl.init {
                Some(expr) => {
                    if let ExpressionKind::Constant(i) = expr.kind {
                        InitialValue::Initial(i)
                    } else {
                        return Err(SemanticError::LocalStaticVarNonConstantInit(decl.identifier.clone(), expr.span));
                    }
                },
                None => InitialValue::Initial(0)
            };
            self.symbols.insert(decl.identifier.clone(),
                Symbol::new_static_var(decl.identifier.clone(), Type::Int, initial_value, false));
        } else {
            self.symbols.insert(decl.identifier.clone(), Symbol::new_var(decl.identifier.clone(), Type::Int));
            walk_var_decl(self, decl)?;
        }
        Ok(())
    }

    fn visit_expression(&mut self, expression: &mut Expression) -> Result<(), SemanticError> {
        match &mut expression.kind {
            ExpressionKind::FunctionCall(identifier, args) => {
                if let Some(sym) = self.symbols.get(identifier) {
                    if let Type::FuncType{params, ret} = &sym.datatype {
                        if params.len() != args.len() {
                            return Err(SemanticError::FuncCalledWithWrongNumArgs(identifier.clone(), expression.span));
                        }
                        for (arg, datatype) in std::iter::zip(args, params) {
                            set_type(arg, *datatype.clone());
                        }
                        set_type(expression, *ret.clone());
                    } else {
                        return Err(SemanticError::VarCalledAsFunc(identifier.clone(), expression.span));
                    }
                }
            },
            ExpressionKind::Var(identifier) => {
                if let Some(sym) = self.symbols.get(identifier) {
                    if matches!(sym.datatype, Type::FuncType {..}) {
                        return Err(SemanticError::FuncUsedAsVar(identifier.clone(), expression.span));
                    } else {
                        set_type(expression, sym.datatype.clone());
                    }
                }
            },
            ExpressionKind::Assignment(exp1, exp2) => {
                if let ExpressionKind::FunctionCall(ident, _) = &**exp1.as_ref() {
                    return Err(SemanticError::FuncUsedAsVar(ident.clone(), expression.span));
                } else {
                    self.visit_expression(exp1)?;
                    self.visit_expression(exp2)?;
                    let exp_type = exp1.expression_type.as_ref().unwrap().clone();
                    convert_type(exp2, exp_type.clone());
                    set_type(expression, exp_type);
                    
                }
            },
            ExpressionKind::Constant(c) => {
                match c {
                    Const::Int(_) => set_type(expression, Type::Int),
                    Const::Long(_) => set_type(expression, Type::Long),
                }
            },
            ExpressionKind::Cast(t, factor) => {
                self.visit_expression(factor)?;
                let exp_type = t.clone();
                set_type(expression, exp_type);
            },
            ExpressionKind::Unary(op, inner) => {
                self.visit_expression(inner)?;
                if *op == UnaryOp::Not {
                    set_type(expression, Type::Int);
                } else {
                    let exp_type = inner.expression_type.as_ref().unwrap().clone();
                    set_type(expression, exp_type);
                }
            },
            ExpressionKind::Binary(op, exp1, exp2) => {
                self.visit_expression(exp1)?;
                self.visit_expression(exp2)?;
                if matches!(op, BinaryOp::LogicalOr | BinaryOp::LogicalAnd ) {
                    set_type(expression, Type::Int);
                } else {
                    let type1 = exp1.expression_type.as_ref().unwrap().clone();
                    let type2 = exp2.expression_type.as_ref().unwrap().clone();
                    let common_type = get_common_type(type1, type2);
                    convert_type(exp1, common_type.clone());
                    convert_type(exp2, common_type.clone());
                    if matches!(op, BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Remainder) {
                        set_type(expression, common_type);
                    } else {
                        set_type(expression, Type::Int);
                    }
                }
            },
            ExpressionKind::PrefixIncrement(x) | ExpressionKind::PostfixIncrement(x) |
            ExpressionKind::PrefixDecrement(x) | ExpressionKind::PostfixDecrement(x) => {
                self.visit_expression(x)?;
                let exp_type = x.expression_type.as_ref().unwrap().clone();
                set_type(expression, exp_type);
            },
            ExpressionKind::Conditional(cond, exp1, exp2) => {
                self.visit_expression(exp1)?;
                self.visit_expression(exp2)?;
                self.visit_expression(cond)?;
                let type1 = exp1.expression_type.as_ref().unwrap().clone();
                let type2 = exp2.expression_type.as_ref().unwrap().clone();
                let common_type = get_common_type(type1, type2);
                convert_type(exp1, common_type.clone());
                convert_type(exp2, common_type.clone());
                set_type(expression, common_type);
            }
        }
        Ok(())
    }

    fn visit_statement(&mut self, statement: &mut Statement) -> Result<(), SemanticError> {
        match &mut statement.kind {
            StatementKind::Return()
        }
    }
}

pub fn type_checking_pass(program: &mut Program, symbols: &mut SymbolTable) -> Result<(), SemanticError> {
    let mut checker = TypeChecker { symbols, scope_depth: 0, current_function_type: None };
    checker.visit_program(program)
}
