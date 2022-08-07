use chunk::{Chunk, OpCode};

mod chunk;
mod disassembler;

fn main() {
    let mut chunk = Chunk::new();
    let constant = chunk.add_constant(1.2) as u8;
    chunk.add_code(OpCode::OpConstant { index: constant }, 1);
    chunk.add_code(OpCode::OpReturn, 1);
    disassembler::Disassembler::disassemble_chunk(chunk, "test chunk".to_string());
}
