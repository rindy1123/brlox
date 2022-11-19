use crate::token::{Token, TokenType};

pub fn report_error(token: &Token, message: &str) {
    let position = match token.token_type {
        TokenType::EOF => "at end".to_string(),
        _ => format!("at '{}'", token.lexeme),
    };
    eprintln!("[line {}] Error {}: {}", token.line, position, message);
}
