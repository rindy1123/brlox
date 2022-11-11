use crate::{
    chunk::{Chunk, OpCode},
    scan::{self, Source},
    token::{Token, TokenType},
    value::{object::ObjFunction, Value},
    vm::InterpretError,
};

use super::precedence::{self, ParseFn, Precedence};

struct Env {
    locals: Vec<Local>,
    scope_depth: usize,
    local_count: usize,
}

impl Env {
    fn new() -> Env {
        // TODO: consider TokenType
        // let token = Token::new(TokenType::EOF, String::new(), 0);
        // let local = Local::new(token, Some(0));
        // let locals = vec![local];
        Env {
            locals: Vec::new(),
            scope_depth: 0,
            local_count: 0,
        }
    }
}

enum FunctionType {
    Function,
    Script,
}

#[derive(Debug)]
struct Local {
    name: Token,
    depth: Option<usize>,
}

impl Local {
    fn new(name: Token, depth: Option<usize>) -> Local {
        Local { name, depth }
    }
}

pub struct Parser {
    current: Option<Token>,
    pub previous: Option<Token>,
    source: Source,
    pub chunk: Chunk,
    env: Env,
    pub function: ObjFunction,
    function_type: FunctionType,
}

impl Parser {
    pub fn new(source: Source, chunk: Chunk) -> Parser {
        Parser {
            current: None,
            previous: None,
            env: Env::new(),
            function_type: FunctionType::Script,
            function: ObjFunction::new(),
            source,
            chunk,
        }
    }

    fn current_chunk_as_mut(&mut self) -> &mut Chunk {
        &mut self.function.chunk
    }

    fn current_chunk_as_ref(&self) -> &Chunk {
        &self.function.chunk
    }

    pub fn parse(&mut self) -> Result<(), InterpretError> {
        self.advance()?;
        while !self.match_token_type(TokenType::EOF) {
            self.declaration()?;
        }
        // consume EOF
        self.advance()?;
        Ok(())
    }

    fn declaration(&mut self) -> Result<(), InterpretError> {
        if self.match_token_type(TokenType::Var) {
            self.advance()?;
            return self.var_declaration();
        }
        self.statement()
    }

    fn var_declaration(&mut self) -> Result<(), InterpretError> {
        let global = self.parse_variable()?;

        if self.match_token_type(TokenType::Equal) {
            // When variable is initialized
            self.advance()?;
            self.expression()?;
        } else {
            // When variable is not initialized, variable should hold nil
            let line = self.previous.as_ref().unwrap().line;
            let chunk = self.current_chunk_as_mut();
            chunk_op::emit_byte(OpCode::OpNil, chunk, line);
        }

        if !self.match_token_type(TokenType::Semicolon) {
            report_error(
                self.current.as_ref().unwrap(),
                "Expect ';' after expression.",
            );
            return Err(InterpretError::CompileError);
        }
        self.advance()?;
        self.define_variable(global);
        Ok(())
    }

    fn parse_variable(&mut self) -> Result<usize, InterpretError> {
        if !self.match_token_type(TokenType::Identifier) {
            report_error(self.current.as_ref().unwrap(), "Expect variable name.");
            return Err(InterpretError::CompileError);
        }
        self.advance()?;
        self.declare_variable()?;
        if self.env.scope_depth > 0 {
            return Ok(0);
        }

        let previous_token = self.previous.clone().unwrap();
        Ok(self.identifier_constant(previous_token))
    }

    fn declare_variable(&mut self) -> Result<(), InterpretError> {
        if self.env.scope_depth == 0 {
            return Ok(());
        }

        let name = self.previous.clone().unwrap();
        for local in self.env.locals.iter().rev() {
            if local.depth.unwrap() < self.env.scope_depth {
                break;
            }

            if name.lexeme == local.name.lexeme {
                report_error(&name, "Already a variable with this name in this scope.");
                return Err(InterpretError::CompileError);
            }
        }
        self.add_local(name);
        Ok(())
    }

    fn add_local(&mut self, token: Token) {
        let local = Local::new(token, None);
        self.env.locals.push(local);
        self.env.local_count += 1
    }

    fn identifier_constant(&mut self, name: Token) -> usize {
        let chunk = self.current_chunk_as_mut();
        chunk.add_constant(Value::LString(name.lexeme))
    }

    fn define_variable(&mut self, global: usize) {
        if self.env.scope_depth > 0 {
            self.mark_initialized();
            return;
        }

        let previous_token = self.previous.clone().unwrap();
        let chunk = self.current_chunk_as_mut();
        chunk_op::emit_byte(
            OpCode::OpDefineGlobal { index: global },
            chunk,
            previous_token.line,
        );
    }

