use crate::parser; 
use crate::types::{IdentAttrs, InitialValue, SymbolTable};

#[derive(Debug)]
pub struct PoiseProg {
    pub top_level_items: Vec<TopLevelItem>,
}

#[derive(Debug)]
pub enum TopLevelItem {
    F(PoiseFunc),
    V(PoiseStaticVar),
}

#[derive(Debug)]
pub struct PoiseFunc {
    pub identifier: String, 
    pub global: bool, 
    pub params: Vec<String>, 
    pub body: Vec<PoiseInstruction>
}        


#[derive(Debug)]
pub struct PoiseStaticVar {
    pub identifier: String, 
    pub global: bool, 
    pub init: i32
}
        
#[derive(Debug, Clone)]
pub enum PoiseInstruction {
    Return(PoiseVal),
    Unary{op: PoiseUnaryOp, src: PoiseVal, dst: PoiseVal},
    Binary{op: PoiseBinaryOp, src1: PoiseVal, src2: PoiseVal, dst: PoiseVal},
    Copy{src: PoiseVal, dst:PoiseVal},
    Jump(String),
    JumpIfZero{condition: PoiseVal, identifier: String},
    JumpIfNotZero{condition: PoiseVal, identifier: String},
    Label(String),
    FunctionCall{ident: String, args: Vec<PoiseVal>, dst: PoiseVal}
}

#[derive(Debug, Clone)]
pub enum PoiseVal {
    Constant(i32),
    Variable(String),
}

#[derive(Debug, Clone)]
pub enum PoiseUnaryOp {
    Complement,
    Negate,
    Not,
}

#[derive(Debug, Clone)]
pub enum PoiseBinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    LeftShift,
    RightShift,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessOrEqual,
    GreaterOrEqual,
}

struct TmpCount {
    var_counter: usize,
    label_counter: usize,
}

impl TmpCount {
    fn new_var(&mut self) -> PoiseVal {
        let name = format!("tmp.{}", self.var_counter);
        self.var_counter += 1;
        PoiseVal::Variable(name)
    }

    fn new_label_string(&mut self) -> String {
        let name = format!("lab.{}", self.label_counter);
        self.label_counter += 1;
        name
    }
}

fn loop_label_string(lab: String, labtype: &str) -> String {
    let ret = format!("{}_{}", labtype, lab);
    ret.into()
}

pub fn gen_poise(tree: &parser::Program, symbols: &SymbolTable) -> PoiseProg {
    let mut count = TmpCount{var_counter: 0, label_counter: 0};
    let mut top_level_items = Vec::new();
    for decl in &tree.declarations {
        match decl {
            parser::Decl::FuncDecl(f) => if f.body.is_some() {
                top_level_items.push(TopLevelItem::F(gen_poisefunc(f, &mut count, symbols)));
            },
            _ => {},
        }
    }
    top_level_items.extend(gen_static_symbols(symbols));
    PoiseProg { top_level_items }
}

fn gen_poisefunc(func: &parser::FuncDeclaration, count: &mut TmpCount, symbols: &SymbolTable) -> PoiseFunc {
    let mut instructions = Vec::new();
    let identifier = func.identifier.clone();
    let params = func.params.clone();
    gen_inst_block(func.body.as_ref().unwrap(), &mut instructions, count, symbols);
    instructions.push(PoiseInstruction::Return(PoiseVal::Constant(0)));
    let mut global = false;
    if let Some(sym) = symbols.get(&identifier) {
        global = sym.attrs.is_global()
    }
    PoiseFunc { identifier, global, params, body: instructions.to_vec() }
}

fn gen_inst_block(block: &parser::Block, instructions: &mut Vec<PoiseInstruction>, count: &mut TmpCount, symbols: &SymbolTable) {
    for blockitem in &block.items {
        match blockitem {
            parser::BlockItem::S(s) => gen_inst_statement(s, instructions, count, symbols),
            parser::BlockItem::D(parser::Decl::VarDecl(d)) => gen_inst_var_declaration(d, instructions, count, symbols),
            parser::BlockItem::D(parser::Decl::FuncDecl(_)) => {},
        }
    }
}

