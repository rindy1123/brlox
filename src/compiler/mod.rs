mod parser;
mod precedence;

use crate::{chunk::Chunk, disassembler, scan::Source, token::TokenType, InterpretError};
use parser::Parser;
const DEBUG: bool = true;

pub fn compile(source: &str) -> Result<Chunk, InterpretError> {
    let source = Source::new(source.to_string());
    let chunk = Chunk::new();
    let mut parser = Parser::new(source, chunk);
    parser.advance()?;
    parser.expression()?;
    parser.consume(TokenType::EOF, "Expect end of expression.".to_string())?;
    end_compiler(&mut parser.chunk, parser.previous.unwrap().line);
    if DEBUG {
        disassembler::Disassembler::disassemble_chunk(&parser.chunk, "code".to_string());
    }
    Ok(parser.chunk)
}

fn end_compiler(current_chunk: &mut Chunk, line: usize) {
    parser::emit_return(current_chunk, line)
}
