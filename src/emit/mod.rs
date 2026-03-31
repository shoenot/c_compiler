use std::{collections::HashMap, fmt};
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

pub fn emit_program(program: AsmProgram, symbols: &mut SymbolTable)-> Result<String, EmissionError> {
    let mut output = String::new();
    for item in program.top_level {
        match item {
            AsmTopLevel::F(f) => emit_function(f, &mut output, symbols)?,
            AsmTopLevel::V(v) => emit_var(v, &mut output)?,
        }
    }
    output.push_str("\n.section .note.GNU-stack,\"\",@progbits");
    Ok(output)
}

fn emit_var(var: AsmStaticVar, output: &mut String) -> Result<(), EmissionError> {
    if var.global {
        output.push_str(&format!("\t.globl {}\n", var.identifier));
    }
    if var.init == 0 {
        output.push_str("\t.bss\n");
        output.push_str("\t.align 4\n");
        output.push_str(&format!("{}:\n", var.identifier));
        output.push_str("\t.zero 4\n");
    } else {
        output.push_str("\t.data\n");
        output.push_str("\t.align 4\n");
        output.push_str(&format!("{}:\n", var.identifier));
        output.push_str(&format!("\t.long {}\n", var.init));
    }
    Ok(())
}

fn emit_function(function: AsmFunction, output: &mut String, symbols: &mut HashMap<String, Symbol>) -> Result<(), EmissionError> {
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


fn emit_instruction(instruction: AsmInstruction, output: &mut String, symbols: &mut HashMap<String, Symbol>) -> Result<(), EmissionError> {
    match instruction {
        AsmInstruction::Mov(src, dst) => { let src = emit_operand(src)?;
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
        AsmInstruction::Cmp(operand1, operand2) => {
            let src = emit_operand(operand1)?;
            let dst = emit_operand(operand2)?;
            output.push_str(&format!("\tcmpl\t{src},\t{dst}\n"));
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
        AsmInstruction::Idiv(operand) => {
            let op = emit_operand(operand)?;
            output.push_str(&format!("\tidivl\t{op}\n"));
        },
        AsmInstruction::Cdq => {
            output.push_str("\tcdq\n");
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
            if let Some(sym) = symbols.get(&id) {
                if let IdentAttrs::FuncAttr { defined:_, global } = sym.attrs {
                    if !global {
                        name.push_str("@PLT");
                    }
                }
            }
            output.push_str(&format!("\tcall\t{name}\n"));
        },
        AsmInstruction::AllocateStack(int) => {
            output.push_str(&format!("\tsubq\t${int},\t%rsp\n"));
        },
        AsmInstruction::DeallocateStack(int) => {
            output.push_str(&format!("\taddq\t${int},\t%rsp\n"));
        },
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
    };
    format!("{first}{second}")
}

fn emit_operand(operand: Operand) -> Result<String, EmissionError> {
    match operand {
        Operand::Imm(value) => Ok(format!("${value}")),
        Operand::Reg(reg, regsize) => {
            let n = regsize as usize;
            let rstr = match reg {
                Register::AX => ["al", "eax", "rax"][n],
                Register::CX => ["cl", "ecx", "rcx"][n],
                Register::DX => ["dl", "edx", "rdx"][n],
                Register::DI => ["dil", "edi", "rdi"][n],
                Register::SI => ["sil", "esi", "rsi"][n],
                Register::R8 => ["r8b", "r8d", "r8"][n],
                Register::R9 => ["r9b", "r9d", "r9"][n],
                Register::R10 => ["r10b", "r10d", "r10"][n],
                Register::R11 => ["r11b", "r11d", "r11"][n],
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
        UnaryOp::Neg => "negl",
        UnaryOp::Not => "notl",
    }
}

fn emit_binary_op(binary_op: BinaryOp) -> &'static str {
    match binary_op {
        BinaryOp::Add => "addl",
        BinaryOp::Sub => "subl",
        BinaryOp::Mult => "imull",
        BinaryOp::Sal => "sall",
        BinaryOp::Sar => "sarl",
        BinaryOp::BitAnd => "andl",
        BinaryOp::BitOr => "orl",
        BinaryOp::BitXor => "xorl",
    }
}
