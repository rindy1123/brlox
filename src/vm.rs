use std::{collections::HashMap, time::SystemTime};

use crate::{
    chunk::OpCode,
    compiler::compile,
    disassembler,
    value::{
        object::{NativeFunction, Obj, ObjFunction, ObjNative},
        Value,
    },
};

#[derive(Debug)]
struct CallFrame {
    function: ObjFunction,
    /// Instruction Pointer
    ip: usize,
    /// Index of the beginning of this frame on stack
    frame_pointer: usize,
}

impl CallFrame {
    fn new(function: ObjFunction, frame_pointer: usize) -> Self {
        CallFrame {
            function,
            frame_pointer,
            ip: 0,
        }
    }
}

pub struct VM {
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    globals: HashMap<String, Value>,
}

const DEBUG: bool = false;
const STACK_MAX: usize = 256;
const FRAMES_MAX: usize = 64;

impl VM {
    pub fn new() -> VM {
        let mut globals = HashMap::new();
        globals.insert("clock".to_string(), Self::define_native(Self::clock));
        VM {
            stack: Vec::with_capacity(STACK_MAX),
            frames: Vec::with_capacity(FRAMES_MAX),
            globals,
        }
    }

    fn run(&mut self) -> Result<(), InterpretError> {
        loop {
            let frame = self.frames.last_mut().unwrap();
            let instruction = &frame.function.chunk.code[frame.ip];
            if DEBUG {
                println!("      ");
                for slot in self.stack.clone() {
                    println!("[ {:#?} ]", slot);
                }
                disassembler::disassemble_instruction(
                    frame.ip,
                    frame.function.chunk.lines[frame.ip],
                    &instruction,
                );
            }
            frame.ip += 1;
            match instruction {
                OpCode::OpReturn => {
                    let result = self.stack.pop().unwrap();
                    let previous_frame_pointer = self.frames.pop().unwrap().frame_pointer;
                    // discard the values the frame had
                    self.stack.drain(previous_frame_pointer..);
                    if self.frames.len() == 0 {
                        return Ok(());
                    }
                    self.stack.push(result);
                }
                OpCode::OpNegate => match self.stack.last().unwrap().clone() {
                    Value::Number(number) => {
                        self.stack.pop().unwrap();
                        self.stack.push(Value::Number(-number));
                    }
                    _ => {
                        let message = "Operand must be a number.".to_string();
                        let err = InterpretError::RuntimeError(message);
                        return Err(err);
                    }
                },
                OpCode::OpConstant { index } => {
                    let constant = frame.function.chunk.constants[*index].clone();
                    self.stack.push(constant);
                }
                OpCode::OpNil => self.stack.push(Value::Nil),
                OpCode::OpTrue => self.stack.push(Value::Bool(true)),
                OpCode::OpFalse => self.stack.push(Value::Bool(false)),
                OpCode::OpNot => {
                    let value = self.stack.pop().unwrap();
                    self.stack.push(Value::Bool(is_falsey(value)));
                }
                OpCode::OpEqual => {
                    let right = self.stack.pop().unwrap();
                    let left = self.stack.pop().unwrap();
                    self.stack.push(Value::Bool(left.values_equal(right)));
                }
                OpCode::OpPrint => self.stack.pop().unwrap().println(),
                OpCode::OpPop => {
                    self.stack.pop();
                }
                OpCode::OpDefineGlobal { index } => {
                    let name = frame.function.chunk.constants[*index].clone().as_string();
                    let value = self.stack.last().unwrap();
                    self.globals.insert(name, value.clone());
                    self.stack.pop();
                }
                OpCode::OpGetGlobal { index } => {
                    let name = frame.function.chunk.constants[*index].clone().as_string();
                    match self.globals.get(&name) {
                        Some(value) => {
                            self.stack.push(value.clone());
                        }
                        _ => {
                            let message = format!("Undefined variable '{}'", name);
                            let err = InterpretError::RuntimeError(message);
                            return Err(err);
                        }
                    }
                }
                OpCode::OpGetLocal { index } => self
                    .stack
                    .push(self.stack[frame.frame_pointer + index].clone()),
                OpCode::OpSetGlobal { index } => {
                    let name = frame.function.chunk.constants[*index].clone().as_string();
                    let value = self.stack.last().unwrap().clone();
                    match self.globals.insert(name.clone(), value) {
                        None => {
                            self.globals.remove(&name);
                            let message = format!("Undefined variable '{}'", name);
                            let err = InterpretError::RuntimeError(message);
                            return Err(err);
                        }
                        _ => {}
                    }
                }
                OpCode::OpSetLocal { index } => {
                    self.stack[frame.frame_pointer + index] = self.stack.last().unwrap().clone();
                }
                OpCode::OpJumpIfFalse { offset } => {
                    let value = self.stack.last().unwrap().clone();
                    if is_falsey(value) {
                        frame.ip += offset;
                    }
                }
                OpCode::OpJump { offset } => {
                    frame.ip += offset;
                }
                OpCode::OpJumpBack { offset } => {
                    frame.ip -= offset;
                }
                OpCode::OpAdd
                | OpCode::OpSubtract
                | OpCode::OpMultiply
                | OpCode::OpDivide
                | OpCode::OpGreater
                | OpCode::OpLess => {
                    Self::binary_operation(&mut self.stack, instruction)?;
                }
                OpCode::OpCall { arg_count } => {
                    let function = self.stack[self.stack.len() - 1 - arg_count].clone();
                    let arg_count = arg_count.clone();
                    let ip = frame.ip;
                    self.call_value(function, arg_count, ip)?;
                }
            }
        }
    }

