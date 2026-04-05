use super::*;
use std::collections::hash_map::HashMap;

pub fn assign_stack_slots(funcs: Vec<AsmFunction>, symbols: &AsmSymbolTable) -> Vec<AsmFunction> {
    let mut functions = Vec::new();
    for func in funcs {
        let mut fixer = OpFixer::new(func, &symbols);
        functions.push(fixer.assign_slots());
    }
    functions
}

fn get_size(asmtype: &AsmType) -> i32 {
    match asmtype {
        AsmType::Byte => 1,
        AsmType::Longword => 4,
        AsmType::Quadword => 8,
        AsmType::Double => 8,
    }
}

enum Scratch{
    Source,
    Dest,
}

fn get_reg(asmtype: &AsmType, scratch: Scratch) -> Operand {
    if matches!(asmtype, AsmType::Double) {
        if matches!(scratch, Scratch::Source) { Operand::Reg(Register::XMM14, RegSize::Quad) } 
        else { Operand::Reg(Register::XMM15, RegSize::Quad) }
    } else {
        if matches!(scratch, Scratch::Source) { Operand::Reg(Register::R10, get_regsize(asmtype)) } 
        else { Operand::Reg(Register::R11, get_regsize(asmtype)) }
    }
}

pub struct OpFixer<'a> {
    pub old: AsmFunction,
    pub symbols: &'a AsmSymbolTable,
    pub new: Vec<AsmInstruction>,
    pub offset: i32,
    pub offset_map: HashMap<String, i32>,
}

