use crate::{
    chunk::{Chunk, OpCode},
    scan::{self, Source},
    token::{Token, TokenType},
    value::Value,
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
        Env {
            locals: Vec::new(),
            scope_depth: 0,
            local_count: 0,
        }
    }
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
}

impl Parser {
    pub fn new(source: Source, chunk: Chunk) -> Parser {
        Parser {
            current: None,
            previous: None,
            env: Env::new(),
            source,
            chunk,
        }
    }

    pub fn parse(&mut self) -> Result<(), InterpretError> {
        self.advance()?;
        while !(self.match_token_type(TokenType::EOF)?) {
            self.declaration()?;
        }
        Ok(())
    }

    fn declaration(&mut self) -> Result<(), InterpretError> {
        if self.match_token_type(TokenType::Var)? {
            return self.var_declaration();
        }
        self.statement()
    }

    fn var_declaration(&mut self) -> Result<(), InterpretError> {
        let global = self.parse_variable("Expect variable name.")?;

        if self.match_token_type(TokenType::Equal)? {
            self.expression()?;
        } else {
            let previous_token = self.previous.clone().unwrap();
            chunk_op::emit_byte(OpCode::OpNil, &mut self.chunk, previous_token.line);
        }

        self.consume(TokenType::Semicolon, "Expect ';' after expression.")?;
        self.define_variable(global);
        Ok(())
    }

    fn parse_variable(&mut self, error_message: &str) -> Result<usize, InterpretError> {
        self.consume(TokenType::Identifier, error_message)?;
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
                self.report_error(
                    name.clone(),
                    "Already a variable with this name in this scope.",
                )?;
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
        self.chunk.add_constant(Value::LString(name.lexeme))
    }

    fn define_variable(&mut self, global: usize) {
        if self.env.scope_depth > 0 {
            self.mark_initialized();
            return;
        }

        let previous_token = self.previous.clone().unwrap();
        chunk_op::emit_byte(
            OpCode::OpDefineGlobal { index: global },
            &mut self.chunk,
            previous_token.line,
        );
    }

    fn mark_initialized(&mut self) {
        self.env.locals[self.env.local_count - 1].depth = Some(self.env.scope_depth);
    }

    fn statement(&mut self) -> Result<(), InterpretError> {
        if self.match_token_type(TokenType::Print)? {
            return self.print_statement();
        } else if self.match_token_type(TokenType::If)? {
            return self.if_statement();
        } else if self.match_token_type(TokenType::While)? {
            return self.while_statement();
        } else if self.match_token_type(TokenType::LeftBrace)? {
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
            chunk_op::emit_byte(OpCode::OpPop, &mut self.chunk, previous_token.line);
            self.env.local_count -= 1
        }
    }

    fn block(&mut self) -> Result<(), InterpretError> {
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::EOF) {
            self.declaration()?;
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block.")
    }

    fn while_statement(&mut self) -> Result<(), InterpretError> {
        let loop_start = self.chunk.code.len() - 1;
        self.consume(TokenType::LeftParen, "Expect '(' after if.")?;
        self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after condition.")?;
        let exit_jump = self.emit_jump(OpCode::OpJumpIfFalse { offset: 0 });
        self.emit_pop();
        self.statement()?;

        self.emit_loop(loop_start);
        self.patch_jump(exit_jump);
        self.emit_pop();
        Ok(())
    }

    fn emit_loop(&mut self, loop_start: usize) {
        let offset = self.chunk.code.len() - loop_start;
        let previous_token = self.previous.clone().unwrap();
        chunk_op::emit_byte(
            OpCode::OpLoop { offset },
            &mut self.chunk,
            previous_token.line,
        );
    }

    fn if_statement(&mut self) -> Result<(), InterpretError> {
        self.consume(TokenType::LeftParen, "Expect '(' after if.")?;
        self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after condition.")?;
        let then_jump = self.emit_jump(OpCode::OpJumpIfFalse { offset: 0 });
        self.emit_pop();
        self.statement()?;

        let else_jump = self.emit_jump(OpCode::OpJump { offset: 0 });
        self.patch_jump(then_jump);
        self.emit_pop();
        if self.match_token_type(TokenType::Else)? {
            self.statement()?;
        }
        self.patch_jump(else_jump);
        Ok(())
    }

