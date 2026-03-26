use std::collections::HashMap;

use crate::parser::{self, BlockItem};

#[derive(Debug)]
pub struct PoiseProg {
    pub function: PoiseFunc,
}

#[derive(Debug)]
pub struct PoiseFunc {
    pub identifier: String,
    pub body: Vec<PoiseInstruction>
}

#[derive(Debug)]
pub enum PoiseInstruction {
    Return(PoiseVal),
    Unary{op: PoiseUnaryOp, src: PoiseVal, dst: PoiseVal},
    Binary{op: PoiseBinaryOp, src1: PoiseVal, src2: PoiseVal, dst: PoiseVal},
    Copy{src: PoiseVal, dst:PoiseVal},
    Jump(String),
    JumpIfZero{condition: PoiseVal, identifier: String},
    JumpIfNotZero{condition: PoiseVal, identifier: String},
    Label(String)
}

#[derive(Debug, Clone)]
pub enum PoiseVal {
    Constant(i32),
    Variable(String),
}

#[derive(Debug)]
pub enum PoiseUnaryOp {
    Complement,
    Negate,
    Not,
}

#[derive(Debug)]
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

    fn loop_label_string(&mut self, lab: String, labtype: &str) -> String {
        let ret = format!("{}_{}", labtype, lab);
        ret.to_string()
    }
}

pub fn gen_poise(tree: parser::Program) -> PoiseProg {
    let mut count = TmpCount{var_counter: 0, label_counter: 0};
    let function = gen_poisefunc(tree.function, &mut count);
    PoiseProg { function }
}

fn gen_poisefunc(func: parser::Function, count: &mut TmpCount) -> PoiseFunc {
    let mut instructions = Vec::new();
    let name = func.identifier;
    gen_inst_block(func.body, &mut instructions, count);
    instructions.push(PoiseInstruction::Return(PoiseVal::Constant(0)));
    PoiseFunc{ identifier: name, body: instructions }
}

fn gen_inst_block(block: parser::Block, instructions: &mut Vec<PoiseInstruction>, count: &mut TmpCount) {
    for blockitem in block.items {
        match blockitem {
            parser::BlockItem::S(s) => gen_inst_statement(s, instructions, count),
            parser::BlockItem::D(d) => gen_inst_declaration(d, instructions, count),
        }
    }
}

fn gen_inst_declaration(declaration: parser::Declaration, instructions: &mut Vec<PoiseInstruction>, count: &mut TmpCount) {
    if let Some(exp) = declaration.init {
        let val = emit_expression(exp, instructions, count);
        instructions.push(PoiseInstruction::Copy { src: val, dst: PoiseVal::Variable(declaration.identifier) });
    }
}

