mod parser;
mod precedence;

use crate::{chunk::Chunk, disassembler, scan::Source, value::object::ObjFunction, InterpretError};
use parser::Parser;
const DEBUG: bool = true;

pub fn compile(source: &str) -> Result<ObjFunction, InterpretError> {
    let source = Source::new(source.to_string());
    let chunk = Chunk::new();
    let mut parser = Parser::new(source, chunk);
    parser.parse()?;
    let mut function = parser.function;
    end_compiler(&mut function.chunk, parser.previous.unwrap().line);
    if DEBUG {
        disassembler::Disassembler::disassemble_chunk(&function.chunk, "code".to_string());
    }
    Ok(function)
}

fn end_compiler(current_chunk: &mut Chunk, line: usize) {
    parser::chunk_op::emit_return(current_chunk, line)
}