    fn mark_initialized(&mut self) {
        self.env.locals[self.env.local_count - 1].depth = Some(self.env.scope_depth);
    }

    fn statement(&mut self) -> Result<(), InterpretError> {
        if self.match_token_type(TokenType::Print) {
            self.advance()?;
            return self.print_statement();
        } else if self.match_token_type(TokenType::If) {
            self.advance()?;
            return self.if_statement();
        } else if self.match_token_type(TokenType::While) {
            self.advance()?;
            return self.while_statement();
        } else if self.match_token_type(TokenType::For) {
            self.advance()?;
            return self.for_statement();
        } else if self.match_token_type(TokenType::LeftBrace) {
            self.advance()?;
            self.begin_scope();
            self.block()?;
            self.end_scope();
            return Ok(());
        }
        self.expression_statement()
    }

    fn begin_scope(&mut self) {
        self.env.scope_depth += 1
    }

    fn end_scope(&mut self) {
        self.env.scope_depth -= 1;
        let previous_token = self.previous.clone().unwrap();
        while self.env.local_count > 0
            && self.env.locals[self.env.local_count - 1].depth.unwrap() > self.env.scope_depth
        {
            let chunk = self.current_chunk_as_mut();
            chunk_op::emit_byte(OpCode::OpPop, chunk, previous_token.line);
            self.env.local_count -= 1
        }
    }

    fn block(&mut self) -> Result<(), InterpretError> {
        while !self.match_token_type(TokenType::RightBrace)
            && !self.match_token_type(TokenType::EOF)
        {
            self.declaration()?;
        }

        if !self.match_token_type(TokenType::RightBrace) {
            report_error(self.current.as_ref().unwrap(), "Expect '}' after block.");
            return Err(InterpretError::CompileError);
        }
        self.advance()
    }

    fn for_statement(&mut self) -> Result<(), InterpretError> {
        self.begin_scope();
        if !self.match_token_type(TokenType::LeftParen) {
            report_error(self.current.as_ref().unwrap(), "Expect '(' after if.");
            return Err(InterpretError::CompileError);
        }
        self.advance()?;
        if self.match_token_type(TokenType::Semicolon) {
            self.advance()?;
        } else if self.match_token_type(TokenType::Var) {
            self.advance()?;
            self.var_declaration()?;
        } else {
            self.expression_statement()?;
        }
        let mut loop_start = self.current_chunk_as_ref().code.len() - 1;
        let exit_jump = if !self.match_token_type(TokenType::Semicolon) {
            self.expression()?;
            if !self.match_token_type(TokenType::Semicolon) {
                report_error(
                    self.current.as_ref().unwrap(),
                    "Expect ';' after condition.",
                );
                return Err(InterpretError::CompileError);
            }
            self.advance()?;

            let jump = self.emit_jump(OpCode::OpJumpIfFalse { offset: 0 });
            self.emit_pop();
            Some(jump)
        } else {
            self.advance()?;
            None
        };

        if !self.match_token_type(TokenType::RightParen) {
            let body_jump = self.emit_jump(OpCode::OpJump { offset: 0 });
            let increment_start = self.current_chunk_as_ref().code.len() - 1;
            self.expression()?;
            self.emit_pop();
            if !self.match_token_type(TokenType::RightParen) {
                report_error(
                    self.current.as_ref().unwrap(),
                    "Expect ')' after condition.",
                );
                return Err(InterpretError::CompileError);
            }
            self.advance()?;

            self.emit_loop(loop_start);
            loop_start = increment_start;
            self.patch_jump(body_jump);
        } else {
            self.advance()?;
        }
        self.statement()?;

        self.emit_loop(loop_start);
        if let Some(jump) = exit_jump {
            self.patch_jump(jump);
            self.emit_pop();
        }
        self.end_scope();
        Ok(())
    }

    fn while_statement(&mut self) -> Result<(), InterpretError> {
        let code_size = self.current_chunk_as_ref().code.len();
        let loop_start = code_size - 1;
        if !self.match_token_type(TokenType::LeftParen) {
            report_error(self.current.as_ref().unwrap(), "Expect '(' after if.");
            return Err(InterpretError::CompileError);
        }
        self.advance()?;
        self.expression()?;
        if !self.match_token_type(TokenType::RightParen) {
            report_error(
                self.current.as_ref().unwrap(),
                "Expect ')' after condition.",
            );
            return Err(InterpretError::CompileError);
        }
        self.advance()?;
        let exit_jump = self.emit_jump(OpCode::OpJumpIfFalse { offset: 0 });
        self.emit_pop();
        self.statement()?;

        self.emit_loop(loop_start);
        self.patch_jump(exit_jump);
        self.emit_pop();
        Ok(())
    }

