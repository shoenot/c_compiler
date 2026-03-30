use std::collections::HashMap;
use visitor_trait::*;
use super::*;

struct LabelMangler {
    label_map: HashMap<String, (String, bool)>,
}

impl LabelMangler {
    fn labelgen(&mut self, name: &str) -> String {
        format!("userlab.{}", name)
    }

    fn check_undeclared_label(&mut self) -> Result<(), SemanticError> {
        for (key, (_, status)) in &self.label_map {
            if !status {
                return Err(SemanticError::UndeclaredLabel(key.clone()));
            }
        }
        Ok(())
    }
}

impl Visitor for LabelMangler {
    fn visit_statement(&mut self, statement: &mut Statement) -> Result<(), SemanticError> {
       match statement {
           Statement::Label(name, st) => {
                let labelname = name.clone();
                if let Some((mangled, status)) = self.label_map.get_mut(&labelname) {
                    let mangled = mangled.clone();
                    if *status {
                        return Err(SemanticError::DuplicateLabel(mangled));
                    } else {
                        *status = true;
                        *name = mangled;
                    }
                } else {
                    let newlabel = self.labelgen(&labelname);
                    self.label_map.insert(labelname, (newlabel.clone(), true));
                    *name = newlabel;
                }
                self.visit_statement(st)?;
                Ok(())
           },
           Statement::Goto(name) => {
                let labelname = name.clone();
                if self.label_map.contains_key(&labelname) {
                    let (newname, _) = self.label_map.get(&labelname).unwrap();
                    *name = String::from(newname);
                    Ok(())
                } else {
                    let newlabel = self.labelgen(&labelname);
                    self.label_map.insert(labelname, (newlabel.clone(), false));
                    *name = newlabel;
                    Ok(())
                }
           },
           _ => self.visit_statement(statement),
        }
    }
}

pub fn label_mangling_pass(program: &mut Program) -> Result<(), SemanticError> {
    let mut mangler = LabelMangler { label_map: HashMap::new() };
    mangler.visit_program(program)?;
    mangler.check_undeclared_label()
}
