use super::*;

use std::collections::hash_map::HashMap;
use ordered_float::OrderedFloat;

use crate::parser::Const;

use crate::poise;

#[derive(Debug)]
pub struct AsmProgram {
    pub top_level: Vec<AsmTopLevel>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AsmType {
    Byte,
    Longword,
    Quadword,
    Double,
}

#[derive(Debug)]
pub enum AsmTopLevel {
    F(AsmFunction),
    V(AsmStaticVar),
    C(StaticConstant),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StaticConstant {
    pub identifier: String,
    pub alignment: usize,
    pub init: StaticInit,
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
    pub alignment: usize,
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
    Cvttsd2si(AsmType, Operand, Operand),
    Cvtsi2sd(AsmType, Operand, Operand),
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Sub,
    Mult,
    DivDouble,
    Sal,
    Sar,
    Shr,
    Shl,
    BitAnd,
    BitOr,
    BitXor,
}

#[derive(Debug, Clone)]
pub enum Operand {
    Imm(i64),
    Reg(Register, RegSize),
    Pseudo(String),
    Stack(i32),
    Data(String),
}

#[derive(Debug, Clone, Copy)]
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
    XMM0,
    XMM1,
    XMM2,
    XMM3,
    XMM4,
    XMM5,
    XMM6,
    XMM7,
    XMM8,
    XMM9,
    XMM10,
    XMM11,
    XMM12,
    XMM13,
    XMM14,
    XMM15,
}

#[derive(Debug, Clone, Copy)]
pub enum RegSize {
    Byte = 0,
    Long = 1,
    Quad = 2,
}

#[derive(Debug, Clone, Copy)]
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
    P,
    NP,
}

pub fn get_func_reg(argn: usize, is_float: bool) -> Register {
    if !is_float {
        match argn {
            0 => Register::DI,
            1 => Register::SI,
            2 => Register::DX,
            3 => Register::CX,
            4 => Register::R8,
            5 => Register::R9,
            _ => panic!(),
        }
    } else {
        match argn {
            0 => Register::XMM0,
            1 => Register::XMM1,
            2 => Register::XMM2,
            3 => Register::XMM3,
            4 => Register::XMM4,
            5 => Register::XMM5,
            6 => Register::XMM6,
            7 => Register::XMM7,
            _ => panic!(),
        }
    }
}

pub struct FuncNum {
    pub int_param: usize,
    pub float_param: usize,
    pub stack: usize,
}

impl FuncNum {
    pub fn new() -> Self {
        FuncNum { int_param: 0, float_param: 0, stack: 0 }
    }

    pub fn get_int(&mut self) -> (bool, usize) {
        let ret = if self.int_param <= 5 { let num = self.int_param; self.int_param += 1; (true, num) }
        else { let num = self.stack; self.stack += 1; (false, num) };
        ret
    }

    pub fn get_float(&mut self) -> (bool, usize) {
        let ret = if self.float_param <= 7 { let num = self.float_param; self.float_param += 1; (true, num) }
        else { let num = self.stack; self.stack += 1; (false, num) };
        ret
    }
}

pub fn get_regsize(asmtype: &AsmType) -> RegSize {
    match asmtype {
        AsmType::Byte => RegSize::Byte,
        AsmType::Longword => RegSize::Long,
        AsmType::Quadword => RegSize::Quad,
        AsmType::Double => RegSize::Quad,
    }
}

pub fn get_alignment(asmtype: &AsmType) -> usize {
    match asmtype {
        AsmType::Byte => 1,
        AsmType::Longword => 4,
        AsmType::Quadword => 8,
        AsmType::Double => 8,
    }
}

#[derive(Debug)]
pub struct Statics {
    pub statics_map: HashMap<(OrderedFloat<f64>, usize), StaticConstant>,
}

impl Statics {
    pub fn new() -> Statics {
        Statics { statics_map: HashMap::new() }
    }

    pub fn intern_static(&mut self, init: OrderedFloat<f64>, alignment: usize) -> String {
        let next_id = self.statics_map.len();
        let stat = self.statics_map.entry((init, alignment)).or_insert_with(|| {
            StaticConstant { identifier: format!(".L.constant.{}", next_id), 
                             alignment, 
                             init: StaticInit::DoubleInit(init) }
        });
        stat.identifier.clone()
    }
}

impl AsmType {
    pub fn is_double(&self) -> bool {
        matches!(self, AsmType::Double)
    }
}

impl Operand {
    pub fn is_memory(&self) -> bool {
        matches!(self, Operand::Stack(_) | Operand::Data(_))
    }

    pub fn is_reg(&self) -> bool {
        matches!(self, Operand::Reg(..))
    }

    pub fn is_imm(&self) -> bool {
        matches!(self, Operand::Imm(_))
    }
    
    pub fn is_large_32bit_imm(&self) -> bool {
        if let Operand::Imm(v) = self {
            i32::try_from(*v).is_err() && u32::try_from(*v).is_ok()
        } else { false }
    }

    pub fn is_large_64bit_imm(&self) -> bool {
        if let Operand::Imm(v) = self {
            u32::try_from(*v).is_err()
        } else { false }
    }
}
