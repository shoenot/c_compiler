use crate::parser;

#[derive(Debug)]
pub struct PoiseProg {
    pub function: PoiseFunc,
}

#[derive(Debug)]
pub struct PoiseFunc {
    pub identifier: String,
    pub body: Vec<PoiseInstruction>
}

#[derive(Debug)]
pub enum PoiseInstruction {
    Return(PoiseVal),
    Unary{op: PoiseUnaryOp, src: PoiseVal, dst: PoiseVal},
}

#[derive(Debug, Clone)]
pub enum PoiseVal {
    Constant(i32),
    Variable(String),
}

#[derive(Debug)]
pub enum PoiseUnaryOp {
    Complement,
    Negate,
}

struct PoiseCount {
    counter: usize,
}

impl PoiseCount {
    fn newtemp(&mut self) -> PoiseVal {
        let name = format!("tmp.{}", self.counter);
        self.counter += 1;
        PoiseVal::Variable(name)
    }
}

pub fn gen_poise(tree: parser::Program) -> PoiseProg {
    let mut count = PoiseCount{counter: 0};
    let function = gen_poisefunc(tree.function, &mut count);
    PoiseProg { function }
}

fn gen_poisefunc(func: parser::Function, count: &mut PoiseCount) -> PoiseFunc {
    let name = func.identifier;
    let instructions = gen_instructions(func.body, count);
    PoiseFunc{ identifier: name, body: instructions }
}

fn gen_instructions(statement: parser::Statement, count: &mut PoiseCount) -> Vec<PoiseInstruction> {
    let mut instructions = Vec::new();
    match statement {
        parser::Statement::Return(expression) => {
            let val = emit_expression(expression, &mut instructions, count);
            instructions.push(PoiseInstruction::Return(val));
        }
    }
    instructions
}

fn emit_expression(
    expr: parser::Expression, 
    instructions: &mut Vec<PoiseInstruction>, 
    count: &mut PoiseCount) -> PoiseVal {
    match expr {
        parser::Expression::Constant(val) => PoiseVal::Constant(val),
        parser::Expression::Unary(op, inner) => {
            let src = emit_expression(*inner, instructions, count);
            let dst = count.newtemp();
            let unary_op = match op {
                parser::UnaryOp::Negate => PoiseUnaryOp::Negate,
                parser::UnaryOp::Complement => PoiseUnaryOp::Complement,
            };
            instructions.push(PoiseInstruction::Unary { op: unary_op, src, dst: dst.clone() });
            dst
        }
    }
}
