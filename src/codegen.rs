use std::collections::HashMap;

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
    Idiv(Operand),
    Cdq,
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
    DX,
    R10,
    R10b,
    R11,
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
                let dst_operand = gen_operand(dst);
                generated.push(AsmInstruction::Mov(gen_operand(src), dst_operand.clone()));
                generated.push(AsmInstruction::Unary(gen_unary(op), dst_operand));
            },
            poise::PoiseInstruction::Binary { op, src1, src2, dst } => {
                binary_handler(op, src1, src2, dst, &mut generated);
            },
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

fn gen_unary(exp: poise::PoiseUnaryOp) -> UnaryOp {
    let operator = match exp {
        poise::PoiseUnaryOp::Negate => UnaryOp::Neg,
        poise::PoiseUnaryOp::Complement => UnaryOp::Not,
    };
    operator
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
                    generated.push(AsmInstruction::Movb(Operand::Reg(Register::R10b), Operand::Reg(Register::CX)));
                    generated.push(AsmInstruction::Binary(gen_binary(op), Operand::Reg(Register::CX), d));
                },
            }
        }
    }
}

fn assign_stack_slots(func: AsmFunction) -> AsmFunction {
    let mut new_instructions = Vec::new();
    let mut map: HashMap<String, i32> = HashMap::new();
    let mut offset: i32 = 0;
    for instruction in func.body {
        match instruction {
            AsmInstruction::Ret => new_instructions.push(AsmInstruction::Ret),
            AsmInstruction::Mov(src, dst)  => {
                let src = resolve_operand(src, &mut map, &mut offset); 
                let dst = resolve_operand(dst, &mut map, &mut offset);
                match (&src, &dst) {
                    (Operand::Stack(_), Operand::Stack(_)) => {
                        new_instructions.push(AsmInstruction::Mov(src, Operand::Reg(Register::R10)));
                        new_instructions.push(AsmInstruction::Mov(Operand::Reg(Register::R10), dst));
                    },
                    _ => new_instructions.push(AsmInstruction::Mov(src, dst)),
                }
            },
            AsmInstruction::Movb(src, dst) => {
                let src = resolve_operand(src, &mut map, &mut offset); 
                let dst = resolve_operand(dst, &mut map, &mut offset);
                new_instructions.push(AsmInstruction::Movb(src, Operand::Reg(Register::R10b)));
                new_instructions.push(AsmInstruction::Movb(Operand::Reg(Register::R10b), dst));
            },
            AsmInstruction::Unary(op, dst) => new_instructions.push(
                AsmInstruction::Unary(op, resolve_operand(dst, &mut map, &mut offset))
            ),
            AsmInstruction::Binary(op, src, dst) => {
                let src = resolve_operand(src, &mut map, &mut offset);
                let dst = resolve_operand(dst, &mut map, &mut offset);
                match op {
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor => {
                       match (&src, &dst) {
                           (Operand::Stack(_), Operand::Stack(_)) => {
                               new_instructions.push(AsmInstruction::Mov(src, Operand::Reg(Register::R10)));
                               new_instructions.push(AsmInstruction::Binary(op, Operand::Reg(Register::R10), dst));
                           },
                           _ => new_instructions.push(AsmInstruction::Binary(op, src, dst)),
                       }
                    },
                    BinaryOp::Mult => {
                        match &dst {
                            Operand::Stack(_) => {
                                new_instructions.push(AsmInstruction::Mov(dst.clone(), Operand::Reg(Register::R11)));
                                new_instructions.push(AsmInstruction::Binary(op, src, Operand::Reg(Register::R11)));
                                new_instructions.push(AsmInstruction::Mov(Operand::Reg(Register::R11), dst));
                            },
                           _ => new_instructions.push(AsmInstruction::Binary(op, src, dst)),
                        }
                    },
                    BinaryOp::Sal | BinaryOp::Sar => {
                        new_instructions.push(AsmInstruction::Binary(op, src, dst));
                    }
                }
            }
            AsmInstruction::Idiv(src) => {
                 let src = resolve_operand(src, &mut map, &mut offset);
                 match &src {
                     Operand::Imm(_) => {
                         new_instructions.push(AsmInstruction::Mov(src, Operand::Reg(Register::R10)));
                         new_instructions.push(AsmInstruction::Idiv(Operand::Reg(Register::R10)));
                     },
                     _ => new_instructions.push(AsmInstruction::Idiv(src)),
                 }
            },
            other => new_instructions.push(other),
        }
    }
    new_instructions.insert(0, AsmInstruction::AllocateStack(offset.abs()));
    AsmFunction { name: func.name, body: new_instructions }
}

fn resolve_operand(op: Operand, map: &mut HashMap<String, i32>, offset: &mut i32) -> Operand {
    match op {
        Operand::Pseudo(ident) => {
            let stackoffset = map.entry(ident).or_insert_with(|| { *offset -= 4; *offset });
            Operand::Stack(*stackoffset)
        },
        other => other,
    }
}
