use std::collections::HashMap;

use crate::poise;

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
    Unary(UnaryOp, Operand),
    AllocateStack(i32),
    Ret,
}

#[derive(Debug)]
pub enum UnaryOp {
    Neg,
    Not,
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
    R10
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

fn assign_stack_slots(func: AsmFunction) -> AsmFunction {
    let mut new_instructions = Vec::new();
    let mut map: HashMap<String, i32> = HashMap::new();
    let mut offset: i32 = 0;
    for instruction in func.body {
        match instruction {
            AsmInstruction::Ret => new_instructions.push(AsmInstruction::Ret),
            AsmInstruction::Mov(src, dst) => {
                let src = resolve_operand(src, &mut map, &mut offset); 
                let dst = resolve_operand(dst, &mut map, &mut offset);
                match (&src, &dst) {
                    (Operand::Stack(_), Operand::Stack(_)) => {
                        new_instructions.push(AsmInstruction::Mov(src, Operand::Reg(Register::R10)));
                        new_instructions.push(AsmInstruction::Mov(Operand::Reg(Register::R10), dst));
                    },
                    _ => new_instructions.push(AsmInstruction::Mov(src, dst)),
                }
            }
            AsmInstruction::Unary(op, dst) => new_instructions.push(
                AsmInstruction::Unary(op, resolve_operand(dst, &mut map, &mut offset))
            ),
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
