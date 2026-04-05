use std::fmt;
use ordered_float::OrderedFloat;

use crate::codegen::*;
use crate::types::*;

#[derive(Debug)]
pub enum EmissionError {
    UnresolvedPseudoRegister(String)
}

impl fmt::Display for EmissionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EmissionError::UnresolvedPseudoRegister(ident) => {
                write!(f, "Unresolved Pseudo Register! {ident}")
            }
        }
    }
}

impl std::error::Error for EmissionError { }

enum CondOp {
    Jmp,
    Set,
}

impl AsmType {
    fn suffix(&self) -> &'static str {
        match self {
            AsmType::Byte => "b",
            AsmType::Longword => "l",
            AsmType::Quadword => "q",
            AsmType::Double => "sd",
        }
    }
}

impl fmt::Display for AsmType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.suffix())
    }
}

pub fn emit_program(program: AsmProgram, symbols: &mut AsmSymbolTable)-> Result<String, EmissionError> {
    let mut output = String::new();
    let mut z_vars = String::new();
    let mut nz_vars = String::new();
    let mut constants = String::new();
    for item in program.top_level {
        match item {
            AsmTopLevel::F(f) => emit_function(f, &mut output, symbols)?,
            AsmTopLevel::V(v) => emit_var(v, &mut z_vars, &mut nz_vars)?,
            AsmTopLevel::C(c) => emit_constant(c, &mut constants)?,
        }
    }
    if z_vars.len() > 0 { output.push_str("\t.bss\n"); output.push_str(&z_vars) }
    if nz_vars.len() > 0 { output.push_str("\t.data\n"); output.push_str(&nz_vars) }
    if constants.len() > 0 { output.push_str("\t.section .rodata\n"); output.push_str(&constants) }
    output.push_str("\n.section .note.GNU-stack,\"\",@progbits");
    Ok(output)
}

fn emit_staticinit(init: StaticInit) -> (String, bool) {
    match init {
        StaticInit::IntInit(0) => (String::from(".zero 4"), false),
        StaticInit::IntInit(i) => (String::from(format!(".long {}", i)), true),
        StaticInit::LongInit(0) => (String::from(".zero 8"), false),
        StaticInit::LongInit(i) => (String::from(format!(".quad {}", i)), true),
        StaticInit::UIntInit(0) => (String::from(".zero 4"), false),
        StaticInit::UIntInit(i) => (String::from(format!(".long {}", i)), true),
        StaticInit::ULongInit(0) => (String::from(".zero 8"), false),
        StaticInit::ULongInit(i) => (String::from(format!(".quad {}", i)), true),
        StaticInit::DoubleInit(i) => {
            let bits = i.into_inner().to_bits();
            (String::from(format!(".quad {}", bits)), true)
        }
    }
}

fn emit_var(var: AsmStaticVar, zero: &mut String, nonzero: &mut String) -> Result<(), EmissionError> {
    let mut output = String::new();
    if var.global {
        output.push_str(&format!("\t.globl {}\n", var.identifier));
    }
    output.push_str(&format!("\t.align {}\n", var.alignment));
    output.push_str(&format!("{}:\n", var.identifier));
    let (init, nz) = emit_staticinit(var.init);
    output.push_str(&format!("\t{}\n", init));
    if nz { nonzero.push_str(&output) } else { zero.push_str(&output) }
    Ok(())
}

fn emit_constant(constant: StaticConstant, cons: &mut String) -> Result<(), EmissionError> {
    cons.push_str(&format!("\t.align {}\n", constant.alignment));
    cons.push_str(&format!("{}:\n", constant.identifier));
    let StaticInit::DoubleInit(val) = constant.init else { unreachable!() };
    let bits = val.into_inner().to_bits();
    cons.push_str(&format!("\t.quad {}\n", bits));
    Ok(())
}

fn emit_function(function: AsmFunction, output: &mut String, symbols: &mut AsmSymbolTable) -> Result<(), EmissionError> {
    if function.global {
        output.push_str(&format!("\t.globl {}\n", function.identifier));
    }
    output.push_str("\t.text\n");
    output.push_str(&format!("{}:\n", function.identifier));
    output.push_str("\tpushq\t%rbp\n");
    output.push_str("\tmovq\t%rsp,\t%rbp\n");
    for instruction in function.body {
       emit_instruction(instruction, output, symbols)?;
    }
    Ok(())
}