impl<'a> OpFixer<'a> {
    pub fn new(old: AsmFunction, symbols: &'a AsmSymbolTable) -> OpFixer<'a> {
        OpFixer { old, symbols, new: Vec::new(), offset: 0, offset_map: HashMap::new() }
    }

    fn push(&mut self, inst: AsmInstruction) {
        // println!("FIXED UP -> {:?}", inst);
        self.new.push(inst);
    }

    fn res_op(&mut self, op: Operand) -> Operand {
        match op {
            Operand::Pseudo(ident) => {
                let Some(AsmSymbol::ObjEntry(asmtype, is_static, false)) = self.symbols.get(&ident) else { unreachable!() };
                if *is_static {
                    return Operand::Data(ident); 
                } else {
                    let size = get_size(&asmtype);
                    let stackoffset = self.offset_map.entry(ident).or_insert_with(|| { 
                        self.offset -= size;
                        self.offset = self.offset & !(get_alignment(&asmtype) as i32 - 1);
                        self.offset
                    });
                    Operand::Stack(*stackoffset)
                }
            },
            other => other,
        }
    }
    
    fn assign_slots(&mut self) -> AsmFunction {
        for instruction in self.old.body.clone() {
            self.assign_instruction_slots(instruction);
        }
        let offset = (self.offset.abs() as u32).next_multiple_of(16) as i32;
        self.new.insert(0, AsmInstruction::Binary(BinaryOp::Sub, 
                                                          AsmType::Quadword, 
                                                          Operand::Imm(offset as i64), 
                                                          Operand::Reg(Register::SP, RegSize::Quad)));
        AsmFunction { identifier: self.old.identifier.clone(), global: self.old.global, body: self.new.clone() }
    }

    fn assign_instruction_slots(&mut self, instruction: AsmInstruction) {
        // println!("PRE FIXUP -> {:?}", instruction);
        match instruction {
            AsmInstruction::Ret => self.push(AsmInstruction::Ret),

            AsmInstruction::Mov(t, src, dst) => {
                let (s, d) = (self.res_op(src), self.res_op(dst));
                if !t.is_double() {
                    if s.is_memory() && d.is_memory() {
                        let temp = get_reg(&t, Scratch::Source);
                        self.push(AsmInstruction::Mov(t, s.clone(), temp.clone()));
                        self.push(AsmInstruction::Mov(t, temp.clone(), d.clone()));
                    } else if s.is_large_32bit_imm() && matches!(t, AsmType::Quadword)  {
                        let temp = get_reg(&AsmType::Longword, Scratch::Source);
                        self.push(AsmInstruction::Mov(AsmType::Longword, s.clone(), temp.clone()));
                        let temp = get_reg(&AsmType::Quadword, Scratch::Source);
                        self.push(AsmInstruction::Mov(AsmType::Quadword, temp.clone(), d.clone()));
                    } else if s.is_large_64bit_imm() {
                        let temp = get_reg(&AsmType::Quadword, Scratch::Source);
                        self.push(AsmInstruction::Mov(AsmType::Quadword, s.clone(), temp.clone()));
                        self.push(AsmInstruction::Mov(AsmType::Quadword, temp.clone(), d.clone()));
                    } else {
                        self.push(AsmInstruction::Mov(t, s.clone(), d.clone()));
                    }
                } else {
                    if s.is_memory() && d.is_memory() {
                        let temp = get_reg(&t, Scratch::Dest);
                        self.push(AsmInstruction::Mov(t, s.clone(), temp.clone()));
                        self.push(AsmInstruction::Mov(t, temp.clone(), d.clone()));
                    } else {
                        self.push(AsmInstruction::Mov(t, s.clone(), d.clone()));
                    }
                }
            },

            AsmInstruction::Movsx(src, dst) => {
                let (mut s, d) = (self.res_op(src), self.res_op(dst));
                if s.is_imm() {
                    let temp = get_reg(&AsmType::Longword, Scratch::Source);
                    self.push(AsmInstruction::Mov(AsmType::Longword, s, temp.clone()));
                    s = temp;
                }
                if !d.is_reg() {
                    let temp = get_reg(&AsmType::Quadword, Scratch::Dest);
                    self.push(AsmInstruction::Movsx(s, temp.clone()));
                    self.push(AsmInstruction::Mov(AsmType::Quadword, temp, d));
                } else {
                    self.push(AsmInstruction::Movsx(s, d));
                }
            },

            AsmInstruction::Unary(op, t, dst) => {
                let d = self.res_op(dst);
                self.push(AsmInstruction::Unary(op, t, d));
            },

            AsmInstruction::Binary(op, t, src, dst) => {
                self.bin_handler(&op, &t, src, dst);
            }

            AsmInstruction::Idiv(t, src) => {
                let s = self.res_op(src);
                if s.is_imm() {
                    let temp = get_reg(&t, Scratch::Source);
                    self.push(AsmInstruction::Mov(t, s, temp.clone()));
                    self.push(AsmInstruction::Idiv(t, temp));
                 } else {
                     self.push(AsmInstruction::Idiv(t, s));
                 }
            },

            AsmInstruction::Div(t, src) => {
                let s = self.res_op(src);
                if s.is_imm() {
                    let temp = get_reg(&t, Scratch::Source);
                    self.push(AsmInstruction::Mov(t, s, temp.clone()));
                    self.push(AsmInstruction::Div(t, temp));
                 } else {
                     self.push(AsmInstruction::Div(t, s));
                 }
            },

            AsmInstruction::Cmp(mut t, v1, v2) => {
                let (mut v1, mut v2) = (self.res_op(v1.clone()), self.res_op(v2.clone()));
                if v1.is_large_64bit_imm() {
                    let temp = get_reg(&AsmType::Quadword, Scratch::Source);
                    self.push(AsmInstruction::Mov(AsmType::Quadword, v1, temp.clone()));
                    v1 = temp;
                    t = AsmType::Quadword;
                } else if v1.is_large_32bit_imm() && matches!(t, AsmType::Quadword)  {
                    let temp = get_reg(&AsmType::Longword, Scratch::Source);
                    self.push(AsmInstruction::Mov(AsmType::Longword, v1, temp.clone()));
                    v1 = get_reg(&AsmType::Quadword, Scratch::Source);
                    t = AsmType::Quadword;
                }
                if v2.is_imm() {
                    let temp = get_reg(&t, Scratch::Dest);
                    self.push(AsmInstruction::Mov(t, v2, temp.clone()));
                    v2 = temp;
                }
                if t.is_double() && !v2.is_reg() {
                    let temp = get_reg(&t, Scratch::Dest);
                    self.push(AsmInstruction::Mov(t, v2, temp.clone()));
                    v2 = temp;
                }
                if v1.is_memory() && v2.is_memory() {
                    let temp = get_reg(&t, Scratch::Source);
                    self.push(AsmInstruction::Mov(t, v1, temp.clone()));
                    v1 = temp;
                }
                self.push(AsmInstruction::Cmp(t, v1, v2));
            },

            AsmInstruction::SetCC(cond, dst) => {
                let d = self.res_op(dst);
                self.push(AsmInstruction::SetCC(cond, d));
            },

            AsmInstruction::Push(val) => {
                let v = self.res_op(val);
                self.push(AsmInstruction::Push(v));
            },

            AsmInstruction::MovZeroExtend(src, dst) => {
                let (s, d) = (self.res_op(src), self.res_op(dst));
                if d.is_reg() { self.push(AsmInstruction::Mov(AsmType::Longword, s, d)); }
                else if d.is_memory() {
                    let mut temp = get_reg(&AsmType::Longword, Scratch::Dest);
                    self.push(AsmInstruction::Mov(AsmType::Longword, s, temp.clone()));
                    temp = get_reg(&AsmType::Quadword, Scratch::Dest);
                    self.push(AsmInstruction::Mov(AsmType::Quadword, temp, d));
                }
            },

            AsmInstruction::Cvttsd2si(t, src, dst) => {
                let (s, d) = (self.res_op(src), self.res_op(dst));
                if !d.is_reg() {
                    let temp = get_reg(&t, Scratch::Dest);
                    self.push(AsmInstruction::Cvttsd2si(t, s, temp.clone()));
                    self.push(AsmInstruction::Mov(t, temp, d));
                } else {
                    self.push(AsmInstruction::Cvttsd2si(t, s, d));
                }
            },

            AsmInstruction::Cvtsi2sd(t, src, dst) => {
                let (mut s, d) = (self.res_op(src), self.res_op(dst));
                if s.is_imm() { 
                    let temp = get_reg(&t, Scratch::Source); 
                    self.push(AsmInstruction::Mov(t, s, temp.clone())); 
                    s = temp; 
                }
                if !d.is_reg() { 
                    let temp = get_reg(&AsmType::Double, Scratch::Dest); 
                    self.push(AsmInstruction::Cvtsi2sd(t, s, temp.clone())); 
                    self.push(AsmInstruction::Mov(AsmType::Double, temp, d)); 
                } else {
                    self.push(AsmInstruction::Cvtsi2sd(t, s, d)); 
                }
            },

            other => self.push(other),
        }
    }

    fn bin_handler(&mut self, op: &BinaryOp, mut t: &AsmType, src: Operand, dst: Operand) {
        let (mut s, d) = (self.res_op(src), self.res_op(dst));
        match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor => {
                if !t.is_double() {
                    if s.is_large_64bit_imm() {
                        let temp = get_reg(&AsmType::Quadword, Scratch::Source);
                        self.push(AsmInstruction::Mov(AsmType::Quadword, s, temp.clone()));
                        s = temp;
                        t = &AsmType::Quadword;
                    } else if s.is_large_32bit_imm() && matches!(t, AsmType::Quadword)  {
                        let temp = get_reg(&AsmType::Longword, Scratch::Source);
                        self.push(AsmInstruction::Mov(AsmType::Longword, s, temp.clone()));
                        s = get_reg(&AsmType::Quadword, Scratch::Source);
                        t = &AsmType::Quadword;
                    }
                    if s.is_memory() && d.is_memory() {
                        let temp = get_reg(&t, Scratch::Source);
                        self.push(AsmInstruction::Mov(*t, s, temp.clone()));
                        s = temp;
                    }
                    self.push(AsmInstruction::Binary(*op, *t, s, d));
                } else {
                    if !d.is_reg() {
                        let temp = get_reg(&t, Scratch::Dest);
                        self.push(AsmInstruction::Mov(*t, d.clone(), temp.clone()));
                        self.push(AsmInstruction::Binary(*op, *t, s, temp.clone()));
                        self.push(AsmInstruction::Mov(*t, temp, d));
                    } else {
                        self.push(AsmInstruction::Binary(*op, *t, s, d));
                    }
                }
            }, 

            BinaryOp::DivDouble => {
                if !d.is_reg() {
                    let temp = get_reg(&t, Scratch::Dest);
                    self.push(AsmInstruction::Mov(*t, d.clone(), temp.clone()));
                    self.push(AsmInstruction::Binary(*op, *t, s, temp.clone()));
                    self.push(AsmInstruction::Mov(*t, temp, d));
                } else {
                    self.push(AsmInstruction::Binary(*op, *t, s, d));
                }
            },
            
            BinaryOp::Mult => {
                if !t.is_double() {
                    if s.is_large_64bit_imm() {
                        let temp = get_reg(&AsmType::Quadword, Scratch::Source);
                        self.push(AsmInstruction::Mov(AsmType::Quadword, s, temp.clone()));
                        s = temp;
                        t = &AsmType::Quadword;
                    } else if s.is_large_32bit_imm() && matches!(t, AsmType::Quadword)  {
                        let temp = get_reg(&AsmType::Longword, Scratch::Source);
                        self.push(AsmInstruction::Mov(AsmType::Longword, s, temp.clone()));
                        s = get_reg(&AsmType::Quadword, Scratch::Source);
                        t = &AsmType::Quadword;
                    }
                } 
                if !d.is_reg() {
                    let temp = get_reg(&t, Scratch::Dest);
                    self.push(AsmInstruction::Mov(*t, d.clone(), temp.clone()));
                    self.push(AsmInstruction::Binary(*op, *t, s, temp.clone()));
                    self.push(AsmInstruction::Mov(*t, temp, d));
                } else {
                    self.push(AsmInstruction::Binary(*op, *t, s, d));
                }
            },

            BinaryOp::Sal | BinaryOp::Sar | BinaryOp::Shl | BinaryOp::Shr  => {
                self.push(AsmInstruction::Binary(*op, *t, s, d));
            },
        }
    }
}
