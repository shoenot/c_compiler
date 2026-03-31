use super::*;
use visitor_trait::*;
use crate::types::Type;

struct TypeChecker<'a> {
    symbols: &'a mut SymbolTable,
    scope_depth: usize,
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

pub fn is_static<T: HasStorage>(decl: &T) -> bool {
    decl.storage_class() == Some(StorageClass::Static)
}

pub fn is_extern<T: HasStorage>(decl: &T) -> bool {
    decl.storage_class() == Some(StorageClass::Extern)
}

impl<'a> TypeChecker<'a> {
    fn check_global_var(&mut self, decl: &mut VarDeclaration) -> Result<(), SemanticError> {
        let mut initial_value = match decl.init {
            Some(Expression::Constant(i)) => InitialValue::Initial(i),
            None => if is_extern(decl) {
                InitialValue::NoInitializer
            } else {
                InitialValue::Tentative
            },
            _ => return Err(SemanticError::NonConstantInitializer(decl.identifier.clone())),
        };

        let mut global = !is_static(decl);

        if let Some(old) = self.symbols.get(&decl.identifier) {
            if let IdentAttrs::StaticAttr { init: old_init, global: old_global } = &old.attrs {
                if old.datatype != Type::Int {
                    return Err(SemanticError::FuncUsedAsVar(decl.identifier.clone()));
                }

                if is_extern(decl) {
                    global = *old_global;
                } else if global != *old_global {
                    return Err(SemanticError::ConflictingStorageTypes(decl.identifier.clone()));
                }

                if matches!(old_init, InitialValue::Initial(..)) {
                    if matches!(initial_value, InitialValue::Initial(..)) {
                        return Err(SemanticError::ConflictingDefinitions(decl.identifier.clone()));
                    } else {
                        initial_value = old_init.clone();
                    }
                } else if !matches!(initial_value, InitialValue::Initial(..)) && matches!(old_init, InitialValue::Tentative) {
                    initial_value = InitialValue::Tentative;
                }
            } else {
                return Err(SemanticError::FuncUsedAsVar(decl.identifier.clone()));
            }
        }
        self.symbols.insert(decl.identifier.clone(),
            Symbol::new_static_var(decl.identifier.clone(), Type::Int, initial_value, global));
        Ok(())
    }
}

impl<'a> Visitor for TypeChecker<'a> {
    fn visit_func_decl(&mut self, function: &mut FuncDeclaration) -> Result<(), SemanticError> {
        let func_type = Type::FuncType(function.params.len());
        let has_body = function.body.is_some();
        let mut alr_def = false;
        let mut global = match function.storage {
            Some(StorageClass::Static) => false,
            _ => true,
        };

        if let Some(old) = self.symbols.get(&function.identifier) {
            if let IdentAttrs::FuncAttr { defined: olddef, global: oldglobal } = old.attrs {
                if old.datatype != func_type {
                    return Err(SemanticError::IncompatibleFuncDeclaration(function.identifier.clone()));
                }
                alr_def = olddef;
                if alr_def && has_body {
                    return Err(SemanticError::DoubleDeclaration(function.identifier.clone()));
                }

                if oldglobal && function.storage == Some(StorageClass::Static) {
                    return Err(SemanticError::StaticAfterNonStatic(function.identifier.clone()));
                }
                global = oldglobal;
            } else {
                return Err(SemanticError::IncompatibleFuncDeclaration(function.identifier.clone()));
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
                return Err(SemanticError::InitializerOnLocalExtern(decl.identifier.clone()));
            }
            if let Some(old) = self.symbols.get(&decl.identifier) {
                if old.datatype != Type::Int {
                    return Err(SemanticError::FuncUsedAsVar(decl.identifier.clone()));
                }
            } else {
                self.symbols.insert(decl.identifier.clone(),
                    Symbol::new_static_var(decl.identifier.clone(), Type::Int, InitialValue::NoInitializer, true));
            }
        } else if is_static(decl) {
            let initial_value;
            if let Some(Expression::Constant(i)) = decl.init {
                initial_value = InitialValue::Initial(i);
            } else if decl.init == None {
                initial_value = InitialValue::Initial(0);
            } else {
                return Err(SemanticError::LocalStaticVarNonConstantInit(decl.identifier.clone()));
            }
            self.symbols.insert(decl.identifier.clone(),
                Symbol::new_static_var(decl.identifier.clone(), Type::Int, initial_value, false));
        } else {
            self.symbols.insert(decl.identifier.clone(), Symbol::new_var(decl.identifier.clone(), Type::Int));
            walk_var_decl(self, decl)?;
        }
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
                    if matches!(sym.attrs, IdentAttrs::FuncAttr { .. }) {
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

pub fn type_checking_pass(program: &mut Program, symbols: &mut SymbolTable) -> Result<(), SemanticError> {
    let mut checker = TypeChecker { symbols, scope_depth: 0 };
    checker.visit_program(program)
}
