use super::*;
use ordered_float::OrderedFloat;

impl Parser {
    pub fn parse_expression(&mut self, min_prec: i32) -> Result<Expression, ParseError> {
        let left = self.parse_factor(None)?;
        self.parse_expression_cont(left, min_prec)
    }

    pub fn parse_expression_cont(&mut self, mut left: Expression, min_prec: i32) -> Result<Expression, ParseError> {
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
        let v = number.parse::<u64>().map_err(|_| ParseError::IntegerOverflow(self.current_span))?;
        if v <= i32::MAX as u64 {
            Ok(Const::Int(v as i32))
        } else if v <= i64::MAX as u64 {
            Ok(Const::Long(v as i64))
        } else {
            Err(ParseError::IntegerOverflow(self.current_span))
        }
    }

    fn parse_u_const(&self, number: String) -> Result<Const, ParseError> {
        let v = number.parse::<u64>().map_err(|_| ParseError::IntegerOverflow(self.current_span))?;
        if v <= u32::MAX as u64 {
            Ok(Const::UInt(v as u32))
        } else if v <= u64::MAX as u64 {
            Ok(Const::ULong(v))
        } else {
            Err(ParseError::IntegerOverflow(self.current_span))
        }
    }

    pub fn parse_factor(&mut self, token: Option<Token>) -> Result<Expression, ParseError> {
        let current_token = match token {
            Some(t) => t, 
            None => self.advance()?,
        };
        let expr = match current_token.token_type {
            TokenType::NumericConstant(n) => {
                match n.numtype {
                    NumericType::Int => {
                        self.new_expr(ExpressionKind::Constant(self.parse_const(n.number)?))
                    },
                    NumericType::Long => {
                        let v = n.number.parse::<i64>().map_err(|_| ParseError::IntegerOverflow(self.current_span))?;
                        self.new_expr(ExpressionKind::Constant(Const::Long(v)))
                    },
                    NumericType::UInt => {
                        self.new_expr(ExpressionKind::Constant(self.parse_u_const(n.number)?))
                    },
                    NumericType::ULong => {
                        let v = n.number.parse::<u64>().map_err(|_| ParseError::IntegerOverflow(self.current_span))?;
                        self.new_expr(ExpressionKind::Constant(Const::ULong(v)))
                    },
                    NumericType::Double => {
                        let v = n.number.parse::<f64>().map_err(|_| ParseError::InvalidFloat(self.current_span))?;
                        self.new_expr(ExpressionKind::Constant(Const::Double(OrderedFloat(v))))
                    },
                }
            },
            TokenType::OpenParen => {
                if self.next_token_is_type() {
                    let mut types = Vec::new();
                    while !self.next_token_is(TokenType::CloseParen) {
                        types.push(self.advance()?.token_type);
                    }
                    let ctype = self.parse_types(&types)?;
                    self.expect(TokenType::CloseParen)?;
                    let factor = Box::new(self.parse_factor(None)?);
                    let expression = self.new_expr(ExpressionKind::Cast(ctype, factor));
                    expression
                } else {
                    let expression = self.parse_expression(0)?;
                    self.expect(TokenType::CloseParen)?;
                    expression
                }
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

    pub fn check_postfix(&mut self, expr: Expression) -> Result<Expression, ParseError> {
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
