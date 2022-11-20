use crate::value::Value;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum OpCode {
    OpReturn,
    OpNegate,
    OpAdd,
    OpSubtract,
    OpMultiply,
    OpDivide,
    OpNil,
    OpTrue,
    OpFalse,
    OpNot,
    OpEqual,
    OpGreater,
    OpLess,
    OpPrint,
    OpPop,
    OpDefineGlobal { index: usize },
    OpGetGlobal { index: usize },
    OpSetGlobal { index: usize },
    OpGetLocal { index: usize },
    OpSetLocal { index: usize },
    OpConstant { index: usize },
    OpCall { arg_count: usize },
    OpJumpIfFalse { offset: usize },
    OpJump { offset: usize },
    OpJumpBack { offset: usize },
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<OpCode>,
    pub constants: Vec<Value>,
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
        let constant1 = Value::Number(1.2);
        let constant2 = Value::Number(8.9);
        let index1 = chunk.add_constant(constant1);
        let index2 = chunk.add_constant(constant2);
        assert_eq!(chunk.constants[0].as_number(), 1.2);
        assert_eq!(chunk.constants[1].as_number(), 8.9);
        assert_eq!(index1, 0);
        assert_eq!(index2, 1);
    }
}