    fn emit_loop(&mut self, loop_start: usize) {
        let code_size = self.current_chunk_as_ref().code.len();
        let offset = code_size - loop_start;
        let previous_token = self.previous.clone().unwrap();
        let chunk = self.current_chunk_as_mut();
        chunk_op::emit_byte(OpCode::OpLoop { offset }, chunk, previous_token.line);
    }

    fn if_statement(&mut self) -> Result<(), InterpretError> {
        if !self.match_token_type(TokenType::LeftParen) {
            report_error(self.current.as_ref().unwrap(), "Expect '(' after if.");
            return Err(InterpretError::CompileError);
        }
        self.advance()?;
        self.expression()?;
        if !self.match_token_type(TokenType::RightParen) {
            report_error(
                self.current.as_ref().unwrap(),
                "Expect ')' after condition.",
            );
            return Err(InterpretError::CompileError);
        }
        self.advance()?;
        let then_jump = self.emit_jump(OpCode::OpJumpIfFalse { offset: 0 });
        self.emit_pop();
        self.statement()?;

        let else_jump = self.emit_jump(OpCode::OpJump { offset: 0 });
        self.patch_jump(then_jump);
        self.emit_pop();
        if self.match_token_type(TokenType::Else) {
            self.advance()?;
            self.statement()?;
        }
        self.patch_jump(else_jump);
        Ok(())
    }

    fn emit_jump(&mut self, instruction: OpCode) -> usize {
        let line = self.previous.as_ref().unwrap().line;
        let chunk = self.current_chunk_as_mut();
        chunk_op::emit_byte(instruction, chunk, line);
        chunk.code.len() - 1
    }

    fn emit_pop(&mut self) {
        let previous_token = self.previous.clone().unwrap();
        let chunk = self.current_chunk_as_mut();
        chunk_op::emit_byte(OpCode::OpPop, chunk, previous_token.line);
    }

    fn patch_jump(&mut self, offset: usize) {
        let code = &mut self.current_chunk_as_mut().code;
        let jump = code.len() - 1 - offset;
        let target = code[offset].clone();
        code[offset] = match target {
            OpCode::OpJumpIfFalse { .. } => OpCode::OpJumpIfFalse { offset: jump },
            OpCode::OpJump { .. } => OpCode::OpJump { offset: jump },
            _ => panic!("Expected jump op code"),
        }
    }

    fn expression_statement(&mut self) -> Result<(), InterpretError> {
        self.expression()?;
        if !self.match_token_type(TokenType::Semicolon) {
            report_error(
                self.current.as_ref().unwrap(),
                "Expect ';' after expression.",
            );
            return Err(InterpretError::CompileError);
        }
        self.advance()?;
        self.emit_pop();
        Ok(())
    }

    fn print_statement(&mut self) -> Result<(), InterpretError> {
        self.expression()?;
        if !self.match_token_type(TokenType::Semicolon) {
            report_error(self.current.as_ref().unwrap(), "Expect ';' after value.");
            return Err(InterpretError::CompileError);
        }
        self.advance()?;
        let line = self.previous.as_ref().unwrap().line;
        let chunk = self.current_chunk_as_mut();
        chunk_op::emit_byte(OpCode::OpPrint, chunk, line);
        Ok(())
    }

    /// Consume the next token from self.source.
    /// self.previous will be the current token and self.current will be the next token.
    pub fn advance(&mut self) -> Result<(), InterpretError> {
        self.previous = self.current.clone();
        let token = scan::scan_token(&mut self.source);
        if let TokenType::Error = token.token_type {
            report_error(&token, &token.lexeme);
            return Err(InterpretError::CompileError);
        }
        self.current = Some(token);
        Ok(())
    }

    pub fn expression(&mut self) -> Result<(), InterpretError> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> Result<(), InterpretError> {
        self.advance()?;
        let previous_token = self.previous.as_ref().unwrap();
        let can_assign = precedence.clone() as i32 <= Precedence::Assignment as i32;
        match precedence::get_rule(previous_token.token_type.clone()).prefix {
            None => {
                report_error(previous_token, "Expect expression");
                return Err(InterpretError::CompileError);
            }
            Some(prefix_rule) => self.exec_parse_function(prefix_rule, can_assign)?,
        };

        while precedence.clone() as i32
            <= precedence::get_rule(self.current.clone().unwrap().token_type).precedence as i32
        {
            self.advance()?;
            let previous_token = self.previous.clone().unwrap();
            match precedence::get_rule(previous_token.token_type).infix {
                None => break,
                Some(infix) => self.exec_parse_function(infix, can_assign)?,
            };
        }

        if can_assign && self.match_token_type(TokenType::Equal) {
            self.advance()?;
            let previous_token = self.previous.as_ref().unwrap();
            report_error(previous_token, "Invalid assignment target.");
            return Err(InterpretError::CompileError);
        }
        Ok(())
    }

