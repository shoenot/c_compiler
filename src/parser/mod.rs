use std::iter::Peekable;
use std::vec::IntoIter;
use std::fmt;
use crate::types::*;

use crate::lexer::*; 

mod ast;
pub use ast::*;
mod expr;
pub use expr::*;
mod stmt;
pub use stmt::*;

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken(TokenType, Span),
    UnexpectedEOF,
    ExpectedStatement(Span),
    ExpectedIdentifier(Span),
    ExpectedExpression(Span),
    ExpectedVarDecl(Span),
    ExpectedParam(Span),
    LabelWithoutStatement(Span),
    InvalidTypes(Span),
    InvalidStorageClasses(Span),
    IntegerOverflow(Span),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken(t, s) => write!(f, "Parse Error: unexpected token! {:#?}\nLine: {}, Col: {}", t, s.line_number, s.col),
            ParseError::UnexpectedEOF => write!(f, "Parse Error: unexpected EOF!"),
            ParseError::ExpectedStatement(s) => write!(f, "Parse Error: expected statement!\nLine: {}, Col: {}", s.line_number, s.col),
            ParseError::ExpectedIdentifier(s) => write!(f, "Parse Error: expected identifier!\nLine: {}, Col: {}", s.line_number, s.col),
            ParseError::ExpectedExpression(s) => write!(f, "Parse Error: expected expression!\nLine: {}, Col: {}", s.line_number, s.col),
            ParseError::ExpectedVarDecl(s) => write!(f, "Parse Error: expected variable declaration!\nLine: {}, Col: {}", s.line_number, s.col),
            ParseError::ExpectedParam(s) => write!(f, "Parse Error: expected parameter!\nLine: {}, Col: {}", s.line_number, s.col),
            ParseError::LabelWithoutStatement(s) => write!(f, "Parse Error: label without statement!\nLine: {}, Col: {}", s.line_number, s.col),
            ParseError::InvalidTypes(s) => write!(f, "Parse Error: invalid types!\nLine: {}, Col: {}", s.line_number, s.col),
            ParseError::InvalidStorageClasses(s) => write!(f, "Parse Error: invalid storage classes!\nLine: {}, Col: {}", s.line_number, s.col),
            ParseError::IntegerOverflow(s) => write!(f, "Parse Error: integer overflow!\nLine: {}, Col: {}", s.line_number, s.col),
        }
    }
}

impl std::error::Error for ParseError { }

#[derive(Debug)]
pub struct Parser {
    tokens: Peekable<IntoIter<Token>>,
    current_span: Span,
}

#[derive(Debug)]
struct TypeFlags {
    saw_int: bool,
    saw_long: bool,
    saw_unsigned: bool,
    saw_signed: bool,
}

static SPECIFIERS: &[TokenType] = &[TokenType::Static, TokenType::Extern, TokenType::Int, TokenType::Long,
                                    TokenType::Unsigned, TokenType::Signed];

static TYPE_SPECIFIERS: &[TokenType] = &[TokenType::Int, TokenType::Long,
                                         TokenType::Unsigned, TokenType::Signed];

fn flip_or_err(flag: &mut bool, span: Span) -> Result<(), ParseError> { 
    if *flag {
        Err(ParseError::InvalidTypes(span))
    } else {
        *flag = true;
        Ok(())
    }
}

impl TypeFlags {
    fn new() -> Self {
        TypeFlags { 
            saw_int: false,
            saw_long: false,
            saw_unsigned: false,
            saw_signed: false,
        }
    }

    fn set_flag(&mut self, dtype: &TokenType, span: &Span) -> Result<(), ParseError> {
        match dtype {
            TokenType::Int => flip_or_err(&mut self.saw_int, span.clone())?,
            TokenType::Long => flip_or_err(&mut self.saw_long, span.clone())?,
            TokenType::Unsigned => flip_or_err(&mut self.saw_unsigned, span.clone())?,
            TokenType::Signed => flip_or_err(&mut self.saw_signed, span.clone())?,
            _ => return Err(ParseError::InvalidTypes(span.clone())),
        }
        Ok(())
    }

