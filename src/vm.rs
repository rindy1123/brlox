use crate::{
    chunk::{Chunk, OpCode},
    compiler::compile,
    disassembler,
    value::Value,
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

    fn run(&mut self) -> Result<(), InterpretError> {
        loop {
            let instruction = &self.chunk.clone().code[self.ip];
            if DEBUG {
                println!("      ");
                for slot in self.stack.clone() {
                    println!("[ {:?} ]", slot);
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
                    return Ok(());
                }
                OpCode::OpNegate => match self.stack.last().unwrap().clone() {
                    Value::Number(number) => {
                        self.stack.pop().unwrap();
                        self.stack.push(Value::Number(-number));
                    }
                    _ => {
                        self.runtime_error("Operand must be a number.");
                        return Err(InterpretError::RuntimeError);
                    }
                },
                OpCode::OpConstant { index } => {
                    let constant = self.chunk.constants[index.clone()].clone();
                    self.stack.push(constant);
                }
                OpCode::OpNil => self.stack.push(Value::Nil),
                OpCode::OpTrue => self.stack.push(Value::Bool(true)),
                OpCode::OpFalse => self.stack.push(Value::Bool(false)),
                OpCode::OpNot => {
                    let value = self.stack.pop().unwrap();
                    self.stack.push(Value::Bool(is_falsey(value)));
                }
                OpCode::OpAdd | OpCode::OpSubtract | OpCode::OpMultiply | OpCode::OpDivide => {
                    self.binary_operation(instruction)?;
                }
            }
        }
    }

    fn binary_operation(&mut self, binary_operator: &OpCode) -> Result<(), InterpretError> {
        let stack_len = self.stack.len();
        if let (Value::Number(right), Value::Number(left)) = (
            self.stack[stack_len - 1].clone(),
            self.stack[stack_len - 2].clone(),
        ) {
            self.stack.pop().unwrap();
            self.stack.pop().unwrap();
            let result = match binary_operator {
                OpCode::OpAdd => left + right,
                OpCode::OpSubtract => left - right,
                OpCode::OpMultiply => left * right,
                OpCode::OpDivide => left / right,
                _ => panic!(
                    "Expected OpAdd, OpSubtract, OpMultiply, OpDivide only. We got {binary_operator:?}."
                ),
            };
            self.stack.push(Value::Number(result));
            return Ok(());
        } else {
            self.runtime_error("Operands mut be numbers.");
            return Err(InterpretError::RuntimeError);
        }
    }

    fn runtime_error(&self, message: &str) {
        println!("{}", message);
        let line = self.chunk.lines[self.ip];
        eprintln!("[line {}] in script", line)
    }
}

fn is_falsey(value: Value) -> bool {
    match value {
        Value::Nil => true,
        Value::Bool(boolean) => !boolean,
        _ => false,
    }
}

pub fn interpret(vm: &mut VM, source: &str) -> Result<(), InterpretError> {
    let chunk = compile(source)?;

    vm.chunk = chunk;
    vm.ip = 0;
    vm.run()
}

#[derive(Debug)]
pub enum InterpretError {
    CompileError,
    RuntimeError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_falsy() {
        assert!(is_falsey(Value::Nil));
        assert!(is_falsey(Value::Bool(false)));
        assert!(!is_falsey(Value::Bool(true)));
        assert!(!is_falsey(Value::Number(1.0)));
    }

    #[test]
    fn test_run_op_constant() {
        let mut vm = VM::new();
        let target_constant = vm.chunk.add_constant(Value::Number(1.2));
        vm.chunk.add_code(
            OpCode::OpConstant {
                index: target_constant,
            },
            1,
        );
        // should be added or target_constant will be poped out of the stack
        let dummy_constant = vm.chunk.add_constant(Value::Number(3.4));
        vm.chunk.add_code(
            OpCode::OpConstant {
                index: dummy_constant,
            },
            1,
        );
        vm.chunk.add_code(OpCode::OpReturn, 1);
        vm.run().unwrap();
        assert_eq!(vm.stack[0].as_number(), 1.2);
        assert_eq!(vm.ip, 3);
    }

    #[test]
    fn test_run_op_negate() {
        let mut vm = VM::new();
        let target_constant = vm.chunk.add_constant(Value::Number(1.2));
        vm.chunk.add_code(
            OpCode::OpConstant {
                index: target_constant,
            },
            1,
        );
        // should be added or target_constant will be poped out of the stack
        vm.chunk.add_code(OpCode::OpNegate, 1);
        let dummy_constant = vm.chunk.add_constant(Value::Number(3.4));
        vm.chunk.add_code(
            OpCode::OpConstant {
                index: dummy_constant,
            },
            1,
        );
        vm.chunk.add_code(OpCode::OpReturn, 1);
        vm.run().unwrap();
        assert_eq!(vm.stack[0].as_number(), -1.2);
        assert_eq!(vm.ip, 4);
    }

    mod binary_operation {
        use super::*;

        #[test]
        fn test_add() {
            let mut vm = VM::new();
            vm.stack.push(Value::Number(1.2));
            vm.stack.push(Value::Number(3.4));
            vm.binary_operation(&OpCode::OpAdd).unwrap();
            assert_eq!(vm.stack[0].as_number(), 4.6);
        }

        #[test]
        fn test_subtract() {
            let mut vm = VM::new();
            vm.stack.push(Value::Number(1.2));
            vm.stack.push(Value::Number(3.4));
            vm.binary_operation(&OpCode::OpSubtract).unwrap();
            assert_eq!(vm.stack[0].as_number(), -2.2);
        }

        #[test]
        fn test_multiply() {
            let mut vm = VM::new();
            vm.stack.push(Value::Number(2.0));
            vm.stack.push(Value::Number(3.4));
            vm.binary_operation(&OpCode::OpMultiply).unwrap();
            assert_eq!(vm.stack[0].as_number(), 6.8);
        }

        #[test]
        fn test_divide() {
            let mut vm = VM::new();
            vm.stack.push(Value::Number(6.0));
            vm.stack.push(Value::Number(2.0));
            vm.binary_operation(&OpCode::OpDivide).unwrap();
            assert_eq!(vm.stack[0].as_number(), 3.0);
        }

        #[test]
        #[should_panic(
            expected = "Expected OpAdd, OpSubtract, OpMultiply, OpDivide only. We got OpReturn."
        )]
        fn test_invalid_opcode() {
            let mut vm = VM::new();
            vm.stack.push(Value::Number(6.0));
            vm.stack.push(Value::Number(2.0));
            vm.binary_operation(&OpCode::OpReturn).unwrap();
        }
    }
}