    fn emit_jump(&mut self, instruction: OpCode) -> usize {
        let previous_token = self.previous.clone().unwrap();
        chunk_op::emit_byte(instruction, &mut self.chunk, previous_token.line);
        self.chunk.code.len() - 1
    }

    fn emit_pop(&mut self) {
        let previous_token = self.previous.clone().unwrap();
        chunk_op::emit_byte(OpCode::OpPop, &mut self.chunk, previous_token.line);
    }

    fn patch_jump(&mut self, offset: usize) {
        let jump = self.chunk.code.len() - 1 - offset;
        let target = self.chunk.code[offset].clone();
        self.chunk.code[offset] = match target {
            OpCode::OpJumpIfFalse { .. } => OpCode::OpJumpIfFalse { offset: jump },
            OpCode::OpJump { .. } => OpCode::OpJump { offset: jump },
            _ => panic!("Expected jump op code"),
        }
    }

    fn expression_statement(&mut self) -> Result<(), InterpretError> {
        self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after expression.")?;
        self.emit_pop();
        Ok(())
    }

    fn print_statement(&mut self) -> Result<(), InterpretError> {
        self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after value.")?;
        let previous_token = self.previous.clone().unwrap();
        chunk_op::emit_byte(OpCode::OpPrint, &mut self.chunk, previous_token.line);
        Ok(())
    }

    pub fn advance(&mut self) -> Result<(), InterpretError> {
        self.previous = self.current.clone();
        loop {
            let token = scan::scan_token(&mut self.source);
            self.current = Some(token.clone());
            if let TokenType::Error = token.token_type {
                self.report_error(token.clone(), &token.lexeme)?;
            } else {
                return Ok(());
            }
        }
    }