fn gen_inst_statement(statement: parser::Statement, instructions: &mut Vec<PoiseInstruction>, count: &mut TmpCount) {
    match statement {
        parser::Statement::Return(expression) => {
            let val = emit_expression(expression, instructions, count);
            instructions.push(PoiseInstruction::Return(val));
        }
        parser::Statement::Expression(expression) => {
            emit_expression(expression, instructions, count);
        }
        parser::Statement::Null => return,
        parser::Statement::If(c, y, n) => {
            let cond = count.new_var();
            let eval = emit_expression(c, instructions, count);
            let no_label = count.new_label_string();
            instructions.push(PoiseInstruction::Copy { src: eval, dst: cond.clone() });
            instructions.push(PoiseInstruction::JumpIfZero { condition: cond, identifier: no_label.clone() });
            gen_inst_statement(*y, instructions, count);
            if let Some(n) = n {
                let yes_label = count.new_label_string();
                instructions.push(PoiseInstruction::Jump(yes_label.clone()));
                instructions.push(PoiseInstruction::Label(no_label));
                gen_inst_statement(*n, instructions, count);
                instructions.push(PoiseInstruction::Label(yes_label));
            } else {
                instructions.push(PoiseInstruction::Label(no_label));
            }
        },
        parser::Statement::Label(name, body) => {
            instructions.push(PoiseInstruction::Label(name));
            gen_inst_statement(*body, instructions, count);
        },
        parser::Statement::Goto(name) => instructions.push(PoiseInstruction::Jump(name)),
        parser::Statement::Compound(block) => gen_inst_block(block, instructions, count),
        parser::Statement::Break(lab) => instructions.push(PoiseInstruction::Jump(count.loop_label_string(lab, "break"))),
        parser::Statement::Continue(lab) => instructions.push(PoiseInstruction::Jump(count.loop_label_string(lab, "cont"))),
        parser::Statement::DoWhile { body, cond, lab } => {
            instructions.push(PoiseInstruction::Label(count.loop_label_string(lab.clone(), "start")));
            gen_inst_statement(*body, instructions, count);
            instructions.push(PoiseInstruction::Label(count.loop_label_string(lab.clone(), "cont")));
            let res = emit_expression(cond, instructions, count);
            instructions.push(PoiseInstruction::JumpIfNotZero { condition: res, identifier: count.loop_label_string(lab.clone(), "start") });
            instructions.push(PoiseInstruction::Label(count.loop_label_string(lab, "break")));
            
        },
        parser::Statement::While { cond, body, lab }  => {
            instructions.push(PoiseInstruction::Label(count.loop_label_string(lab.clone(), "cont")));
            let res = emit_expression(cond, instructions, count);
            instructions.push(PoiseInstruction::JumpIfZero { condition: res, identifier: count.loop_label_string(lab.clone(), "break") });
            gen_inst_statement(*body, instructions, count);
            instructions.push(PoiseInstruction::Jump(count.loop_label_string(lab.clone(), "cont")));
            instructions.push(PoiseInstruction::Label(count.loop_label_string(lab, "break")));
            
        },
        parser::Statement::For { init, cond, post, body, lab } => {
            if let parser::ForInit::InitExp(Some(exp)) = init {
                emit_expression(exp, instructions, count);
            } else if let parser::ForInit::InitDec(dec) = init {
                gen_inst_declaration(dec, instructions, count);
            }
            instructions.push(PoiseInstruction::Label(count.loop_label_string(lab.clone(), "start")));
            if let Some(cond) = cond {
                let res = emit_expression(cond, instructions, count);
                instructions.push(PoiseInstruction::JumpIfZero { condition: res, identifier: count.loop_label_string(lab.clone(), "break") });
            }
            gen_inst_statement(*body, instructions, count);
            instructions.push(PoiseInstruction::Label(count.loop_label_string(lab.clone(), "cont")));
            if let Some(post) = post {
                emit_expression(post, instructions, count);
            }
            instructions.push(PoiseInstruction::Jump(count.loop_label_string(lab.clone(), "start")));
            instructions.push(PoiseInstruction::Label(count.loop_label_string(lab, "break")));
        },
        parser::Statement::Switch { scrutinee, body, lab, cases } => {
            let scr = emit_expression(scrutinee, instructions, count);
            for case in cases.clone() {
                if let (Some(value), clab) = case {
                    let caseval = emit_expression(value, instructions, count);
                    let cmp = count.new_var();
                    instructions.push(PoiseInstruction::Binary { op: PoiseBinaryOp::Equal, src1: caseval, src2: scr.clone(), dst: cmp.clone() });
                    instructions.push(PoiseInstruction::JumpIfNotZero { condition: cmp, identifier: count.loop_label_string(clab.clone(), "case") });
                } 
            }
            for case in cases {
                if let (None, clab) = case {
                    instructions.push(PoiseInstruction::Jump(count.loop_label_string(clab.clone(), "default")));
                }
            }
            instructions.push(PoiseInstruction::Jump(count.loop_label_string(lab.clone(), "break")));
            gen_inst_statement(*body, instructions, count);
            instructions.push(PoiseInstruction::Label(count.loop_label_string(lab.clone(), "break")));
        },
        parser::Statement::Case { lab,.. } => {
            instructions.push(PoiseInstruction::Label(count.loop_label_string(lab.clone(), "case")));
        }, 
        parser::Statement::Default { lab } => {
            instructions.push(PoiseInstruction::Label(count.loop_label_string(lab.clone(), "default")));
        },
        _ => todo!(),
    }
}

