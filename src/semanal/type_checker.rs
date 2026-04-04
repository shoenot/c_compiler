use std::iter::zip;

use super::*;
use visitor_trait::*;

impl Symbol {
    fn new_func(ident: String, ftype: Type, defined: bool, global: bool) -> Symbol {
        Symbol { ident, datatype: ftype, attrs: IdentAttrs::FuncAttr { defined, global } }
    }

    fn new_static_var(ident: String, vtype: Type, init: InitialValue, global: bool) -> Symbol { Symbol { ident, datatype: vtype,  attrs: IdentAttrs::StaticAttr { init, global } }
    }

    fn new_var(ident: String, vtype: Type) -> Symbol {
        Symbol { ident, datatype: vtype, attrs: IdentAttrs::LocalAttr }
    }
}

struct TypeChecker<'a> {
    symbols: &'a mut SymbolTable,
    scope_depth: usize,
    current_function_type: Option<Type>,
}

fn set_type(expr: &mut Expression, expression_type: Type) -> Type {
    expr.expression_type = Some(expression_type.clone());
    expression_type
}

fn get_common_type(t1: Type, t2: Type) -> Type {
    if t1 == t2 {
        t1
    } else if t1.size() == t2.size() {
        if t1.is_signed() { t2 } else { t1 }
    } else if t1.size() > t2.size() { t1 } else { t2 }
}

fn convert_type(expr: &mut Expression, datatype: Type) {
    if *expr.expression_type.as_mut().unwrap() != datatype {
        expr.kind = ExpressionKind::Cast(datatype.clone(), Box::new(expr.clone()));
        set_type(expr, datatype);
    }
}

pub fn get_static_init(constant: Const) -> StaticInit {
    match constant {
        Const::Long(i) => StaticInit::LongInit(i),
        Const::Int(i) => StaticInit::IntInit(i),
        Const::UInt(i) => StaticInit::UIntInit(i),
        Const::ULong(i) => StaticInit::ULongInit(i),
    }
}

