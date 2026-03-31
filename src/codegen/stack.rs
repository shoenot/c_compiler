use crate::types::IdentAttrs;

use super::*;
use std::collections::hash_map::HashMap;

pub fn assign_stack_slots(funcs: Vec<AsmFunction>, symbols: &SymbolTable) -> Vec<AsmFunction> {
    let mut functions = Vec::new();
    for func in funcs {
        functions.push(assign_func_slots(func, symbols));
    }
    functions
}

fn assign_func_slots(func: AsmFunction, symbols: &SymbolTable) -> AsmFunction {
    let mut map: HashMap<String, i32> = HashMap::new();
    let mut new_instructions = Vec::new();
    let mut offset: i32 = 0;
    for instruction in func.body {
        match instruction {
            AsmInstruction::Ret => new_instructions.push(AsmInstruction::Ret),
            AsmInstruction::Mov(src, dst)  => {
                let src = resolve_operand(src, &mut map, &mut offset, symbols);
                let dst = resolve_operand(dst, &mut map, &mut offset, symbols);
                match (&src, &dst) {
                    (Operand::Stack(_) | Operand::Data(_), Operand::Stack(_) | Operand::Data(_)) => {
                        new_instructions.push(AsmInstruction::Mov(src, Operand::Reg(Register::R10, RegSize::Long)));
                        new_instructions.push(AsmInstruction::Mov(Operand::Reg(Register::R10, RegSize::Long), dst));
                    },
                    _ => new_instructions.push(AsmInstruction::Mov(src, dst)),
                }
            },
            AsmInstruction::Movb(src, dst) => {
                let src = resolve_operand(src, &mut map, &mut offset, symbols);
                let dst = resolve_operand(dst, &mut map, &mut offset, symbols);
                new_instructions.push(AsmInstruction::Movb(src, Operand::Reg(Register::R10, RegSize::Byte)));
                new_instructions.push(AsmInstruction::Movb(Operand::Reg(Register::R10, RegSize::Byte), dst));
            },
            AsmInstruction::Unary(op, dst) => new_instructions.push(
                AsmInstruction::Unary(op, resolve_operand(dst, &mut map, &mut offset, symbols))
            ),
            AsmInstruction::Binary(op, src, dst) => {
                let src = resolve_operand(src, &mut map, &mut offset, symbols);
                let dst = resolve_operand(dst, &mut map, &mut offset, symbols);
                match op {
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor => {
                       match (&src, &dst) {
                           (Operand::Stack(_) | Operand::Data(_), Operand::Stack(_) | Operand::Data(_)) => {
                               new_instructions.push(AsmInstruction::Mov(src, Operand::Reg(Register::R10, RegSize::Long)));
                               new_instructions.push(AsmInstruction::Binary(op, Operand::Reg(Register::R10, RegSize::Long), dst));
                           },
                           _ => new_instructions.push(AsmInstruction::Binary(op, src, dst)),
                       }
                    },
                    BinaryOp::Mult => {
                        match &dst {
                            Operand::Stack(_) | Operand::Data(_) => {
                                new_instructions.push(AsmInstruction::Mov(dst.clone(), Operand::Reg(Register::R11, RegSize::Long)));
                                new_instructions.push(AsmInstruction::Binary(op, src, Operand::Reg(Register::R11, RegSize::Long)));
                                new_instructions.push(AsmInstruction::Mov(Operand::Reg(Register::R11, RegSize::Long), dst));
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
                 let src = resolve_operand(src, &mut map, &mut offset, symbols);
                 match &src {
                     Operand::Imm(_) => {
                         new_instructions.push(AsmInstruction::Mov(src, Operand::Reg(Register::R10, RegSize::Long)));
                         new_instructions.push(AsmInstruction::Idiv(Operand::Reg(Register::R10, RegSize::Long)));
                     },
                     _ => new_instructions.push(AsmInstruction::Idiv(src)),
                 }
            },
            AsmInstruction::Cmp(v1, v2) => {
                let v1 = resolve_operand(v1, &mut map, &mut offset, symbols);
                let v2 = resolve_operand(v2, &mut map, &mut offset, symbols);
                match (&v1, &v2) {
                   (Operand::Stack(_) | Operand::Data(_), Operand::Stack(_) | Operand::Data(_)) | (_, Operand::Imm(_)) => {
                       new_instructions.push(AsmInstruction::Mov(v2, Operand::Reg(Register::R11, RegSize::Long)));
                       new_instructions.push(AsmInstruction::Cmp(v1, Operand::Reg(Register::R11, RegSize::Long)));
                   },
                   _ => new_instructions.push(AsmInstruction::Cmp(v1, v2)),
                }
            },
            AsmInstruction::SetCC(cond, dst) => {
                let dst = resolve_operand(dst, &mut map, &mut offset, symbols);
                new_instructions.push(AsmInstruction::SetCC(cond, dst));
            },
            AsmInstruction::Push(val) => {
                let val = resolve_operand(val, &mut map, &mut offset, symbols);
                new_instructions.push(AsmInstruction::Push(val));
            },
            other => new_instructions.push(other),
        }
    }
    let offset = (offset.abs() as u32).next_multiple_of(16) as i32;
    new_instructions.insert(0, AsmInstruction::AllocateStack(offset));
    AsmFunction { identifier: func.identifier, global: func.global, body: new_instructions }
}

fn resolve_operand(op: Operand, map: &mut HashMap<String, i32>, offset: &mut i32, symbols: &SymbolTable) -> Operand {
    match op {
        Operand::Pseudo(ident) => {
            if let Some(sym) = symbols.get(&ident) {
                if let IdentAttrs::StaticAttr { .. } = sym.attrs {
                    return Operand::Data(ident); 
                }
            }
            let stackoffset = map.entry(ident).or_insert_with(|| { *offset -= 4; *offset });
            Operand::Stack(*stackoffset)
        },
        other => other,
    }
}
