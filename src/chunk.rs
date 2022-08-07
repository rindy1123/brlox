#[derive(Debug)]
pub enum OpCode {
    OpReturn,
    OpConstant { index: u8 },
}

type Value = f64;

#[derive(Debug)]
pub struct Chunk {
    pub code: Vec<OpCode>,
    constants: Vec<Value>,
    pub lines: Vec<usize>,
}

impl Chunk {
    pub fn new() -> Chunk {
        Chunk {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn add_code(&mut self, op_code: OpCode, line: usize) {
        self.code.push(op_code);
        self.lines.push(line);
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1 // index of value in constants
    }
}
