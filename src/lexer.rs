use std::{
    fs::read_to_string,
    path::PathBuf
};

struct Span {
    line_number: usize,
    start_char: usize,
    end_char: usize,
}

enum TokenType {
    Identifier(String),
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    Semicolon,
    Constant(usize),
    Keyword(String),
}


struct Token {
    token_type: TokenType,
    location: Span,
}


fn load_source(input_file: PathBuf) -> Result<String, std::io::Error> {
    let source = read_to_string(input_file)?;
    Ok(source)
}

fn lex_file(source: String) -> Vec<TokenType> {

}
