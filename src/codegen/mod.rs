mod stack;
use stack::*;

use crate::poise::{self, PoiseBinaryOp, PoiseVal};
#[derive(Debug)]
pub struct AsmProgram {
    pub function: AsmFunction,
}

#[derive(Debug)]
pub struct AsmFunction {
    pub name: String,
    pub body: Vec<AsmInstruction>,
}

#[derive(Debug)]
pub enum AsmInstruction {
    Mov(Operand, Operand),
    Movb(Operand, Operand),
    Unary(UnaryOp, Operand),
    Binary(BinaryOp, Operand, Operand),
    Cmp(Operand, Operand),
    Idiv(Operand),
    Cdq,
    Jmp(String),
    JmpCC(Condition, String),
    SetCC(Condition, Operand),
    Label(String),
    AllocateStack(i32),
    Ret,
}

#[derive(Debug)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug)]
pub enum BinaryOp {
    Add,
    Sub,
    Mult,
    Sal,
    Sar,
    BitAnd,
    BitOr,
    BitXor,
}

#[derive(Debug,Clone)]
pub enum Operand {
    Imm(i32),
    Reg(Register),
    Pseudo(String),
    Stack(i32),
}

#[derive(Debug,Clone)]
pub enum Register {
    AX,
    CX,
    CL,
    DX,
    R10,
    R11,
}

#[derive(Debug)]
pub enum Condition {
    E,
    NE,
    L,
    LE,
    G,
    GE,
}

pub fn gen_program(tree: poise::PoiseProg) -> AsmProgram {
    let mut function = gen_function(tree.function);
    function = assign_stack_slots(function);
    AsmProgram { function }
}

fn gen_function(func: poise::PoiseFunc) -> AsmFunction {
    let name = func.identifier;
    let instructions = gen_instructions(func.body);
    AsmFunction { name, body: instructions }
}

fn gen_instructions(instructions: Vec<poise::PoiseInstruction>) -> Vec<AsmInstruction> {
    let mut generated = Vec::new();
    for instruction in instructions {
        match instruction {
            poise::PoiseInstruction::Return(val) => {
                generated.push(AsmInstruction::Mov(gen_operand(val), Operand::Reg(Register::AX)));
                generated.push(AsmInstruction::Ret);
            },
            poise::PoiseInstruction::Unary { op,src,dst } => {
                unary_handler(op, src, dst, &mut generated);
            },
            poise::PoiseInstruction::Binary { op, src1, src2, dst } => {
                binary_handler(op, src1, src2, dst, &mut generated);
            },
            poise::PoiseInstruction::Jump(id) => generated.push(AsmInstruction::Jmp(id)),
            poise::PoiseInstruction::JumpIfZero{condition: cnd, identifier: id} => {
                generated.push(AsmInstruction::Cmp(Operand::Imm(0), gen_operand(cnd)));
                generated.push(AsmInstruction::JmpCC(Condition::E, id))
            }
            poise::PoiseInstruction::JumpIfNotZero{condition: cnd, identifier: id} => {
                generated.push(AsmInstruction::Cmp(Operand::Imm(0), gen_operand(cnd)));
                generated.push(AsmInstruction::JmpCC(Condition::NE, id))
            },
            poise::PoiseInstruction::Copy{src: s, dst: d} => generated.push(AsmInstruction::Mov(gen_operand(s), gen_operand(d))),
            poise::PoiseInstruction::Label(id) => generated.push(AsmInstruction::Label(id)),
        }
    }
    generated
}

fn gen_operand(exp: poise::PoiseVal) -> Operand {
    let operand = match exp {
        poise::PoiseVal::Constant(val) => Operand::Imm(val),
        poise::PoiseVal::Variable(ident) => Operand::Pseudo(ident),
    };
    operand
}

fn unary_handler(op: poise::PoiseUnaryOp, src: PoiseVal, dst: PoiseVal, generated: &mut Vec<AsmInstruction>) {
    let (s, d) = (gen_operand(src), gen_operand(dst));
    generated.push(AsmInstruction::Mov(s.clone(), d.clone()));
    match op {
        poise::PoiseUnaryOp::Negate => generated.push(AsmInstruction::Unary(UnaryOp::Neg, d)),
        poise::PoiseUnaryOp::Complement => generated.push(AsmInstruction::Unary(UnaryOp::Not, d)),
        poise::PoiseUnaryOp::Not => {
            generated.push(AsmInstruction::Cmp(Operand::Imm(0), s));
            generated.push(AsmInstruction::Mov(Operand::Imm(0), d.clone()));
            generated.push(AsmInstruction::SetCC(Condition::E, d));
        },
    };
}

