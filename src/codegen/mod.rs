mod stack;
use super::*;

use ordered_float::OrderedFloat;

use stack::*;

use crate::parser::Const;

use crate::poise::{self, PoiseBinaryOp, PoiseVal, TopLevelItem};
use crate::types::*;

pub mod asm_ast;
pub use asm_ast::*;
pub mod asm_symtab;
pub use asm_symtab::*;

pub fn gen_static_var(var: poise::PoiseStaticVar) -> AsmTopLevel {
    AsmTopLevel::V(AsmStaticVar { identifier: var.identifier, 
                                  global: var.global, 
                                  alignment: get_alignment(&convert_type(&var.datatype)),
                                  init: var.init })
}

fn gen_binary(exp: poise::PoiseBinaryOp, signed: bool) -> BinaryOp {
    match (exp, signed) {
        (poise::PoiseBinaryOp::Add, _) => BinaryOp::Add,
        (poise::PoiseBinaryOp::Subtract, _) => BinaryOp::Sub,
        (poise::PoiseBinaryOp::Multiply, _) => BinaryOp::Mult,
        (poise::PoiseBinaryOp::LeftShift, true) =>  BinaryOp::Sal,
        (poise::PoiseBinaryOp::RightShift, true) => BinaryOp::Sar,
        (poise::PoiseBinaryOp::LeftShift, false) =>  BinaryOp::Shl,
        (poise::PoiseBinaryOp::RightShift, false) => BinaryOp::Shr,
        (poise::PoiseBinaryOp::BitwiseAnd, _) => BinaryOp::BitAnd,
        (poise::PoiseBinaryOp::BitwiseOr , _) => BinaryOp::BitOr,
        (poise::PoiseBinaryOp::BitwiseXor, _) => BinaryOp::BitXor,
        _ => unreachable!(),
    }
}

fn gen_division(exp: PoiseBinaryOp) -> Register {
    match exp {
        poise::PoiseBinaryOp::Divide => Register::AX,
        poise::PoiseBinaryOp::Remainder => Register::DX,
        _ => unreachable!(),
    }
}

fn gen_conditional(op: PoiseBinaryOp, signed: bool) -> Condition {
    match (op, signed) {
        (PoiseBinaryOp::Equal, _) => Condition::E,
        (PoiseBinaryOp::NotEqual, _) => Condition::NE,
        (PoiseBinaryOp::GreaterThan, true) => Condition::G,
        (PoiseBinaryOp::GreaterOrEqual, true) => Condition::GE,
        (PoiseBinaryOp::LessThan, true) => Condition::L,
        (PoiseBinaryOp::LessOrEqual, true) => Condition::LE,
        (PoiseBinaryOp::GreaterThan, false) => Condition::A,
        (PoiseBinaryOp::GreaterOrEqual, false) => Condition::AE,
        (PoiseBinaryOp::LessThan, false) => Condition::B,
        (PoiseBinaryOp::LessOrEqual, false) => Condition::BE,
        _ => unreachable!(),
    }
}

pub fn gen_program(tree: poise::PoiseProg, symbols: SymbolTable, mut asm_symbols: AsmSymbolTable) -> (AsmProgram, AsmSymbolTable) {
    let mut top_level = Vec::new();
    let mut functions = Vec::new();
    let mut statics = Statics::new();
    for item in tree.top_level_items {
        match item {
            TopLevelItem::F(f) => { 
                let funcgen = FuncGen::new(&symbols, &mut statics, f.identifier.clone());
                let func = funcgen.gen_function(f);
                functions.push(func);
            },
            TopLevelItem::V(v) => top_level.push(gen_static_var(v)),
        }
    }
    let statics_vec = statics.statics_map.into_values().map(|v| AsmTopLevel::C(v)).collect();
    convert_symtable(&symbols, &mut asm_symbols, &statics_vec);
    functions = assign_stack_slots(functions, &mut asm_symbols);
    top_level.extend(statics_vec);
    top_level.extend(functions.iter().map(|func| AsmTopLevel::F(func.clone())));
    (AsmProgram { top_level }, asm_symbols)
}

struct FuncGen<'a> {
    ident: String,
    generated: Vec<AsmInstruction>,
    statics: &'a mut Statics,
    symbols: &'a SymbolTable,
    count: usize,
}

