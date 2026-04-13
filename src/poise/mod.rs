use crate::parser; 
use crate::types::*;
use crate::parser::Const;
use crate::semanal::type_checker::{
    get_static_init, 
    convert_constant,
};

#[derive(Debug)]
pub struct PoiseProg {
    pub top_level_items: Vec<TopLevelItem>,
}

#[derive(Debug)]
pub enum TopLevelItem {
    F(PoiseFunc),
    V(PoiseStaticVar),
}

#[derive(Debug)]
pub struct PoiseFunc {
    pub identifier: String, 
    pub global: bool, 
    pub params: Vec<String>, 
    pub body: Vec<PoiseInstruction>
}        


#[derive(Debug)]
pub struct PoiseStaticVar {
    pub identifier: String, 
    pub global: bool, 
    pub datatype: Type,
    pub init: StaticInit
}

#[derive(Debug, Clone)]
pub enum PoiseInstruction {
    Return(PoiseVal),
    SignExtend{src: PoiseVal, dst: PoiseVal},
    Truncate{src: PoiseVal, dst: PoiseVal},
    ZeroExtend{src: PoiseVal, dst: PoiseVal},
    DoubleToInt{src: PoiseVal, dst: PoiseVal},
    DoubleToUInt{src: PoiseVal, dst: PoiseVal},
    IntToDouble{src: PoiseVal, dst: PoiseVal},
    UIntToDouble{src: PoiseVal, dst: PoiseVal},
    Unary{op: PoiseUnaryOp, src: PoiseVal, dst: PoiseVal},
    Binary{op: PoiseBinaryOp, src1: PoiseVal, src2: PoiseVal, dst: PoiseVal},
    Copy{src: PoiseVal, dst:PoiseVal},
    GetAddress{src: PoiseVal, dst:PoiseVal},
    Load{src_ptr: PoiseVal, dst:PoiseVal},
    Store{src: PoiseVal, dst_ptr:PoiseVal},
    Jump(String),
    JumpIfZero{condition: PoiseVal, identifier: String},
    JumpIfNotZero{condition: PoiseVal, identifier: String},
    Label(String),
    FunctionCall{ident: String, args: Vec<PoiseVal>, dst: PoiseVal}
}

#[derive(Debug, Clone, PartialEq)]
pub enum PoiseVal {
    Constant(Const),
    Variable(String),
}

#[derive(Debug, Clone)]
pub enum PoiseUnaryOp {
    Complement,
    Negate,
    Not,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum ExpRes {
    Plain(PoiseVal),
    Deref(PoiseVal),
}

struct TmpCount {
    var_counter: usize,
    label_counter: usize,
}

impl TmpCount {
    fn new_var(&mut self, var_type: Type, symbols: &mut SymbolTable) -> PoiseVal {
        let name = format!("tmp.{}", self.var_counter);
        symbols.insert(name.clone(), Symbol { ident: name.clone(), datatype: var_type, attrs: IdentAttrs::LocalAttr });
        self.var_counter += 1;
        PoiseVal::Variable(name)
    }

