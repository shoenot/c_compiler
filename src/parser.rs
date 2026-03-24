use std::iter::Peekable;
use std::vec::IntoIter;
use std::fmt;

use crate::lexer::*; use crate::tokens::TokenType;

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken(TokenType, Span),
    UnexpectedEOF,
    Unimplemented(Span),
    ExpectedIdentifier(Span),
    ExpectedExpression(Span),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken(t, s) => write!(f, "Parse Error: unexpected token! {:#?}\nLine: {}, Col: {}", t, s.line_number, s.col),
            ParseError::Unimplemented(s) => write!(f, "This operation is not implemented yet!\nLine: {}, Col: {}", s.line_number, s.col),
            ParseError::UnexpectedEOF => write!(f, "Parse Error: unexpected EOF!"),
            ParseError::ExpectedIdentifier(s) => write!(f, "Parse Error: expected identifier!\nLine: {}, Col: {}", s.line_number, s.col),
            ParseError::ExpectedExpression(s) => write!(f, "Parse Error: expected expression!\nLine: {}, Col: {}", s.line_number, s.col),
        }
    }
}

impl std::error::Error for ParseError { }

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

#[derive(Debug)]
pub enum Statement {
    Return(Expression),
    Expression(Expression),
    If(Expression, Box<Statement>, Option<Box<Statement>>), // Else statements not mandatory. 
    Compound(Block),
    Label(String), 
    Goto(String),
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

#[derive(Debug)]
pub struct Parser {
    tokens: Peekable<IntoIter<Token>>,
    current_span: Span,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Parser {
        Parser {
            tokens: tokens.into_iter().peekable(),
            current_span: Span {
                line_number: 0,
                col: 0,
            },
        }
    }

    fn advance(&mut self) -> Result<Token, ParseError> {
        match self.tokens.next() {
            None => Err(ParseError::UnexpectedEOF),
            Some(token) => {
                self.current_span = token.location;
                return Ok(token);
            }
        }
    }

    fn expect(&mut self, expected: TokenType) -> Result<Token, ParseError> {
        let token = self.advance()?;
        if token.token_type == expected {
            Ok(token)
        } else {
            Err(ParseError::UnexpectedToken(token.token_type, token.location))
        }
    }

    fn peek(&mut self) -> Result<&Token, ParseError> {
        self.tokens.peek().ok_or(ParseError::UnexpectedEOF)
    }

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        let token = self.advance()?;
        match token.token_type {
            TokenType::Identifier(name) => Ok(name),
            _ => Err(ParseError::ExpectedIdentifier(self.current_span))
        }
    }

    fn expect_eof(&mut self) -> Result<(), ParseError> {
        let eof = self.tokens.peek();
        match eof {
            None => Ok(()),
            Some(token) => Err(ParseError::UnexpectedToken(token.token_type.clone(), token.location)),
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let function = self.parse_function()?;
        self.expect_eof()?;
        Ok(Program {
            function
        })
    }

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        self.expect(TokenType::Int)?;
        let identifier = self.expect_ident()?;
        self.expect(TokenType::OpenParen)?;
        self.expect(TokenType::Void)?;
        self.expect(TokenType::CloseParen)?;
        let body = self.parse_block()?;
        Ok(Function {
            identifier, body
        })
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let mut blockitems = Vec::new();
        self.expect(TokenType::OpenBrace)?;
        while self.peek()?.token_type != TokenType::CloseBrace {
            blockitems.push(self.parse_blockitem()?);
        }
        self.expect(TokenType::CloseBrace)?;
        Ok(Block{ items: blockitems })
    }

    fn parse_blockitem(&mut self) -> Result<BlockItem, ParseError> {
        let item = match self.peek()?.token_type {
            TokenType::Int => BlockItem::D(self.parse_declaration()?),
            _ => BlockItem::S(self.parse_statement()?),
        };
        Ok(item)
    }

