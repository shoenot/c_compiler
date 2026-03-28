mod stack;
use std::collections::VecDeque;

use stack::*;

use crate::poise::{self, PoiseBinaryOp, PoiseVal};
#[derive(Debug)]
pub struct AsmProgram {
    pub functions: Vec<AsmFunction>,
}

#[derive(Debug)]
pub struct AsmFunction {
    pub name: String,
    pub body: Vec<AsmInstruction>,
    pub stack: i32,
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
    DeallocateStack(i32),
    Push(Operand),
    Call(String),
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
    DX,
    CL,
    DI,
    SI,
    R8,
    R9,
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
    let mut functions = Vec::new();
    for func in tree.functions {
        let function = gen_function(func);
        functions.push(function);
    }
    functions = assign_stack_slots(functions);
    AsmProgram { functions }
}

fn copy_param(num: usize, param: String) -> AsmInstruction {
    match num {
        0 => AsmInstruction::Mov(Operand::Reg(Register::DI), Operand::Pseudo(param)), 
        1 => AsmInstruction::Mov(Operand::Reg(Register::SI), Operand::Pseudo(param)), 
        2 => AsmInstruction::Mov(Operand::Reg(Register::DX), Operand::Pseudo(param)), 
        3 => AsmInstruction::Mov(Operand::Reg(Register::CX), Operand::Pseudo(param)), 
        4 => AsmInstruction::Mov(Operand::Reg(Register::R8), Operand::Pseudo(param)), 
        5 => AsmInstruction::Mov(Operand::Reg(Register::R9), Operand::Pseudo(param)), 
        _ => AsmInstruction::Mov(Operand::Stack(((num.saturating_sub(6))*8 + 16) as i32), Operand::Pseudo(param)),
    }
}

fn gen_function(func: poise::PoiseFunc) -> AsmFunction {
    let mut generated = Vec::new();
    let name = func.identifier;
    func.params.iter()
        .enumerate()
        .for_each(|(num , param)| generated.push(copy_param(num, param.into())));
    gen_instructions(func.body, &mut generated);
    AsmFunction { name, body: generated, stack: 0 }
}

fn gen_instructions(instructions: Vec<poise::PoiseInstruction>, generated: &mut Vec<AsmInstruction>) {
    for instruction in instructions {
        match instruction {
            poise::PoiseInstruction::Return(val) => {
                generated.push(AsmInstruction::Mov(gen_operand(val), Operand::Reg(Register::AX)));
                generated.push(AsmInstruction::Ret);
            },
            poise::PoiseInstruction::Unary { op,src,dst } => {
                unary_handler(op, src, dst, generated);
            },
            poise::PoiseInstruction::Binary { op, src1, src2, dst } => {
                binary_handler(op, src1, src2, dst, generated);
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
            poise::PoiseInstruction::FunctionCall { ident, args, dst } => {
                let mut stack_padding: i32 = 0;

                if args.len() % 2 != 0 {
                    stack_padding = 8;
                    generated.push(AsmInstruction::AllocateStack(stack_padding));
                }

                let removal_bytes = 8 * (args.len().saturating_sub(6) as i32) + stack_padding;
                let mut args: VecDeque<(usize, &PoiseVal)> = VecDeque::from(args.iter().enumerate().collect::<Vec<_>>());

                let first_six = args.drain(..args.len().min(6));

                for (num, arg) in first_six {
                    copy_arg(num, arg.clone(), generated);
                }

                while let Some((num, arg)) = args.pop_back() {
                    copy_arg(num, arg.clone(), generated);
                }

                generated.push(AsmInstruction::Call(ident));
                
                if removal_bytes != 0 {
                    generated.push(AsmInstruction::DeallocateStack(removal_bytes));
                }

                generated.push(AsmInstruction::Mov(Operand::Reg(Register::AX), gen_operand(dst)));
            }
        }
    }
}

fn copy_arg(num: usize, arg: poise::PoiseVal, generated: &mut Vec<AsmInstruction>) {
    match num {
        0 => generated.push(AsmInstruction::Mov(gen_operand(arg), Operand::Reg(Register::DI))), 
        1 => generated.push(AsmInstruction::Mov(gen_operand(arg), Operand::Reg(Register::SI))), 
        2 => generated.push(AsmInstruction::Mov(gen_operand(arg), Operand::Reg(Register::DX))), 
        3 => generated.push(AsmInstruction::Mov(gen_operand(arg), Operand::Reg(Register::CX))), 
        4 => generated.push(AsmInstruction::Mov(gen_operand(arg), Operand::Reg(Register::R8))), 
        5 => generated.push(AsmInstruction::Mov(gen_operand(arg), Operand::Reg(Register::R9))), 
        _ => {
            let operand = gen_operand(arg);
            match operand {
                Operand::Pseudo(_) | Operand::Stack(_)=> {
                    generated.push(AsmInstruction::Mov(operand, Operand::Reg(Register::AX)));
                    generated.push(AsmInstruction::Push(Operand::Reg(Register::AX)));
                }
                Operand::Imm(_) | Operand::Reg(_) => generated.push(AsmInstruction::Push(operand)),
            }
        },
    }
}

fn gen_operand(exp: poise::PoiseVal) -> Operand {
    match exp {
        poise::PoiseVal::Constant(val) => Operand::Imm(val),
        poise::PoiseVal::Variable(ident) => Operand::Pseudo(ident),
    }
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
        _ => unreachable!(),
    };
    operator
}

fn gen_division(exp: PoiseBinaryOp) -> Register {
    let operator = match exp {
        poise::PoiseBinaryOp::Divide => Register::AX,
        poise::PoiseBinaryOp::Remainder => Register::DX,
        _ => unreachable!(),
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
        _ => unreachable!(),
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
            generated.push(AsmInstruction::Cmp(s2.clone(), s1));
            generated.push(AsmInstruction::Mov(Operand::Imm(0), d.clone()));
            generated.push(AsmInstruction::SetCC(gen_conditional(op), d));
        }
    }
}