    fn get_type(&self, span: &Span) -> Result<Type, ParseError> {
        if self.saw_signed && self.saw_unsigned {
            return Err(ParseError::InvalidTypes(*span));
        }
        match self {
            TypeFlags { saw_int:true, saw_long:false, saw_unsigned: false, .. } => Ok(Type::Int),
            TypeFlags { saw_int:false, saw_long:true, saw_unsigned: false, .. } => Ok(Type::Long),
            TypeFlags { saw_int:true, saw_long:true, saw_unsigned: false, .. } => Ok(Type::Long),
            TypeFlags { saw_int:true, saw_long:false, saw_unsigned: true, .. } => Ok(Type::UInt),
            TypeFlags { saw_int:false, saw_long:true, saw_unsigned: true, .. } => Ok(Type::ULong),
            TypeFlags { saw_int:true, saw_long:true, saw_unsigned: true, .. } => Ok(Type::ULong),
            TypeFlags { saw_int:false, saw_long:false, saw_unsigned: true, .. } => Ok(Type::UInt),
            TypeFlags { saw_int:false, saw_long:false, saw_unsigned: false, saw_signed: true } => Ok(Type::Int),
            TypeFlags { saw_int:false, saw_long:false, saw_unsigned: false, .. } => Err(ParseError::InvalidTypes(span.clone())),
        }
    }
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

    fn peek(&mut self) -> Option<&Token> {
        self.tokens.peek()
    }

    fn next_token_is(&mut self, tokentype: TokenType) -> bool {
        self.peek().map_or(false, |token| token.token_type == tokentype)
    }

    fn next_token_type(&mut self) -> Result<TokenType, ParseError> {
        if let Some(token) = self.peek() {
            Ok(token.token_type.clone())
        } else {
            Err(ParseError::UnexpectedEOF)
        }
    }

    fn next_token_is_specifier(&mut self) -> bool {
        self.peek().map_or(false, |token| {
            SPECIFIERS.contains(&token.token_type)
        })
    }