    fn binary(&mut self) -> Result<(), InterpretError> {
        let previous_token = self.previous.clone().unwrap();
        let operator_type = previous_token.token_type;
        let rule = precedence::get_rule(operator_type.clone());
        let precedence = num::FromPrimitive::from_i32(rule.precedence as i32 + 1).unwrap();
        self.parse_precedence(precedence)?;

        let chunk = self.current_chunk_as_mut();
        match operator_type {
            TokenType::Plus => chunk_op::emit_byte(OpCode::OpAdd, chunk, previous_token.line),
            TokenType::Minus => chunk_op::emit_byte(OpCode::OpSubtract, chunk, previous_token.line),
            TokenType::Star => chunk_op::emit_byte(OpCode::OpMultiply, chunk, previous_token.line),
            TokenType::Slash => chunk_op::emit_byte(OpCode::OpDivide, chunk, previous_token.line),
            TokenType::BangEqual => {
                chunk_op::emit_byte(OpCode::OpEqual, chunk, previous_token.line);
                chunk_op::emit_byte(OpCode::OpNot, chunk, previous_token.line);
            }
            TokenType::EqualEqual => {
                chunk_op::emit_byte(OpCode::OpEqual, chunk, previous_token.line);
            }
            TokenType::Greater => {
                chunk_op::emit_byte(OpCode::OpGreater, chunk, previous_token.line);
            }
            TokenType::GreaterEqual => {
                chunk_op::emit_byte(OpCode::OpLess, chunk, previous_token.line);
                chunk_op::emit_byte(OpCode::OpNot, chunk, previous_token.line);
            }
            TokenType::Less => {
                chunk_op::emit_byte(OpCode::OpLess, chunk, previous_token.line);
            }
            TokenType::LessEqual => {
                chunk_op::emit_byte(OpCode::OpGreater, chunk, previous_token.line);
                chunk_op::emit_byte(OpCode::OpNot, chunk, previous_token.line);
            }
            _ => (),
        }
        Ok(())
    }

    fn grouping(&mut self) -> Result<(), InterpretError> {
        self.expression()?;
        if !self.match_token_type(TokenType::RightParen) {
            report_error(
                self.current.as_ref().unwrap(),
                "Expect ')' after expression.",
            );
            return Err(InterpretError::CompileError);
        }
        self.advance()
    }

    fn unary(&mut self) -> Result<(), InterpretError> {
        let previous_token = self.previous.clone().unwrap();
        let operator_type = previous_token.token_type;

        // Compile the operand
        self.parse_precedence(Precedence::Unary)?;

        let chunk = self.current_chunk_as_mut();
        // Emit the operator instruction
        match operator_type {
            TokenType::Minus => chunk_op::emit_byte(OpCode::OpNegate, chunk, previous_token.line),
            TokenType::Bang => chunk_op::emit_byte(OpCode::OpNot, chunk, previous_token.line),
            _ => (),
        }
        Ok(())
    }

    fn exec_parse_function(
        &mut self,
        function_type: ParseFn,
        can_assign: bool,
    ) -> Result<(), InterpretError> {
        match function_type {
            ParseFn::Binary => self.binary(),
            ParseFn::Unary => self.unary(),
            ParseFn::Grouping => self.grouping(),
            ParseFn::Number => self.number(),
            ParseFn::Literal => self.literal(),
            ParseFn::String => self.string(),
            ParseFn::Variable => self.variable(can_assign),
            ParseFn::And => self.and(),
            ParseFn::Or => self.or(),
        }
    }

    fn variable(&mut self, can_assign: bool) -> Result<(), InterpretError> {
        let previous_token = self.previous.clone().unwrap();
        self.named_variable(previous_token, can_assign)?;
        Ok(())
    }

    fn named_variable(&mut self, name: Token, can_assign: bool) -> Result<(), InterpretError> {
        let arg = self.resolve_local(name.clone())?;
        let (get_op, set_op) = match arg {
            None => {
                let index = self.identifier_constant(name.clone());
                (OpCode::OpGetGlobal { index }, OpCode::OpSetGlobal { index })
            }
            Some(index) => (OpCode::OpGetLocal { index }, OpCode::OpSetLocal { index }),
        };
        if can_assign && self.match_token_type(TokenType::Equal) {
            self.advance()?;
            self.expression()?;
            let chunk = self.current_chunk_as_mut();
            chunk_op::emit_byte(set_op, chunk, name.line);
        } else {
            let chunk = self.current_chunk_as_mut();
            chunk_op::emit_byte(get_op, chunk, name.line);
        }
        Ok(())
    }

