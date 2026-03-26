#[derive(Debug)]
pub struct Program {
    pub function: Function,
}

#[derive(Debug)]
pub struct Function {
    pub identifier: String,
    pub body: Block,
}

#[derive(Debug)]
pub struct Block {
    pub items: Vec<BlockItem>
}

#[derive(Debug)]
pub enum BlockItem {
    S(Statement),
    D(Declaration),
}

#[derive(Debug)]
pub struct Declaration {
    pub identifier: String,
    pub init: Option<Expression>,
}

// For loop initiator
#[derive(Debug)]
pub enum ForInit {
    InitDec(Declaration),
    InitExp(Option<Expression>),
}

#[derive(Debug)]
pub enum Statement {
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


#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Constant(i32),
    Var(String),
    Assignment(Box<Expression>, Box<Expression>),
    Unary(UnaryOp, Box<Expression>),
    Binary(BinaryOp, Box<Expression>, Box<Expression>),
    Conditional(Box<Expression>, Box<Expression>, Box<Expression>),
    PrefixIncrement(Box<Expression>),
    PostfixIncrement(Box<Expression>),
    PrefixDecrement(Box<Expression>),
    PostfixDecrement(Box<Expression>),
}

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