    fn new_label_string(&mut self) -> String {
        let name = format!("lab.{}", self.label_counter);
        self.label_counter += 1;
        name
    }
}

fn get_type(expr: &parser::Expression) -> Type {
    expr.expression_type.as_ref().unwrap().clone()
}

fn loop_label_string(lab: String, labtype: &str) -> String {
    let ret = format!("{}_{}", labtype, lab);
    ret.into()
}

fn emit_type_conversion(
    src_val: PoiseVal,
    src_type: &Type,
    dst_type: &Type,
    instructions: &mut Vec<PoiseInstruction>,
    symbols: &mut SymbolTable,
    count: &mut TmpCount) -> PoiseVal {

    if src_type == dst_type {
        return src_val;
    }

    let dst_val = count.new_var(dst_type.clone(), symbols);

    if *src_type == Type::Double {
        if dst_type.is_signed() { instructions.push(PoiseInstruction::DoubleToInt { src: src_val, dst: dst_val.clone() }); } 
        else { instructions.push(PoiseInstruction::DoubleToUInt { src: src_val, dst: dst_val.clone() }); }
    } else if *dst_type == Type::Double {
        if src_type.is_signed() { instructions.push(PoiseInstruction::IntToDouble { src: src_val, dst: dst_val.clone() }); } 
        else { instructions.push(PoiseInstruction::UIntToDouble { src: src_val, dst: dst_val.clone() }); }
    } 
    else if src_type.size() == dst_type.size() { instructions.push(PoiseInstruction::Copy { src: src_val, dst: dst_val.clone() }); } 
    else if src_type.size() > dst_type.size() { instructions.push(PoiseInstruction::Truncate { src: src_val, dst: dst_val.clone() }); } 
    else if src_type.is_signed() { instructions.push(PoiseInstruction::SignExtend { src: src_val, dst: dst_val.clone() }); } 
    else { instructions.push(PoiseInstruction::ZeroExtend { src: src_val, dst: dst_val.clone() }); }

    dst_val
}

pub fn gen_poise(tree: &parser::Program, symbols: &mut SymbolTable) -> PoiseProg {
    let mut count = TmpCount{var_counter: 0, label_counter: 0};
    let mut top_level_items = Vec::new();
    for decl in &tree.declarations {
        match decl {
            parser::Decl::FuncDecl(f) => if f.body.is_some() {
                top_level_items.push(TopLevelItem::F(gen_poisefunc(f, symbols, &mut count)));
            },
            _ => {},
        }
    }
    top_level_items.extend(gen_static_symbols(symbols));
    PoiseProg { top_level_items }
}

fn gen_poisefunc(func: &parser::FuncDeclaration, symbols: &mut SymbolTable, count: &mut TmpCount) -> PoiseFunc {
    let mut instructions = Vec::new();
    let identifier = func.identifier.clone();
    let params = func.params.clone();
    gen_inst_block(func.body.as_ref().unwrap(), &mut instructions, symbols, count);
    instructions.push(PoiseInstruction::Return(PoiseVal::Constant(Const::Int(0))));
    let mut global = false;
    if let Some(sym) = symbols.get(&identifier) {
        global = sym.attrs.is_global()
    }
    PoiseFunc { identifier, global, params, body: instructions.to_vec() }
}

fn gen_inst_block(
    block: &parser::Block, 
    instructions: &mut Vec<PoiseInstruction>, 
    symbols: &mut SymbolTable,
    count: &mut TmpCount) {
    for blockitem in &block.items {
        match blockitem {
            parser::BlockItem::S(s) => gen_inst_statement(s, instructions, symbols, count),
            parser::BlockItem::D(parser::Decl::VarDecl(d)) => gen_inst_var_declaration(d, instructions, symbols, count),
            parser::BlockItem::D(parser::Decl::FuncDecl(_)) => {},
        }
    }
}

fn gen_inst_var_declaration(
    declaration: &parser::VarDeclaration, 
    instructions: &mut Vec<PoiseInstruction>, 
    symbols: &mut SymbolTable,
    count: &mut TmpCount) {
    if let Some(sym) = symbols.get(&declaration.identifier) {
        if matches!(sym.attrs, IdentAttrs::StaticAttr { .. }) {
            return;
        }
    }
    if let Some(exp) = declaration.init.as_ref() {
        let val = emit_converted_expr(exp, instructions, symbols, count);
        let s = get_type(exp);
        let d = symbols.get(&declaration.identifier).unwrap().datatype.clone();

        let conv = emit_type_conversion(val, &s, &d, instructions, symbols, count);
        instructions.push(PoiseInstruction::Copy { src: conv, dst: PoiseVal::Variable(declaration.identifier.clone()) });
    }
}

fn gen_inst_statement(
    statement: &parser::Statement, 
    instructions: &mut Vec<PoiseInstruction>, 
    symbols: &mut SymbolTable,
    count: &mut TmpCount) {
    match &statement.kind {
        parser::StatementKind::Return(expression) => {
            let val = emit_converted_expr(expression, instructions, symbols, count);
            instructions.push(PoiseInstruction::Return(val));
        }
        parser::StatementKind::Expression(expression) => {
            emit_converted_expr(expression, instructions, symbols, count);
        }
        parser::StatementKind::Null => return,
        parser::StatementKind::If(c, y, n) => {
            let cond = count.new_var(get_type(c), symbols);
            let eval = emit_converted_expr(c, instructions, symbols, count);
            let no_label = count.new_label_string();
            instructions.push(PoiseInstruction::Copy { src: eval, dst: cond.clone() });
            instructions.push(PoiseInstruction::JumpIfZero { condition: cond, identifier: no_label.clone() });
            gen_inst_statement(y, instructions, symbols, count);
            if let Some(n) = n {
                let yes_label = count.new_label_string();
                instructions.push(PoiseInstruction::Jump(yes_label.clone()));
                instructions.push(PoiseInstruction::Label(no_label));
                gen_inst_statement(n, instructions, symbols, count);
                instructions.push(PoiseInstruction::Label(yes_label));
            } else {
                instructions.push(PoiseInstruction::Label(no_label));
            }
        },
        parser::StatementKind::Label(name, body) => {
            instructions.push(PoiseInstruction::Label(name.clone()));
            gen_inst_statement(body, instructions, symbols, count);
        },
        parser::StatementKind::Goto(name) => instructions.push(PoiseInstruction::Jump(name.clone())),
        parser::StatementKind::Compound(block) => gen_inst_block(block, instructions, symbols, count),
        parser::StatementKind::Break(lab) => instructions.push(PoiseInstruction::Jump(loop_label_string(lab.clone(), "break"))),
        parser::StatementKind::Continue(lab) => instructions.push(PoiseInstruction::Jump(loop_label_string(lab.clone(), "cont"))),
        parser::StatementKind::DoWhile { body, cond, lab } => {
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "start")));
            gen_inst_statement(body, instructions, symbols, count);
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "cont")));
            let res = emit_converted_expr(cond, instructions, symbols, count);
            instructions.push(PoiseInstruction::JumpIfNotZero { condition: res, identifier: loop_label_string(lab.clone(), "start") });
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "break")));
            
        },
        parser::StatementKind::While { cond, body, lab }  => {
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "cont")));
            let res = emit_converted_expr(cond, instructions, symbols, count);
            instructions.push(PoiseInstruction::JumpIfZero { condition: res, identifier: loop_label_string(lab.clone(), "break") });
            gen_inst_statement(body, instructions, symbols, count);
            instructions.push(PoiseInstruction::Jump(loop_label_string(lab.clone(), "cont")));
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "break")));
            
        },
        parser::StatementKind::For { init, cond, post, body, lab } => {
            if let parser::ForInit::InitExp(Some(exp)) = init {
                emit_converted_expr(exp, instructions, symbols, count);
            } else if let parser::ForInit::InitDec(dec) = init {
                gen_inst_var_declaration(dec, instructions, symbols, count);
            }
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "start")));
            if let Some(cond) = cond {
                let res = emit_converted_expr(cond, instructions, symbols, count);
                instructions.push(PoiseInstruction::JumpIfZero { condition: res, identifier: loop_label_string(lab.clone(), "break") });
            }
            gen_inst_statement(body, instructions, symbols, count);
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "cont")));
            if let Some(post) = post {
                emit_converted_expr(post, instructions, symbols, count);
            }
            instructions.push(PoiseInstruction::Jump(loop_label_string(lab.clone(), "start")));
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "break")));
        },
        parser::StatementKind::Switch { scrutinee, body, lab, cases } => {
            let scr = emit_converted_expr(scrutinee, instructions, symbols, count);
            for case in cases.clone() {
                if let (Some(value), clab) = case {
                    let caseval = emit_converted_expr(&value, instructions, symbols, count);
                    let cmp = count.new_var(get_type(&value), symbols);
                    instructions.push(PoiseInstruction::Binary { op: PoiseBinaryOp::Equal, src1: caseval, src2: scr.clone(), dst: cmp.clone() });
                    instructions.push(PoiseInstruction::JumpIfNotZero { condition: cmp, identifier: loop_label_string(clab.clone(), "case") });
                } 
            }
            for case in cases {
                if let (None, clab) = case {
                    instructions.push(PoiseInstruction::Jump(loop_label_string(clab.clone(), "default")));
                }
            }
            instructions.push(PoiseInstruction::Jump(loop_label_string(lab.clone(), "break")));
            gen_inst_statement(body, instructions, symbols, count);
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "break")));
        },
        parser::StatementKind::Case { lab,.. } => {
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "case")));
        }, 
        parser::StatementKind::Default { lab } => {
            instructions.push(PoiseInstruction::Label(loop_label_string(lab.clone(), "default")));
        },
    }
}