fn emit_instruction(instruction: AsmInstruction, output: &mut String, symbols: &mut AsmSymbolTable) -> Result<(), EmissionError> {
    match instruction {
        AsmInstruction::Mov(t, src, dst) => { 
            let src = emit_operand(src)?;
            let dst = emit_operand(dst)?;
            output.push_str(&format!("\tmov{}\t{src},\t{dst}\n", t.suffix()));
        },
        AsmInstruction::Movsx(src, dst) => { 
            let src = emit_operand(src)?;
            let dst = emit_operand(dst)?;
            output.push_str(&format!("\tmovslq\t{src},\t{dst}\n"));
        },
        AsmInstruction::Unary(unary_op, t, operand) => {
            let dst = emit_operand(operand)?;
            let op = emit_unary_op(unary_op);
            output.push_str(&format!("\t{op}{}\t{dst}\n", t.suffix()));
        },
        AsmInstruction::Binary(binary_op, t, operand1, operand2) => {
            // println!("BINARY: Binary(op:{:?}, t:{:?}, src:{:?}, dst:{:?}", binary_op, t, operand1, operand2);
            let src = emit_operand(operand1.clone())?;
            let dst = emit_operand(operand2.clone())?;
            let op = emit_binary_op(binary_op);
            match (t, binary_op) {
                (AsmType::Double, BinaryOp::Mult) => output.push_str(&format!("\tmulsd\t{src},\t{dst}\n")),
                (AsmType::Double, BinaryOp::BitXor) => output.push_str(&format!("\txorpd\t{src},\t{dst}\n")),
                (AsmType::Double, BinaryOp::DivDouble) => output.push_str(&format!("\tdivsd\t{src},\t{dst}\n")),
                (_, _) => output.push_str(&format!("\t{op}{}\t{src},\t{dst}\n", t.suffix())),
            }
        },
        AsmInstruction::Cmp(t, operand1, operand2) => {
            let src = emit_operand(operand1)?;
            let dst = emit_operand(operand2)?;
            if matches!(t, AsmType::Double) {
                output.push_str(&format!("\tcomisd\t{src},\t{dst}\n"));
            } else {
                output.push_str(&format!("\tcmp{}\t{src},\t{dst}\n", t.suffix()));
            }
        },
        AsmInstruction::SetCC(cond_code, dst) => {
            let op = emit_conditional_op(CondOp::Set, cond_code);
            let dst = emit_operand(dst)?;
            output.push_str(&format!("\t{op}\t{dst}\n"));
        }
        AsmInstruction::JmpCC(cond_code, label) => {
            let op = emit_conditional_op(CondOp::Jmp, cond_code);
            output.push_str(&format!("\t{op}\t.L{label}\n"));
        }
        AsmInstruction::Jmp(label) => output.push_str(&format!("\tjmp\t.L{label}\n")),
        AsmInstruction::Label(label) => output.push_str(&format!(".L{label}:\n")),
        AsmInstruction::Idiv(t, operand) => {
            let op = emit_operand(operand)?;
            output.push_str(&format!("\tidiv{}\t{op}\n", t.suffix()));
        },
        AsmInstruction::Div(t, operand) => {
            let op = emit_operand(operand)?;
            output.push_str(&format!("\tdiv{}\t{op}\n", t.suffix()));
        },
        AsmInstruction::Cdq(AsmType::Byte) => {
            unreachable!();
        },
        AsmInstruction::Cdq(AsmType::Longword) => {
            output.push_str("\tcltd\n");
        },
        AsmInstruction::Cdq(AsmType::Quadword) => {
            output.push_str("\tcqto\n");
        },
        AsmInstruction::Ret => {
            output.push_str("\tmovq\t%rbp,\t%rsp\n");
            output.push_str("\tpopq\t%rbp\n");
            output.push_str("\tret\n");
        },
        AsmInstruction::Push(op) => {
            let op = emit_operand(op)?;
            output.push_str(&format!("\tpushq\t{op}\n"));
        },
        AsmInstruction::Call(id) => {
            let mut name = id.clone();
            let Some(AsmSymbol::FuncEntry(defined)) = symbols.get(&id) else { unreachable!() };
            if !defined {
                name.push_str("@PLT");
            }
            output.push_str(&format!("\tcall\t{name}\n"));
        },
        AsmInstruction::Cvtsi2sd(t, src, dst) => {
            let src = emit_operand(src)?;
            let dst = emit_operand(dst)?;
            output.push_str(&format!("\tcvtsi2sd{}\t{src},\t{dst}\n", t.suffix()));
        },
        AsmInstruction::Cvttsd2si(t, src, dst) => {
            let src = emit_operand(src)?;
            let dst = emit_operand(dst)?;
            output.push_str(&format!("\tcvttsd2si{}\t{src},\t{dst}\n", t.suffix()));
        },
        _ => unreachable!(),
    }
    Ok(())
}

