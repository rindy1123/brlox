use crate::{
    chunk::{Chunk, OpCode, Value},
    scan::{self, Source},
    token::{Token, TokenType},
    vm::InterpretError,
};

use super::precedence::{self, ParseFn, Precedence};

pub struct Parser {
    current: Option<Token>,
    pub previous: Option<Token>,
    source: Source,
    pub chunk: Chunk,
}

impl Parser {
    pub fn new(source: Source, chunk: Chunk) -> Parser {
        Parser {
            current: None,
            previous: None,
            source,
            chunk,
        }
    }

    pub fn advance(&mut self) -> Result<(), InterpretError> {
        self.previous = self.current.clone();
        loop {
            let token = scan::scan_token(&mut self.source);
            self.current = Some(token.clone());
            if let TokenType::Error = token.token_type {
                self.report_error(token.clone(), token.lexeme)?;
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
        match precedence::get_rule(previous_token.token_type.clone()).prefix {
            None => self.report_error(previous_token, "Expect expression".to_string())?,
            Some(prefix_rule) => self.exec_parse_function(prefix_rule)?,
        };

        while precedence.clone() as i32
            <= precedence::get_rule(self.current.clone().unwrap().token_type).precedence as i32
        {
            self.advance()?;
            let previous_token = self.previous.clone().unwrap();
            match precedence::get_rule(previous_token.token_type).infix {
                None => break,
                Some(infix) => self.exec_parse_function(infix)?,
            };
        }
        Ok(())
    }

    pub fn consume(
        &mut self,
        token_type: TokenType,
        message: String,
    ) -> Result<(), InterpretError> {
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
            _ => (),
        }
        Ok(())
    }

    fn grouping(&mut self) -> Result<(), InterpretError> {
        self.expression()?;
        self.consume(
            TokenType::RightParen,
            "Expect ')' after expression.".to_string(),
        )
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
            _ => (),
        }
        Ok(())
    }

    fn exec_parse_function(&mut self, function_type: ParseFn) -> Result<(), InterpretError> {
        match function_type {
            ParseFn::Binary => self.binary(),
            ParseFn::Unary => self.unary(),
            ParseFn::Grouping => self.grouping(),
            ParseFn::Number => self.number(),
        }
    }

    fn report_error(&mut self, token: Token, message: String) -> Result<(), InterpretError> {
        let position = match token.token_type {
            TokenType::EOF => "at end".to_string(),
            _ => format!("at '{}'", token.lexeme),
        };
        eprintln!("[line {}] Error {}: {}", token.line, position, message);
        Err(InterpretError::CompileError)
    }

    fn number(&mut self) -> Result<(), InterpretError> {
        let token = self.previous.as_ref().unwrap();
        let value = token.lexeme.parse::<Value>().unwrap();
        chunk_op::emit_constant(value, &mut self.chunk, token.line);
        Ok(())
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
        let result = parser.expression().err();
        assert!(result.is_some());
    }

    #[test]
    fn test_consume() {
        let source = Source::new("1 + 1".to_string());
        let chunk = Chunk::new();
        let mut parser = Parser::new(source, chunk);
        parser.advance().unwrap();
        let result = parser.consume(TokenType::Number, "".to_string()).unwrap();
        assert_eq!(result, ());
    }

    #[test]
    fn test_consume_failure() {
        let source = Source::new("1 + 1".to_string());
        let chunk = Chunk::new();
        let mut parser = Parser::new(source, chunk);
        parser.advance().unwrap();
        let result = parser.consume(TokenType::EOF, "".to_string()).err();
        assert!(result.is_some());
    }
}