fn prefix_op(expr: &parser::Expression, instructions: &mut Vec<PoiseInstruction>, symbols: &mut SymbolTable, count: &mut TmpCount) -> ExpRes {
    let (e, incr) = match &expr.kind {
        parser::ExpressionKind::PrefixIncrement(exp) => (exp, true),
        parser::ExpressionKind::PrefixDecrement(exp) => (exp, false),
        _ => unreachable!(),
    };
    let lval = emit_expr_result(e, instructions, symbols, count);
    match lval {
        ExpRes::Plain(v) => {
            instructions.push(PoiseInstruction::Binary{
                op: if incr { PoiseBinaryOp::Add } else { PoiseBinaryOp::Subtract },
                src1: v.clone(),
                src2: PoiseVal::Constant(convert_constant(Const::Int(1), get_type(e))),
                dst: v.clone(),
            });
            ExpRes::Plain(v)
        },
        ExpRes::Deref(ptr) => {
            let tmp = count.new_var(get_type(expr), symbols);
            instructions.push(PoiseInstruction::Load { src_ptr: ptr.clone(), dst: tmp.clone() });
            instructions.push(PoiseInstruction::Binary{
                op: if incr { PoiseBinaryOp::Add } else { PoiseBinaryOp::Subtract },
                src1: tmp.clone(),
                src2: PoiseVal::Constant(convert_constant(Const::Int(1), get_type(expr))),
                dst: tmp.clone(),
            });
            instructions.push(PoiseInstruction::Store { src: tmp.clone(), dst_ptr: ptr.clone() });
            ExpRes::Plain(tmp)
        }
    }
}

