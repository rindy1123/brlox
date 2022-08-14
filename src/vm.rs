use crate::{
    chunk::{Chunk, OpCode, Value},
    compiler::Compiler,
    disassembler,
};

pub struct VM {
    chunk: Chunk,
    // TODO: use pointer
    ip: usize,
    stack: Vec<Value>,
}

const DEBUG: bool = false;
const STACK_MAX: usize = 256;

impl VM {
    pub fn new() -> VM {
        VM {
            chunk: Chunk::new(),
            ip: 0,
            stack: Vec::with_capacity(STACK_MAX),
        }
    }

    pub fn interpret(&mut self, source: &str) -> Result<(), InterpretError> {
        Compiler::compile(source);
        Ok(())
    }

    fn run(&mut self) -> Result<(), InterpretError> {
        loop {
            let instruction = &self.chunk.clone().code[self.ip];
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
                    return Ok(());
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

pub enum InterpretError {
    CompileError,
    RuntimeError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_op_constant() {
        let mut vm = VM::new();
        let target_constant = vm.chunk.add_constant(1.2);
        vm.chunk.add_code(
            OpCode::OpConstant {
                index: target_constant,
            },
            1,
        );
        // should be added or target_constant will be poped out of the stack
        let dummy_constant = vm.chunk.add_constant(3.4);
        vm.chunk.add_code(
            OpCode::OpConstant {
                index: dummy_constant,
            },
            1,
        );
        vm.chunk.add_code(OpCode::OpReturn, 1);
        vm.run();
        assert_eq!(vm.stack[0], 1.2);
        assert_eq!(vm.ip, 3);
    }

    #[test]
    fn test_run_op_negate() {
        let mut vm = VM::new();
        let target_constant = vm.chunk.add_constant(1.2);
        vm.chunk.add_code(
            OpCode::OpConstant {
                index: target_constant,
            },
            1,
        );
        // should be added or target_constant will be poped out of the stack
        vm.chunk.add_code(OpCode::OpNegate, 1);
        let dummy_constant = vm.chunk.add_constant(3.4);
        vm.chunk.add_code(
            OpCode::OpConstant {
                index: dummy_constant,
            },
            1,
        );
        vm.chunk.add_code(OpCode::OpReturn, 1);
        vm.run();
        assert_eq!(vm.stack[0], -1.2);
        assert_eq!(vm.ip, 4);
    }

    mod binary_operation {
        use super::*;

        #[test]
        fn test_add() {
            let mut vm = VM::new();
            vm.stack.push(1.2);
            vm.stack.push(3.4);
            vm.binary_operation(&OpCode::OpAdd);
            assert_eq!(vm.stack[0], 4.6);
        }

        #[test]
        fn test_subtract() {
            let mut vm = VM::new();
            vm.stack.push(1.2);
            vm.stack.push(3.4);
            vm.binary_operation(&OpCode::OpSubtract);
            assert_eq!(vm.stack[0], -2.2);
        }

        #[test]
        fn test_multiply() {
            let mut vm = VM::new();
            vm.stack.push(2.0);
            vm.stack.push(3.4);
            vm.binary_operation(&OpCode::OpMultiply);
            assert_eq!(vm.stack[0], 6.8);
        }

        #[test]
        fn test_divide() {
            let mut vm = VM::new();
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
            let mut vm = VM::new();
            vm.stack.push(6.0);
            vm.stack.push(2.0);
            vm.binary_operation(&OpCode::OpReturn);
        }
    }
}