fn gen_inst_var_declaration(declaration: &parser::VarDeclaration, instructions: &mut Vec<PoiseInstruction>, count: &mut TmpCount, symbols: &SymbolTable) {
    if let Some(sym) = symbols.get(&declaration.identifier) {
        if matches!(sym.attrs, IdentAttrs::StaticAttr { .. }) {
            return;
        }
    }
    if let Some(exp) = declaration.init.as_ref() {
        let val = emit_expression(exp, instructions, count);
        instructions.push(PoiseInstruction::Copy { src: val, dst: PoiseVal::Variable(declaration.identifier.clone()) });
    }
}

fn gen_inst_statement(statement: &parser::Statement, instructions: &mut Vec<PoiseInstruction>, count: &mut TmpCount, symbols: &SymbolTable) {
    match &statement.kind {
        parser::StatementKind::Return(expression) => {
            let val = emit_expression(expression, instructions, count);
            instructions.push(PoiseInstruction::Return(val));
        }
        parser::StatementKind::Expression(expression) => {
            emit_expression(expression, instructions, count);
        }
        parser::StatementKind::Null => return,
        parser::StatementKind::If(c, y, n) => {
            let cond = count.new_var();
            let eval = emit_expression(c, instructions, count);
            let no_label = count.new_label_string();
            instructions.push(PoiseInstruction::Copy { src: eval, dst: cond.clone() });
            instructions.push(PoiseInstruction::JumpIfZero { condition: cond, identifier: no_label.clone() });
            gen_inst_statement(y, instructions, count, symbols);
            if let Some(n) = n {
                let yes_label = count.new_label_string();
                instructions.push(PoiseInstruction::Jump(yes_label.clone()));
                instructions.push(PoiseInstruction::Label(no_label));
                gen_inst_statement(n, instructions, count, symbols);
                instructions.push(PoiseInstruction::Label(yes_label));
            } else {
                instructions.push(PoiseInstruction::Label(no_label));
            }
        },
        parser::StatementKind::Label(name, body) => {
            instructions.push(PoiseInstruction::Label(name.clone()));
            gen_inst_statement(body, instructions, count, symbols);
        },
        parser::StatementKind::Goto(name) => instructions.push(PoiseInstruction::Jump(name.clone())),
        parser::StatementKind::Compound(block) => gen_inst_block(block, instructions, count, symbols),
        parser::StatementKind::Break(lab) => instructions.push(PoiseInstruction::Jump(loop_label_string(lab.clone(), "break"))),
        parser::StatementKind::Continue(lab) => instructions.push(PoiseInstruction::Jump(loop_label_string(lab.clone(), "cont"))),
        parser::StatementKind::DoWhile { body, cond, lab } => {
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "start")));
            gen_inst_statement(body, instructions, count, symbols);
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "cont")));
            let res = emit_expression(cond, instructions, count);
            instructions.push(PoiseInstruction::JumpIfNotZero { condition: res, identifier: loop_label_string(lab.clone(), "start") });
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "break")));
            
        },
        parser::StatementKind::While { cond, body, lab }  => {
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "cont")));
            let res = emit_expression(cond, instructions, count);
            instructions.push(PoiseInstruction::JumpIfZero { condition: res, identifier: loop_label_string(lab.clone(), "break") });
            gen_inst_statement(body, instructions, count, symbols);
            instructions.push(PoiseInstruction::Jump(loop_label_string(lab.clone(), "cont")));
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "break")));
            
        },
        parser::StatementKind::For { init, cond, post, body, lab } => {
            if let parser::ForInit::InitExp(Some(exp)) = init {
                emit_expression(exp, instructions, count);
            } else if let parser::ForInit::InitDec(dec) = init {
                gen_inst_var_declaration(dec, instructions, count, symbols);
            }
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "start")));
            if let Some(cond) = cond {
                let res = emit_expression(cond, instructions, count);
                instructions.push(PoiseInstruction::JumpIfZero { condition: res, identifier: loop_label_string(lab.clone(), "break") });
            }
            gen_inst_statement(body, instructions, count, symbols);
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "cont")));
            if let Some(post) = post {
                emit_expression(post, instructions, count);
            }
            instructions.push(PoiseInstruction::Jump(loop_label_string(lab.clone(), "start")));
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "break")));
        },
        parser::StatementKind::Switch { scrutinee, body, lab, cases } => {
            let scr = emit_expression(scrutinee, instructions, count);
            for case in cases.clone() {
                if let (Some(value), clab) = case {
                    let caseval = emit_expression(&value, instructions, count);
                    let cmp = count.new_var();
                    instructions.push(PoiseInstruction::Binary { op: PoiseBinaryOp::Equal, src1: caseval, src2: scr.clone(), dst: cmp.clone() });
                    instructions.push(PoiseInstruction::JumpIfNotZero { condition: cmp, identifier: loop_label_string(clab.clone(), "case") });
                } 
            }
            for case in cases {
                if let (None, clab) = case {
                    instructions.push(PoiseInstruction::Jump(loop_label_string(clab.clone(), "default")));
                }
            }
            instructions.push(PoiseInstruction::Jump(loop_label_string(lab.clone(), "break")));
            gen_inst_statement(body, instructions, count, symbols);
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "break")));
        },
        parser::StatementKind::Case { lab,.. } => {
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "case")));
        }, 
        parser::StatementKind::Default { lab } => {
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "default")));
        },
    }
}