fn postfix_op(expr: &parser::Expression, instructions: &mut Vec<PoiseInstruction>, symbols: &mut SymbolTable, count: &mut TmpCount) -> ExpRes {
    let (e, incr) = match &expr.kind {
        parser::ExpressionKind::PostfixIncrement(exp) => (exp, true),
        parser::ExpressionKind::PostfixDecrement(exp) => (exp, false),
        _ => unreachable!(),
    };
    let lval = emit_expr_result(e, instructions, symbols, count);
    let orig = count.new_var(get_type(e), symbols);
    match lval {
        ExpRes::Plain(v) => {
            instructions.push(PoiseInstruction::Copy { src: v.clone(), dst: orig.clone() });
            instructions.push(PoiseInstruction::Binary{
                op: if incr { PoiseBinaryOp::Add } else { PoiseBinaryOp::Subtract },
                src1: v.clone(),
                src2: PoiseVal::Constant(convert_constant(Const::Int(1), get_type(e))),
                dst: v.clone(),
            });
            ExpRes::Plain(orig)
        },
        ExpRes::Deref(ptr) => {
            let tmp = count.new_var(get_type(expr), symbols);
            instructions.push(PoiseInstruction::Load { src_ptr: ptr.clone(), dst: tmp.clone() });
            instructions.push(PoiseInstruction::Copy { src: tmp.clone(), dst: orig.clone() });
            instructions.push(PoiseInstruction::Binary{
                op: if incr { PoiseBinaryOp::Add } else { PoiseBinaryOp::Subtract },
                src1: tmp.clone(),
                src2: PoiseVal::Constant(convert_constant(Const::Int(1), get_type(expr))),
                dst: tmp.clone(),
            });
            instructions.push(PoiseInstruction::Store { src: tmp.clone(), dst_ptr: ptr.clone() });
            ExpRes::Plain(orig)
        },
    }
}

