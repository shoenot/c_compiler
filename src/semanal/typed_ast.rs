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
    Return(TypedExpr),
    TypedExpr(TypedExpr),
    If(TypedExpr, Box<Statement>, Option<Box<Statement>>), // Else statements not mandatory. 
    Compound(Block),
    Label(String, Box<Statement>), 
    Goto(String),
    While{cond: TypedExpr, body: Box<Statement>, lab: String},
    DoWhile{body: Box<Statement>, cond: TypedExpr, lab: String},
    For{init: ForInit, cond: Option<TypedExpr>, post: Option<TypedExpr>, body: Box<Statement>, lab: String},
    Switch{scrutinee: TypedExpr, body: Box<Statement>, lab: String, cases:Vec<(Option<TypedExpr>, String)>},
    Case{expr: TypedExpr, lab: String},
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
    InitExp(Option<TypedExpr>),
}

/// Expressions

#[derive(Debug, Clone)]
pub struct TypedExpr {
    pub kind: TypedExprKind,
    pub expr_type: Type,
    pub span: Span,
}

impl PartialEq for TypedExpr {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind 
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedExprKind {
    Constant(Const),
    Var(String),
    Cast(Type, Box<TypedExpr>),
    Assignment(Box<TypedExpr>, Box<TypedExpr>),
    Unary(TypedUnOp, Box<TypedExpr>),
    Binary(TypedBinOp, Box<TypedExpr>, Box<TypedExpr>),
    Conditional(Box<TypedExpr>, Box<TypedExpr>, Box<TypedExpr>),
    FunctionCall(String, Vec<TypedExpr>),
    Dereference(Box<TypedExpr>),
    AddrOf(Box<TypedExpr>),
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
    type Target = TypedExprKind;

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
    pub fn new(kind: ExpressionKind, span: Span) -> Self {
        Expression{kind, span}
    }
}

/// Operators

#[derive(Debug, Clone, PartialEq)]
pub enum TypedUnOp {
    Complement,
    Negate,
    Not,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TypedBinOp {
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
}

