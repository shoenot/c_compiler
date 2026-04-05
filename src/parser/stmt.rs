use super::*;

impl Parser {
    pub fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        let next = self.next_token_type()?;
        let statement = match next {
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
            _ if TYPE_SPECIFIERS.contains(&next) => {
                return Err(ParseError::ExpectedStatement(self.current_span))
            },
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
                if TYPE_SPECIFIERS.contains(&self.next_token_type()?) {
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
}