// Constructs IR instructions and returns the destination
fn emit_expression(
    expr: parser::Expression,
    instructions: &mut Vec<PoiseInstruction>,
    count: &mut TmpCount) -> PoiseVal {
    match expr {
        parser::Expression::Constant(val) => PoiseVal::Constant(val),
        parser::Expression::Unary(op, inner) => emit_un_exp(op, *inner, instructions, count),
        parser::Expression::Binary(op, exp1, exp2) => emit_bin_exp(op, *exp1, *exp2, instructions, count),
        parser::Expression::Var(name) => PoiseVal::Variable(name),
        parser::Expression::Assignment(lhs, rhs) => {
            let result = emit_expression(*rhs, instructions, count);
            let dest = emit_expression(*lhs, instructions, count);
            instructions.push(PoiseInstruction::Copy { src: result, dst: dest.clone()});
            dest
        },
        parser::Expression::Conditional(c, y, n) => {
            let cond = count.new_var();
            let eval = emit_expression(*c, instructions, count);
            let no_label = count.new_label_string();
            let yes_label = count.new_label_string();
            let dest = count.new_var();

            instructions.push(PoiseInstruction::Copy { src: eval, dst: cond.clone() });

            instructions.push(PoiseInstruction::JumpIfZero { condition: cond, identifier: no_label.clone() });
            let result = emit_expression(*y, instructions, count);
            instructions.push(PoiseInstruction::Copy { src: result, dst: dest.clone()});
            instructions.push(PoiseInstruction::Jump(yes_label.clone()));
            instructions.push(PoiseInstruction::Label(no_label));

            let result = emit_expression(*n, instructions, count);
            instructions.push(PoiseInstruction::Copy { src: result, dst: dest.clone()});
            instructions.push(PoiseInstruction::Label(yes_label));
            dest
        },
        parser::Expression::PrefixIncrement(e) => {
            let var = emit_expression(*e, instructions, count);
            instructions.push(PoiseInstruction::Binary{
                op: PoiseBinaryOp::Add,
                src1: var.clone(),
                src2: PoiseVal::Constant(1),
                dst: var.clone(),
            });
            var
        },
        parser::Expression::PrefixDecrement(e) => {
            let var = emit_expression(*e, instructions, count);
            instructions.push(PoiseInstruction::Binary{
                op: PoiseBinaryOp::Subtract,
                src1: var.clone(),
                src2: PoiseVal::Constant(1),
                dst: var.clone(),
            });
            var
        },
        parser::Expression::PostfixIncrement(e) => {
            let var = emit_expression(*e, instructions, count);
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
        parser::Expression::PostfixDecrement(e) => {
            let var = emit_expression(*e, instructions, count);
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
        _ => todo!(),
    }
}

fn emit_bin_exp(op: parser::BinaryOp,
    exp1: parser::Expression,
    exp2: parser::Expression,
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
            _ => todo!()
        };
        let v1 = emit_expression(exp1, instructions, count);
        let v2 = emit_expression(exp2, instructions, count);
        let dst = count.new_var();
        instructions.push(PoiseInstruction::Binary {op: binop, src1: v1, src2: v2, dst: dst.clone() });
        dst
}

fn emit_un_exp(op: parser::UnaryOp,
    exp: parser::Expression,
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

fn emit_short_circuit_exp(op: parser::BinaryOp,
    exp1: parser::Expression,
    exp2: parser::Expression,
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