// Constructs IR instructions and returns the destination
fn emit_expression(
    expr: &parser::Expression,
    instructions: &mut Vec<PoiseInstruction>,
    count: &mut TmpCount) -> PoiseVal {
    match &expr.kind {
        parser::ExpressionKind::Constant(val) => PoiseVal::Constant(*val),
        parser::ExpressionKind::Unary(op, inner) => emit_un_exp(op, inner, instructions, count),
        parser::ExpressionKind::Binary(op, exp1, exp2) => emit_bin_exp(op, exp1, exp2, instructions, count),
        parser::ExpressionKind::Var(name) => PoiseVal::Variable(name.clone()),
        parser::ExpressionKind::Assignment(lhs, rhs) => {
            let result = emit_expression(rhs, instructions, count);
            let dest = emit_expression(lhs, instructions, count);
            instructions.push(PoiseInstruction::Copy { src: result, dst: dest.clone()});
            dest
        },
        parser::ExpressionKind::Conditional(c, y, n) => {
            let cond = count.new_var();
            let eval = emit_expression(c, instructions, count);
            let no_label = count.new_label_string();
            let yes_label = count.new_label_string();
            let dest = count.new_var();

            instructions.push(PoiseInstruction::Copy { src: eval, dst: cond.clone() });

            instructions.push(PoiseInstruction::JumpIfZero { condition: cond, identifier: no_label.clone() });
            let result = emit_expression(y, instructions, count);
            instructions.push(PoiseInstruction::Copy { src: result, dst: dest.clone()});
            instructions.push(PoiseInstruction::Jump(yes_label.clone()));
            instructions.push(PoiseInstruction::Label(no_label));

            let result = emit_expression(n, instructions, count);
            instructions.push(PoiseInstruction::Copy { src: result, dst: dest.clone()});
            instructions.push(PoiseInstruction::Label(yes_label));
            dest
        },
        parser::ExpressionKind::PrefixIncrement(e) => {
            let var = emit_expression(e, instructions, count);
            instructions.push(PoiseInstruction::Binary{
                op: PoiseBinaryOp::Add,
                src1: var.clone(),
                src2: PoiseVal::Constant(1),
                dst: var.clone(),
            });
            var
        },
        parser::ExpressionKind::PrefixDecrement(e) => {
            let var = emit_expression(e, instructions, count);
            instructions.push(PoiseInstruction::Binary{
                op: PoiseBinaryOp::Subtract,
                src1: var.clone(),
                src2: PoiseVal::Constant(1),
                dst: var.clone(),
            });
            var
        },
        parser::ExpressionKind::PostfixIncrement(e) => {
            let var = emit_expression(e, instructions, count);
            let tmp = count.new_var();
            instructions.push(PoiseInstruction::Copy { src: var.clone(), dst: tmp.clone() });
            instructions.push(PoiseInstruction::Binary{
                op: PoiseBinaryOp::Add,
                src1: var.clone(),
                src2: PoiseVal::Constant(1),
                dst: var.clone(),
            });
            tmp
        },
        parser::ExpressionKind::PostfixDecrement(e) => {
            let var = emit_expression(e, instructions, count);
            let tmp = count.new_var();
            instructions.push(PoiseInstruction::Copy { src: var.clone(), dst: tmp.clone() });
            instructions.push(PoiseInstruction::Binary{
                op: PoiseBinaryOp::Subtract,
                src1: var.clone(),
                src2: PoiseVal::Constant(1),
                dst: var.clone(),
            });
            tmp
        },
        parser::ExpressionKind::FunctionCall(ident, args) => {
            let mut poiseargs = Vec::new();
            let dst = count.new_var();
            for arg in args {
                let exp = emit_expression(arg, instructions, count);
                let tmp = count.new_var();
                instructions.push(PoiseInstruction::Copy { src: exp, dst: tmp.clone() });
                poiseargs.push(tmp);
            }
            instructions.push(PoiseInstruction::FunctionCall { ident: ident.clone(), args: poiseargs, dst: dst.clone() });
            dst
        }
    }
}

