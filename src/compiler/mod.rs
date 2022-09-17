mod parser;
mod precedence;

use crate::{chunk::Chunk, disassembler, scan::Source, InterpretError};
use parser::Parser;
const DEBUG: bool = false;

pub fn compile(source: &str) -> Result<Chunk, InterpretError> {
    let source = Source::new(source.to_string());
    let chunk = Chunk::new();
    let mut parser = Parser::new(source, chunk);
    parser.parse()?;
    end_compiler(&mut parser.chunk, parser.previous.unwrap().line);
    if DEBUG {
        disassembler::Disassembler::disassemble_chunk(&parser.chunk, "code".to_string());
    }
    Ok(parser.chunk)
}

fn end_compiler(current_chunk: &mut Chunk, line: usize) {
    parser::chunk_op::emit_return(current_chunk, line)
}
