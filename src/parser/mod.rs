use std::iter::Peekable;
use std::vec::IntoIter;
use std::fmt;
use crate::types::*;

use crate::lexer::*; 

mod ast;
pub use ast::*;

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
    DuplicateTypes(Span),
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
            ParseError::DuplicateTypes(s) => write!(f, "Parse Error: duplicate types!\nLine: {}, Col: {}", s.line_number, s.col),
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
}

impl TypeFlags {
    fn new() -> Self {
        TypeFlags { 
            saw_int: false,
            saw_long: false,
        }
    }

    fn set_flag(&mut self, dtype: &TokenType, span: &Span) -> Result<(), ParseError> {
        match dtype {
            TokenType::Int => if self.saw_int != true {self.saw_int = true} else {
                return Err(ParseError::InvalidTypes(span.clone())) },
            TokenType::Long => if self.saw_long != true {self.saw_long = true} else {
                return Err(ParseError::InvalidTypes(span.clone())) },
            _ => return Err(ParseError::InvalidTypes(span.clone())),
        }
        Ok(())
    }

    fn get_type(&self, span: &Span) -> Result<Type, ParseError> {
        match self {
            TypeFlags { saw_int:true, saw_long:false } => Ok(Type::Int),
            TypeFlags { saw_int:false, saw_long:true } => Ok(Type::Long),
            TypeFlags { saw_int:true, saw_long:true } => Ok(Type::Long),
            _ => Err(ParseError::InvalidTypes(span.clone())),
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
            matches!(token.token_type, TokenType::Int | TokenType::Static |
                TokenType::Extern | TokenType::Long)
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
        Expression::new(kind, self.current_span)
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
                    TokenType::Int | TokenType::Long => types.push(token),
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
                TokenType::Int | TokenType::Long => {
                    self.advance()?;
                    types.push(next);
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

    //////////////////
    /// STATEMENTS ///
    //////////////////

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        let statement = match self.next_token_type()? {
            TokenType::Semicolon => { 
                let ret = StatementKind::Null;
                self.expect(TokenType::Semicolon)?;
                self.new_stmt(ret)
            },
            TokenType::Return => {
                self.advance()?;
                let ret = StatementKind::Return(self.parse_expression(0)?);
                self.expect(TokenType::Semicolon)?;
                self.new_stmt(ret)
            },
            TokenType::If => {
                self.advance()?;
                self.expect(TokenType::OpenParen)?;
                let cond = self.parse_expression(0)?;
                self.expect(TokenType::CloseParen)?;
                let yes = self.parse_statement()?;
                if self.next_token_is(TokenType::Else) { 
                    self.advance()?;
                    let no = self.parse_statement()?;
                    self.new_stmt(StatementKind::If(cond, Box::new(yes), Some(Box::new(no))))
                } else {
                    self.new_stmt(StatementKind::If(cond, Box::new(yes), None))
                }
            },
            TokenType::Identifier(name) => {
                let tok = self.advance()?;
                if self.next_token_is(TokenType::Colon) {
                    self.expect(TokenType::Colon)?;
                    let body = self.parse_statement();
                    let body = match body {
                        Ok(st) => st,
                        Err(e) => match e {
                            ParseError::ExpectedStatement(_) => {
                                return Err(ParseError::LabelWithoutStatement(self.current_span));
                            },
                            _ => return Err(e),
                        }
                    };
                    self.new_stmt(StatementKind::Label(name, Box::new(body)))
                } else if self.next_token_is(TokenType::OpenParen) {
                    let mut expr = self.parse_factor(Some(tok))?;
                    expr = self.parse_expression_cont(expr, 0)?;
                    self.expect(TokenType::Semicolon)?;
                    self.new_stmt(StatementKind::Expression(expr))
                } else {
                    let mut expr = self.new_expr(ExpressionKind::Var(name));
                    expr = self.check_postfix(expr)?;
                    expr = self.parse_expression_cont(expr, 0)?;
                    self.expect(TokenType::Semicolon)?;
                    self.new_stmt(StatementKind::Expression(expr))
                }
            },
            TokenType::Goto => {
                self.advance()?;
                let token = self.advance()?;
                match token.token_type {
                    TokenType::Identifier(name) => {
                        self.expect(TokenType::Semicolon)?;
                        self.new_stmt(StatementKind::Goto(name))
                    },
                    _ => return Err(ParseError::ExpectedIdentifier(self.current_span))
                }
            },
            TokenType::OpenBrace => {
                let block = self.parse_block()?;
                self.new_stmt(StatementKind::Compound(block))
            },
            TokenType::While => {
                self.advance()?;
                self.expect(TokenType::OpenParen)?;
                let cond = self.parse_expression(0)?;
                self.expect(TokenType::CloseParen)?;
                let body = Box::new(self.parse_statement()?);
                self.new_stmt(StatementKind::While { cond, body, lab: "".into() })
            },
            TokenType::Do => {
                self.advance()?;
                let body = Box::new(self.parse_statement()?);
                self.expect(TokenType::While)?;
                self.expect(TokenType::OpenParen)?;
                let cond = self.parse_expression(0)?;
                self.expect(TokenType::CloseParen)?;
                self.expect(TokenType::Semicolon)?;
                self.new_stmt(StatementKind::DoWhile { cond, body, lab: "".into() })
            },
            TokenType::For => self.parse_for_loop()?,
            TokenType::Break => {
                self.advance()?;
                let ret = StatementKind::Break("".into());
                self.expect(TokenType::Semicolon)?;
                self.new_stmt(ret)
            },
            TokenType::Continue => {
                self.advance()?;
                let ret = StatementKind::Continue("".into());
                self.expect(TokenType::Semicolon)?;
                self.new_stmt(ret)
            },
            TokenType::Switch => {
                self.advance()?;
                self.expect(TokenType::OpenParen)?;
                let scrutinee = self.parse_expression(0)?;
                self.expect(TokenType::CloseParen)?;
                let body = self.parse_statement()?;
                self.new_stmt(StatementKind::Switch{ scrutinee, body: Box::new(body), lab:"".into(), cases: Vec::new() })
            },
            TokenType::Case => {
                self.advance()?;
                let expr = self.parse_expression(0)?;
                self.expect(TokenType::Colon)?;
                self.new_stmt(StatementKind::Case{ expr, lab:"".into() })
            },
            TokenType::Default => {
                self.advance()?;
                self.expect(TokenType::Colon)?;
                self.new_stmt(StatementKind::Default{lab:"".into()})
            },
            TokenType::Int => return Err(ParseError::ExpectedStatement(self.current_span)),
            _ => {
                let ret = StatementKind::Expression(self.parse_expression(0)?);
                self.expect(TokenType::Semicolon)?;
                self.new_stmt(ret)
            },
        };
        Ok(statement)
    }

    fn parse_for_loop(&mut self) -> Result<Statement, ParseError> {
        self.advance()?;
        self.expect(TokenType::OpenParen)?;
        let init = match self.next_token_type()? {
            TokenType::Semicolon => {
                self.advance()?;
                ForInit::InitExp(None)
            },
            _ => {
                if self.next_token_is(TokenType::Int) | self.next_token_is(TokenType::Long) {
                    let dec = match self.parse_declaration()? {
                        Decl::VarDecl(v) => v,
                        Decl::FuncDecl(_) => return Err(ParseError::ExpectedVarDecl(self.current_span)),
                    };
                    ForInit::InitDec(dec)
                } else {
                    let exp = self.parse_expression(0)?;
                    self.expect(TokenType::Semicolon)?;
                    ForInit::InitExp(Some(exp))
                }
            },
        };

        let mut cond = None;
        if !self.next_token_is(TokenType::Semicolon) {
            cond = Some(self.parse_expression(0)?);
        } 
        self.expect(TokenType::Semicolon)?;

        let mut post = None;
        if !self.next_token_is(TokenType::CloseParen) {
            post = Some(self.parse_expression(0)?);
        } 
        self.expect(TokenType::CloseParen)?;

        let body = Box::new(self.parse_statement()?);

        Ok(self.new_stmt(StatementKind::For { init, cond, post, body, lab: "".into() }))
    }

    ///////////////////
    /// EXPRESSIONS ///
    ///////////////////
    
    fn parse_expression(&mut self, min_prec: i32) -> Result<Expression, ParseError> {
        let left = self.parse_factor(None)?;
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
                    left = self.new_expr(ExpressionKind::Assignment(Box::new(left), Box::new(right)));
                }
                BinaryOp::OpSet(op) => {
                    let right = self.parse_expression(prec)?;
                    let binary = self.new_expr(ExpressionKind::Binary(*op, Box::new(left.clone()), Box::new(right)));
                    left = self.new_expr(ExpressionKind::Assignment(Box::new(left), Box::new(binary)));
                }
                BinaryOp::Ternary => {
                    let middle = self.parse_conditional_middle()?;
                    let right = self.parse_expression(prec)?;
                    left = self.new_expr(ExpressionKind::Conditional(Box::new(left), Box::new(middle), Box::new(right)));
                },
                _ => {
                    let right = self.parse_expression(prec + 1)?;
                    left = self.new_expr(ExpressionKind::Binary(op, Box::new(left), Box::new(right)));
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

    fn parse_const(&self, number: String) -> Result<Const, ParseError> {
        let v = number.parse::<i64>().map_err(|_| ParseError::IntegerOverflow(self.current_span))?;

        if v <= 2_i64.pow(31) - 1 {
            Ok(Const::Int(v as i32))
        } else {
            Ok(Const::Long(v))
        }
    }

    fn parse_factor(&mut self, token: Option<Token>) -> Result<Expression, ParseError> {
        let current_token = match token {
            Some(t) => t, 
            None => self.advance()?,
        };
        let expr = match current_token.token_type {
            TokenType::Constant(value) => self.new_expr(ExpressionKind::Constant(self.parse_const(value)?)),
            TokenType::LongConstant(value) => {
                let v = value.parse::<i64>().map_err(|_| ParseError::IntegerOverflow(self.current_span))?;
                self.new_expr(ExpressionKind::Constant(Const::Long(v)))
            },
            TokenType::OpenParen => {
                let expression = self.parse_expression(0)?;
                self.expect(TokenType::CloseParen)?;
                expression
            },
            TokenType::Exclamation => self.parse_unop(UnaryOp::Not)?,
            TokenType::Tilde => self.parse_unop(UnaryOp::Complement)?,
            TokenType::Minus => self.parse_unop(UnaryOp::Negate)?,
            TokenType::Identifier(name) => {
                if self.next_token_is(TokenType::OpenParen) {
                    let args = self.parse_func_args()?;
                    self.new_expr(ExpressionKind::FunctionCall(name, args))
                } else {
                    self.new_expr(ExpressionKind::Var(name))
                }
            },
            TokenType::DoublePlus => {
                let operand = self.parse_factor(None)?;
                self.new_expr(ExpressionKind::PrefixIncrement(Box::new(operand)))
            },
            TokenType::DoubleMinus => {
                let operand = self.parse_factor(None)?;
                self.new_expr(ExpressionKind::PrefixDecrement(Box::new(operand)))
            },
            _ => return Err(ParseError::ExpectedExpression(self.current_span)),
        };
        let expr = self.check_postfix(expr)?;
        Ok(expr)
    }

    fn parse_func_args(&mut self) -> Result<Vec<Expression>, ParseError> {
        self.expect(TokenType::OpenParen)?;
        let mut args = Vec::new();

        while !self.next_token_is(TokenType::CloseParen) {
            // Parse first arg 
            args.push(self.parse_expression(0)?);

            while self.next_token_is(TokenType::Comma) {
                self.expect(TokenType::Comma)?;
                args.push(self.parse_expression(0)?);
            }
        }

        self.expect(TokenType::CloseParen)?;
        Ok(args)
    }

    fn check_postfix(&mut self, expr: Expression) -> Result<Expression, ParseError> {
        let mut expr = expr.clone();
        loop {
            match self.next_token_type()? {
                TokenType::DoublePlus => {
                    self.advance()?;
                    expr = self.new_expr(ExpressionKind::PostfixIncrement(Box::new(expr.clone())));
                },
                TokenType::DoubleMinus => {
                    self.advance()?;
                    expr = self.new_expr(ExpressionKind::PostfixDecrement(Box::new(expr.clone())));
                },
                _ => return Ok(expr),
            }
        }
    }

    fn parse_unop(&mut self, op: UnaryOp) -> Result<Expression, ParseError> {
        let operand = self.parse_factor(None)?;
        Ok(self.new_expr(ExpressionKind::Unary(op, Box::new(operand))))
    }

    fn peek_binop(&mut self) -> Result<Option<BinaryOp>, ParseError> {
        match self.next_token_type()? {
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

    fn precedence(&self, op: &BinaryOp) -> i32 {
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