// Constructs IR instructions and returns the destination
fn emit_expr_result(
    expr: &parser::Expression,
    instructions: &mut Vec<PoiseInstruction>,
    symbols: &mut SymbolTable,
    count: &mut TmpCount) -> ExpRes {
    match &expr.kind {
        parser::ExpressionKind::Constant(val) => ExpRes::Plain(PoiseVal::Constant(*val)),
        parser::ExpressionKind::Unary(op, inner) => ExpRes::Plain(emit_un_exp(op, inner, instructions, symbols, count, get_type(expr))),
        parser::ExpressionKind::Binary(op, exp1, exp2) => ExpRes::Plain(emit_bin_exp(op, exp1, exp2, instructions, symbols, count, get_type(expr))),
        parser::ExpressionKind::Var(name) => ExpRes::Plain(PoiseVal::Variable(name.clone())),
        parser::ExpressionKind::Assignment(lhs, rhs) => {
            let lval = emit_expr_result(lhs, instructions, symbols, count);
            let rval = emit_converted_expr(rhs, instructions, symbols, count);
            match lval {
                ExpRes::Plain(ref obj) => {
                    instructions.push(PoiseInstruction::Copy { src: rval.clone(), dst: obj.clone()});
                    lval
                }, 
                ExpRes::Deref(ptr) => {
                    instructions.push(PoiseInstruction::Store { src: rval.clone(), dst_ptr: ptr.clone()});
                    ExpRes::Plain(rval)
                },
            }
        },
        parser::ExpressionKind::CompoundAssignment(op, lhs, rhs, topt) => {
            let lval = emit_expr_result(lhs, instructions, symbols, count);
            let rval = emit_converted_expr(rhs, instructions, symbols, count);
            let ltype = get_type(lhs);
            let ctype = topt.clone().unwrap();
            match lval {
                ExpRes::Plain(ref obj) => {
                    let pre_op = emit_type_conversion(obj.clone(), &ltype, &ctype, instructions, symbols, count);
                    let op_res = count.new_var(ctype.clone(), symbols);
                    instructions.push(PoiseInstruction::Binary { op: get_bin_op(op), src1: pre_op, src2: rval, dst: op_res.clone() });
                    let post_op = emit_type_conversion(op_res, &ctype, &ltype, instructions, symbols, count);
                    instructions.push(PoiseInstruction::Copy { src: post_op, dst: obj.clone() });
                    lval
                },
                ExpRes::Deref(ref ptr) => {
                    let tmp = count.new_var(ltype.clone(), symbols);
                    instructions.push(PoiseInstruction::Load { src_ptr: ptr.clone(), dst: tmp.clone() });
                    let pre_op = emit_type_conversion(tmp.clone(), &ltype, &ctype, instructions, symbols, count);
                    let op_res = count.new_var(ctype.clone(), symbols);
                    instructions.push(PoiseInstruction::Binary { op: get_bin_op(op), src1: pre_op, src2: rval, dst: op_res.clone() });
                    let post_op = emit_type_conversion(op_res, &ctype, &ltype, instructions, symbols, count);
                    instructions.push(PoiseInstruction::Store { src: post_op, dst_ptr: ptr.clone() });
                    lval
                },
            }
        },
        parser::ExpressionKind::Conditional(c, y, n) => {
            let cond = count.new_var(get_type(c), symbols);
            let eval = emit_converted_expr(c, instructions, symbols, count);
            let no_label = count.new_label_string();
            let yes_label = count.new_label_string();
            let dest = count.new_var(get_type(expr), symbols);

            instructions.push(PoiseInstruction::Copy { src: eval, dst: cond.clone() });

            instructions.push(PoiseInstruction::JumpIfZero { condition: cond, identifier: no_label.clone() });
            let result = emit_converted_expr(y, instructions, symbols, count);
            instructions.push(PoiseInstruction::Copy { src: result, dst: dest.clone()});
            instructions.push(PoiseInstruction::Jump(yes_label.clone()));
            instructions.push(PoiseInstruction::Label(no_label));

            let result = emit_converted_expr(n, instructions, symbols, count);
            instructions.push(PoiseInstruction::Copy { src: result, dst: dest.clone()});
            instructions.push(PoiseInstruction::Label(yes_label));
            ExpRes::Plain(dest)
        },
        parser::ExpressionKind::PrefixIncrement(_) | parser::ExpressionKind::PrefixDecrement(_) => {
            prefix_op(expr, instructions, symbols, count)
        },
        parser::ExpressionKind::PostfixIncrement(_) | parser::ExpressionKind::PostfixDecrement(_) => {
            postfix_op(expr, instructions, symbols, count)
        },
        parser::ExpressionKind::FunctionCall(ident, args) => {
            let mut poiseargs = Vec::new();
            let dst = count.new_var(get_type(expr), symbols);
            for arg in args {
                let exp = emit_converted_expr(arg, instructions, symbols, count);
                let tmp = count.new_var(get_type(arg), symbols);
                instructions.push(PoiseInstruction::Copy { src: exp, dst: tmp.clone() });
                poiseargs.push(tmp);
            }
            instructions.push(PoiseInstruction::FunctionCall { ident: ident.clone(), args: poiseargs, dst: dst.clone() });
            ExpRes::Plain(dst)
        },
        parser::ExpressionKind::Cast(t, e) => {
            let var = emit_converted_expr(e, instructions, symbols, count);
            let conv = emit_type_conversion(var, &get_type(e), t, instructions, symbols, count);
            ExpRes::Plain(conv)
        },
        parser::ExpressionKind::Dereference(p) => {
            let res = emit_converted_expr(p, instructions, symbols, count);
            ExpRes::Deref(res)
        },
        parser::ExpressionKind::AddrOf(e) => {
            let val = emit_expr_result(e, instructions, symbols, count);
            match val {
                ExpRes::Plain(obj) => {
                    let dst = count.new_var(get_type(expr), symbols);
                    instructions.push(PoiseInstruction::GetAddress { src: obj, dst: dst.clone() });
                    ExpRes::Plain(dst)
                },
                ExpRes::Deref(ptr) => {
                    ExpRes::Plain(ptr)
                },
            }
        },
    }
}