    fn binary_operation(
        stack: &mut Vec<Value>,
        binary_operator: &OpCode,
    ) -> Result<(), InterpretError> {
        let stack_len = stack.len();
        match (&stack[stack_len - 1], &stack[stack_len - 2]) {
            (Value::Number(right), Value::Number(left)) => {
                let result = match binary_operator {
                    OpCode::OpAdd => Value::Number(left + right),
                    OpCode::OpSubtract => Value::Number(left - right),
                    OpCode::OpMultiply => Value::Number(left * right),
                    OpCode::OpDivide => Value::Number(left / right),
                    OpCode::OpGreater => Value::Bool(left > right),
                    OpCode::OpLess => Value::Bool(left < right),
                    _ => panic!("We got {binary_operator:?}."),
                };
                stack.pop().unwrap();
                stack.pop().unwrap();
                stack.push(result);
                return Ok(());
            }
            (Value::LString(right), Value::LString(left)) => {
                let result = match binary_operator {
                    OpCode::OpAdd => Value::LString(format!("{left}{right}")),
                    OpCode::OpSubtract
                    | OpCode::OpMultiply
                    | OpCode::OpDivide
                    | OpCode::OpGreater
                    | OpCode::OpLess => {
                        let message = "You cannot use that operator for strings.".to_string();
                        let err = InterpretError::RuntimeError(message);
                        return Err(err);
                    }
                    _ => panic!("We got {binary_operator:?}."),
                };
                stack.pop().unwrap();
                stack.pop().unwrap();
                stack.push(result);
                return Ok(());
            }
            (_, _) => {
                let message = "Operands must be two numbers or two strings.".to_string();
                let err = InterpretError::RuntimeError(message);
                return Err(err);
            }
        }
    }

    fn call_value(
        &mut self,
        callee: Value,
        arg_count: usize,
        ip: usize,
    ) -> Result<(), InterpretError> {
        if let Value::Obj(obj) = callee {
            match obj {
                Obj::Function(function) => {
                    return self.call(function, arg_count);
                }
                Obj::NativeFunction(function) => {
                    let native_function = function.native_function;
                    let result = native_function(arg_count, ip);
                    let stack_tail = self.stack.len() - 1;
                    // remove argument values and function from stack
                    self.stack.drain((stack_tail - arg_count)..);
                    self.stack.push(result);
                    return Ok(());
                }
            }
        }
        let message = "Can only call functions and classes.".to_string();
        let err = InterpretError::RuntimeError(message);
        return Err(err);
    }

    fn call(&mut self, function: ObjFunction, arg_count: usize) -> Result<(), InterpretError> {
        let arity = function.arity;
        if arg_count != arity {
            let message = format!("Expected {arity} arguments but got {arg_count}.");
            let err = InterpretError::RuntimeError(message);
            return Err(err);
        }
        if self.frames.len() == FRAMES_MAX {
            let message = "Stack overflow.".to_string();
            let err = InterpretError::RuntimeError(message);
            return Err(err);
        }
        let stack_size = self.stack.len() - 1;
        let frame = CallFrame::new(function, stack_size - arg_count);
        self.frames.push(frame);
        Ok(())
    }