impl<'a> FuncGen<'a> {
    fn new(table: &'a SymbolTable, statics: &'a mut Statics, ident: String) -> FuncGen<'a> {
        FuncGen {ident, generated: Vec::new(), statics, symbols: table, count: 0}
    }

    fn push(&mut self, inst: AsmInstruction) {
        self.generated.push(inst);
    }

    fn intern_static(&mut self, init: OrderedFloat<f64>, alignment: usize) -> String {
        self.statics.intern_static(init, alignment)
    }

    fn get_count(&mut self) -> String {
        let ret = format!("asm_{}.{}", self.ident, self.count);
        self.count += 1;
        ret
    }

    // at this point all the symbols should be in the table. so if the unwrap fails that means
    // something is fucked. the unwrap shouldn't fail. 
    pub fn get_symbol(&self, ident: &String) -> Symbol {
        self.symbols.get(ident).unwrap().clone()
    }

    pub fn get_var_type(&self, ident: &String) -> AsmType {
        convert_type(&self.get_symbol(ident).datatype)
    }

    pub fn is_double(&self, val: &poise::PoiseVal) -> bool {
        match val {
            poise::PoiseVal::Constant(Const::Double(_)) => true,
            poise::PoiseVal::Variable(ident) => self.get_symbol(ident).datatype == Type::Double,
            _ => false,
        }
    }

    pub fn get_num_type(&self, val: &poise::PoiseVal) -> Type {
        match val {
            poise::PoiseVal::Variable(ident) => self.get_symbol(ident).datatype,
            poise::PoiseVal::Constant(i) => {
                match i {
                        Const::Int(_) => Type::Int,
                        Const::Long(_) => Type::Long,
                        Const::UInt(_) => Type::UInt,
                        Const::ULong(_) => Type::ULong,
                        Const::Double(_) => Type::Double,
                }
            },
        }
    }

    pub fn get_asmtype(&self, val: &poise::PoiseVal) -> AsmType {
        match val {
            poise::PoiseVal::Variable(ident) => self.get_var_type(ident),
            poise::PoiseVal::Constant(Const::Int(_)) => AsmType::Longword,
            poise::PoiseVal::Constant(Const::Long(_)) => AsmType::Quadword,
            poise::PoiseVal::Constant(Const::UInt(_)) => AsmType::Longword,
            poise::PoiseVal::Constant(Const::ULong(_)) => AsmType::Quadword,
            poise::PoiseVal::Constant(Const::Double(_)) => AsmType::Double,
        }
    }

    fn gen_operand(&mut self, exp: poise::PoiseVal) -> Operand {
        match exp {
            poise::PoiseVal::Constant(cst) => match cst {
                Const::Int(val) => Operand::Imm(val as i64),
                Const::Long(val) => Operand::Imm(val),
                Const::UInt(val) => Operand::Imm(val as i64),
                Const::ULong(val) => Operand::Imm(val as i64),
                Const::Double(val) => Operand::Data(self.intern_static(val, 8)),
            }
            poise::PoiseVal::Variable(ident) => Operand::Pseudo(ident),
        }
    }

    fn gen_function(mut self, func: poise::PoiseFunc) -> AsmFunction {
        let identifier = func.identifier;
        let mut param_count = FuncNum::new();
        let params: Vec<AsmInstruction> = func.params.iter()
                                .map(|param| self.copy_param(&mut param_count, param.into()))
                                .collect();
        self.generated.extend(params);
        self.gen_instructions(func.body);
        AsmFunction { identifier, global: func.global, body: self.generated }
    }

    fn copy_reg(&self, num: usize, param: String, param_type: AsmType, is_float: bool,) -> AsmInstruction {
            let param_reg = get_func_reg(num, is_float);
            let param_regsize = get_regsize(&param_type);
            AsmInstruction::Mov(param_type, Operand::Reg(param_reg, param_regsize), Operand::Pseudo(param))
    }

    fn copy_stack(&self, num: usize, param: String, param_type: AsmType) -> AsmInstruction {
            let offset = (num * 8) + 16;
            AsmInstruction::Mov(param_type, Operand::Stack(offset as i32), Operand::Pseudo(param))
    }

    fn copy_param(&mut self, count: &mut FuncNum, param: String) -> AsmInstruction {
        let param_type = self.get_asmtype(&PoiseVal::Variable(param.clone()));
        if param_type != AsmType::Double {
            let (is_reg, num) = count.get_int();
            if is_reg { self.copy_reg(num, param, param_type, false) }
            else { self.copy_stack(num, param, param_type) }
        } else {
            let (is_reg, num) = count.get_float();
            if is_reg { self.copy_reg(num, param, param_type, true) }
            else { self.copy_stack(num, param, param_type) }
        }
    }

    fn gen_instructions(&mut self, instructions: Vec<poise::PoiseInstruction>) {
        for instruction in instructions {
            match instruction {
                poise::PoiseInstruction::Return(val) => {
                    let asmtype = self.get_asmtype(&val);
                    let regsize = get_regsize(&asmtype);
                    if asmtype != AsmType::Double {
                        let v = self.gen_operand(val);
                        self.push(AsmInstruction::Mov(asmtype, v, Operand::Reg(Register::AX, regsize)));
                    } else {
                        let v = self.gen_operand(val);
                        self.push(AsmInstruction::Mov(asmtype, v, Operand::Reg(Register::XMM0, regsize)));
                    }
                    self.push(AsmInstruction::Ret);
                },
                poise::PoiseInstruction::Unary { op,src,dst } => {
                    self.unary_handler(op, src, dst);
                },
                poise::PoiseInstruction::Binary { op, src1, src2, dst } => {
                    self.binary_handler(op, src1, src2, dst);
                },
                poise::PoiseInstruction::Jump(id) => self.push(AsmInstruction::Jmp(id)),
                poise::PoiseInstruction::JumpIfZero{condition: cnd, identifier: id} => {
                    let cndt = self.get_asmtype(&cnd);
                    let cndv = self.gen_operand(cnd.clone());
                    if !self.is_double(&cnd) {
                        self.push(AsmInstruction::Cmp(cndt, Operand::Imm(0), cndv));
                        self.push(AsmInstruction::JmpCC(Condition::E, id))
                    } else {
                        let scratch = Operand::Reg(Register::XMM0, RegSize::Quad);
                        let parity_jmptgt = self.get_count();
                        self.push(AsmInstruction::Binary(BinaryOp::BitXor, cndt, scratch.clone(), scratch.clone()));
                        self.push(AsmInstruction::Cmp(AsmType::Double, scratch, cndv));
                        self.push(AsmInstruction::JmpCC(Condition::P, parity_jmptgt.clone()));
                        self.push(AsmInstruction::JmpCC(Condition::E, id));
                        self.push(AsmInstruction::Label(parity_jmptgt));
                    }
                }
                poise::PoiseInstruction::JumpIfNotZero{condition: cnd, identifier: id} => {
                    let cndt = self.get_asmtype(&cnd);
                    let cndv = self.gen_operand(cnd.clone());
                    if !self.is_double(&cnd) {
                        self.push(AsmInstruction::Cmp(cndt, Operand::Imm(0), cndv));
                        self.push(AsmInstruction::JmpCC(Condition::NE, id));
                    } else {
                        let scratch = Operand::Reg(Register::XMM0, RegSize::Quad);
                        self.push(AsmInstruction::Binary(BinaryOp::BitXor, cndt, scratch.clone(), scratch.clone()));
                        self.push(AsmInstruction::Cmp(AsmType::Double, scratch, cndv));
                        self.push(AsmInstruction::JmpCC(Condition::P, id.clone()));
                        self.push(AsmInstruction::JmpCC(Condition::NE, id));
                    }
                },
                poise::PoiseInstruction::Copy{src, dst} => {
                    let (t, s, d) = (self.get_asmtype(&src), self.gen_operand(src), self.gen_operand(dst));
                    self.push(AsmInstruction::Mov(t, s, d))
                }
                poise::PoiseInstruction::Label(id) => self.push(AsmInstruction::Label(id)),
                poise::PoiseInstruction::FunctionCall { ident, args, dst } => {
                    let mut stack_padding: i32 = 0;


                    let mut gp6 = Vec::new();
                    let mut fp8 = Vec::new();
                    let mut stack = Vec::new();

                    for arg in args {
                        if !self.is_double(&arg) && gp6.len() < 6 { gp6.push(arg) }
                        else if self.is_double(&arg) && fp8.len() < 8 { fp8.push(arg) }
                        else { stack.push(arg) }
                    }

                    if stack.len() % 2 != 0 {
                        stack_padding = 8;
                        self.push(AsmInstruction::Binary(BinaryOp::Sub, 
                                                              AsmType::Quadword, 
                                                              Operand::Imm(stack_padding as i64), 
                                                              Operand::Reg(Register::SP, RegSize::Quad)));
                    }
                    let removal_bytes = 8 * (stack.len() as i32) + stack_padding;
                    
                    let mut arg_count = FuncNum::new();
                    gp6.iter().for_each(|arg| self.copy_arg(&mut arg_count, arg));
                    fp8.iter().for_each(|arg| self.copy_arg(&mut arg_count, arg));
                    stack.iter().rev().for_each(|arg| self.copy_arg(&mut arg_count, arg));

                    self.push(AsmInstruction::Call(ident));
                    
                    if removal_bytes != 0 {
                        self.push(AsmInstruction::Binary(BinaryOp::Add, 
                                                              AsmType::Quadword, 
                                                              Operand::Imm(removal_bytes as i64), 
                                                              Operand::Reg(Register::SP, RegSize::Quad)));
                    }
                    
                    let dst_type = self.get_asmtype(&dst);
                    let dst_regsize = get_regsize(&dst_type);
                    let d = self.gen_operand(dst);
                    if dst_type != AsmType::Double {
                    self.push(AsmInstruction::Mov(dst_type, 
                                                       Operand::Reg(Register::AX, dst_regsize), 
                                                       d));
                    } else {
                    self.push(AsmInstruction::Mov(dst_type, 
                                                       Operand::Reg(Register::XMM0, dst_regsize), 
                                                       d));
                    }
                },
                poise::PoiseInstruction::SignExtend { src, dst } => {
                    let (s, d) = (self.gen_operand(src), self.gen_operand(dst));
                    self.push(AsmInstruction::Movsx(s, d));
                },
                poise::PoiseInstruction::Truncate { src, dst } => {
                    let (s, d) = (self.gen_operand(src), self.gen_operand(dst));
                    self.push(AsmInstruction::Mov(AsmType::Longword, s, d));
                },
                poise::PoiseInstruction::ZeroExtend { src, dst } => {
                    let (s, d) = (self.gen_operand(src), self.gen_operand(dst));
                    self.push(AsmInstruction::MovZeroExtend(s, d));
                }
                poise::PoiseInstruction::IntToDouble { src, dst } => {
                    let (t, s, d) = (self.get_asmtype(&src), self.gen_operand(src), self.gen_operand(dst));
                    self.push(AsmInstruction::Cvtsi2sd(t, s, d));
                }
                poise::PoiseInstruction::DoubleToInt { src, dst } => {
                    let (t, s, d) = (self.get_asmtype(&dst), self.gen_operand(src), self.gen_operand(dst));
                    self.push(AsmInstruction::Cvttsd2si(t, s, d));
                }
                poise::PoiseInstruction::UIntToDouble { src, dst } => self.uint_to_double(src, dst),
                poise::PoiseInstruction::DoubleToUInt { src, dst } => self.double_to_uint(src, dst),
            }
        }
    }

    fn copy_arg_reg(&mut self, num: usize, arg: Operand, arg_type: AsmType, is_float: bool) {
            let arg_reg = get_func_reg(num, is_float);
            let arg_regsize = get_regsize(&arg_type);
            self.push(AsmInstruction::Mov(arg_type, arg, Operand::Reg(arg_reg, arg_regsize)))
    }

    fn copy_arg_stack(&mut self, arg: Operand, arg_type: AsmType) {
        match arg {
            Operand::Pseudo(_) | Operand::Stack(_) | Operand::Data(_) => {
                match arg_type {
                    AsmType::Longword => {
                        self.push(AsmInstruction::Mov(arg_type, arg, Operand::Reg(Register::AX, RegSize::Long)));
                        self.push(AsmInstruction::Push(Operand::Reg(Register::AX, RegSize::Quad)));
                    },
                    AsmType::Quadword => self.push(AsmInstruction::Push(arg)),
                    AsmType::Double => self.push(AsmInstruction::Push(arg)),
                    AsmType::Byte => unreachable!()
                }
            }
            Operand::Imm(_) | Operand::Reg(_, _) => self.push(AsmInstruction::Push(arg)),
        }
    }

    fn copy_arg(&mut self, count: &mut FuncNum, arg: &poise::PoiseVal) {
        let arg_type = self.get_asmtype(&arg);
        if arg_type != AsmType::Double {
            let (is_reg, num) = count.get_int();
            let asm_arg = self.gen_operand(arg.clone());
            if is_reg { self.copy_arg_reg(num, asm_arg, arg_type, false) }
            else { self.copy_arg_stack(asm_arg, arg_type) }
        } else {
            let (is_reg, num) = count.get_float();
            let asm_arg = self.gen_operand(arg.clone());
            if is_reg { self.copy_arg_reg(num, asm_arg, arg_type, true) }
            else { self.copy_arg_stack(asm_arg, arg_type) }
        }
    }

    fn unary_handler(&mut self, op: poise::PoiseUnaryOp, src: PoiseVal, dst: PoiseVal) {
        let srct = self.get_asmtype(&src);
        let dstt = self.get_asmtype(&dst);
        let (s, d) = (self.gen_operand(src.clone()), self.gen_operand(dst));
        match op {
            poise::PoiseUnaryOp::Negate => {
                if srct != AsmType::Double {
                    self.push(AsmInstruction::Mov(srct, s.clone(), d.clone()));
                    self.push(AsmInstruction::Unary(UnaryOp::Neg, srct, d))
                } else {
                    let lab = self.intern_static(OrderedFloat::from(-0.0), 16);
                    self.push(AsmInstruction::Mov(srct, s.clone(), d.clone()));
                    self.push(AsmInstruction::Binary(BinaryOp::BitXor, AsmType::Double, Operand::Data(lab), d));
                }
            }
            poise::PoiseUnaryOp::Complement => { 
                self.push(AsmInstruction::Mov(srct, s.clone(), d.clone()));
                self.push(AsmInstruction::Unary(UnaryOp::Not, srct, d))
            }
            poise::PoiseUnaryOp::Not => {
                if !self.is_double(&src.clone()) {
                    self.push(AsmInstruction::Cmp(srct, Operand::Imm(0), s.clone()));
                    self.push(AsmInstruction::Mov(dstt, Operand::Imm(0), d.clone()));
                    self.push(AsmInstruction::SetCC(Condition::E, d));
                } else {
                    let scratch = Operand::Reg(Register::XMM14, RegSize::Quad);
                    self.push(AsmInstruction::Binary(BinaryOp::BitXor, AsmType::Double, scratch.clone(), scratch.clone()));
                    self.push(AsmInstruction::Cmp(AsmType::Double, scratch.clone(), s));
                    self.push(AsmInstruction::Mov(dstt, Operand::Imm(0), d.clone()));
                    self.push(AsmInstruction::SetCC(Condition::E, d.clone()));
                    self.push(AsmInstruction::SetCC(Condition::NP, Operand::Reg(Register::R11, RegSize::Byte)));
                    self.push(AsmInstruction::Binary(BinaryOp::BitAnd, AsmType::Byte, Operand::Reg(Register::R11, RegSize::Byte), d.clone()));
                }
            },
        };
    }

    fn binary_handler(&mut self, op: PoiseBinaryOp, src1: PoiseVal, src2: PoiseVal, dst: PoiseVal) {
        let srct = self.get_asmtype(&src1);
        let dstt = self.get_asmtype(&dst);
        let srcd = self.is_double(&src1);
        let signed = if !srcd { self.get_num_type(&src1).is_signed() } else { false };
        let regsize = get_regsize(&srct);
        let (s1, s2, d) = (self.gen_operand(src1), self.gen_operand(src2), self.gen_operand(dst));
        // println!("OPERANDS: {:?}, {:?}, {:?}", s1, s2, d);
        match op {
            PoiseBinaryOp::Add | PoiseBinaryOp::Subtract | PoiseBinaryOp::Multiply |
            PoiseBinaryOp::BitwiseAnd | PoiseBinaryOp::BitwiseOr | PoiseBinaryOp::BitwiseXor => {
                self.push(AsmInstruction::Mov(srct, s1, d.clone()));
                self.push(AsmInstruction::Binary(gen_binary(op, signed), srct, s2, d));
            },
            PoiseBinaryOp::Divide if srcd => {
                self.push(AsmInstruction::Mov(srct, s1, d.clone()));
                self.push(AsmInstruction::Binary(BinaryOp::DivDouble, srct, s2, d));
            },
            PoiseBinaryOp::Divide | PoiseBinaryOp::Remainder if !srcd => {
                if signed {
                    self.push(AsmInstruction::Mov(srct, s1, Operand::Reg(Register::AX, regsize)));
                    self.push(AsmInstruction::Mov(srct, s2, Operand::Reg(Register::R10, regsize)));
                    self.push(AsmInstruction::Cdq(srct));
                    self.push(AsmInstruction::Idiv(srct, Operand::Reg(Register::R10, regsize)));
                    self.push(AsmInstruction::Mov(srct, Operand::Reg(gen_division(op), regsize), d));
                } else {
                    self.push(AsmInstruction::Mov(srct, s1, Operand::Reg(Register::AX, regsize)));
                    self.push(AsmInstruction::Mov(srct, s2, Operand::Reg(Register::R10, regsize)));
                    self.push(AsmInstruction::Mov(srct, Operand::Imm(0), Operand::Reg(Register::DX, regsize)));
                    self.push(AsmInstruction::Div(srct, Operand::Reg(Register::R10, regsize)));
                    self.push(AsmInstruction::Mov(srct, Operand::Reg(gen_division(op), regsize), d));
                }
            },
            PoiseBinaryOp::LeftShift | PoiseBinaryOp::RightShift => {
                self.push(AsmInstruction::Mov(srct, s1, d.clone()));
                match &s2 {
                    Operand::Imm(_) => {
                        self.push(AsmInstruction::Binary(gen_binary(op, signed), srct, s2, d));
                    },
                    _ => {
                        self.push(AsmInstruction::Mov(AsmType::Longword, s2, Operand::Reg(Register::R10, RegSize::Long)));
                        self.push(AsmInstruction::Mov(AsmType::Byte, Operand::Reg(Register::R10, RegSize::Byte), Operand::Reg(Register::CX, RegSize::Byte)));
                        self.push(AsmInstruction::Binary(gen_binary(op, signed), srct, Operand::Reg(Register::CX, RegSize::Byte), d));
                    },
                }
            }, 
            PoiseBinaryOp::Equal | PoiseBinaryOp::GreaterThan | PoiseBinaryOp::GreaterOrEqual |
            PoiseBinaryOp::LessThan | PoiseBinaryOp::LessOrEqual if srcd => {
                self.push(AsmInstruction::Cmp(srct, s2.clone(), s1));
                self.push(AsmInstruction::Mov(dstt, Operand::Imm(0), d.clone()));
                self.push(AsmInstruction::SetCC(gen_conditional(op, signed), d.clone()));
                self.push(AsmInstruction::SetCC(Condition::NP, Operand::Reg(Register::R11, RegSize::Byte)));
                self.push(AsmInstruction::Binary(BinaryOp::BitAnd, AsmType::Byte, Operand::Reg(Register::R11, RegSize::Byte), d.clone()));
            },
            PoiseBinaryOp::NotEqual if srcd => {
                self.push(AsmInstruction::Cmp(srct, s2.clone(), s1));
                self.push(AsmInstruction::Mov(dstt, Operand::Imm(0), d.clone()));
                self.push(AsmInstruction::SetCC(gen_conditional(op, signed), d.clone()));
                self.push(AsmInstruction::SetCC(Condition::P, Operand::Reg(Register::R11, RegSize::Byte)));
                self.push(AsmInstruction::Binary(BinaryOp::BitOr, AsmType::Byte, Operand::Reg(Register::R11, RegSize::Byte), d.clone()));
            },
            PoiseBinaryOp::Equal | PoiseBinaryOp::NotEqual | PoiseBinaryOp::GreaterThan |
            PoiseBinaryOp::GreaterOrEqual | PoiseBinaryOp::LessThan | PoiseBinaryOp::LessOrEqual => {
                self.push(AsmInstruction::Cmp(srct, s2.clone(), s1));
                self.push(AsmInstruction::Mov(dstt, Operand::Imm(0), d.clone()));
                self.push(AsmInstruction::SetCC(gen_conditional(op, signed), d));
            },
            _ => unreachable!(),
        }
    }

    fn uint_to_double(&mut self, src: PoiseVal, dst: PoiseVal) {
        let srct = self.get_asmtype(&src);
        let regsize = get_regsize(&srct);
        let (s, d) = (self.gen_operand(src), self.gen_operand(dst));
        match srct {
            AsmType::Longword => {
                self.push(AsmInstruction::MovZeroExtend(s, Operand::Reg(Register::AX, RegSize::Long)));
                self.push(AsmInstruction::Cvtsi2sd(AsmType::Quadword, Operand::Reg(Register::AX, RegSize::Quad), d));
            },
            AsmType::Quadword => {
                let (lab1, lab2) = (self.get_count(), self.get_count());
                self.generated.extend(vec![
                    AsmInstruction::Cmp(AsmType::Quadword, Operand::Imm(0), s.clone()),
                    AsmInstruction::JmpCC(Condition::L, lab1.clone()),
                    AsmInstruction::Cvtsi2sd(AsmType::Quadword, s.clone(), d.clone()),
                    AsmInstruction::Jmp(lab2.clone()),
                    AsmInstruction::Label(lab1),
                    AsmInstruction::Mov(AsmType::Quadword, s, Operand::Reg(Register::R10, regsize)),
                    AsmInstruction::Mov(AsmType::Quadword, Operand::Reg(Register::R10, regsize), Operand::Reg(Register::R11, regsize)),
                    AsmInstruction::Binary(BinaryOp::Shr, AsmType::Quadword, Operand::Imm(1), Operand::Reg(Register::R11, regsize)),
                    AsmInstruction::Binary(BinaryOp::BitAnd, AsmType::Quadword, Operand::Imm(1), Operand::Reg(Register::R10, regsize)),
                    AsmInstruction::Binary(BinaryOp::BitOr, AsmType::Quadword, Operand::Reg(Register::R10, regsize), Operand::Reg(Register::R11, regsize)),
                    AsmInstruction::Cvtsi2sd(AsmType::Quadword, Operand::Reg(Register::R11, regsize), d.clone()),
                    AsmInstruction::Binary(BinaryOp::Add, AsmType::Double, d.clone(), d),
                    AsmInstruction::Label(lab2),
                ])
            },
            _ => unreachable!(),
        }
    }

    fn double_to_uint(&mut self, src: PoiseVal, dst: PoiseVal) {
        let dstt = self.get_asmtype(&dst);
        let regsize = get_regsize(&dstt);
        let (s, d) = (self.gen_operand(src), self.gen_operand(dst));
        match dstt {
            AsmType::Longword => {
                self.push(AsmInstruction::Cvttsd2si(AsmType::Quadword, s, Operand::Reg(Register::AX, RegSize::Quad)));
                self.push(AsmInstruction::Mov(AsmType::Longword, Operand::Reg(Register::AX, regsize), d));
            },
            AsmType::Quadword => {
                let (lab1, lab2) = (self.get_count(), self.get_count());
                let upper_bound = Operand::Data(self.intern_static(OrderedFloat::from(9223372036854775808.0), 8));
                self.generated.extend(vec![
                    AsmInstruction::Cmp(AsmType::Double, upper_bound.clone(), s.clone()),
                    AsmInstruction::JmpCC(Condition::AE, lab1.clone()),
                    AsmInstruction::Cvttsd2si(AsmType::Quadword, s.clone(), d.clone()),
                    AsmInstruction::Jmp(lab2.clone()),
                    AsmInstruction::Label(lab1),
                    AsmInstruction::Mov(AsmType::Double, s, Operand::Reg(Register::XMM0, RegSize::Quad)),
                    AsmInstruction::Binary(BinaryOp::Sub, AsmType::Double, upper_bound, Operand::Reg(Register::XMM0, RegSize::Quad)),
                    AsmInstruction::Cvttsd2si(AsmType::Quadword, Operand::Reg(Register::XMM0, RegSize::Quad), d.clone()),
                    AsmInstruction::Mov(AsmType::Quadword, Operand::Imm(-9223372036854775808), Operand::Reg(Register::R10, RegSize::Quad)),
                    AsmInstruction::Binary(BinaryOp::Add, AsmType::Quadword, Operand::Reg(Register::R10, RegSize::Quad), d),
                    AsmInstruction::Label(lab2),
                ])
            },
            _ => unreachable!(),
        }
    }
}
