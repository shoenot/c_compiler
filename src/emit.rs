use crate::{
    codegen::{FunctionAsm, InstructionAsm, Operand, ProgramAsm, Register},
};

pub fn emit_program(program: ProgramAsm) -> String {
    let mut output = String::new();
    emit_function(program.function, &mut output);
    output.push_str("\n.section .note.GNU-stack,\"\",@progbits");
    output
}

fn emit_function(function: FunctionAsm, output: &mut String) {
    output.push_str(&format!("\t.globl {}\n{}:\n", function.name, function.name));
    for instruction in function.body {
       emit_instruction(instruction, output);
    }
}

fn emit_instruction(instruction: InstructionAsm, output: &mut String) {
    match instruction {
        InstructionAsm::Mov(src, dst) => {
            let src = emit_operand(src);
            let dst = emit_operand(dst);
            output.push_str(&format!("\tmov {src}, {dst}\n"));
        },
        InstructionAsm::Ret => {
            output.push_str("\tret\n")
        }
    }
}

fn emit_operand(operand: Operand) -> String {
    match operand {
        Operand::Imm(value) => format!("${value}"),
        Operand::Reg(reg) => {
            match reg {
                Register::EAX => String::from("%eax")
            }
        },
    }
}