    fn parse_declaration(&mut self) -> Result<Declaration, ParseError> {
        self.expect(TokenType::Int)?;
        let identifier = self.expect_ident()?;
        let mut init = None;
        if self.peek()?.token_type != TokenType::Semicolon {
            self.expect(TokenType::Equal)?;
            init = Some(self.parse_expression(0)?);
        }
        self.expect(TokenType::Semicolon)?;
        Ok(Declaration{identifier, init})
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        let statement = match self.peek()?.token_type.clone() {
            TokenType::Semicolon => { 
                let ret = Statement::Null;
                self.expect(TokenType::Semicolon)?;
                ret
            },
            TokenType::Return => {
                self.advance()?;
                let ret = Statement::Return(self.parse_expression(0)?);
                self.expect(TokenType::Semicolon)?;
                ret 
            },
            TokenType::If => {
                self.advance()?;
                self.expect(TokenType::OpenParen)?;
                let cond = self.parse_expression(0)?;
                self.expect(TokenType::CloseParen)?;
                let yes = self.parse_statement()?;
                if self.peek()?.token_type == TokenType::Else { 
                    self.advance()?;
                    let no = self.parse_statement()?;
                    Statement::If(cond, Box::new(yes), Some(Box::new(no)))
                } else {
                    Statement::If(cond, Box::new(yes), None)
                }
            },
            TokenType::Identifier(name) => {
                self.advance()?;
                if self.peek()?.token_type == TokenType::Colon {
                    let ret = Statement::Label(name);
                    self.expect(TokenType::Colon)?;
                    ret
                } else {
                    let mut expr = Expression::Var(name);
                    let expr = self.check_postfix(expr)?;
                    let expr = self.parse_expression_cont(expr, 0)?;
                    self.expect(TokenType::Semicolon)?;
                    Statement::Expression(expr)
                }
            },
            TokenType::Goto => {
                self.advance()?;
                let token = self.advance()?;
                match token.token_type {
                    TokenType::Identifier(name) => {
                        self.expect(TokenType::Semicolon)?;
                        Statement::Goto(name)
                    },
                    _ => return Err(ParseError::ExpectedIdentifier(self.current_span))
                }
            },
            TokenType::OpenBrace => {
                let block = self.parse_block()?;
                Statement::Compound(block)
            },
            _ => {
                let ret = Statement::Expression(self.parse_expression(0)?);
                self.expect(TokenType::Semicolon)?;
                ret
            },
        };
        Ok(statement)
    }

    fn parse_expression(&mut self, min_prec: i32) -> Result<Expression, ParseError> {
        let left = self.parse_factor()?;
        self.parse_expression_cont(left, min_prec)
    }

    fn parse_expression_cont(&mut self, mut left: Expression, min_prec: i32) -> Result<Expression, ParseError> {
        loop {
            let Some(op) = self.peek_binop()? else { break };
            if self.precedence(&op) < min_prec { break }
            self.advance()?;
            let prec = self.precedence(&op);
            match op {
                BinaryOp::Set => {
                    let right = self.parse_expression(prec)?;
                    left = Expression::Assignment(Box::new(left), Box::new(right));
                }
                BinaryOp::OpSet(op) => {
                    let right = self.parse_expression(prec)?;
                    let binary = Expression::Binary(*op, Box::new(left.clone()), Box::new(right));
                    left = Expression::Assignment(Box::new(left), Box::new(binary));
                }
                BinaryOp::Ternary => {
                    let middle = self.parse_conditional_middle()?;
                    let right = self.parse_expression(prec)?;
                    left = Expression::Conditional(Box::new(left), Box::new(middle), Box::new(right));
                },
                _ => {
                    let right = self.parse_expression(prec + 1)?;
                    left = Expression::Binary(op, Box::new(left), Box::new(right));
                }
            }
        }
        Ok(left)
    }


    fn parse_conditional_middle(&mut self) -> Result<Expression, ParseError> {
        let exp = self.parse_expression(0)?;
        self.expect(TokenType::Colon)?;
        Ok(exp)
    }

    fn parse_factor(&mut self) -> Result<Expression, ParseError> {
        let token = self.advance()?;
        let mut expr = match token.token_type {
            TokenType::Constant(value) => Expression::Constant(value),
            TokenType::OpenParen => {
                let expression = self.parse_expression(0)?;
                self.expect(TokenType::CloseParen)?;
                expression
            },
            TokenType::Exclamation => self.parse_unop(UnaryOp::Not)?,
            TokenType::Tilde => self.parse_unop(UnaryOp::Complement)?,
            TokenType::Minus => self.parse_unop(UnaryOp::Negate)?,
            TokenType::Identifier(name) => Expression::Var(name),
            TokenType::DoublePlus => {
                let operand = self.parse_factor()?;
                Expression::PrefixIncrement(Box::new(operand))
            },
            TokenType::DoubleMinus => {
                let operand = self.parse_factor()?;
                Expression::PrefixDecrement(Box::new(operand))
            },
            _ => {
                eprintln!("{:#?}", token);
                return Err(ParseError::ExpectedExpression(self.current_span))
            }
        };
        let expr = self.check_postfix(expr)?;
        Ok(expr)
    }

    fn check_postfix(&mut self, expr: Expression) -> Result<Expression, ParseError> {
        let mut expr = expr;
        loop {
            match self.peek()?.token_type {
                TokenType::DoublePlus => {
                    self.advance()?;
                    expr = Expression::PostfixIncrement(Box::new(expr.clone()));
                },
                TokenType::DoubleMinus => {
                    self.advance()?;
                    expr = Expression::PostfixDecrement(Box::new(expr.clone()));
                },
                _ => return Ok(expr),
            }
        }
    }

