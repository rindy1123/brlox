#[derive(Debug, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_code() {
        let mut chunk = Chunk::new();
        chunk.add_code(OpCode::OpConstant { index: 1 }, 1);
        chunk.add_code(OpCode::OpReturn, 2);
        assert_eq!(
            chunk.code,
            vec![OpCode::OpConstant { index: 1 }, OpCode::OpReturn]
        );
        assert_eq!(chunk.lines, vec![1, 2]);
    }

    #[test]
    fn test_add_constant() {
        let mut chunk = Chunk::new();
        let index1 = chunk.add_constant(1.2);
        let index2 = chunk.add_constant(8.9);
        assert_eq!(chunk.constants, vec![1.2, 8.9]);
        assert_eq!(index1, 0);
        assert_eq!(index2, 1);
    }
}