fn emit_conditional_op(instruction: CondOp, condition: Condition) -> String {
    let first = match instruction {
        CondOp::Set => "set",
        CondOp::Jmp => "j",
    };
    let second = match condition {
        Condition::E => "e",
        Condition::NE => "ne",
        Condition::L => "l",
        Condition::LE => "le",
        Condition::G => "g",
        Condition::GE => "ge",
        Condition::A => "a",
        Condition::AE => "ae",
        Condition::B => "b",
        Condition::BE => "be",
    };
    format!("{first}{second}")
}

fn emit_operand(operand: Operand) -> Result<String, EmissionError> {
    match operand {
        Operand::Imm(value) => Ok(format!("${value}")),
        Operand::Reg(reg, regsize) => {
            let n = regsize as usize;
            let rstr = match reg {
                Register::AX   => ["al", "eax", "rax"][n],
                Register::CX   => ["cl", "ecx", "rcx"][n],
                Register::DX   => ["dl", "edx", "rdx"][n],
                Register::DI   => ["dil", "edi", "rdi"][n],
                Register::SI   => ["sil", "esi", "rsi"][n],
                Register::R8   => ["r8b", "r8d", "r8"][n],
                Register::R9   => ["r9b", "r9d", "r9"][n],
                Register::R10  => ["r10b", "r10d", "r10"][n],
                Register::R11  => ["r11b", "r11d", "r11"][n],
                Register::SP   => "rsp",
                Register::XMM0  => "xmm0",
                Register::XMM1  => "xmm1",
                Register::XMM2  => "xmm2",
                Register::XMM3  => "xmm3",
                Register::XMM4  => "xmm4",
                Register::XMM5  => "xmm5",
                Register::XMM6  => "xmm6",
                Register::XMM7  => "xmm7",
                Register::XMM8  => "xmm8",
                Register::XMM9  => "xmm9",
                Register::XMM10 => "xmm10",
                Register::XMM11 => "xmm11",
                Register::XMM12 => "xmm12",
                Register::XMM13 => "xmm13",
                Register::XMM14 => "xmm14",
                Register::XMM15 => "xmm15",
            };
            Ok(format!("%{rstr}"))
        },
        Operand::Stack(int) => Ok(format!("{int}(%rbp)")),
        Operand::Data(ident) => Ok(format!("{ident}(%rip)")),
        Operand::Pseudo(ident) => Err(EmissionError::UnresolvedPseudoRegister(ident)),
    }
}

fn emit_unary_op(unary_op: UnaryOp) -> &'static str  {
    match unary_op {
        UnaryOp::Neg => "neg",
        UnaryOp::Not => "not",
    }
}

fn emit_binary_op(binary_op: BinaryOp) -> &'static str {
    match binary_op {
        BinaryOp::Add       => "add",
        BinaryOp::Sub       => "sub",
        BinaryOp::Mult      => "imul",
        BinaryOp::Sal       => "sal",
        BinaryOp::Sar       => "sar",
        BinaryOp::Shl       => "shl",
        BinaryOp::Shr       => "shr",
        BinaryOp::BitAnd    => "and",
        BinaryOp::BitOr     => "or",
        BinaryOp::BitXor    => "xor",
        BinaryOp::DivDouble => "divsd",
    }
}
