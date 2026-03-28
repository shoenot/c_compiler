use super::*;
use std::collections::hash_map::HashMap;

pub fn assign_stack_slots(funcs: Vec<AsmFunction>) -> Vec<AsmFunction> {
    let mut map: HashMap<String, i32> = HashMap::new();
    let mut functions = Vec::new();
    for func in funcs {
        functions.push(assign_func_slots(func, &mut map));
    }
    functions
}

fn assign_func_slots(func: AsmFunction, map: &mut HashMap<String, i32>) -> AsmFunction {
    let mut new_instructions = Vec::new();
    let mut offset: i32 = 0;
    for instruction in func.body {
        match instruction {
            AsmInstruction::Ret => new_instructions.push(AsmInstruction::Ret),
            AsmInstruction::Mov(src, dst)  => {
                let src = resolve_operand(src, map, &mut offset);
                let dst = resolve_operand(dst, map, &mut offset);
                match (&src, &dst) {
                    (Operand::Stack(_), Operand::Stack(_)) => {
                        new_instructions.push(AsmInstruction::Mov(src, Operand::Reg(Register::R10)));
                        new_instructions.push(AsmInstruction::Mov(Operand::Reg(Register::R10), dst));
                    },
                    _ => new_instructions.push(AsmInstruction::Mov(src, dst)),
                }
            },
            AsmInstruction::Movb(src, dst) => {
                let src = resolve_operand(src, map, &mut offset);
                let dst = resolve_operand(dst, map, &mut offset);
                new_instructions.push(AsmInstruction::Movb(src, Operand::Reg(Register::R10)));
                new_instructions.push(AsmInstruction::Movb(Operand::Reg(Register::R10), dst));
            },
            AsmInstruction::Unary(op, dst) => new_instructions.push(
                AsmInstruction::Unary(op, resolve_operand(dst, map, &mut offset))
            ),
            AsmInstruction::Binary(op, src, dst) => {
                let src = resolve_operand(src, map, &mut offset);
                let dst = resolve_operand(dst, map, &mut offset);
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
                 let src = resolve_operand(src, map, &mut offset);
                 match &src {
                     Operand::Imm(_) => {
                         new_instructions.push(AsmInstruction::Mov(src, Operand::Reg(Register::R10)));
                         new_instructions.push(AsmInstruction::Idiv(Operand::Reg(Register::R10)));
                     },
                     _ => new_instructions.push(AsmInstruction::Idiv(src)),
                 }
            },
            AsmInstruction::Cmp(v1, v2) => {
                let v1 = resolve_operand(v1, map, &mut offset);
                let v2 = resolve_operand(v2, map, &mut offset);
                match (&v1, &v2) {
                   (Operand::Stack(_), Operand::Stack(_)) | (_, Operand::Imm(_)) => {
                       new_instructions.push(AsmInstruction::Mov(v2, Operand::Reg(Register::R11)));
                       new_instructions.push(AsmInstruction::Cmp(v1, Operand::Reg(Register::R11)));
                   },
                   _ => new_instructions.push(AsmInstruction::Cmp(v1, v2)),
                }
            },
            AsmInstruction::SetCC(cond, dst) => {
                let dst = resolve_operand(dst, map, &mut offset);
                new_instructions.push(AsmInstruction::SetCC(cond, dst));
            },
            AsmInstruction::Push(val) => {
                let val = resolve_operand(val, map, &mut offset);
                new_instructions.push(AsmInstruction::Push(val));
            },
            other => new_instructions.push(other),
        }
    }
    let offset = (offset.abs() as u32).next_multiple_of(16) as i32;
    new_instructions.insert(0, AsmInstruction::AllocateStack(offset));
    AsmFunction { name: func.name, body: new_instructions, stack: offset }
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
