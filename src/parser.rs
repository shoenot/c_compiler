use std::iter::Peekable;
use std::vec::IntoIter;
use std::fmt;

use crate::lexer::*;

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken(TokenType, Span),
    UnexpectedEOF,
    ExpectedIdentifier(Span),
    ExpectedConstant(Span),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken(t, s) => write!(f, "Parse Error: unexpected token! {:#?}\nLine: {}, Col: {}", t, s.line_number, s.start_idx),
            ParseError::UnexpectedEOF => write!(f, "Parse Error: unexpected EOF!"),
            ParseError::ExpectedIdentifier(s) => write!(f, "Parse Error: expected identifier!\nLine: {}, Col: {}", s.line_number, s.start_idx),
            ParseError::ExpectedConstant(s) => write!(f, "Parse Error: expected constant!\nLine: {}, Col: {}", s.line_number, s.start_idx),
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
    pub body: Statement,
}

#[derive(Debug)]
pub enum Statement {
    Return(Expression)
}

#[derive(Debug)]
pub enum Expression {
    Constant(i32),
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
                start_idx: 0,
                end_idx: 0,
            },
        }
    }

    fn advance(& mut self) -> Result<Token, ParseError> {
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

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        let token = self.advance()?;
        match token.token_type {
            TokenType::Identifier(name) => Ok(name),
            _ => Err(ParseError::ExpectedIdentifier(self.current_span))
        }
    }

    fn expect_const(&mut self) -> Result<i32, ParseError> {
        let token = self.advance()?;
        match token.token_type {
            TokenType::Constant(value) => Ok(value),
            _ => Err(ParseError::ExpectedConstant(self.current_span))
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
        self.expect(TokenType::OpenBrace)?;
        let body = self.parse_statement()?;
        self.expect(TokenType::Semicolon)?;
        self.expect(TokenType::CloseBrace)?;
        Ok(Function {
            identifier, body
        })
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(TokenType::Return)?;
        let expression = self.parse_expression()?;
        Ok(Statement::Return(expression))
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        let constant = self.expect_const()?;
        Ok(Expression::Constant(constant))
    }

}

pub fn pretty_print(tree: Program) {
    println!("{:#?}", tree);
}
        
