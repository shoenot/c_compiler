use std::{
    iter::Peekable, vec::IntoIter, fmt,
};

#[derive(Debug)]
pub enum LexerError {
    IntegerOverflow(Span),
    InvalidCharacter(char, Span),
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LexerError::IntegerOverflow(s) => write!(f, "Lexer Error: int overflow! \nLine: {}, Col: {}", s.line_number, s.start_idx),
            LexerError::InvalidCharacter(c, s) => write!(f, "Lexer Error: invalid character! {}\nLine: {}, Col: {}", c, s.line_number, s.start_idx),
        }
    }
}

impl std::error::Error for LexerError {}

#[derive(Debug, Copy, Clone)]
pub struct Span {
    pub line_number: usize,
    pub start_idx: usize,
    pub end_idx: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    Identifier(String),
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    Semicolon,
    Constant(i32),
    Int,
    Void,
    Return,
}

#[derive(Debug)]
pub struct Token {
    pub token_type: TokenType,
    pub location: Span,
}

pub struct Tokenizer {
    chars: Peekable<IntoIter<char>>,
    start: usize,
    current: usize,
    len: usize,
    line: usize
}

impl Tokenizer {
    pub fn new(source: String) -> Tokenizer {
        Tokenizer {
            chars: source.chars().collect::<Vec<char>>().into_iter().peekable(),
            start: 0,
            current: 0,
            len: source.len(),
            line: 1,
        }
    }

    fn advance(&mut self) -> char {
        let char = self.chars.next().unwrap_or(' ');
        self.current += 1;
        char
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.chars.peek() {
            match c {
                '\n' => { self.line += 1; self.chars.next(); self.current += 1},
                ' ' | '\r' | '\t' => {self.chars.next(); self.current += 1},
                _ => break,
            }
        }
    }

    fn at_end(&self) -> bool {
        self.current >= self.len 
    }

    fn next_token(&mut self) -> Result<Option<Token>, LexerError> {
        self.skip_whitespace();
        self.start = self.current;
        if self.at_end() {
            return Ok(None);
        }
        
        let c = self.advance();

        let token_type = match c {
            '(' => TokenType::OpenParen,
            ')' => TokenType::CloseParen,
            '{' => TokenType::OpenBrace,
            '}' => TokenType::CloseBrace,
            ';' => TokenType::Semicolon,
            other => {
                if other.is_digit(10) {
                    self.scan_constant(other)?
                } else if other.is_ascii_alphabetic() || other == '_' {
                    self.scan_text(other)
                } else {
                    return Err(LexerError::InvalidCharacter(other, self.make_span()))
                }
            }
        };
        Ok(Some(Token {
            token_type,
            location: self.make_span(),
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

        if self.peek().is_ascii_alphabetic() {
            return Err(LexerError::InvalidCharacter(self.peek(), self.make_span()))
        }
        number.parse().map(TokenType::Constant)
            .map_err(|_| LexerError::IntegerOverflow(self.make_span()))
    }

    fn parse_keyword(&self, lexeme: &str) -> Option<TokenType> {
        let token_type = match lexeme {
            "return" => TokenType::Return,
            "int" => TokenType::Int,
            "void" => TokenType::Void,
            _ => return None,
        };

        Some(token_type)
    }

    fn scan_text(&mut self, first: char) -> TokenType {
        let mut word = String::from(first);
        while self.peek().is_ascii_alphanumeric() {
            word.push(self.advance());
        }
        match self.parse_keyword(&word) {
            Some(tokentype) => tokentype,
            None => TokenType::Identifier(word)
        }
    }

    fn make_span(&mut self) -> Span {
        Span { 
            line_number: self.line,
            start_idx: self.start,
            end_idx: self.current,
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


