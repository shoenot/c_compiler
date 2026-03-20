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
    Binary{op: PoiseBinaryOp, src1: PoiseVal, src2: PoiseVal, dst: PoiseVal},
    Copy{src: PoiseVal, dst:PoiseVal},
    Jump(String),
    JumpIfZero{condition: PoiseVal, identifier: String},
    JumpIfNotZero{condition: PoiseVal, identifier: String},
    Label(String)
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
    Not,
}

#[derive(Debug)]
pub enum PoiseBinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    LeftShift,
    RightShift,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    Equal, 
    NotEqual,
    LessThan,
    GreaterThan,
    LessOrEqual,
    GreaterOrEqual,
}

struct TmpCount {
    var_counter: usize,
    label_counter: usize,
}

impl TmpCount {
    fn new_var(&mut self) -> PoiseVal {
        let name = format!("tmp.{}", self.var_counter);
        self.var_counter += 1;
        PoiseVal::Variable(name)
    }

    fn new_label_string(&mut self) -> String {
        let name = format!("lab.{}", self.label_counter);
        self.label_counter += 1;
        name
    }   
}

pub fn gen_poise(tree: parser::Program) -> PoiseProg {
    let mut count = TmpCount{var_counter: 0, label_counter: 0};
    let function = gen_poisefunc(tree.function, &mut count);
    PoiseProg { function }
}

fn gen_poisefunc(func: parser::Function, count: &mut TmpCount) -> PoiseFunc {
    let name = func.identifier;
    let instructions = gen_instructions(func.body, count);
    PoiseFunc{ identifier: name, body: instructions }
}

fn gen_instructions(statement: parser::Statement, count: &mut TmpCount) -> Vec<PoiseInstruction> {
    let mut instructions = Vec::new();
    match statement {
        parser::Statement::Return(expression) => {
            let val = emit_expression(expression, &mut instructions, count);
            instructions.push(PoiseInstruction::Return(val));
        }
    }
    instructions
}

// Constructs IR instructions and returns the destination
fn emit_expression(
    expr: parser::Expression, 
    instructions: &mut Vec<PoiseInstruction>, 
    count: &mut TmpCount) -> PoiseVal {
    match expr {
        parser::Expression::Constant(val) => PoiseVal::Constant(val),
        parser::Expression::Unary(op, inner) => emit_un_exp(op, *inner, instructions, count),
        parser::Expression::Binary(op, exp1, exp2) => emit_bin_exp(op, *exp1, *exp2, instructions, count),
    }
}

fn emit_bin_exp(op: parser::BinaryOp, 
    exp1: parser::Expression, 
    exp2: parser::Expression,
    instructions: &mut Vec<PoiseInstruction>,
    count: &mut TmpCount) -> PoiseVal {
        let binop = match op {
            parser::BinaryOp::LogicalAnd | parser::BinaryOp::LogicalOr => {
                return emit_short_circuit_exp(op, exp1, exp2, instructions, count);
            },
            parser::BinaryOp::Add => PoiseBinaryOp::Add,
            parser::BinaryOp::Subtract => PoiseBinaryOp::Subtract,
            parser::BinaryOp::Multiply => PoiseBinaryOp::Multiply,
            parser::BinaryOp::Divide => PoiseBinaryOp::Divide,
            parser::BinaryOp::Remainder => PoiseBinaryOp::Remainder,
            parser::BinaryOp::LeftShift => PoiseBinaryOp::LeftShift,
            parser::BinaryOp::RightShift => PoiseBinaryOp::RightShift,
            parser::BinaryOp::BitwiseAnd => PoiseBinaryOp::BitwiseAnd,
            parser::BinaryOp::BitwiseOr => PoiseBinaryOp::BitwiseOr,
            parser::BinaryOp::BitwiseXor => PoiseBinaryOp::BitwiseXor,
            parser::BinaryOp::Equal => PoiseBinaryOp::Equal,
            parser::BinaryOp::NotEqual => PoiseBinaryOp::NotEqual,
            parser::BinaryOp::LessThan => PoiseBinaryOp::LessThan,
            parser::BinaryOp::GreaterThan => PoiseBinaryOp::GreaterThan,
            parser::BinaryOp::LessOrEqual => PoiseBinaryOp::LessOrEqual,
            parser::BinaryOp::GreaterOrEqual => PoiseBinaryOp::GreaterOrEqual,
            _ => todo!()
        };
        let v1 = emit_expression(exp1, instructions, count);
        let v2 = emit_expression(exp2, instructions, count);
        let dst = count.new_var();
        instructions.push(PoiseInstruction::Binary {op: binop, src1: v1, src2: v2, dst: dst.clone() });
        dst
}

