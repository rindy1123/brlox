use crate::{
    chunk::{Chunk, OpCode, Value},
    disassembler,
};

pub struct VM<'a> {
    chunk: &'a Chunk,
    // TODO: use pointer
    ip: usize,
    stack: Vec<Value>,
}

const DEBUG: bool = false;
const STACK_MAX: usize = 256;

impl<'a> VM<'a> {
    pub fn new(chunk: &'a Chunk) -> VM<'a> {
        VM {
            chunk,
            ip: 0,
            stack: Vec::with_capacity(STACK_MAX),
        }
    }

    pub fn interpret(mut self) -> InterpretResult {
        // self.chunk = chunk;
        // self.ip = chunk.code.as_ptr();
        self.run()
    }

    fn run(&mut self) -> InterpretResult {
        loop {
            let instruction = &self.chunk.code[self.ip];
            if DEBUG {
                println!("      ");
                for slot in self.stack.clone() {
                    println!("[ {} ]", slot);
                }
                disassembler::disassemble_instruction(
                    self.ip,
                    self.chunk.lines[self.ip],
                    instruction,
                );
            }
            self.ip += 1;
            match instruction {
                OpCode::OpReturn => {
                    println!("{}", self.stack.pop().unwrap());
                    return InterpretResult::InterpretOk;
                }
                OpCode::OpNegate => {
                    let negated_value = -self.stack.pop().unwrap();
                    self.stack.push(negated_value);
                }
                OpCode::OpConstant { index } => {
                    let constant = self.chunk.constants[index.clone()];
                    self.stack.push(constant);
                    println!("{constant}");
                }
                OpCode::OpAdd | OpCode::OpSubtract | OpCode::OpMultiply | OpCode::OpDivide => {
                    self.binary_operation(instruction)
                }
            }
        }
    }

    fn binary_operation(&mut self, binary_operator: &OpCode) {
        let right = self.stack.pop().unwrap();
        let left = self.stack.pop().unwrap();
        let result = match binary_operator {
            OpCode::OpAdd => left + right,
            OpCode::OpSubtract => left - right,
            OpCode::OpMultiply => left * right,
            OpCode::OpDivide => left / right,
            _ => panic!(
                "Expected OpAdd, OpSubtract, OpMultiply, OpDivide only. We got {binary_operator:?}."
            ),
        };
        self.stack.push(result);
    }
}

pub enum InterpretResult {
    InterpretOk,
    InterpretCompileError,
    InterpretRuntimeError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_op_constant() {
        let mut chunk = Chunk::new();
        let target_constant = chunk.add_constant(1.2);
        chunk.add_code(
            OpCode::OpConstant {
                index: target_constant,
            },
            1,
        );
        // should be added or target_constant will be poped out of the stack
        let dummy_constant = chunk.add_constant(3.4);
        chunk.add_code(
            OpCode::OpConstant {
                index: dummy_constant,
            },
            1,
        );
        chunk.add_code(OpCode::OpReturn, 1);
        let mut vm = VM::new(&chunk);
        vm.run();
        assert_eq!(vm.stack[0], 1.2);
        assert_eq!(vm.ip, 3);
    }

    #[test]
    fn test_run_op_negate() {
        let mut chunk = Chunk::new();
        let target_constant = chunk.add_constant(1.2);
        chunk.add_code(
            OpCode::OpConstant {
                index: target_constant,
            },
            1,
        );
        // should be added or target_constant will be poped out of the stack
        chunk.add_code(OpCode::OpNegate, 1);
        let dummy_constant = chunk.add_constant(3.4);
        chunk.add_code(
            OpCode::OpConstant {
                index: dummy_constant,
            },
            1,
        );
        chunk.add_code(OpCode::OpReturn, 1);
        let mut vm = VM::new(&chunk);
        vm.run();
        assert_eq!(vm.stack[0], -1.2);
        assert_eq!(vm.ip, 4);
    }

    mod binary_operation {
        use super::*;

        #[test]
        fn test_add() {
            let chunk = Chunk::new();
            let mut vm = VM::new(&chunk);
            vm.stack.push(1.2);
            vm.stack.push(3.4);
            vm.binary_operation(&OpCode::OpAdd);
            assert_eq!(vm.stack[0], 4.6);
        }

        #[test]
        fn test_subtract() {
            let chunk = Chunk::new();
            let mut vm = VM::new(&chunk);
            vm.stack.push(1.2);
            vm.stack.push(3.4);
            vm.binary_operation(&OpCode::OpSubtract);
            assert_eq!(vm.stack[0], -2.2);
        }

        #[test]
        fn test_multiply() {
            let chunk = Chunk::new();
            let mut vm = VM::new(&chunk);
            vm.stack.push(2.0);
            vm.stack.push(3.4);
            vm.binary_operation(&OpCode::OpMultiply);
            assert_eq!(vm.stack[0], 6.8);
        }

        #[test]
        fn test_divide() {
            let chunk = Chunk::new();
            let mut vm = VM::new(&chunk);
            vm.stack.push(6.0);
            vm.stack.push(2.0);
            vm.binary_operation(&OpCode::OpDivide);
            assert_eq!(vm.stack[0], 3.0);
        }

        #[test]
        #[should_panic(
            expected = "Expected OpAdd, OpSubtract, OpMultiply, OpDivide only. We got OpReturn."
        )]
        fn test_invalid_opcode() {
            let chunk = Chunk::new();
            let mut vm = VM::new(&chunk);
            vm.stack.push(6.0);
            vm.stack.push(2.0);
            vm.binary_operation(&OpCode::OpReturn);
        }
    }
}