    fn resolve_local(&mut self, name: Token) -> Result<Option<usize>, InterpretError> {
        let locals_len = self.env.locals.len();
        for (i, local) in self.env.locals.iter().rev().enumerate() {
            if name.lexeme == local.name.lexeme {
                if let None = local.depth {
                    report_error(&name, "Can't read local variable in own initializer");
                    return Err(InterpretError::CompileError);
                }
                return Ok(Some(locals_len - 1 - i));
            }
        }

        Ok(None)
    }

    fn number(&mut self) -> Result<(), InterpretError> {
        let token = self.previous.clone().unwrap();
        let value = token.lexeme.parse::<f64>().unwrap();
        let chunk = self.current_chunk_as_mut();
        chunk_op::emit_constant(Value::Number(value), chunk, token.line);
        Ok(())
    }

    fn literal(&mut self) -> Result<(), InterpretError> {
        let token = self.previous.clone().unwrap();
        let chunk = self.current_chunk_as_mut();
        match token.token_type {
            TokenType::False => chunk_op::emit_byte(OpCode::OpFalse, chunk, token.line),
            TokenType::Nil => chunk_op::emit_byte(OpCode::OpNil, chunk, token.line),
            TokenType::True => chunk_op::emit_byte(OpCode::OpTrue, chunk, token.line),
            _ => panic!("Expected literal"),
        }
        Ok(())
    }

    fn string(&mut self) -> Result<(), InterpretError> {
        let token = self.previous.clone().unwrap();
        let value = token.lexeme;
        let chunk = self.current_chunk_as_mut();
        chunk_op::emit_constant(
            Value::LString(value[1..value.len() - 1].to_string()),
            chunk,
            token.line,
        );
        Ok(())
    }

    fn and(&mut self) -> Result<(), InterpretError> {
        let end_jump = self.emit_jump(OpCode::OpJumpIfFalse { offset: 0 });

        self.emit_pop();
        self.parse_precedence(Precedence::And)?;

        self.patch_jump(end_jump);
        Ok(())
    }

    fn or(&mut self) -> Result<(), InterpretError> {
        let else_jump = self.emit_jump(OpCode::OpJumpIfFalse { offset: 0 });
        let end_jump = self.emit_jump(OpCode::OpJump { offset: 0 });

        self.patch_jump(else_jump);
        self.emit_pop();
        self.parse_precedence(Precedence::Or)?;
        self.patch_jump(end_jump);
        Ok(())
    }

    fn match_token_type(&self, token_type: TokenType) -> bool {
        self.current.as_ref().unwrap().token_type == token_type
    }
}

pub mod chunk_op {
    use super::{Chunk, OpCode, Value};

    pub fn emit_byte(byte: OpCode, current_chunk: &mut Chunk, line: usize) {
        current_chunk.add_code(byte, line)
    }

    pub fn emit_constant(value: Value, chunk: &mut Chunk, line: usize) {
        let constant = chunk.add_constant(value);
        emit_byte(OpCode::OpConstant { index: constant }, chunk, line)
    }

    pub fn emit_return(current_chunk: &mut Chunk, line: usize) {
        emit_byte(OpCode::OpReturn, current_chunk, line)
    }
}

fn report_error(token: &Token, message: &str) {
    let position = match token.token_type {
        TokenType::EOF => "at end".to_string(),
        _ => format!("at '{}'", token.lexeme),
    };
    eprintln!("[line {}] Error {}: {}", token.line, position, message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advance() {
        let source = Source::new("1 + 1".to_string());
        let chunk = Chunk::new();
        let mut parser = Parser::new(source, chunk);
        let result = parser.advance().unwrap();
        assert_eq!(result, ());
    }

    #[test]
    fn test_expression() {
        let source = Source::new("1 + 1".to_string());
        let chunk = Chunk::new();
        let mut parser = Parser::new(source, chunk);
        parser.advance().unwrap();
        let result = parser.expression().unwrap();
        assert_eq!(result, ());
    }

    #[test]
    fn test_expression_failure() {
        let source = Source::new("+ 1".to_string());
        let chunk = Chunk::new();
        let mut parser = Parser::new(source, chunk);
        parser.advance().unwrap();
        let result = parser.expression();
        assert!(result.is_err());
    }
}