    fn parse_unop(&mut self, op: UnaryOp) -> Result<Expression, ParseError> {
        let operand = self.parse_factor()?;
        Ok(Expression::Unary(op, Box::new(operand)))
    }

    fn peek_binop(&mut self) -> Result<Option<BinaryOp>, ParseError> {
        match self.peek()?.token_type {
            TokenType::Plus => Ok(Some(BinaryOp::Add)),
            TokenType::Minus => Ok(Some(BinaryOp::Subtract)),
            TokenType::Asterisk => Ok(Some(BinaryOp::Multiply)),
            TokenType::FwdSlash => Ok(Some(BinaryOp::Divide)),
            TokenType::Percent => Ok(Some(BinaryOp::Remainder)),
            TokenType::DoubleLeftAngled => Ok(Some(BinaryOp::LeftShift)),
            TokenType::DoubleRightAngled => Ok(Some(BinaryOp::RightShift)),
            TokenType::Ampersand => Ok(Some(BinaryOp::BitwiseAnd)),
            TokenType::Pipe => Ok(Some(BinaryOp::BitwiseOr)),
            TokenType::Caret => Ok(Some(BinaryOp::BitwiseXor)),
            TokenType::DoubleAmpersand => Ok(Some(BinaryOp::LogicalAnd)),
            TokenType::DoublePipe => Ok(Some(BinaryOp::LogicalOr)),
            TokenType::DoubleEqual => Ok(Some(BinaryOp::Equal)),
            TokenType::NotEqual => Ok(Some(BinaryOp::NotEqual)),
            TokenType::LessThan => Ok(Some(BinaryOp::LessThan)),
            TokenType::LessOrEqual => Ok(Some(BinaryOp::LessOrEqual)),
            TokenType::GreaterThan => Ok(Some(BinaryOp::GreaterThan)),
            TokenType::GreaterOrEqual => Ok(Some(BinaryOp::GreaterOrEqual)),
            TokenType::Equal => Ok(Some(BinaryOp::Set)),
            TokenType::PlusEqual => Ok(Some(BinaryOp::OpSet(Box::new(BinaryOp::Add)))),
            TokenType::MinusEqual => Ok(Some(BinaryOp::OpSet(Box::new(BinaryOp::Subtract)))),
            TokenType::AsteriskEqual => Ok(Some(BinaryOp::OpSet(Box::new(BinaryOp::Multiply)))),
            TokenType::FwdSlashEqual => Ok(Some(BinaryOp::OpSet(Box::new(BinaryOp::Divide)))),
            TokenType::PercentEqual => Ok(Some(BinaryOp::OpSet(Box::new(BinaryOp::Remainder)))),
            TokenType::AmpersandEqual => Ok(Some(BinaryOp::OpSet(Box::new(BinaryOp::BitwiseAnd)))),
            TokenType::PipeEqual => Ok(Some(BinaryOp::OpSet(Box::new(BinaryOp::BitwiseOr)))),
            TokenType::CaretEqual => Ok(Some(BinaryOp::OpSet(Box::new(BinaryOp::BitwiseXor)))),
            TokenType::DLAngledEqual => Ok(Some(BinaryOp::OpSet(Box::new(BinaryOp::LeftShift)))),
            TokenType::DRAngledEqual => Ok(Some(BinaryOp::OpSet(Box::new(BinaryOp::RightShift)))),
            TokenType::QuestionMark => Ok(Some(BinaryOp::Ternary)),
            _ => Ok(None),
        }
    }

    fn precedence(&mut self, op: &BinaryOp) -> i32 {
        match op {
            BinaryOp::Multiply       => 50,
            BinaryOp::Divide         => 50,
            BinaryOp::Remainder      => 50,
            BinaryOp::Add            => 45,
            BinaryOp::Subtract       => 45,
            BinaryOp::LeftShift      => 42,
            BinaryOp::RightShift     => 42,
            BinaryOp::LessThan       => 35,
            BinaryOp::LessOrEqual    => 35,
            BinaryOp::GreaterThan    => 35,
            BinaryOp::GreaterOrEqual => 35,
            BinaryOp::Equal          => 30,
            BinaryOp::NotEqual       => 30,
            BinaryOp::BitwiseAnd     => 28,
            BinaryOp::BitwiseXor     => 26,
            BinaryOp::BitwiseOr      => 24,
            BinaryOp::LogicalAnd     => 10,
            BinaryOp::LogicalOr      => 5,
            BinaryOp::Ternary        => 3,
            BinaryOp::Set            => 1,
            BinaryOp::OpSet(_)       => 1,
        }
    }
}

pub fn pretty_print(tree: Program) {
    println!("{:#?}", tree);
}