    pub fn expression(&mut self) -> Result<(), InterpretError> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> Result<(), InterpretError> {
        self.advance()?;
        let previous_token = self.previous.clone().unwrap();
        let can_assign = precedence.clone() as i32 <= Precedence::Assignment as i32;
        match precedence::get_rule(previous_token.token_type.clone()).prefix {
            None => self.report_error(previous_token, "Expect expression")?,
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

        if can_assign && self.match_token_type(TokenType::Equal)? {
            let previous_token = self.previous.clone().unwrap();
            self.report_error(previous_token, "Invalid assignment target.")?;
        }
        Ok(())
    }

    pub fn consume(&mut self, token_type: TokenType, message: &str) -> Result<(), InterpretError> {
        let current_token = self.current.clone().unwrap();
        if current_token.token_type == token_type {
            return self.advance();
        }
        self.report_error(current_token, message)
    }

    fn binary(&mut self) -> Result<(), InterpretError> {
        let previous_token = self.previous.clone().unwrap();
        let operator_type = previous_token.token_type;
        let rule = precedence::get_rule(operator_type.clone());
        let precedence = num::FromPrimitive::from_i32(rule.precedence as i32 + 1).unwrap();
        self.parse_precedence(precedence)?;

        match operator_type {
            TokenType::Plus => {
                chunk_op::emit_byte(OpCode::OpAdd, &mut self.chunk, previous_token.line)
            }
            TokenType::Minus => {
                chunk_op::emit_byte(OpCode::OpSubtract, &mut self.chunk, previous_token.line)
            }
            TokenType::Star => {
                chunk_op::emit_byte(OpCode::OpMultiply, &mut self.chunk, previous_token.line)
            }
            TokenType::Slash => {
                chunk_op::emit_byte(OpCode::OpDivide, &mut self.chunk, previous_token.line)
            }
            TokenType::BangEqual => {
                chunk_op::emit_byte(OpCode::OpEqual, &mut self.chunk, previous_token.line);
                chunk_op::emit_byte(OpCode::OpNot, &mut self.chunk, previous_token.line);
            }
            TokenType::EqualEqual => {
                chunk_op::emit_byte(OpCode::OpEqual, &mut self.chunk, previous_token.line);
            }
            TokenType::Greater => {
                chunk_op::emit_byte(OpCode::OpGreater, &mut self.chunk, previous_token.line);
            }
            TokenType::GreaterEqual => {
                chunk_op::emit_byte(OpCode::OpLess, &mut self.chunk, previous_token.line);
                chunk_op::emit_byte(OpCode::OpNot, &mut self.chunk, previous_token.line);
            }
            TokenType::Less => {
                chunk_op::emit_byte(OpCode::OpLess, &mut self.chunk, previous_token.line);
            }
            TokenType::LessEqual => {
                chunk_op::emit_byte(OpCode::OpGreater, &mut self.chunk, previous_token.line);
                chunk_op::emit_byte(OpCode::OpNot, &mut self.chunk, previous_token.line);
            }
            _ => (),
        }
        Ok(())
    }

    fn grouping(&mut self) -> Result<(), InterpretError> {
        self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after expression.")
    }

    fn unary(&mut self) -> Result<(), InterpretError> {
        let previous_token = self.previous.clone().unwrap();
        let operator_type = previous_token.token_type;

        // Compile the operand
        self.parse_precedence(Precedence::Unary)?;

        // Emit the operator instruction
        match operator_type {
            TokenType::Minus => {
                chunk_op::emit_byte(OpCode::OpNegate, &mut self.chunk, previous_token.line)
            }
            TokenType::Bang => {
                chunk_op::emit_byte(OpCode::OpNot, &mut self.chunk, previous_token.line)
            }
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

    fn report_error(&self, token: Token, message: &str) -> Result<(), InterpretError> {
        let position = match token.token_type {
            TokenType::EOF => "at end".to_string(),
            _ => format!("at '{}'", token.lexeme),
        };
        eprintln!("[line {}] Error {}: {}", token.line, position, message);
        Err(InterpretError::CompileError)
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
        if can_assign && self.match_token_type(TokenType::Equal)? {
            self.expression()?;
            chunk_op::emit_byte(set_op, &mut self.chunk, name.line);
        } else {
            chunk_op::emit_byte(get_op, &mut self.chunk, name.line);
        }
        Ok(())
    }

    fn resolve_local(&mut self, name: Token) -> Result<Option<usize>, InterpretError> {
        let locals_len = self.env.locals.len();
        for (i, local) in self.env.locals.iter().rev().enumerate() {
            if name.lexeme == local.name.lexeme {
                if let None = local.depth {
                    self.report_error(name, "Can't read local variable in own initializer")?;
                }
                return Ok(Some(locals_len - 1 - i));
            }
        }

        Ok(None)
    }

    fn number(&mut self) -> Result<(), InterpretError> {
        let token = self.previous.as_ref().unwrap();
        let value = token.lexeme.parse::<f64>().unwrap();
        chunk_op::emit_constant(Value::Number(value), &mut self.chunk, token.line);
        Ok(())
    }

    fn literal(&mut self) -> Result<(), InterpretError> {
        let token = self.previous.as_ref().unwrap();
        match token.token_type {
            TokenType::False => chunk_op::emit_byte(OpCode::OpFalse, &mut self.chunk, token.line),
            TokenType::Nil => chunk_op::emit_byte(OpCode::OpNil, &mut self.chunk, token.line),
            TokenType::True => chunk_op::emit_byte(OpCode::OpTrue, &mut self.chunk, token.line),
            _ => panic!("Expected literal"),
        }
        Ok(())
    }

    fn string(&mut self) -> Result<(), InterpretError> {
        let token = self.previous.as_ref().unwrap();
        let value = token.lexeme.clone();
        chunk_op::emit_constant(
            Value::LString(value[1..value.len() - 1].to_string()),
            &mut self.chunk,
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

    fn match_token_type(&mut self, token_type: TokenType) -> Result<bool, InterpretError> {
        if !self.check(token_type) {
            return Ok(false);
        }
        self.advance()?;
        Ok(true)
    }

    fn check(&self, token_type: TokenType) -> bool {
        self.current.clone().unwrap().token_type == token_type
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

    #[test]
    fn test_consume() {
        let source = Source::new("1 + 1".to_string());
        let chunk = Chunk::new();
        let mut parser = Parser::new(source, chunk);
        parser.advance().unwrap();
        let result = parser.consume(TokenType::Number, "").unwrap();
        assert_eq!(result, ());
    }

    #[test]
    fn test_consume_failure() {
        let source = Source::new("1 + 1".to_string());
        let chunk = Chunk::new();
        let mut parser = Parser::new(source, chunk);
        parser.advance().unwrap();
        let result = parser.consume(TokenType::EOF, "");
        assert!(result.is_err());
    }
}
