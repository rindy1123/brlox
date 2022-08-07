use crate::chunk::Chunk;

pub struct Disassembler {}

impl Disassembler {
    pub fn disassemble_chunk(chunk: Chunk, name: String) {
        println!("== {name} ==");

        for (i, op_code) in chunk.code.iter().enumerate() {
            let line = chunk.lines[i];
            println!("{i:0>4} {line} {op_code:?}");
        }
    }
}