pub fn convert_constant(constant: Const, into: Type) -> Const {
    match (constant, into) {
        (Const::Int(i), Type::Long) => Const::Long(i as i64),
        (Const::Long(i), Type::Int) => Const::Int(i as i32),
        (Const::Int(i), Type::UInt) => Const::UInt(i as u32),
        (Const::Int(i), Type::ULong) => Const::ULong(i as u64),
        (Const::Long(i), Type::UInt) => Const::UInt(i as u32),
        (Const::Long(i), Type::ULong) => Const::ULong(i as u64),
        (Const::UInt(i), Type::Int) => Const::Int(i as i32),
        (Const::UInt(i), Type::Long) => Const::Long(i as i64),
        (Const::ULong(i), Type::Int) => Const::Int(i as i32),
        (Const::ULong(i), Type::Long) => Const::Long(i as i64),
        (c, _) => c,
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
                    let new = convert_constant(i, decl.var_type.clone());
                    let static_init = get_static_init(new);
                    InitialValue::Initial(static_init)
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
                if old.datatype != decl.var_type {
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
            Symbol::new_static_var(decl.identifier.clone(), decl.var_type.clone(), initial_value, global));
        Ok(())
    }
    
    // helper func that returns resolved type
    fn type_expression(&mut self, expr: &mut Expression) -> Result<Type, SemanticError> {
        match &mut expr.kind {
            ExpressionKind::FunctionCall(identifier, args) => {
                // check that function is in symbols table (which it should be, because its being
                // called. otherwise we wouldve caught this error earlier.
                if let Some(sym) = self.symbols.get(identifier) {
                    if let Type::FuncType{params, ret} = &sym.datatype {
                        let parameters = params.clone();
                        let ret_type = ret.clone();
                        if parameters.len() != args.len() {
                            return Err(SemanticError::FuncCalledWithWrongNumArgs(identifier.clone(), expr.span))
                        }
                        for (arg, datatype) in std::iter::zip(args, parameters) {
                            self.type_expression(arg)?;
                            convert_type(arg, *datatype);
                        }
                        Ok(set_type(expr, *ret_type))

                    } else {
                        return Err(SemanticError::VarCalledAsFunc(identifier.clone(), expr.span))
                    } 
                } else {
                    //hence the unreachable
                    unreachable!()
                }
            },
            ExpressionKind::Var(identifier) => {
                if let Some(sym) = self.symbols.get(identifier) {
                    if matches!(sym.datatype, Type::FuncType {..}) {
                        return Err(SemanticError::FuncUsedAsVar(identifier.clone(), expr.span));
                    } else {
                        Ok(set_type(expr, sym.datatype.clone()))
                    }
                } else {
                    unreachable!()
                }
            },
            ExpressionKind::Assignment(exp1, exp2) => {
                if let ExpressionKind::FunctionCall(ident, _) = &**exp1.as_ref() {
                    return Err(SemanticError::FuncUsedAsVar(ident.clone(), expr.span));
                } else {
                    let exp1_type = self.type_expression(exp1)?;
                    self.type_expression(exp2)?;
                    convert_type(exp2, exp1_type.clone());
                    Ok(set_type(expr, exp1_type))
                }
            },
            ExpressionKind::Constant(c) => {
                match c {
                    Const::Int(_) => Ok(set_type(expr, Type::Int)),
                    Const::Long(_) => Ok(set_type(expr, Type::Long)),
                    Const::UInt(_) => Ok(set_type(expr, Type::UInt)),
                    Const::ULong(_) => Ok(set_type(expr, Type::ULong)),
                }
            },
            ExpressionKind::Cast(t, factor) => {
                self.type_expression(factor)?;
                let exp_type = t.clone();
                Ok(set_type(expr, exp_type))
            },
            ExpressionKind::Unary(op, inner) => {
                let inner_exp = self.type_expression(inner)?;
                if *op == UnaryOp::Not {
                    Ok(set_type(expr, Type::Int))
                } else {
                    Ok(set_type(expr, inner_exp))
                }
            },
            ExpressionKind::Binary(op, exp1, exp2) => {
                let exp1_type = self.type_expression(exp1)?;
                let exp2_type = self.type_expression(exp2)?;
                if matches!(op, BinaryOp::LogicalOr | BinaryOp::LogicalAnd ) {
                    Ok(set_type(expr, Type::Int))
                } else {
                    let common_type = get_common_type(exp1_type.clone(), exp2_type.clone());
                    if !matches!(op, BinaryOp::LeftShift | BinaryOp::RightShift ) {
                        convert_type(exp1, common_type.clone());
                        convert_type(exp2, common_type.clone());
                    }
                    if matches!(op, BinaryOp::Equal | BinaryOp::NotEqual |
                                    BinaryOp::GreaterThan | BinaryOp::LessThan |
                                    BinaryOp::GreaterOrEqual | BinaryOp::LessOrEqual) {
                        Ok(set_type(expr, Type::Int))
                    } else if matches!(op, BinaryOp::LeftShift | BinaryOp::RightShift) {
                        Ok(set_type(expr, exp1_type))
                    } else {
                        Ok(set_type(expr, common_type))
                    }
                } 
            },
            ExpressionKind::PrefixIncrement(x) | ExpressionKind::PostfixIncrement(x) |
            ExpressionKind::PrefixDecrement(x) | ExpressionKind::PostfixDecrement(x) => {
                let exp_type = self.type_expression(x)?;
                Ok(set_type(expr, exp_type))
            },
            ExpressionKind::Conditional(cond, exp1, exp2) => {
                let exp1_type = self.type_expression(exp1)?;
                let exp2_type = self.type_expression(exp2)?;
                self.type_expression(cond)?;
                let common_type = get_common_type(exp1_type, exp2_type);
                convert_type(exp1, common_type.clone());
                convert_type(exp2, common_type.clone());
                Ok(set_type(expr, common_type))
            }
        }
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
                if old.datatype != function.func_type.clone() {
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
        
        let Type::FuncType { params: param_types, ret: return_type } = function.func_type.clone() else {
            unreachable!()
        };

        self.symbols.insert(function.identifier.clone(),
            Symbol::new_func(function.identifier.clone(), function.func_type.clone(), alr_def || has_body, global));
        
        if has_body {
            for (param, param_type) in zip(function.params.clone(), param_types)  {
                self.symbols.insert(param.clone(), Symbol::new_var(param.clone(), *param_type));
            }
            self.current_function_type = Some(*return_type);
            self.scope_depth += 1;
            walk_func_decl(self, function)?;
            self.scope_depth -= 1;
            self.current_function_type = None;
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
                if old.datatype != decl.var_type {
                    return Err(SemanticError::FuncUsedAsVar(decl.identifier.clone(), decl.span));
                }
            } else {
                self.symbols.insert(decl.identifier.clone(),
                    Symbol::new_static_var(decl.identifier.clone(), decl.var_type.clone(), InitialValue::NoInitializer, true));
            }
        } else if is_static(decl) {
            let initial_value = match &decl.init {
                Some(expr) => {
                    if let ExpressionKind::Constant(i) = expr.kind {
                        let new = convert_constant(i, decl.var_type.clone());
                        let static_init = get_static_init(new);
                        InitialValue::Initial(static_init)
                    } else {
                        return Err(SemanticError::LocalStaticVarNonConstantInit(decl.identifier.clone(), expr.span));
                    }
                },
                None => {
                    let new = convert_constant(Const::Int(0), decl.var_type.clone());
                    let static_init = get_static_init(new);
                    InitialValue::Initial(static_init)
                }
            };
            self.symbols.insert(decl.identifier.clone(),
                Symbol::new_static_var(decl.identifier.clone(), decl.var_type.clone(), initial_value, false));
        } else {
            self.symbols.insert(decl.identifier.clone(), Symbol::new_var(decl.identifier.clone(), decl.var_type.clone()));
            walk_var_decl(self, decl)?;
        }
        Ok(())
    }

    fn visit_expression(&mut self, expression: &mut Expression) -> Result<(), SemanticError> {
        self.type_expression(expression)?;
        Ok(())
    }

    fn visit_statement(&mut self, statement: &mut Statement) -> Result<(), SemanticError> {
        match &mut statement.kind {
            StatementKind::Return(expr) => {
                self.type_expression(expr)?;
                convert_type(expr, self.current_function_type.clone().unwrap());
            },
            _ => walk_statement(self, statement)?,
        }
        Ok(())
    }
}

pub fn type_checking_pass(program: &mut Program, symbols: &mut SymbolTable) -> Result<(), SemanticError> {
    let mut checker = TypeChecker { symbols, scope_depth: 0, current_function_type: None };
    checker.visit_program(program)
}