    fn next_token_is_type(&mut self) -> bool {
        self.peek().map_or(false, |token| {
            TYPE_SPECIFIERS.contains(&token.token_type)
        })
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

    fn new_expr(&self, kind: ExpressionKind) -> Expression {
        Expression::new(kind, None, self.current_span)
    }

    fn new_stmt(&self, kind: StatementKind) -> Statement {
        Statement::new(kind, self.current_span)
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut declarations = Vec::new();
        while self.next_token_is_specifier() {
            declarations.push(self.parse_declaration()?);
        }
        self.expect_eof()?;
        Ok(Program { declarations })
    } 

    ////////////////////
    /// DECLARATIONS ///
    ////////////////////

    pub fn parse_declaration(&mut self) -> Result<Decl, ParseError> {
        let (dtype, storage) = self.parse_specifiers()?;
        let identifier = self.expect_ident()?;
        let decl = match self.next_token_type()? {
            TokenType::OpenParen => Decl::FuncDecl(self.parse_func_declaration(identifier, dtype, storage)?),
            _ => Decl::VarDecl(self.parse_var_declaration(identifier, dtype, storage)?),
        };
        Ok(decl)
    }

    fn parse_types(&mut self, types: &Vec<TokenType>) -> Result<Type, ParseError> {
        let mut flags = TypeFlags::new();
        for item in types {
            flags.set_flag(&item, &self.current_span)?;
        }
        flags.get_type(&self.current_span)
    }

    fn parse_specifiers(&mut self) -> Result<(Type, Option<StorageClass>), ParseError> {
        let mut storage = None;
        let mut types = Vec::new();
        loop {
            if let TokenType::Identifier(_) = self.next_token_type()? {
                break;
            } else {
                let token = self.advance()?.token_type;
                match token {
                    tk if TYPE_SPECIFIERS.contains(&token) => types.push(tk),
                    TokenType::Static => { 
                        if storage.is_none() { storage = Some(StorageClass::Static) } else 
                        { return Err(ParseError::InvalidStorageClasses(self.current_span)) }
                    },
                    TokenType::Extern => { 
                        if storage.is_none() { storage = Some(StorageClass::Extern) } else 
                        { return Err(ParseError::InvalidStorageClasses(self.current_span)) }
                    },
                    other => return Err(ParseError::UnexpectedToken(other, self.current_span)),
                }
            }
        }
        let dtype = self.parse_types(&types)?;
        Ok((dtype, storage))
    }


    fn parse_func_declaration(&mut self, identifier: String, return_type: Type, storage: Option<StorageClass>) 
        -> Result<FuncDeclaration, ParseError> {
        let (param_types, params) = self.parse_func_params()?;
        let func_type = Type::FuncType { params: param_types, ret: Box::new(return_type) };
        let mut body = None;
        if self.next_token_is(TokenType::OpenBrace) {
            body = Some(self.parse_block()?);
        } else {
            self.expect(TokenType::Semicolon)?;
        }
        Ok(FuncDeclaration { identifier, func_type, params, body, storage, span: self.current_span }) 
    }

    fn collect_param_type(&mut self) -> Result<Type, ParseError> {
        let mut types = Vec::new();
        while !matches!(self.next_token_type()?, TokenType::Identifier(_)) {
            let next = self.next_token_type()?;
            match next {
                tk if TYPE_SPECIFIERS.contains(&next) => {
                    self.advance()?;
                    types.push(tk);
                },
                _ => return Err(ParseError::InvalidTypes(self.current_span)),
            }
        }
        self.parse_types(&types)
    }

    fn parse_func_params(&mut self) -> Result<(Vec<Box<Type>>, Vec<String>), ParseError> {
        self.expect(TokenType::OpenParen)?;
        let mut types_list = Vec::new();
        let mut params_list = Vec::new();
        if self.next_token_is(TokenType::Void) {
            self.advance()?;
            self.expect(TokenType::CloseParen)?;
            return Ok((types_list, params_list))
        }

        while !self.next_token_is(TokenType::CloseParen) {
            let ptype = self.collect_param_type()?;
            if let TokenType::Identifier(param) = self.advance()?.token_type {
                types_list.push(Box::new(ptype));
                params_list.push(param);
            } else {
                return Err(ParseError::ExpectedParam(self.current_span));
            }

            while self.next_token_is(TokenType::Comma) {
                self.expect(TokenType::Comma)?;
                let ptype = self.collect_param_type()?;
                if let TokenType::Identifier(param) = self.advance()?.token_type {
                    types_list.push(Box::new(ptype));
                    params_list.push(param);
                } else {
                    return Err(ParseError::ExpectedParam(self.current_span));
                }
            }
        
        }

        self.expect(TokenType::CloseParen)?;
        Ok((types_list, params_list))
    }

    fn parse_var_declaration(&mut self, identifier: String, var_type: Type, storage: Option<StorageClass>) 
        -> Result<VarDeclaration, ParseError> {
        let mut init = None;
        if !self.next_token_is(TokenType::Semicolon) {
            self.expect(TokenType::Equal)?;
            init = Some(self.parse_expression(0)?);
        }
        self.expect(TokenType::Semicolon)?;
        Ok(VarDeclaration{identifier, var_type, init, storage, span: self.current_span})
    }

    //////////////
    /// BLOCKS ///
    //////////////

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let mut blockitems = Vec::new();
        self.expect(TokenType::OpenBrace)?;
        while !self.next_token_is(TokenType::CloseBrace) {
            blockitems.push(self.parse_blockitem()?);
        }
        self.expect(TokenType::CloseBrace)?;
        Ok(Block{ items: blockitems })
    }

    fn parse_blockitem(&mut self) -> Result<BlockItem, ParseError> {
        let item = match self.next_token_is_specifier() {
            true => BlockItem::D(self.parse_declaration()?),
            false => BlockItem::S(self.parse_statement()?),
        };
        Ok(item)
    }
}
