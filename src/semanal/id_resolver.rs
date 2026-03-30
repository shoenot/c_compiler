use std::collections::hash_map::HashMap;
use crate::parser::*;
use super::*;
use visitor_trait::*;

#[derive(Debug)]
pub struct MapEntry {
    mapped_name: String,
    scope: usize,
    has_linkage: bool,
}

struct IdentResolver {
    ident_map: HashMap<String, MapEntry>,
    current_id: usize,
    next_id: usize,
}

impl IdentResolver {
    fn enter_block(&mut self) -> usize {
        let outer_id = self.current_id;
        self.current_id = self.next_id;
        self.next_id += 1;
        outer_id
    }

    fn leave_block(&mut self, outer: usize) {
        self.current_id = outer;
    }

    fn namegen(&mut self, name: &str) -> String {
        let new = format!("{}.{}", name, self.current_id);
        new
    }

    fn resolve_parameter(&mut self, param: &String, scope: usize) -> Result<String, SemanticError> {
        if let Some(entry) = self.ident_map.get(param) {
            if entry.scope == scope {
                return Err(SemanticError::DoubleDeclaration(param.into()));
            }
        }

        let newname = self.namegen(param);
        self.ident_map.insert(param.to_string(), MapEntry { 
            mapped_name: newname.clone(), 
            scope, 
            has_linkage: false, 
        });
        Ok(newname)
    }

}

impl Visitor for IdentResolver {
    fn visit_var_decl(&mut self, var: &mut VarDeclaration) -> Result<(), SemanticError> {
        if self.current_id == 0 {
            self.ident_map.insert(var.identifier.clone(), MapEntry { 
                mapped_name: var.identifier.clone(), 
                scope: 0,  
                has_linkage: true, 
            });
        } else {
            if let Some(entry) = self.ident_map.get(&var.identifier) {
                if entry.scope == self.current_id &&
                   !(entry.has_linkage && var.storage == Some(StorageClass::Extern)) {
                    return Err(SemanticError::DoubleDeclaration(var.identifier.clone()));
                }
            }
            
            if var.storage == Some(StorageClass::Extern) {
                self.ident_map.insert(var.identifier.clone(), MapEntry { 
                    mapped_name: var.identifier.clone(),
                    scope: self.current_id, 
                    has_linkage: true 
                });
            } else {
                let newname = self.namegen(&var.identifier);
                self.ident_map.insert(var.identifier.clone(), MapEntry { 
                    mapped_name: newname.clone(),
                    scope: self.current_id, 
                    has_linkage: true 
                });
                var.identifier = newname.into();
            }

            match &mut var.init {
                None => return Ok(()),
                Some(e) => {
                    self.visit_expression(e)?;
                },
            }
        }
        Ok(())
    }

    fn visit_func_decl(&mut self, func: &mut FuncDeclaration) -> Result<(), SemanticError> {
        if self.current_id != 0 {
            if func.body.is_some() {
                return Err(SemanticError::NestedFunctionDefinition(func.identifier.to_string()));
            }
        }

        if let Some(entry) = self.ident_map.get(&func.identifier) {
            if self.current_id != 0 && !entry.has_linkage {
                return Err(SemanticError::DoubleDeclaration(func.identifier.to_string()));
            }
        }

        self.ident_map.insert(func.identifier.clone(), MapEntry { 
            mapped_name: func.identifier.clone(), 
            scope: self.current_id,  
            has_linkage: true, 
        });
        

        match &mut func.body {
            None => return Ok(()),
            Some(blk) => {
                let param_names: Vec<String> = func.params.iter().cloned().collect();
                let outer = self.enter_block();
                
                let mut new_params = Vec::new();
                for param in &param_names {
                    new_params.push(self.resolve_parameter(param, self.current_id)?);
                }
                func.params = new_params;
                
                for item in &mut blk.items {
                    match item {
                        BlockItem::D(d) => self.visit_declaration(d)?,
                        BlockItem::S(s) => self.visit_statement(s)?,
                    }
                }
                
                self.leave_block(outer);
                Ok(())
            }
        }
    }

    fn visit_block(&mut self, block: &mut Block) -> Result<(), SemanticError> {
        let outer = self.enter_block();
        for item in &mut block.items {
            match item {
                BlockItem::D(d) => self.visit_declaration(d)?,
                BlockItem::S(s) => self.visit_statement(s)?,
            }
        }
        self.leave_block(outer);
        Ok(())
    }

    fn visit_expression(&mut self, expression: &mut Expression) -> Result<(), SemanticError> {
        match expression {
            Expression::Var(x) => {
                if let Some(entry) = self.ident_map.get(x) {
                    *x = entry.mapped_name.clone();
                } else {
                    return Err(SemanticError::UseBeforeDeclaration(x.clone()));
                }
            },
            Expression::Assignment(lhs, rhs) => {
                match lhs.as_mut() {
                    Expression::Var(_) => {},
                    _ => {
                        eprintln!("Invalid L-value: {:?}", lhs);
                        return Err(SemanticError::InvalidLValue);
                    }
                }
                self.visit_expression(lhs.as_mut())?;
                self.visit_expression(rhs.as_mut())?;
            },
            Expression::PostfixIncrement(exp) | Expression::PrefixIncrement(exp) | 
            Expression::PostfixDecrement(exp) | Expression::PrefixDecrement(exp) => {
                match **exp {
                    Expression::Var(_) => self.visit_expression(exp.as_mut())?,
                    _ => return Err(SemanticError::InvalidLValue),
                }
            },
            Expression::FunctionCall(name, args) => {
                if let Some(entry) = self.ident_map.get(name) {
                    *name = entry.mapped_name.clone();
                    for arg in args {
                        self.visit_expression(arg)?;
                    } 
                } else {
                    return Err(SemanticError::UseBeforeDeclaration(name.to_string()));
                }
            },
            _ => walk_expression(self, expression)?,
        }
        Ok(())
    }
}


pub fn identifier_resolution_pass(program: &mut Program) 
    -> Result<HashMap<String, MapEntry>, SemanticError>{
    let mut resolver = IdentResolver {
        ident_map: HashMap::new(),
        current_id: 0,
        next_id: 1,
    };
    resolver.visit_program(program)?;
    Ok(resolver.ident_map)
}
