use crate::scan::{scan_token, Source};

pub struct Compiler {}

impl Compiler {
    pub fn compile(source: &str) {
        let mut source = Source::new(source.to_string());
        loop {
            let token = scan_token(&mut source);
            println!("{} {:?} {}", token.line, token.token_type, token.lexeme);
        }
    }
}
