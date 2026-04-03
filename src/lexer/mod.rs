use std::{
    iter::Peekable, vec::IntoIter, fmt,
};
pub mod tokens;
pub use tokens::TokenType;

#[derive(Debug)]
pub enum LexerError { 
    InvalidCharacter(char, Span),
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LexerError::InvalidCharacter(c, s) => write!(f, "Lexer Error: invalid character! {}\nLine: {}, Col: {}", c, s.line_number, s.col),
        }
    }
}

impl std::error::Error for LexerError {}

#[derive(Debug, Copy, Clone)]
pub struct Span {
    pub line_number: usize,
    pub col: usize,
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "line {}, col {}", self.line_number, self.col)
    }
}

#[derive(Debug)]
pub struct Token {
    pub token_type: TokenType,
    pub location: Span,
}

pub struct Tokenizer {
    chars: Peekable<IntoIter<char>>,
    current: usize,
    col: usize,
    len: usize,
    line: usize
}

impl Tokenizer {
    pub fn new(source: String) -> Tokenizer {
        Tokenizer {
            chars: source.chars().collect::<Vec<char>>().into_iter().peekable(),
            current: 0,
            col: 0,
            len: source.len(),
            line: 1,
        }
    }

    fn advance(&mut self) -> char {
        let char = self.chars.next().unwrap_or('\0');
        self.current += 1;
        self.col += 1;
        char
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.chars.peek() {
            match c {
                '\n' => { self.line += 1; self.chars.next(); self.current += 1; self.col = 0},
                ' ' | '\r' | '\t' => {self.chars.next(); self.current += 1; self.col +=1},
                _ => break,
            }
        }
    }

    fn at_end(&self) -> bool { 
        self.current >= self.len
    }

    fn is_double_char(&mut self, nextchar: char, no: TokenType, yes: TokenType) -> TokenType {
        if self.peek() != nextchar {
            no
        } else {
            self.advance();
            yes
        }
    }

    fn is_double_char_three(&mut self, not_double_char: TokenType,
        firstchar: char, first: TokenType,
        secondchar: char, second: TokenType) -> TokenType {

        let next = self.peek();
        if next == firstchar {
            self.advance();
            first
        } else if next == secondchar {
            self.advance();
            second
        } else {
            not_double_char
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>, LexerError> {
        self.skip_whitespace();
        if self.at_end() {
            return Ok(None);
        }
        let start_location = self.make_span();

        let c = self.advance();

        let token_type = match c {
            '(' => TokenType::OpenParen,
            ')' => TokenType::CloseParen,
            '{' => TokenType::OpenBrace,
            '}' => TokenType::CloseBrace,
            ';' => TokenType::Semicolon,
            '~' => TokenType::Tilde,
            '?' => TokenType::QuestionMark,
            ':' => TokenType::Colon,
            ',' => TokenType::Comma,
            '*' => self.is_double_char('=', TokenType::Asterisk, TokenType::AsteriskEqual),
            '/' => self.is_double_char('=', TokenType::FwdSlash, TokenType::FwdSlashEqual),
            '%' => self.is_double_char('=', TokenType::Percent, TokenType::PercentEqual),
            '^' => self.is_double_char('=', TokenType::Caret, TokenType::CaretEqual),
            '!' => self.is_double_char('=', TokenType::Exclamation, TokenType::NotEqual),
            '&' => self.is_double_char_three(TokenType::Ampersand, '&', TokenType::DoubleAmpersand, '=', TokenType::AmpersandEqual),
            '|' => self.is_double_char_three(TokenType::Pipe, '|', TokenType::DoublePipe, '=', TokenType::PipeEqual),
            '-' => self.is_double_char_three(TokenType::Minus, '-', TokenType::DoubleMinus, '=', TokenType::MinusEqual),
            '+' => self.is_double_char_three(TokenType::Plus, '+', TokenType::DoublePlus, '=', TokenType::PlusEqual),
            '<' => match self.peek() {
                '<' => { self.advance(); self.is_double_char('=', TokenType::DoubleLeftAngled, TokenType::DLAngledEqual) },
                '=' => { self.advance(); TokenType::LessOrEqual },
                _   => TokenType::LessThan,
            },
            '>' => match self.peek() {
                '>' => { self.advance(); self.is_double_char('=', TokenType::DoubleRightAngled, TokenType::DRAngledEqual) },
                '=' => { self.advance(); TokenType::GreaterOrEqual },
                _   => TokenType::GreaterThan,
            },
            '=' => self.is_double_char('=', TokenType::Equal, TokenType::DoubleEqual),
            other => {
                if other.is_digit(10) {
                    self.scan_constant(other)?
                } else if other.is_ascii_alphabetic() || other == '_' {
                    self.scan_text(other)
                } else {
                    return Err(LexerError::InvalidCharacter(other, start_location))
                }
            }
        };
        Ok(Some(Token {
            token_type,
            location: start_location,
        }))
    }

    fn peek(&mut self) -> char {
            self.chars.peek().copied().unwrap_or('\0')
    }

    fn scan_constant(&mut self, first: char) -> Result<TokenType, LexerError> {
        let mut number = String::from(first);

        while self.peek().is_ascii_digit() {
            number.push(self.advance());
        }

        let mut parsed = None;

        if self.peek().is_ascii_alphabetic() {
            let suffix = self.peek();
            match suffix {
                'l' | 'L' => {
                    self.advance();
                    parsed = Some(TokenType::LongConstant(number.clone()));
                },
                _ => return Err(LexerError::InvalidCharacter(suffix, self.make_span())),
            }
        }
        
        if self.peek().is_ascii_alphabetic() {
            return Err(LexerError::InvalidCharacter(self.peek(), self.make_span()));
        }
        
        if let Some(token) = parsed {
            Ok(token)
        } else {
            Ok(TokenType::Constant(number))
        }
    }

    fn parse_keyword(&self, lexeme: &str) -> Option<TokenType> {
        let token_type = match lexeme {
            "return" => TokenType::Return,
            "int" => TokenType::Int,
            "void" => TokenType::Void,
            "if" => TokenType::If,
            "else" => TokenType::Else,
            "goto" => TokenType::Goto,
            "do" => TokenType::Do,
            "while" => TokenType::While,
            "for" => TokenType::For,
            "break" => TokenType::Break,
            "continue" => TokenType::Continue,
            "switch" => TokenType::Switch,
            "case" => TokenType::Case,
            "default" => TokenType::Default,
            "static" => TokenType::Static,
            "extern" => TokenType::Extern,
            "long" => TokenType::Long,
            _ => return None,
        };

        Some(token_type)
    }

    fn scan_text(&mut self, first: char) -> TokenType {
        let mut word = String::from(first);
        while self.peek().is_ascii_alphanumeric() || self.peek() == '_' {
            word.push(self.advance());
        }
        match self.parse_keyword(&word) {
            Some(tokentype) => tokentype,
            None => TokenType::Identifier(word)
        }
    }

    fn make_span(&self) -> Span {
        Span {
            line_number: self.line,
            col: self.col,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens: Vec<Token> = Vec::new();
        while let Some(token) = self.next_token()? {
            tokens.push(token);
        }
        Ok(tokens)
    }
}