fn emit_converted_expr(
    expr: &parser::Expression,
    instructions: &mut Vec<PoiseInstruction>,
    symbols: &mut SymbolTable,
    count: &mut TmpCount) -> PoiseVal {
    let res = emit_expr_result(expr, instructions, symbols, count);
    match res {
        ExpRes::Plain(val) => val,
        ExpRes::Deref(ptr) => {
            let dst = count.new_var(get_type(expr), symbols);
            instructions.push(PoiseInstruction::Load { src_ptr: ptr, dst: dst.clone() });
            dst
        }
    }
}

fn get_bin_op(op: &parser::BinaryOp) -> PoiseBinaryOp {
    match op {
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
        _ => unreachable!()
    }
}

fn emit_bin_exp(op: &parser::BinaryOp,
    exp1: &parser::Expression,
    exp2: &parser::Expression,
    instructions: &mut Vec<PoiseInstruction>,
    symbols: &mut SymbolTable,
    count: &mut TmpCount,
    dest_type: Type) -> PoiseVal {
        let binop = match op {
            parser::BinaryOp::LogicalOr | parser::BinaryOp::LogicalAnd => {
                return emit_short_circuit_exp(op, exp1, exp2, instructions, symbols, count);
            },
            _ => get_bin_op(op),
        };
        let v1 = emit_converted_expr(exp1, instructions, symbols, count);
        let v2 = emit_converted_expr(exp2, instructions, symbols, count);
        let dst = count.new_var(dest_type, symbols);
        instructions.push(PoiseInstruction::Binary {op: binop, src1: v1, src2: v2, dst: dst.clone() });
        dst
}