fn emit_un_exp(op: parser::UnaryOp,
    exp: parser::Expression,
    instructions: &mut Vec<PoiseInstruction>,
    count: &mut TmpCount) -> PoiseVal {
        let src = emit_expression(exp, instructions, count);
        let dst = count.new_var();
        let unary_op = match op {
            parser::UnaryOp::Negate => PoiseUnaryOp::Negate,
            parser::UnaryOp::Complement => PoiseUnaryOp::Complement,
            parser::UnaryOp::Not => PoiseUnaryOp::Not,
            _ => todo!()
        };
        instructions.push(PoiseInstruction::Unary { op: unary_op, src, dst: dst.clone() });
        dst 
}

fn emit_short_circuit_exp(op: parser::BinaryOp, 
    exp1: parser::Expression, 
    exp2: parser::Expression,
    instructions: &mut Vec<PoiseInstruction>,
    count: &mut TmpCount) -> PoiseVal {

    let false_label = count.new_label_string();
    let true_label = count.new_label_string();
    let dst = count.new_var();
    
    match op {
        parser::BinaryOp::LogicalAnd => {
            let v1 = emit_expression(exp1, instructions, count);
            instructions.push(PoiseInstruction::Copy{src: v1.clone(), dst: dst.clone()});
            instructions.push(PoiseInstruction::JumpIfZero { condition: dst.clone(), identifier: false_label.clone() });

            let v2 = emit_expression(exp2, instructions, count);
            instructions.push(PoiseInstruction::Copy{src: v2.clone(), dst: dst.clone()});
            instructions.push(PoiseInstruction::JumpIfZero { condition: dst.clone(), identifier: false_label.clone() });

            instructions.push(PoiseInstruction::Copy{src: PoiseVal::Constant(1), dst: dst.clone()});
            instructions.push(PoiseInstruction::Jump(true_label.clone()));
            instructions.push(PoiseInstruction::Label(false_label));
            instructions.push(PoiseInstruction::Copy{src: PoiseVal::Constant(0), dst: dst.clone() });
            instructions.push(PoiseInstruction::Label(true_label));
            return dst
        },
        parser::BinaryOp::LogicalOr => {
            let v1 = emit_expression(exp1, instructions, count);
            instructions.push(PoiseInstruction::Copy{src: v1.clone(), dst: dst.clone()});
            instructions.push(PoiseInstruction::JumpIfNotZero { condition: dst.clone(), identifier: true_label.clone() });

            let v2 = emit_expression(exp2, instructions, count);
            instructions.push(PoiseInstruction::Copy{src: v2.clone(), dst: dst.clone()});
            instructions.push(PoiseInstruction::JumpIfNotZero { condition: dst.clone(), identifier: true_label.clone() });

            instructions.push(PoiseInstruction::Copy{src: PoiseVal::Constant(0), dst: dst.clone()});
            instructions.push(PoiseInstruction::Jump(false_label.clone()));
            instructions.push(PoiseInstruction::Label(true_label));
            instructions.push(PoiseInstruction::Copy{src: PoiseVal::Constant(1), dst: dst.clone() });
            instructions.push(PoiseInstruction::Label(false_label));
            return dst
        },
        _ => panic!(),
    }
}

