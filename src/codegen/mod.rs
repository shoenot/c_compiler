mod stack;
use std::collections::VecDeque;

use stack::*;

use crate::parser::Const;

use crate::poise::{self, PoiseBinaryOp, PoiseVal, TopLevelItem};
use crate::types::*;

pub mod asm_symtab;
pub use asm_symtab::*;

#[derive(Debug)]
pub struct AsmProgram {
    pub top_level: Vec<AsmTopLevel>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AsmType {
    Byte,
    Longword,
    Quadword,
}

#[derive(Debug)]
pub enum AsmTopLevel {
    F(AsmFunction),
    V(AsmStaticVar),
}

#[derive(Debug, Clone)]
pub struct AsmFunction { 
    pub identifier: String, 
    pub global: bool, 
    pub body: Vec<AsmInstruction>,
}

#[derive(Debug)]
pub struct AsmStaticVar {
    pub identifier: String, 
    pub global: bool, 
    pub alignment: i32,
    pub init: StaticInit,
}

#[derive(Debug, Clone)]
pub enum AsmInstruction {
    Mov(AsmType, Operand, Operand),
    Movsx(Operand, Operand),
    MovZeroExtend(Operand, Operand),
    Unary(UnaryOp, AsmType, Operand),
    Binary(BinaryOp, AsmType, Operand, Operand),
    Cmp(AsmType, Operand, Operand),
    Idiv(AsmType, Operand),
    Div(AsmType, Operand),
    Cdq(AsmType),
    Jmp(String),
    JmpCC(Condition, String),
    SetCC(Condition, Operand),
    Label(String),
    Push(Operand),
    Call(String),
    Ret,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Sub,
    Mult,
    Sal,
    Sar,
    Shr,
    Shl,
    BitAnd,
    BitOr,
    BitXor,
}

#[derive(Debug,Clone)]
pub enum Operand {
    Imm(i64),
    Reg(Register, RegSize),
    Pseudo(String),
    Stack(i32),
    Data(String),
}

#[derive(Debug,Clone)]
pub enum Register {
    AX,
    CX,
    DX,
    DI,
    SI,
    R8,
    R9,
    R10,
    R11,
    SP,
}

#[derive(Debug,Clone)]
pub enum RegSize {
    Byte = 0,
    Long = 1,
    Quad = 2,
}

#[derive(Debug, Clone)]
pub enum Condition {
    E,
    NE,
    L,
    LE,
    G,
    GE,
    A,
    AE,
    B,
    BE,
}

// at this point all the symbols should be in the table. so if the unwrap fails that means
// something is fucked. the unwrap shouldn't fail. 
fn get_symbol(symbols: &SymbolTable, ident: &String) -> Symbol {
    symbols.get(ident).unwrap().clone()
}

fn get_var_type(symbols: &SymbolTable, ident: &String) -> AsmType {
    convert_type(&get_symbol(symbols, ident).datatype)
}

fn get_num_type(symbols: &SymbolTable, val: &poise::PoiseVal) -> Type {
    match val {
        poise::PoiseVal::Variable(ident) => get_symbol(symbols, ident).datatype,
        poise::PoiseVal::Constant(i) => {
            match i {
                    Const::Int(_) => Type::Int,
                    Const::Long(_) => Type::Long,
                    Const::UInt(_) => Type::UInt,
                    Const::ULong(_) => Type::ULong,
            }
        },
    }
}

fn get_asmtype(symbols: &SymbolTable, val: &poise::PoiseVal) -> AsmType {
    match val {
        poise::PoiseVal::Variable(ident) => get_var_type(symbols, ident),
        poise::PoiseVal::Constant(Const::Int(_)) => AsmType::Longword,
        poise::PoiseVal::Constant(Const::Long(_)) => AsmType::Quadword,
        poise::PoiseVal::Constant(Const::UInt(_)) => AsmType::Longword,
        poise::PoiseVal::Constant(Const::ULong(_)) => AsmType::Quadword,
    }
}

fn get_func_reg(argn: usize) -> Register {
    match argn {
        0 => Register::DI,
        1 => Register::SI,
        2 => Register::DX,
        3 => Register::CX,
        4 => Register::R8,
        5 => Register::R9,
        _ => panic!(),
    }
}

fn get_regsize(asmtype: &AsmType) -> RegSize {
    match asmtype {
        AsmType::Byte => RegSize::Byte,
        AsmType::Longword => RegSize::Long,
        AsmType::Quadword => RegSize::Quad,
    }
}

fn get_alignment(asmtype: &AsmType) -> i32 {
    match asmtype {
        AsmType::Byte => 1,
        AsmType::Longword => 4,
        AsmType::Quadword => 8,
    }
}

pub fn gen_program(tree: poise::PoiseProg, symbols: &mut SymbolTable, asm_symbols: &mut AsmSymbolTable) -> AsmProgram {
    let mut top_level = Vec::new();
    let mut functions = Vec::new();
    for item in tree.top_level_items {
        match item {
            TopLevelItem::F(f) => functions.push(gen_function(f,symbols)),
            TopLevelItem::V(v) => top_level.push(gen_static_var(v)),
        }
    }
    convert_symtable(symbols, asm_symbols);
    functions = assign_stack_slots(functions, asm_symbols);
    top_level.extend(functions.iter().map(|func| AsmTopLevel::F(func.clone())));
    AsmProgram { top_level }
}

pub fn gen_static_var(var: poise::PoiseStaticVar) -> AsmTopLevel {
    AsmTopLevel::V(AsmStaticVar { identifier: var.identifier, 
                                  global: var.global, 
                                  alignment: get_alignment(&convert_type(&var.datatype)),
                                  init: var.init })
}

fn copy_param(num: usize, param: String, symbols: &SymbolTable) -> AsmInstruction {
    match num {
        0..=5 => {
            let param_type = get_asmtype(symbols, &PoiseVal::Variable(param.clone()));
            let param_reg = get_func_reg(num);
            let param_regsize = get_regsize(&param_type);
            AsmInstruction::Mov(param_type, Operand::Reg(param_reg, param_regsize), Operand::Pseudo(param))
        },
        _ => {
            let offset = (num.saturating_sub(6) * 8) + 16;
            let param_type = get_asmtype(symbols, &PoiseVal::Variable(param.clone()));
            AsmInstruction::Mov(param_type, Operand::Stack(offset as i32), Operand::Pseudo(param))
        }
    }
}

fn gen_function(func: poise::PoiseFunc, symbols: &mut SymbolTable) -> AsmFunction {
    let mut generated = Vec::new();
    let identifier = func.identifier;
    func.params.iter()
        .enumerate()
        .for_each(|(num , param)| generated.push(copy_param(num, param.into(), symbols)));
    gen_instructions(func.body, &mut generated, symbols);
    AsmFunction { identifier, global: func.global, body: generated }
}

fn gen_instructions(
    instructions: Vec<poise::PoiseInstruction>, 
    generated: &mut Vec<AsmInstruction>,
    symbols: &SymbolTable) {
    for instruction in instructions {
        match instruction {
            poise::PoiseInstruction::Return(val) => {
                let asmtype = get_asmtype(symbols, &val);
                let regsize = get_regsize(&asmtype);
                generated.push(AsmInstruction::Mov(asmtype, 
                                                   gen_operand(val), 
                                                   Operand::Reg(Register::AX, regsize)));
                generated.push(AsmInstruction::Ret);
            },
            poise::PoiseInstruction::Unary { op,src,dst } => {
                unary_handler(op, src, dst, generated, symbols);
            },
            poise::PoiseInstruction::Binary { op, src1, src2, dst } => {
                binary_handler(op, src1, src2, dst, generated, symbols);
            },
            poise::PoiseInstruction::Jump(id) => generated.push(AsmInstruction::Jmp(id)),
            poise::PoiseInstruction::JumpIfZero{condition: cnd, identifier: id} => {
                generated.push(AsmInstruction::Cmp(get_asmtype(symbols, &cnd), 
                                                   Operand::Imm(0), 
                                                   gen_operand(cnd)));
                generated.push(AsmInstruction::JmpCC(Condition::E, id))
            }
            poise::PoiseInstruction::JumpIfNotZero{condition: cnd, identifier: id} => {
                generated.push(AsmInstruction::Cmp(get_asmtype(symbols, &cnd), 
                                                   Operand::Imm(0), 
                                                   gen_operand(cnd)));
                generated.push(AsmInstruction::JmpCC(Condition::NE, id))
            },
            poise::PoiseInstruction::Copy{src: s, dst: d} => generated.push(
                                                                AsmInstruction::Mov(get_asmtype(symbols, &s),
                                                                                    gen_operand(s), 
                                                                                    gen_operand(d))),
            poise::PoiseInstruction::Label(id) => generated.push(AsmInstruction::Label(id)),
            poise::PoiseInstruction::FunctionCall { ident, args, dst } => {
                let mut stack_padding: i32 = 0;

                if args.len() % 2 != 0 {
                    stack_padding = 8;
                    generated.push(AsmInstruction::Binary(BinaryOp::Sub, 
                                                          AsmType::Quadword, 
                                                          Operand::Imm(stack_padding as i64), 
                                                          Operand::Reg(Register::SP, RegSize::Quad)));
                }

                let removal_bytes = 8 * (args.len().saturating_sub(6) as i32) + stack_padding;
                let mut args: VecDeque<(usize, &PoiseVal)> = VecDeque::from(args.iter().enumerate().collect::<Vec<_>>());

                let first_six = args.drain(..args.len().min(6));

                for (num, arg) in first_six {
                    copy_arg(num, arg.clone(), generated, symbols);
                }

                while let Some((num, arg)) = args.pop_back() {
                    copy_arg(num, arg.clone(), generated, symbols);
                }

                generated.push(AsmInstruction::Call(ident));
                
                if removal_bytes != 0 {
                    generated.push(AsmInstruction::Binary(BinaryOp::Add, 
                                                          AsmType::Quadword, 
                                                          Operand::Imm(removal_bytes as i64), 
                                                          Operand::Reg(Register::SP, RegSize::Quad)));
                }
                
                let dst_type = get_asmtype(symbols, &dst);
                let dst_regsize = get_regsize(&dst_type);
                generated.push(AsmInstruction::Mov(dst_type, 
                                                   Operand::Reg(Register::AX, dst_regsize), 
                                                   gen_operand(dst)));
            },
            poise::PoiseInstruction::SignExtend { src, dst } => {
                generated.push(AsmInstruction::Movsx(gen_operand(src), gen_operand(dst)));
            },
            poise::PoiseInstruction::Truncate { src, dst } => {
                generated.push(AsmInstruction::Mov(AsmType::Longword, gen_operand(src), gen_operand(dst)));
            },
            poise::PoiseInstruction::ZeroExtend { src, dst } => generated.push(AsmInstruction::MovZeroExtend(gen_operand(src), gen_operand(dst))),
        }
    }
}

fn copy_arg(num: usize, arg: poise::PoiseVal, generated: &mut Vec<AsmInstruction>, symbols: &SymbolTable) {
    match num {
        0..=5 => {
            let arg_type = get_asmtype(symbols, &arg);
            let arg_reg = get_func_reg(num);
            let arg_regsize = get_regsize(&arg_type);
            generated.push(AsmInstruction::Mov(arg_type, gen_operand(arg), Operand::Reg(arg_reg, arg_regsize)))
        }
        _ => {
            let arg_type = get_asmtype(symbols, &arg);
            let operand = gen_operand(arg);
            match operand {
                Operand::Pseudo(_) | Operand::Stack(_) | Operand::Data(_) => {
                    match arg_type {
                        AsmType::Longword => {
                            generated.push(AsmInstruction::Mov(arg_type, operand, Operand::Reg(Register::AX, RegSize::Long)));
                            generated.push(AsmInstruction::Push(Operand::Reg(Register::AX, RegSize::Quad)));
                        },
                        AsmType::Quadword => generated.push(AsmInstruction::Push(operand)),
                        AsmType::Byte => unreachable!()
                    }
                }
                Operand::Imm(_) | Operand::Reg(_, _) => generated.push(AsmInstruction::Push(operand)),
            }
        },
    }
}

fn gen_operand(exp: poise::PoiseVal) -> Operand {
    match exp {
        poise::PoiseVal::Constant(cst) => match cst {
            Const::Int(val) => Operand::Imm(val as i64),
            Const::Long(val) => Operand::Imm(val),
            Const::UInt(val) => Operand::Imm(val as i64),
            Const::ULong(val) => Operand::Imm(val as i64),
        }
        poise::PoiseVal::Variable(ident) => Operand::Pseudo(ident),
    }
}

fn unary_handler(
    op: poise::PoiseUnaryOp, 
    src: PoiseVal, 
    dst: PoiseVal, 
    generated: &mut Vec<AsmInstruction>, 
    symbols: &SymbolTable) {
    let srct = get_asmtype(symbols, &src);
    let dstt = get_asmtype(symbols, &dst);
    let (s, d) = (gen_operand(src), gen_operand(dst));
    match op {
        poise::PoiseUnaryOp::Negate => {
            generated.push(AsmInstruction::Mov(srct.clone(), s.clone(), d.clone()));
            generated.push(AsmInstruction::Unary(UnaryOp::Neg, srct.clone(), d))
        }
        poise::PoiseUnaryOp::Complement => { 
            generated.push(AsmInstruction::Mov(srct.clone(), s.clone(), d.clone()));
            generated.push(AsmInstruction::Unary(UnaryOp::Not, srct.clone(), d))
        }
        poise::PoiseUnaryOp::Not => {
            generated.push(AsmInstruction::Cmp(srct, Operand::Imm(0), s));
            generated.push(AsmInstruction::Mov(dstt, Operand::Imm(0), d.clone()));
            generated.push(AsmInstruction::SetCC(Condition::E, d));
        },
    };
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

fn binary_handler(
    op: PoiseBinaryOp, 
    src1: PoiseVal, 
    src2: PoiseVal, 
    dst: PoiseVal, 
    generated: &mut Vec<AsmInstruction>,
    symbols: &SymbolTable) {
    let srct = get_asmtype(symbols, &src1);
    let signed = get_num_type(symbols, &src1).is_signed();
    let regsize = get_regsize(&srct);
    let (s1, s2, d) = (gen_operand(src1), gen_operand(src2), gen_operand(dst));
    match op {
        PoiseBinaryOp::Add | PoiseBinaryOp::Subtract | PoiseBinaryOp::Multiply |
        PoiseBinaryOp::BitwiseAnd | PoiseBinaryOp::BitwiseOr | PoiseBinaryOp::BitwiseXor => {
            generated.push(AsmInstruction::Mov(srct.clone(), s1, d.clone()));
            generated.push(AsmInstruction::Binary(gen_binary(op, signed), srct.clone(), s2, d));
        },
        PoiseBinaryOp::Divide | PoiseBinaryOp::Remainder => {
            if signed {
                generated.push(AsmInstruction::Mov(srct.clone(), s1, Operand::Reg(Register::AX, regsize.clone())));
                generated.push(AsmInstruction::Mov(srct.clone(), s2, Operand::Reg(Register::R10, regsize.clone())));
                generated.push(AsmInstruction::Cdq(srct.clone()));
                generated.push(AsmInstruction::Idiv(srct.clone(), Operand::Reg(Register::R10, regsize.clone())));
                generated.push(AsmInstruction::Mov(srct.clone(), Operand::Reg(gen_division(op), regsize.clone()), d));
            } else {
                generated.push(AsmInstruction::Mov(srct.clone(), s1, Operand::Reg(Register::AX, regsize.clone())));
                generated.push(AsmInstruction::Mov(srct.clone(), s2, Operand::Reg(Register::R10, regsize.clone())));
                generated.push(AsmInstruction::Mov(srct.clone(), Operand::Imm(0), Operand::Reg(Register::DX, regsize.clone())));
                generated.push(AsmInstruction::Div(srct.clone(), Operand::Reg(Register::R10, regsize.clone())));
                generated.push(AsmInstruction::Mov(srct.clone(), Operand::Reg(gen_division(op), regsize.clone()), d));
            }
        },
        PoiseBinaryOp::LeftShift | PoiseBinaryOp::RightShift => {
            generated.push(AsmInstruction::Mov(srct.clone(), s1, d.clone()));
            match &s2 {
                Operand::Imm(_) => {
                    generated.push(AsmInstruction::Binary(gen_binary(op, signed), srct.clone(), s2, d));
                },
                _ => {
                    generated.push(AsmInstruction::Mov(AsmType::Longword, s2, Operand::Reg(Register::R10, RegSize::Long)));
                    generated.push(AsmInstruction::Mov(AsmType::Byte, Operand::Reg(Register::R10, RegSize::Byte), Operand::Reg(Register::CX, RegSize::Byte)));
                    generated.push(AsmInstruction::Binary(gen_binary(op, signed), srct.clone(), Operand::Reg(Register::CX, RegSize::Byte), d));
                },
            }
        }
        PoiseBinaryOp::Equal | PoiseBinaryOp::NotEqual | PoiseBinaryOp::GreaterThan |
        PoiseBinaryOp::GreaterOrEqual | PoiseBinaryOp::LessThan | PoiseBinaryOp::LessOrEqual => {
            generated.push(AsmInstruction::Cmp(srct, s2.clone(), s1));
            generated.push(AsmInstruction::Mov(AsmType::Longword, Operand::Imm(0), d.clone()));
            generated.push(AsmInstruction::SetCC(gen_conditional(op, signed), d));
        }
    }
}