fn emit_un_exp(op: &parser::UnaryOp,
    exp: &parser::Expression,
    instructions: &mut Vec<PoiseInstruction>,
    symbols: &mut SymbolTable,
    count: &mut TmpCount,
    dest_type: Type) -> PoiseVal {
        let src = emit_converted_expr(exp, instructions, symbols, count);
        let dst = count.new_var(dest_type, symbols);
        let unary_op = match op {
            parser::UnaryOp::Negate => PoiseUnaryOp::Negate,
            parser::UnaryOp::Complement => PoiseUnaryOp::Complement,
            parser::UnaryOp::Not => PoiseUnaryOp::Not,
        };
        instructions.push(PoiseInstruction::Unary { op: unary_op, src, dst: dst.clone() });
        dst
}

fn emit_short_circuit_exp(op: &parser::BinaryOp,
    exp1: &parser::Expression,
    exp2: &parser::Expression,
    instructions: &mut Vec<PoiseInstruction>,
    symbols: &mut SymbolTable,
    count: &mut TmpCount) -> PoiseVal {

    let false_label = count.new_label_string();
    let true_label = count.new_label_string();
    let dst = count.new_var(Type::Int, symbols);

    match op {
        parser::BinaryOp::LogicalAnd => {
            let v1 = emit_converted_expr(exp1, instructions, symbols, count);
            instructions.push(PoiseInstruction::JumpIfZero { condition: v1.clone(), identifier: false_label.clone() });

            let v2 = emit_converted_expr(exp2, instructions, symbols, count);
            instructions.push(PoiseInstruction::JumpIfZero { condition: v2.clone(), identifier: false_label.clone() });

            instructions.push(PoiseInstruction::Copy{src: PoiseVal::Constant(Const::Int(1)), dst: dst.clone()});
            instructions.push(PoiseInstruction::Jump(true_label.clone()));
            instructions.push(PoiseInstruction::Label(false_label));
            instructions.push(PoiseInstruction::Copy{src: PoiseVal::Constant(Const::Int(0)), dst: dst.clone() });
            instructions.push(PoiseInstruction::Label(true_label));
            dst
        },
        parser::BinaryOp::LogicalOr => {
            let v1 = emit_converted_expr(exp1, instructions, symbols, count);
            instructions.push(PoiseInstruction::JumpIfNotZero { condition: v1.clone(), identifier: true_label.clone() });

            let v2 = emit_converted_expr(exp2, instructions, symbols, count);
            instructions.push(PoiseInstruction::JumpIfNotZero { condition: v2.clone(), identifier: true_label.clone() });

            instructions.push(PoiseInstruction::Copy{src: PoiseVal::Constant(Const::Int(0)), dst: dst.clone()});
            instructions.push(PoiseInstruction::Jump(false_label.clone()));
            instructions.push(PoiseInstruction::Label(true_label));
            instructions.push(PoiseInstruction::Copy{src: PoiseVal::Constant(Const::Int(1)), dst: dst.clone() });
            instructions.push(PoiseInstruction::Label(false_label));
            dst
        },
        _ => panic!(),
    }
}

fn gen_static_symbols(symbols: &SymbolTable) -> Vec<TopLevelItem> {
    let mut defs = Vec::new();
    for (k, v) in symbols {
        let var_type = &v.datatype;
        if let IdentAttrs::StaticAttr { init, global } = &v.attrs {
            match init {
                InitialValue::Initial(v) => {
                    defs.push(TopLevelItem::V(PoiseStaticVar { 
                        identifier: k.clone(), 
                        datatype: var_type.clone(),
                        global:*global, 
                        init: v.clone(), 
                    }));
                },
                InitialValue::Tentative => {
                    let val = convert_constant(Const::Int(0), var_type.clone());
                    defs.push(TopLevelItem::V(PoiseStaticVar { 
                        identifier: k.clone(), 
                        datatype: var_type.clone(),
                        global:*global, 
                        init: get_static_init(val) 
                    }));
                },
                InitialValue::NoInitializer => {}
            }
        }
    }
    defs
}
