use crate::{
    chunk::Chunk,
    scan::{scan_token, Source},
    token::{Token, TokenType},
};

struct Parser {
    current: Option<Token>,
    previous: Option<Token>,
    had_error: bool,
    panic_mode: bool,
}

impl Parser {
    fn new() -> Parser {
        Parser {
            current: None,
            previous: None,
            // TODO: eliminate flags
            had_error: false,
            panic_mode: false,
        }
    }
}

pub fn compile(source: &str, chunk: &mut Chunk) -> bool {
    let mut source = Source::new(source.to_string());
    let mut parser = Parser::new();
    advance(&mut parser, &mut source);
    expression();
    consume(
        &mut parser,
        &mut source,
        TokenType::EOF,
        "Expect end of expression.".to_string(),
    );
    !parser.had_error
}

fn advance(parser: &mut Parser, source: &mut Source) {
    parser.previous = parser.current.clone();
    loop {
        let token = scan_token(source);
        parser.current = Some(token.clone());
        if let TokenType::Error = token.token_type {
            report_error(parser, token.clone(), token.lexeme);
        } else {
            break;
        }
    }
}

fn expression() {
    todo!()
}

fn consume(parser: &mut Parser, source: &mut Source, token_type: TokenType, message: String) {
    let current_token = parser.current.clone().unwrap();
    if current_token.token_type == token_type {
        return advance(parser, source);
    }
    report_error(parser, current_token, message);
}

fn report_error(parser: &mut Parser, token: Token, message: String) {
    parser.panic_mode = true;
    let position = match token.token_type {
        TokenType::EOF => "at end".to_string(),
        _ => format!("at '{}'", token.lexeme),
    };
    eprintln!("[line {}] Error {}: {}", token.line, position, message);
    parser.had_error = true;
}
