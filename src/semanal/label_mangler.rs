use std::collections::HashMap;
use visitor_trait::*;
use super::*;

struct LabelMangler {
    label_map: HashMap<String, (String, bool)>,
    counter: usize,
}

impl LabelMangler {
    fn labelgen(&mut self, name: &str) -> String {
        let lab = format!("userlab.{}_{}", name, self.counter);
        self.counter += 1;
        lab
    }

    fn check_undeclared_label(&mut self, span: Span) -> Result<(), SemanticError> {
        for (key, (_, status)) in &self.label_map {
            if !status {
                return Err(SemanticError::UndeclaredLabel(key.clone(), span));
            }
        }
        Ok(())
    }
}

impl Visitor for LabelMangler {
    fn visit_declaration(&mut self, declaration: &mut Decl) -> Result<(), SemanticError> {
        if let Decl::FuncDecl(f) = declaration {
            self.label_map = HashMap::new();
            self.visit_func_decl(f)?;
            self.check_undeclared_label(f.span)?;
        } else {
            walk_declaration(self, declaration)?;
        }
        Ok(())
    }        

    fn visit_statement(&mut self, statement: &mut Statement) -> Result<(), SemanticError> {
       match &mut statement.kind {
           StatementKind::Label(name, st) => {
                let labelname = name.clone();
                if let Some((mangled, status)) = self.label_map.get_mut(&labelname) {
                    let mangled = mangled.clone();
                    if *status {
                        return Err(SemanticError::DuplicateLabel(mangled, statement.span));
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
           StatementKind::Goto(name) => {
                let labelname = name.clone();
                if self.label_map.contains_key(&labelname) {
                    let (newname, _) = self.label_map.get(&labelname).unwrap();
                    *name = newname.clone();
                    Ok(())
                } else {
                    let newlabel = self.labelgen(&labelname);
                    self.label_map.insert(labelname, (newlabel.clone(), false));
                    *name = newlabel;
                    Ok(())
                }
           },
           _ => walk_statement(self, statement),
        }
    }
}

pub fn label_mangling_pass(program: &mut Program) -> Result<(), SemanticError> {
    let mut mangler = LabelMangler { label_map: HashMap::new(), counter: 0 };
    mangler.visit_program(program)
}
