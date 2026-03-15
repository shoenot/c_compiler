use crate::parser;

#[derive(Debug)]
pub struct ProgramAsm {
    pub function: FunctionAsm,
}

#[derive(Debug)]
pub struct FunctionAsm {
    pub name: String,
    pub body: Vec<InstructionAsm>,
}

#[derive(Debug)]
pub enum InstructionAsm {
    Mov(Operand, Operand),
    Ret,
}

#[derive(Debug)]
pub enum Operand {
    Imm(i32),
    Reg(Register),
}

#[derive(Debug)]
pub enum Register {
    EAX,
}

pub fn gen_program(tree: parser::Program) -> ProgramAsm {
    let function = gen_function(tree.function);
    ProgramAsm { function }
}

fn gen_function(func: parser::Function) -> FunctionAsm {
    let name = func.identifier;
    let instructions = gen_instructions(func.body);
    FunctionAsm { name, body: instructions }
}

fn gen_instructions(statement: parser::Statement) -> Vec<InstructionAsm> {
    match statement {
        parser::Statement::Return(src) => {
            vec![InstructionAsm::Mov(gen_operand(src), Operand::Reg(Register::EAX)), InstructionAsm::Ret]
        }
    }
}

fn gen_operand(exp: parser::Expression) -> Operand {
    let expression = match exp {
        parser::Expression::Constant(val) => return Operand::Imm(val),
    };
}
