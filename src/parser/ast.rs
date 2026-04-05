use std::ops::{Deref, DerefMut};
use ordered_float::OrderedFloat;
use crate::types::Type;
use crate::lexer::Span;

#[derive(Debug)]
pub struct Program {
    pub declarations: Vec<Decl>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub items: Vec<BlockItem>
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockItem {
    S(Statement),
    D(Decl),
}

/// Declarations

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    VarDecl(VarDeclaration),
    FuncDecl(FuncDeclaration),
}

#[derive(Debug, Clone)]
pub struct FuncDeclaration {
    pub identifier: String,
    pub func_type: Type,
    pub params: Vec<String>,
    pub body: Option<Block>,
    pub storage: Option<StorageClass>,
    pub span: Span,
}

impl PartialEq for FuncDeclaration {
    fn eq(&self, other: &Self) -> bool {
        self.identifier == other.identifier &&
        self.params == other.params &&
        self.body == other.body &&
        self.storage == other.storage 
    }
}

#[derive(Debug, Clone)]
pub struct VarDeclaration {
    pub identifier: String,
    pub var_type: Type,
    pub init: Option<Expression>,
    pub storage: Option<StorageClass>,
    pub span: Span,
}

impl PartialEq for VarDeclaration {
    fn eq(&self, other: &Self) -> bool {
        self.identifier == other.identifier &&
        self.var_type == other.var_type &&
        self.init == other.init &&
        self.storage == other.storage 
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StorageClass {
    Static,
    Extern,
}

pub trait HasStorage {
    fn storage_class(&self) -> Option<StorageClass>;
}

impl HasStorage for VarDeclaration {
    fn storage_class(&self) -> Option<StorageClass> {
        self.storage.clone()
    }
}

impl HasStorage for FuncDeclaration {
    fn storage_class(&self) -> Option<StorageClass> {
        self.storage.clone()
    }
}

/// Statements

#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: StatementKind,
    pub span: Span,
}

impl PartialEq for Statement {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind 
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatementKind {
    Return(Expression),
    Expression(Expression),
    If(Expression, Box<Statement>, Option<Box<Statement>>), // Else statements not mandatory. 
    Compound(Block),
    Label(String, Box<Statement>), 
    Goto(String),
    While{cond: Expression, body: Box<Statement>, lab: String},
    DoWhile{body: Box<Statement>, cond: Expression, lab: String},
    For{init: ForInit, cond: Option<Expression>, post: Option<Expression>, body: Box<Statement>, lab: String},
    Switch{scrutinee: Expression, body: Box<Statement>, lab: String, cases:Vec<(Option<Expression>, String)>},
    Case{expr: Expression, lab: String},
    Default{lab: String},
    Break(String),
    Continue(String),
    Null,
}

impl Deref for Statement {
    type Target = StatementKind;

    fn deref(&self) -> &Self::Target {
        &self.kind
    }
}

impl DerefMut for Statement {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.kind
    }
}

impl Statement {
    pub fn new(kind: StatementKind, span: Span) -> Self {
        Statement{kind, span}
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ForInit {
    InitDec(VarDeclaration),
    InitExp(Option<Expression>),
}

/// Expressions

#[derive(Debug, Clone)]
pub struct Expression {
    pub kind: ExpressionKind,
    pub expression_type: Option<Type>,
    pub span: Span,
}

impl PartialEq for Expression {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind 
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionKind {
    Constant(Const),
    Var(String),
    Cast(Type, Box<Expression>),
    Assignment(Box<Expression>, Box<Expression>),
    Unary(UnaryOp, Box<Expression>),
    Binary(BinaryOp, Box<Expression>, Box<Expression>),
    Conditional(Box<Expression>, Box<Expression>, Box<Expression>),
    PrefixIncrement(Box<Expression>),
    PostfixIncrement(Box<Expression>),
    PrefixDecrement(Box<Expression>),
    PostfixDecrement(Box<Expression>),
    FunctionCall(String, Vec<Expression>),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Const {
    Int(i32),
    Long(i64),
    UInt(u32),
    ULong(u64),
    Double(OrderedFloat<f64>),
}

impl Deref for Expression {
    type Target = ExpressionKind;

    fn deref(&self) -> &Self::Target {
        &self.kind
    }
}

impl DerefMut for Expression {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.kind
    }
}

impl Expression {
    pub fn new(kind: ExpressionKind, expression_type: Option<Type>, span: Span) -> Self {
        Expression{kind, expression_type, span}
    }
}

/// Operators

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Complement,
    Negate,
    Not,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,
    LogicalAnd,
    LogicalOr,
    Equal,
    NotEqual,
    LessThan,
    LessOrEqual,
    GreaterThan,
    GreaterOrEqual,
    Set,
    OpSet(Box<BinaryOp>),
    Ternary,
}

