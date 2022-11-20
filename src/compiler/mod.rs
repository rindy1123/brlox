mod error_report;
mod parser;
mod precedence;

use crate::{
    chunk::{Chunk, OpCode},
    disassembler,
    scan::Source,
    token::Token,
    value::{
        object::{Obj, ObjFunction},
        Value,
    },
    InterpretError,
};
use parser::Parser;

const DEBUG: bool = false;

#[derive(Clone, Debug)]
struct Env {
    // Local Variables are stored here
    locals: Vec<Local>,
    // In global env, scope_depth will be 0
    scope_depth: usize,
    local_count: usize,
}

impl Env {
    fn new() -> Env {
        Env {
            locals: Vec::new(),
            scope_depth: 0,
            local_count: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub enum FunctionType {
    Function,
    Script,
}

#[derive(Clone, Debug)]
struct Local {
    name: Token,
    /// Env's scope_depth when the local variable is defined.
    depth: usize,
    // Just in case the defined local variable is initialized with itself,
    // it holds whether it's in initialized state or not.
    // Check out sample/self_reference_variable.lox to see the example.
    initialized: bool,
}

impl Local {
    fn new(name: Token, depth: usize) -> Local {
        Local {
            name,
            depth,
            initialized: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Compiler {
    env: Env,
    pub function: ObjFunction,
    function_type: FunctionType,
}

impl Compiler {
    fn new(function_type: FunctionType) -> Compiler {
        Compiler {
            env: Env::new(),
            function: ObjFunction::new(),
            function_type,
        }
    }

    fn add_local(&mut self, token: Token) {
        let local = Local::new(token, self.env.scope_depth);
        self.env.locals.push(local);
        self.env.local_count += 1
    }

    fn mark_initialized(&mut self) {
        // other than local variable
        if self.env.scope_depth == 0 {
            return;
        }
        // local variable
        let mut initialized_local_variable = &mut self.env.locals[self.env.local_count - 1];
        initialized_local_variable.initialized = true;
    }

    fn begin_scope(&mut self) {
        self.env.scope_depth += 1
    }

    fn end_scope(&mut self, line: usize) {
        self.env.scope_depth -= 1;
        while self.env.local_count > 0
            && self.env.locals[self.env.local_count - 1].depth > self.env.scope_depth
        {
            self.emit_byte(OpCode::OpPop, line);
            self.env.local_count -= 1
        }
    }

    fn current_chunk_as_mut(&mut self) -> &mut Chunk {
        &mut self.function.chunk
    }

    fn current_chunk_as_ref(&self) -> &Chunk {
        &self.function.chunk
    }

    fn check_variable_already_exists(&self, variable_name: &Token) -> Result<(), InterpretError> {
        for local in self.env.locals.iter().rev() {
            if local.depth < self.env.scope_depth {
                break;
            }

            if variable_name.lexeme == local.name.lexeme {
                error_report::report_error(
                    variable_name,
                    "Already a variable with this name in this scope.",
                );
                return Err(InterpretError::CompileError);
            }
        }
        Ok(())
    }

    fn is_local(&self) -> bool {
        self.env.scope_depth > 0
    }

    // TODO: refactor
    fn identifier_constant(&mut self, name: String) -> usize {
        let chunk = self.current_chunk_as_mut();
        chunk.add_constant(Value::LString(name))
    }

    fn resolve_local(&mut self, name: &Token) -> Result<Option<usize>, InterpretError> {
        let locals_len = self.env.locals.len();
        for (i, local) in self.env.locals.iter().rev().enumerate() {
            if name.lexeme == local.name.lexeme {
                if !local.initialized {
                    error_report::report_error(
                        &name,
                        "Can't read local variable in own initializer",
                    );
                    return Err(InterpretError::CompileError);
                }
                return Ok(Some(locals_len - i));
            }
        }

        Ok(None)
    }

    fn declare_variable(&mut self, name: &Token) -> Result<(), InterpretError> {
        if !self.is_local() {
            return Ok(());
        }

        self.check_variable_already_exists(name)?;
        self.add_local(name.clone());
        Ok(())
    }

    fn define_local_variable(&mut self) {
        self.mark_initialized();
    }

    fn define_global_variable(&mut self, global: usize, line: usize) {
        self.emit_byte(OpCode::OpDefineGlobal { index: global }, line);
    }

    /// Patch the jump instruction
    /// jump_start is the jump instruction's address which emit_jump returns
    fn patch_jump(&mut self, jump_start: usize) {
        let code = &mut self.current_chunk_as_mut().code;
        let offset = code.len() - 1 - jump_start;
        let target = code[jump_start].clone();
        code[jump_start] = match target {
            OpCode::OpJumpIfFalse { .. } => OpCode::OpJumpIfFalse { offset },
            OpCode::OpJump { .. } => OpCode::OpJump { offset },
            _ => panic!("Expected jump op code"),
        }
    }

    fn emit_byte(&mut self, byte: OpCode, line: usize) {
        self.current_chunk_as_mut().add_code(byte, line)
    }

    pub fn emit_constant(&mut self, value: Value, line: usize) {
        let constant = self.current_chunk_as_mut().add_constant(value);
        self.emit_byte(OpCode::OpConstant { index: constant }, line)
    }

    /// Returns the jump instruction's address to patch the jump instruction later
    fn emit_jump(&mut self, instruction: OpCode, line: usize) -> usize {
        self.emit_byte(instruction, line);
        self.current_chunk_as_ref().code.len() - 1
    }

    fn emit_jump_back(&mut self, jump_back_address: usize, line: usize) {
        let code_size = self.current_chunk_as_ref().code.len();
        let offset = code_size - jump_back_address;
        self.emit_byte(OpCode::OpJumpBack { offset }, line);
    }

    fn emit_pop(&mut self, line: usize) {
        self.emit_byte(OpCode::OpPop, line);
    }

    fn end_compiler(&mut self, line: usize) -> ObjFunction {
        self.emit_byte(OpCode::OpNil, line);
        self.emit_byte(OpCode::OpReturn, line);
        self.function.clone()
    }
}

pub fn compile(source: &str) -> Result<ObjFunction, InterpretError> {
    let source = Source::new(source.to_string());
    let mut root_compiler = Compiler::new(FunctionType::Script);
    let function = Obj::Function(root_compiler.function.clone());
    root_compiler.emit_constant(Value::Obj(function), 0);
    let mut parser = Parser::new(source, root_compiler);
    let mut compiler = parser.parse()?;
    let function = compiler.end_compiler(parser.previous.unwrap().line);
    if DEBUG {
        disassembler::Disassembler::disassemble_chunk(&function.chunk, "code".to_string());
    }
    Ok(function)
}