fn emit_bin_exp(op: &parser::BinaryOp,
    exp1: &parser::Expression,
    exp2: &parser::Expression,
    instructions: &mut Vec<PoiseInstruction>,
    count: &mut TmpCount) -> PoiseVal {
        let binop = match op {
            parser::BinaryOp::LogicalAnd | parser::BinaryOp::LogicalOr => {
                return emit_short_circuit_exp(op, exp1, exp2, instructions, count);
            },
            parser::BinaryOp::Add => PoiseBinaryOp::Add,
            parser::BinaryOp::Subtract => PoiseBinaryOp::Subtract,
            parser::BinaryOp::Multiply => PoiseBinaryOp::Multiply,
            parser::BinaryOp::Divide => PoiseBinaryOp::Divide,
            parser::BinaryOp::Remainder => PoiseBinaryOp::Remainder,
            parser::BinaryOp::LeftShift => PoiseBinaryOp::LeftShift,
            parser::BinaryOp::RightShift => PoiseBinaryOp::RightShift,
            parser::BinaryOp::BitwiseAnd => PoiseBinaryOp::BitwiseAnd,
            parser::BinaryOp::BitwiseOr => PoiseBinaryOp::BitwiseOr,
            parser::BinaryOp::BitwiseXor => PoiseBinaryOp::BitwiseXor,
            parser::BinaryOp::Equal => PoiseBinaryOp::Equal,
            parser::BinaryOp::NotEqual => PoiseBinaryOp::NotEqual,
            parser::BinaryOp::LessThan => PoiseBinaryOp::LessThan,
            parser::BinaryOp::GreaterThan => PoiseBinaryOp::GreaterThan,
            parser::BinaryOp::LessOrEqual => PoiseBinaryOp::LessOrEqual,
            parser::BinaryOp::GreaterOrEqual => PoiseBinaryOp::GreaterOrEqual,
            _ => unreachable!()
        };
        let v1 = emit_expression(exp1, instructions, count);
        let v2 = emit_expression(exp2, instructions, count);
        let dst = count.new_var();
        instructions.push(PoiseInstruction::Binary {op: binop, src1: v1, src2: v2, dst: dst.clone() });
        dst
}

