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
            }
        }
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
}
