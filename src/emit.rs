use crate::codegen::*;
use std::fmt;

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

pub fn emit_program(program: AsmProgram) -> Result<String, EmissionError> {
    let mut output = String::new();
    emit_function(program.function, &mut output)?;
    output.push_str("\n.section .note.GNU-stack,\"\",@progbits");
    Ok(output)
}

fn emit_function(function: AsmFunction, output: &mut String) -> Result<(), EmissionError> {
    output.push_str(&format!("\t.globl {}\n", function.name));
    output.push_str(&format!("{}:\n", function.name));
    output.push_str("\tpushq\t%rbp\n");
    output.push_str("\tmovq\t%rsp,\t%rbp\n");
    for instruction in function.body {
       emit_instruction(instruction, output)?;
    }
    Ok(())
}

fn emit_instruction(instruction: AsmInstruction, output: &mut String) -> Result<(), EmissionError> {
    match instruction {
        AsmInstruction::Mov(src, dst) => {
            let src = emit_operand(src)?;
            let dst = emit_operand(dst)?;
            output.push_str(&format!("\tmovl\t{src},\t{dst}\n"));
        },
        AsmInstruction::Movb(src, dst) => {
            let src = emit_operand(src)?;
            let dst = emit_operand(dst)?;
            output.push_str(&format!("\tmovb\t{src},\t{dst}\n"));
        },
        AsmInstruction::Unary(unary_op, operand) => {
            let dst = emit_operand(operand)?;
            let op = emit_unary_op(unary_op);
            output.push_str(&format!("\t{op}\t{dst}\n"));
        },
        AsmInstruction::Binary(binary_op, operand1, operand2) => {
            let src = emit_operand(operand1)?;
            let dst = emit_operand(operand2)?;
            let op = emit_binary_op(binary_op);
            output.push_str(&format!("\t{op}\t{src},\t{dst}\n"));
        },
        AsmInstruction::Idiv(operand) => {
            let op = emit_operand(operand)?;
            output.push_str(&format!("\tidivl\t{op}\n"));
        },
        AsmInstruction::AllocateStack(int) => {
            output.push_str(&format!("\tsubq\t${int},\t%rsp\n"));
        },
        AsmInstruction::Cdq => {
            output.push_str("\tcdq\n");
        },
        AsmInstruction::Ret => {
            output.push_str("\tmovq\t%rbp,\t%rsp\n");
            output.push_str("\tpopq\t%rbp\n");
            output.push_str("\tret\n");
        },
    }
    Ok(())
}

fn emit_operand(operand: Operand) -> Result<String, EmissionError> {
    match operand {
        Operand::Imm(value) => Ok(format!("${value}")),
        Operand::Reg(reg) => {
            match reg {
                Register::AX => Ok(String::from("%eax")),
                Register::CX => Ok(String::from("%cl")),
                Register::DX => Ok(String::from("%edx")),
                Register::R10 => Ok(String::from("%r10d")),
                Register::R10b => Ok(String::from("%r10b")),
                Register::R11 => Ok(String::from("%r11d")),
            }
        },
        Operand::Stack(int) => Ok(format!("{int}(%rbp)")),
        Operand::Pseudo(ident) => Err(EmissionError::UnresolvedPseudoRegister(ident)),
    }
}

fn emit_unary_op(unary_op: UnaryOp) -> String {
    match unary_op {
        UnaryOp::Neg => String::from("negl"),
        UnaryOp::Not => String::from("notl"),
    }
}

fn emit_binary_op(binary_op: BinaryOp) -> String {
    match binary_op {
        BinaryOp::Add => String::from("addl"),
        BinaryOp::Sub => String::from("subl"),
        BinaryOp::Mult => String::from("imull"),
        BinaryOp::Sal => String::from("sall"),
        BinaryOp::Sar => String::from("sarl"),
        BinaryOp::BitAnd => String::from("andl"),
        BinaryOp::BitOr => String::from("orl"),
        BinaryOp::BitXor => String::from("xorl"),
    }
}
