use crate::chunk::{Chunk, OpCode};

/// For Debugging
pub struct Disassembler {}

impl Disassembler {
    pub fn disassemble_chunk(chunk: &Chunk, name: String) {
        println!("== {name} ==");

        for (i, op_code) in chunk.code.iter().enumerate() {
            disassemble_instruction(i, chunk.lines[i], op_code);
        }
    }
}

pub fn disassemble_instruction(i: usize, line: usize, op_code: &OpCode) {
    println!("{i:0>4} {line} {op_code:?}");
}