fn emit_un_exp(op: &parser::UnaryOp,
    exp: &parser::Expression,
    instructions: &mut Vec<PoiseInstruction>,
    count: &mut TmpCount) -> PoiseVal {
        let src = emit_expression(exp, instructions, count);
        let dst = count.new_var();
        let unary_op = match op {
            parser::UnaryOp::Negate => PoiseUnaryOp::Negate,
            parser::UnaryOp::Complement => PoiseUnaryOp::Complement,
            parser::UnaryOp::Not => PoiseUnaryOp::Not,
        };
        instructions.push(PoiseInstruction::Unary { op: unary_op, src, dst: dst.clone() });
        dst
}

fn emit_short_circuit_exp(op: &parser::BinaryOp,
    exp1: &parser::Expression,
    exp2: &parser::Expression,
    instructions: &mut Vec<PoiseInstruction>,
    count: &mut TmpCount) -> PoiseVal {

    let false_label = count.new_label_string();
    let true_label = count.new_label_string();
    let dst = count.new_var();

    match op {
        parser::BinaryOp::LogicalAnd => {
            let v1 = emit_expression(exp1, instructions, count);
            instructions.push(PoiseInstruction::Copy{src: v1.clone(), dst: dst.clone()});
            instructions.push(PoiseInstruction::JumpIfZero { condition: dst.clone(), identifier: false_label.clone() });

            let v2 = emit_expression(exp2, instructions, count);
            instructions.push(PoiseInstruction::Copy{src: v2.clone(), dst: dst.clone()});
            instructions.push(PoiseInstruction::JumpIfZero { condition: dst.clone(), identifier: false_label.clone() });

            instructions.push(PoiseInstruction::Copy{src: PoiseVal::Constant(1), dst: dst.clone()});
            instructions.push(PoiseInstruction::Jump(true_label.clone()));
            instructions.push(PoiseInstruction::Label(false_label));
            instructions.push(PoiseInstruction::Copy{src: PoiseVal::Constant(0), dst: dst.clone() });
            instructions.push(PoiseInstruction::Label(true_label));
            dst
        },
        parser::BinaryOp::LogicalOr => {
            let v1 = emit_expression(exp1, instructions, count);
            instructions.push(PoiseInstruction::Copy{src: v1.clone(), dst: dst.clone()});
            instructions.push(PoiseInstruction::JumpIfNotZero { condition: dst.clone(), identifier: true_label.clone() });

            let v2 = emit_expression(exp2, instructions, count);
            instructions.push(PoiseInstruction::Copy{src: v2.clone(), dst: dst.clone()});
            instructions.push(PoiseInstruction::JumpIfNotZero { condition: dst.clone(), identifier: true_label.clone() });

            instructions.push(PoiseInstruction::Copy{src: PoiseVal::Constant(0), dst: dst.clone()});
            instructions.push(PoiseInstruction::Jump(false_label.clone()));
            instructions.push(PoiseInstruction::Label(true_label));
            instructions.push(PoiseInstruction::Copy{src: PoiseVal::Constant(1), dst: dst.clone() });
            instructions.push(PoiseInstruction::Label(false_label));
            dst
        },
        _ => panic!(),
    }
}

fn gen_static_symbols(symbols: &SymbolTable) -> Vec<TopLevelItem> {
    let mut defs = Vec::new();
    for (k, v) in symbols {
        if let IdentAttrs::StaticAttr { init, global } = &v.attrs {
            match init {
                InitialValue::Initial(v) => defs.push(TopLevelItem::V(PoiseStaticVar { identifier: k.clone(), global:*global, init: *v })),
                InitialValue::Tentative => defs.push(TopLevelItem::V(PoiseStaticVar { identifier: k.clone(), global:*global, init: 0 })),
                InitialValue::NoInitializer => {}
            }
        }
    }
    defs
}
