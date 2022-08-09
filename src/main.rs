use chunk::{Chunk, OpCode};
use vm::VM;

mod chunk;
mod disassembler;
mod vm;

fn main() {
    let mut chunk = Chunk::new();
    let constant = chunk.add_constant(1.2);
    chunk.add_code(OpCode::OpConstant { index: constant }, 1);
    chunk.add_code(OpCode::OpNegate, 1);
    chunk.add_code(OpCode::OpReturn, 1);
    disassembler::Disassembler::disassemble_chunk(&chunk, "test chunk".to_string());
    let vm = VM::new(&chunk);
    vm.interpret();
}