fn gen_binary(exp: poise::PoiseBinaryOp) -> BinaryOp {
    let operator = match exp {
        poise::PoiseBinaryOp::Add => BinaryOp::Add,
        poise::PoiseBinaryOp::Subtract => BinaryOp::Sub,
        poise::PoiseBinaryOp::Multiply => BinaryOp::Mult,
        poise::PoiseBinaryOp::LeftShift =>  BinaryOp::Sal,
        poise::PoiseBinaryOp::RightShift => BinaryOp::Sar,
        poise::PoiseBinaryOp::BitwiseAnd => BinaryOp::BitAnd,
        poise::PoiseBinaryOp::BitwiseOr  => BinaryOp::BitOr,
        poise::PoiseBinaryOp::BitwiseXor => BinaryOp::BitXor,
        _ => panic!(),
    };
    operator
}

fn gen_division(exp: PoiseBinaryOp) -> Register {
    let operator = match exp {
        poise::PoiseBinaryOp::Divide => Register::AX,
        poise::PoiseBinaryOp::Remainder => Register::DX,
        _ => panic!(),
    };
    operator
}

fn gen_conditional(op: PoiseBinaryOp) -> Condition {
    match op {
        PoiseBinaryOp::Equal => Condition::E,
        PoiseBinaryOp::NotEqual => Condition::NE,
        PoiseBinaryOp::GreaterThan => Condition::G,
        PoiseBinaryOp::GreaterOrEqual => Condition::GE,
        PoiseBinaryOp::LessThan => Condition::L,
        PoiseBinaryOp::LessOrEqual => Condition::LE,
        _ => panic!(),
    }
}

fn binary_handler(op: PoiseBinaryOp, src1: PoiseVal, src2: PoiseVal, dst: PoiseVal, generated: &mut Vec<AsmInstruction>) {
    let (s1, s2, d) = (gen_operand(src1), gen_operand(src2), gen_operand(dst));
    match op {
        PoiseBinaryOp::Add | PoiseBinaryOp::Subtract | PoiseBinaryOp::Multiply |
        PoiseBinaryOp::BitwiseAnd | PoiseBinaryOp::BitwiseOr | PoiseBinaryOp::BitwiseXor => {
            generated.push(AsmInstruction::Mov(s1, d.clone()));
            generated.push(AsmInstruction::Binary(gen_binary(op), s2, d));
        },
        PoiseBinaryOp::Divide | PoiseBinaryOp::Remainder => {
            generated.push(AsmInstruction::Mov(s1, Operand::Reg(Register::AX)));
            generated.push(AsmInstruction::Mov(s2, Operand::Reg(Register::R10)));
            generated.push(AsmInstruction::Cdq);
            generated.push(AsmInstruction::Idiv(Operand::Reg(Register::R10)));
            generated.push(AsmInstruction::Mov(Operand::Reg(gen_division(op)), d));
        },
        PoiseBinaryOp::LeftShift | PoiseBinaryOp::RightShift => {
            generated.push(AsmInstruction::Mov(s1, d.clone()));
            match &s2 {
                Operand::Imm(_) => {
                    generated.push(AsmInstruction::Binary(gen_binary(op), s2, d));
                },
                _ => {
                    generated.push(AsmInstruction::Mov(s2, Operand::Reg(Register::R10)));
                    generated.push(AsmInstruction::Movb(Operand::Reg(Register::R10), Operand::Reg(Register::CX)));
                    generated.push(AsmInstruction::Binary(gen_binary(op), Operand::Reg(Register::CL), d));
                },
            }
        }
        PoiseBinaryOp::Equal | PoiseBinaryOp::NotEqual | PoiseBinaryOp::GreaterThan |
        PoiseBinaryOp::GreaterOrEqual | PoiseBinaryOp::LessThan | PoiseBinaryOp::LessOrEqual => {
            println!("s1:{:?} s2:{:?} d:{:?}", s1.clone(), s2.clone(), d.clone());
            generated.push(AsmInstruction::Cmp(s2.clone(), s1));
            generated.push(AsmInstruction::Mov(Operand::Imm(0), d.clone()));
            generated.push(AsmInstruction::SetCC(gen_conditional(op), d));
        }
    }
}
