use super::*;
use std::collections::hash_map::HashMap;

pub fn assign_stack_slots(funcs: Vec<AsmFunction>, symbols: &AsmSymbolTable) -> Vec<AsmFunction> {
    let mut functions = Vec::new();
    for func in funcs {
        functions.push(assign_func_slots(func, symbols));
    }
    functions
}

fn get_size(asmtype: &AsmType) -> i32 {
    match asmtype {
        AsmType::Byte => 1,
        AsmType::Longword => 4,
        AsmType::Quadword => 8,
    }
}

fn assign_func_slots(func: AsmFunction, symbols: &AsmSymbolTable) -> AsmFunction {
    let mut map: HashMap<String, i32> = HashMap::new();
    let mut new_instructions = Vec::new();
    let mut offset: i32 = 0;
    for instruction in func.body {
        match instruction {
            AsmInstruction::Ret => new_instructions.push(AsmInstruction::Ret),
            AsmInstruction::Mov(asmtype, src, dst)  => {
                let src = resolve_operand(src, &mut map, &mut offset, symbols);
                let dst = resolve_operand(dst, &mut map, &mut offset, symbols);
                let regsize = get_regsize(&asmtype);
                match (&src, &dst) {
                   (Operand::Stack(_) | Operand::Data(_), Operand::Stack(_) | Operand::Data(_)) => {
                        new_instructions.push(AsmInstruction::Mov(asmtype.clone(), src, Operand::Reg(Register::R10, regsize.clone())));
                        new_instructions.push(AsmInstruction::Mov(asmtype, Operand::Reg(Register::R10, regsize), dst));
                   },
                   (Operand::Imm(v), Operand::Stack(_)) => {
                       if i32::try_from(*v).is_err() {
                           new_instructions.push(AsmInstruction::Mov(AsmType::Quadword, src, Operand::Reg(Register::R10, RegSize::Quad)));
                           new_instructions.push(AsmInstruction::Mov(AsmType::Quadword, Operand::Reg(Register::R10, RegSize::Quad), dst));
                       } else {
                           new_instructions.push(AsmInstruction::Mov(asmtype, src, dst));
                       }
                   },
                    _ => new_instructions.push(AsmInstruction::Mov(asmtype, src, dst)),
                }
            },
            AsmInstruction::Movsx(src, dst) => {
                let src = resolve_operand(src, &mut map, &mut offset, symbols);
                let dst = resolve_operand(dst, &mut map, &mut offset, symbols);
                match (&src, &dst) {
                   (Operand::Stack(_) | Operand::Data(_), Operand::Stack(_) | Operand::Data(_)) => {
                        new_instructions.push(AsmInstruction::Movsx(src, Operand::Reg(Register::R10, RegSize::Quad)));
                        new_instructions.push(AsmInstruction::Mov(AsmType::Quadword, Operand::Reg(Register::R10, RegSize::Quad), dst));
                   },
                   _ => new_instructions.push(AsmInstruction::Movsx(src, dst)),
                }
            },
            AsmInstruction::Unary(op, asmtype, dst) => new_instructions.push(
                AsmInstruction::Unary(op, asmtype, resolve_operand(dst, &mut map, &mut offset, symbols))
            ),
            AsmInstruction::Binary(op, asmtype, src, dst) => {
                let src = resolve_operand(src, &mut map, &mut offset, symbols);
                let dst = resolve_operand(dst, &mut map, &mut offset, symbols);
                let regsize = get_regsize(&asmtype);
                match op {
                    BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor |
                    BinaryOp::Add | BinaryOp::Sub => {
                       match (&src, &dst) {
                           (Operand::Stack(_) | Operand::Data(_), Operand::Stack(_) | Operand::Data(_)) => {
                               new_instructions.push(AsmInstruction::Mov(asmtype.clone(), src, Operand::Reg(Register::R10, regsize.clone())));
                               new_instructions.push(AsmInstruction::Binary(op, asmtype, Operand::Reg(Register::R10, regsize), dst));
                           },
                           (Operand::Imm(v), _) => {
                               if i32::try_from(*v).is_err() {
                                   new_instructions.push(AsmInstruction::Mov(AsmType::Quadword, src, Operand::Reg(Register::R10, RegSize::Quad)));
                                   new_instructions.push(AsmInstruction::Binary(op, AsmType::Quadword, Operand::Reg(Register::R10, RegSize::Quad), dst));
                               } else {
                                   new_instructions.push(AsmInstruction::Binary(op, asmtype, src, dst));
                               }
                           },
                           _ => new_instructions.push(AsmInstruction::Binary(op, asmtype, src, dst)),
                       }
                    },
                    BinaryOp::Mult => {
                        match (&src, &dst) {
                            (Operand::Imm(v), Operand::Stack(_) | Operand::Data(_)) => {
                                if i32::try_from(*v).is_err() {
                                    new_instructions.push(AsmInstruction::Mov(AsmType::Quadword, src, Operand::Reg(Register::R10, RegSize::Quad)));
                                    new_instructions.push(AsmInstruction::Mov(asmtype.clone(), dst.clone(), Operand::Reg(Register::R11, regsize.clone())));
                                    new_instructions.push(AsmInstruction::Binary(op, asmtype.clone(), Operand::Reg(Register::R10, RegSize::Quad), Operand::Reg(Register::R11, regsize.clone())));
                                    new_instructions.push(AsmInstruction::Mov(asmtype, Operand::Reg(Register::R11, regsize), dst));
                                } else {
                                    new_instructions.push(AsmInstruction::Mov(asmtype.clone(), dst.clone(), Operand::Reg(Register::R11, regsize.clone())));
                                    new_instructions.push(AsmInstruction::Binary(op, asmtype.clone(), src, Operand::Reg(Register::R11, regsize.clone())));
                                    new_instructions.push(AsmInstruction::Mov(asmtype, Operand::Reg(Register::R11, regsize), dst));
                                }
                            },
                            (_, Operand::Stack(_) | Operand::Data(_)) => {
                                new_instructions.push(AsmInstruction::Mov(asmtype.clone(), dst.clone(), Operand::Reg(Register::R11, regsize.clone())));
                                new_instructions.push(AsmInstruction::Binary(op, asmtype.clone(), src, Operand::Reg(Register::R11, regsize.clone())));
                                new_instructions.push(AsmInstruction::Mov(asmtype, Operand::Reg(Register::R11, regsize), dst));
                            },
                            (Operand::Imm(v), _) => {
                                if i32::try_from(*v).is_err() {
                                    new_instructions.push(AsmInstruction::Mov(AsmType::Quadword, src, Operand::Reg(Register::R10, RegSize::Quad)));
                                    new_instructions.push(AsmInstruction::Binary(op, AsmType::Quadword, Operand::Reg(Register::R10, RegSize::Quad), dst));
                                } else {
                                    new_instructions.push(AsmInstruction::Binary(op, asmtype, src, dst));
                                }
                            },
                            _ => new_instructions.push(AsmInstruction::Binary(op, asmtype, src, dst)),
                        }
                    },
                    BinaryOp::Sal | BinaryOp::Sar => {
                        new_instructions.push(AsmInstruction::Binary(op, asmtype, src, dst));
                    }
                }
            }
            AsmInstruction::Idiv(asmtype, src) => {
                 let src = resolve_operand(src, &mut map, &mut offset, symbols);
                 let regsize = get_regsize(&asmtype);
                 match &src {
                     Operand::Imm(_) => {
                         new_instructions.push(AsmInstruction::Mov(asmtype.clone(), src, Operand::Reg(Register::R10, regsize.clone())));
                         new_instructions.push(AsmInstruction::Idiv(asmtype, Operand::Reg(Register::R10, regsize)));
                     },
                     _ => new_instructions.push(AsmInstruction::Idiv(asmtype, src)),
                 }
            },
            AsmInstruction::Cmp(asmtype, v1, v2) => {
                let v1 = resolve_operand(v1, &mut map, &mut offset, symbols);
                let v2 = resolve_operand(v2, &mut map, &mut offset, symbols);
                let regsize = get_regsize(&asmtype);
                match (&v1, &v2) {
                   (Operand::Stack(_) | Operand::Data(_), Operand::Stack(_) | Operand::Data(_)) | (_, Operand::Imm(_)) => {
                       new_instructions.push(AsmInstruction::Mov(asmtype.clone(), v2, Operand::Reg(Register::R11, regsize.clone())));
                       new_instructions.push(AsmInstruction::Cmp(asmtype, v1, Operand::Reg(Register::R11, regsize)));
                   },
                   (Operand::Imm(v), _) => {
                       if i32::try_from(*v).is_err() {
                           new_instructions.push(AsmInstruction::Mov(AsmType::Quadword, v1, Operand::Reg(Register::R10, RegSize::Quad)));
                           new_instructions.push(AsmInstruction::Cmp(asmtype, Operand::Reg(Register::R10, regsize), v2));
                       } else {
                           new_instructions.push(AsmInstruction::Cmp(asmtype, v1, v2));
                       }
                   },
                   _ => new_instructions.push(AsmInstruction::Cmp(asmtype, v1, v2)),
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
    new_instructions.insert(0, AsmInstruction::Binary(BinaryOp::Sub, 
                                                      AsmType::Quadword, 
                                                      Operand::Imm(offset as i64), 
                                                      Operand::Reg(Register::SP, RegSize::Quad)));
    AsmFunction { identifier: func.identifier, global: func.global, body: new_instructions }
}

fn resolve_operand(op: Operand, map: &mut HashMap<String, i32>, offset: &mut i32, symbols: &AsmSymbolTable) -> Operand {
    match op {
        Operand::Pseudo(ident) => {
            let Some(AsmSymbol::ObjEntry(asmtype, is_static)) = symbols.get(&ident) else { unreachable!() };
            if *is_static {
                return Operand::Data(ident); 
            } else {
                let size = get_size(&asmtype);
                let stackoffset = map.entry(ident).or_insert_with(|| { 
                    *offset -= size;
                    *offset = *offset & !(get_alignment(&asmtype) - 1); 
                    *offset
                });
                Operand::Stack(*stackoffset)
            }
        },
        other => other,
    }
}