    fn define_native(function: NativeFunction) -> Value {
        let obj_native = ObjNative::new(function);
        let native_function = Obj::NativeFunction(obj_native);
        Value::Obj(native_function)
    }

    /// Native Function
    fn clock(_: usize, _: usize) -> Value {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        Value::Number(now.as_secs_f64())
    }

    fn runtime_error(&self, message: &str) {
        println!("{}", message);
        for frame in self.frames.iter().rev() {
            let function = &frame.function;
            eprint!("[line {}] in ", function.chunk.lines[frame.ip]);
            if function.name.len() == 0 {
                eprintln!("script");
            } else {
                eprintln!("{}()", function.name);
            }
        }
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
    let function = compile(source)?;

    let frame = CallFrame::new(function, 0);
    vm.frames.push(frame);
    if let Err(err) = vm.run() {
        match err {
            InterpretError::RuntimeError(ref message) => {
                vm.runtime_error(&message);
                return Err(err);
            }
            _ => panic!("Not supposed to raise other than RuntimeError"),
        }
    }
    Ok(())
}

#[derive(Debug)]
pub enum InterpretError {
    CompileError,
    RuntimeError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    mod interpret {
        use std::fs;

        use super::*;

        fn execute_file(path: &str) -> Result<(), InterpretError> {
            let source = fs::read_to_string(path).unwrap();
            let mut vm = VM::new();
            interpret(&mut vm, &source)
        }

        #[test]
        fn test_logical_ops() {
            assert!(execute_file("samples/logical_ops.lox").is_ok())
        }

        #[test]
        fn test_variables() {
            assert!(execute_file("samples/variables.lox").is_ok())
        }

        #[test]
        fn test_self_reference_variable() {
            assert!(execute_file("samples/self_reference_variable.lox").is_err())
        }

        #[test]
        fn test_if_statement() {
            assert!(execute_file("samples/if_statement.lox").is_ok())
        }

        #[test]
        fn test_loops() {
            assert!(execute_file("samples/loops.lox").is_ok())
        }

        #[test]
        fn test_function() {
            assert!(execute_file("samples/function.lox").is_ok())
        }

        #[test]
        fn test_closure() {
            assert!(execute_file("samples/closure.lox").is_ok())
        }
    }

    #[test]
    fn test_is_falsy() {
        assert!(is_falsey(Value::Nil));
        assert!(is_falsey(Value::Bool(false)));
        assert!(!is_falsey(Value::Bool(true)));
        assert!(!is_falsey(Value::Number(1.0)));
    }

    mod binary_operation {
        use super::*;

        #[test]
        fn test_add_num() {
            let mut stack = Vec::new();
            stack.push(Value::Number(1.2));
            stack.push(Value::Number(3.4));
            VM::binary_operation(&mut stack, &OpCode::OpAdd).unwrap();
            assert_eq!(stack[0].as_number(), 4.6);
        }

        #[test]
        fn test_add_string() {
            let mut stack = Vec::new();
            stack.push(Value::LString("AAA".to_string()));
            stack.push(Value::LString("BBB".to_string()));
            VM::binary_operation(&mut stack, &OpCode::OpAdd).unwrap();
            assert_eq!(stack[0].as_string(), "AAABBB".to_string());
        }

        #[test]
        fn test_subtract() {
            let mut stack = Vec::new();
            stack.push(Value::Number(1.2));
            stack.push(Value::Number(3.4));
            VM::binary_operation(&mut stack, &OpCode::OpSubtract).unwrap();
            assert_eq!(stack[0].as_number(), -2.2);
        }

        #[test]
        fn test_multiply() {
            let mut stack = Vec::new();
            stack.push(Value::Number(2.0));
            stack.push(Value::Number(3.4));
            VM::binary_operation(&mut stack, &OpCode::OpMultiply).unwrap();
            assert_eq!(stack[0].as_number(), 6.8);
        }

        #[test]
        fn test_divide() {
            let mut stack = Vec::new();
            stack.push(Value::Number(6.0));
            stack.push(Value::Number(2.0));
            VM::binary_operation(&mut stack, &OpCode::OpDivide).unwrap();
            assert_eq!(stack[0].as_number(), 3.0);
        }

        #[test]
        #[should_panic(expected = "We got OpReturn.")]
        fn test_invalid_opcode() {
            let mut stack = Vec::new();
            stack.push(Value::Number(6.0));
            stack.push(Value::Number(2.0));
            VM::binary_operation(&mut stack, &OpCode::OpReturn).unwrap();
        }
    }
}
