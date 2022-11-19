use crate::{
    chunk::OpCode,
    scan::{self, Source},
    token::{Token, TokenType},
    value::{object::Obj, Value},
    vm::InterpretError,
};

use super::{
    error_report,
    precedence::{self, ParseFn, Precedence},
    Compiler, FunctionType,
};

pub struct Parser {
    current: Option<Token>,
    pub previous: Option<Token>,
    source: Source,
    compiler: Compiler,
    enclosing: Vec<Compiler>,
}

impl Parser {
    pub fn new(source: Source, compiler: Compiler) -> Parser {
        Parser {
            current: None,
            previous: None,
            enclosing: Vec::new(),
            source,
            compiler,
        }
    }

    pub fn parse(&mut self) -> Result<Compiler, InterpretError> {
        self.advance()?;
        while !self.match_token_type(TokenType::EOF) {
            self.declaration()?;
        }
        // consume EOF
        self.advance()?;
        Ok(self.compiler.clone())
    }

    fn declaration(&mut self) -> Result<(), InterpretError> {
        if self.match_token_type(TokenType::Var) {
            self.advance()?;
            return self.var_declaration();
        }
        self.statement()
    }

    /// parse declaration like
    /// ```
    /// var a = 1;
    /// ```
    /// or
    /// ```
    /// var a;
    /// ```
    fn var_declaration(&mut self) -> Result<(), InterpretError> {
        let global = self.parse_variable("Expect variable name.")?;

        if self.match_token_type(TokenType::Equal) {
            // When variable is initialized
            self.advance()?;
            self.expression()?;
        } else {
            // If variable is not initialized, variable should hold nil
            let line = self.previous.as_ref().unwrap().line;
            self.compiler.emit_byte(OpCode::OpNil, line);
        }

        self.consume(TokenType::Semicolon, "Expect ';' after expression.")?;
        if self.compiler.is_local() {
            self.compiler.define_local_variable();
        } else {
            let line = self.previous.as_ref().unwrap().line;
            self.compiler.define_global_variable(global, line);
        }
        Ok(())
    }

    fn parse_variable(&mut self, message: &str) -> Result<usize, InterpretError> {
        self.consume(TokenType::Identifier, message)?;
        self.compiler
            .declare_variable(self.previous.as_ref().unwrap())?;
        if self.compiler.is_local() {
            return Ok(0);
        }

        let global_variable_name = self.previous.as_ref().unwrap().lexeme.clone();
        Ok(self.compiler.identifier_constant(global_variable_name))
    }

    fn statement(&mut self) -> Result<(), InterpretError> {
        match self.current.as_ref().unwrap().token_type {
            TokenType::Fun => {
                self.advance()?;
                self.fun_statement()
            }
            TokenType::Print => {
                self.advance()?;
                self.print_statement()
            }
            TokenType::If => {
                self.advance()?;
                self.if_statement()
            }
            TokenType::While => {
                self.advance()?;
                self.while_statement()
            }
            TokenType::For => {
                self.advance()?;
                self.for_statement()
            }
            TokenType::LeftBrace => {
                self.advance()?;
                self.compiler.begin_scope();
                self.block()?;
                let line = self.previous.as_ref().unwrap().line;
                self.compiler.end_scope(line);
                Ok(())
            }
            _ => self.expression_statement(),
        }
    }

    fn block(&mut self) -> Result<(), InterpretError> {
        while !self.match_token_type(TokenType::RightBrace)
            && !self.match_token_type(TokenType::EOF)
        {
            self.declaration()?;
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block.")
    }

    fn fun_statement(&mut self) -> Result<(), InterpretError> {
        let global = self.parse_variable("Expect function name.")?;
        // mark as initialized to be able to be referenced in function body
        self.compiler.mark_initialized();
        self.parse_function(FunctionType::Function)?;
        let line = self.previous.as_ref().unwrap().line;
        self.compiler.define_global_variable(global, line);
        Ok(())
    }

    fn parse_function(&mut self, function_type: FunctionType) -> Result<(), InterpretError> {
        let previous_compiler = self.compiler.clone();
        self.enclosing.push(previous_compiler);
        self.compiler = Compiler::new(function_type);
        let function_name = self.previous.as_ref().unwrap().lexeme.clone();
        self.compiler.function.name = function_name;

        self.compiler.begin_scope();
        self.parse_argument()?;
        self.block()?;

        let token = self.previous.as_ref().unwrap();
        let line = token.line;
        let function = Obj::Function(self.compiler.end_compiler(line));
        self.compiler = self.enclosing.pop().unwrap();
        self.compiler.emit_constant(Value::Obj(function), line);
        Ok(())
    }

    fn parse_argument(&mut self) -> Result<(), InterpretError> {
        self.consume(TokenType::LeftParen, "Expect '(' after function name.")?;
        if !self.match_token_type(TokenType::RightParen) {
            loop {
                self.compiler.function.arity += 1;
                self.parse_variable("Expect parameter name.")?;
                self.compiler.define_local_variable();
                if !self.match_token_type(TokenType::Comma) {
                    break;
                }
                self.advance()?;
            }
        }
        self.consume(TokenType::RightParen, "Expect ')' after parameters.")?;
        self.consume(TokenType::LeftBrace, "Expect '{' before function body.")
    }

    /// The order of execution in for loop:
    /// 1. initialization
    /// 2. condition
    /// 3. body
    /// 4. increment
    ///
    /// and start again from No.2
    fn for_statement(&mut self) -> Result<(), InterpretError> {
        self.compiler.begin_scope();
        self.consume(TokenType::LeftParen, "Expect '(' after if.")?;
        self.for_loop_init()?;
        // For loop restarts after the initialization
        let loop_start = self.compiler.current_chunk_as_ref().code.len() - 1;
        let loop_exit_jump = self.for_loop_condition()?;

        let jump_after_body = self.for_loop_increment(loop_start)?;
        self.statement()?;

        let line = self.previous.as_ref().unwrap().line;
        // Go back condition or increment clause
        self.compiler.emit_jump_back(jump_after_body, line);
        if let Some(jump) = loop_exit_jump {
            self.compiler.patch_jump(jump);
            self.compiler.emit_pop(line);
        }
        self.compiler.end_scope(line);
        Ok(())
    }

    /// Initialization clause of for loop
    fn for_loop_init(&mut self) -> Result<(), InterpretError> {
        match self.current.as_ref().unwrap().token_type {
            // When omitted
            TokenType::Semicolon => self.advance()?,
            // When variable declared
            TokenType::Var => {
                self.advance()?;
                self.var_declaration()?;
            }
            // Any other expression
            _ => self.expression_statement()?,
        }
        Ok(())
    }

    /// Condition clause of for loop.
    /// If condition expression exists, it returns current chunk's address
    fn for_loop_condition(&mut self) -> Result<Option<usize>, InterpretError> {
        if self.match_token_type(TokenType::Semicolon) {
            self.advance()?;
            return Ok(None);
        }

        self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after condition.")?;

        let line = self.previous.as_ref().unwrap().line;
        let jump = self
            .compiler
            .emit_jump(OpCode::OpJumpIfFalse { offset: 0 }, line);
        self.compiler.emit_pop(line);
        Ok(Some(jump))
    }

    /// Increment clause of for loop.
    fn for_loop_increment(&mut self, loop_start: usize) -> Result<usize, InterpretError> {
        if self.match_token_type(TokenType::RightParen) {
            self.advance()?;
            return Ok(loop_start);
        }

        let line = self.previous.as_ref().unwrap().line;
        let body_jump = self.compiler.emit_jump(OpCode::OpJump { offset: 0 }, line);
        let increment_start = self.compiler.current_chunk_as_ref().code.len() - 1;
        self.expression()?;
        let line = self.previous.as_ref().unwrap().line;
        self.compiler.emit_pop(line);
        self.consume(TokenType::RightParen, "Expect ')' after condition.")?;

        let line = self.previous.as_ref().unwrap().line;
        // Back to the condition clause since the loop ends here.
        self.compiler.emit_jump_back(loop_start, line);
        // Hop over increment clause to the body of the loop.
        self.compiler.patch_jump(body_jump);
        // To get back to the increment clause after executing the body,
        // return the address where increment starts.
        Ok(increment_start)
    }

    fn while_statement(&mut self) -> Result<(), InterpretError> {
        let code_size = self.compiler.current_chunk_as_ref().code.len();
        let loop_start = code_size - 1;
        self.condition()?;
        let line = self.previous.as_ref().unwrap().line;
        let exit_jump = self
            .compiler
            .emit_jump(OpCode::OpJumpIfFalse { offset: 0 }, line);
        self.compiler.emit_pop(line);
        self.statement()?;

        let line = self.previous.as_ref().unwrap().line;
        self.compiler.emit_jump_back(loop_start, line);
        self.compiler.patch_jump(exit_jump);
        self.compiler.emit_pop(line);
        Ok(())
    }

    fn condition(&mut self) -> Result<(), InterpretError> {
        self.consume(TokenType::LeftParen, "Expect '(' before condition.")?;
        self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after condition.")
    }

    fn if_statement(&mut self) -> Result<(), InterpretError> {
        self.condition()?;
        let line = self.previous.as_ref().unwrap().line;
        let then_jump = self
            .compiler
            .emit_jump(OpCode::OpJumpIfFalse { offset: 0 }, line);
        self.compiler.emit_pop(line);
        self.statement()?;

        let line = self.previous.as_ref().unwrap().line;
        let else_jump = self.compiler.emit_jump(OpCode::OpJump { offset: 0 }, line);
        self.compiler.patch_jump(then_jump);
        self.compiler.emit_pop(line);
        if self.match_token_type(TokenType::Else) {
            self.advance()?;
            self.statement()?;
        }
        self.compiler.patch_jump(else_jump);
        Ok(())
    }

    fn expression_statement(&mut self) -> Result<(), InterpretError> {
        self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after expression.")?;
        let line = self.previous.as_ref().unwrap().line;
        self.compiler.emit_pop(line);
        Ok(())
    }

    fn print_statement(&mut self) -> Result<(), InterpretError> {
        self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after value.")?;
        let line = self.previous.as_ref().unwrap().line;
        self.compiler.emit_byte(OpCode::OpPrint, line);
        Ok(())
    }

    /// Consume the next token from self.source.
    /// self.previous will be the current token and self.current will be the next token.
    fn advance(&mut self) -> Result<(), InterpretError> {
        self.previous = self.current.clone();
        let token = scan::scan_token(&mut self.source);
        if let TokenType::Error = token.token_type {
            error_report::report_error(&token, &token.lexeme);
            return Err(InterpretError::CompileError);
        }
        self.current = Some(token);
        Ok(())
    }

    fn consume(&mut self, token_type: TokenType, message: &str) -> Result<(), InterpretError> {
        if !self.match_token_type(token_type) {
            error_report::report_error(self.current.as_ref().unwrap(), message);
            return Err(InterpretError::CompileError);
        }
        self.advance()
    }

    fn expression(&mut self) -> Result<(), InterpretError> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> Result<(), InterpretError> {
        self.advance()?;
        let previous_token = self.previous.as_ref().unwrap();
        let can_assign = precedence.clone() as u32 <= Precedence::Assignment as u32;
        match precedence::get_rule(&previous_token.token_type).prefix {
            None => {
                error_report::report_error(previous_token, "Expect expression");
                return Err(InterpretError::CompileError);
            }
            Some(prefix_rule) => self.exec_parse_function(prefix_rule, can_assign)?,
        };

        while precedence.clone() as u32
            <= precedence::get_rule(&self.current.as_ref().unwrap().token_type).precedence as u32
        {
            self.advance()?;
            let previous_token_type = &self.previous.as_ref().unwrap().token_type;
            match precedence::get_rule(previous_token_type).infix {
                None => break,
                Some(infix) => self.exec_parse_function(infix, can_assign)?,
            };
        }

        if can_assign && self.match_token_type(TokenType::Equal) {
            self.advance()?;
            let previous_token = self.previous.as_ref().unwrap();
            error_report::report_error(previous_token, "Invalid assignment target.");
            return Err(InterpretError::CompileError);
        }
        Ok(())
    }

    fn binary(&mut self) -> Result<(), InterpretError> {
        let previous_token = self.previous.as_ref().unwrap();
        let operator_type = previous_token.token_type.clone();
        let rule = precedence::get_rule(&operator_type);
        let line = previous_token.line.clone();
        let precedence = rule.precedence.next();
        self.parse_precedence(precedence)?;

        match operator_type {
            TokenType::Plus => self.compiler.emit_byte(OpCode::OpAdd, line),
            TokenType::Minus => self.compiler.emit_byte(OpCode::OpSubtract, line),
            TokenType::Star => self.compiler.emit_byte(OpCode::OpMultiply, line),
            TokenType::Slash => self.compiler.emit_byte(OpCode::OpDivide, line),
            TokenType::BangEqual => {
                self.compiler.emit_byte(OpCode::OpEqual, line);
                self.compiler.emit_byte(OpCode::OpNot, line);
            }
            TokenType::EqualEqual => {
                self.compiler.emit_byte(OpCode::OpEqual, line);
            }
            TokenType::Greater => {
                self.compiler.emit_byte(OpCode::OpGreater, line);
            }
            TokenType::GreaterEqual => {
                self.compiler.emit_byte(OpCode::OpLess, line);
                self.compiler.emit_byte(OpCode::OpNot, line);
            }
            TokenType::Less => {
                self.compiler.emit_byte(OpCode::OpLess, line);
            }
            TokenType::LessEqual => {
                self.compiler.emit_byte(OpCode::OpGreater, line);
                self.compiler.emit_byte(OpCode::OpNot, line);
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
        let previous_token = self.previous.as_ref().unwrap();
        let operator_type = previous_token.token_type.clone();
        let line = previous_token.line;

        // Compile the operand
        self.parse_precedence(Precedence::Unary)?;

        // Emit the operator instruction
        match operator_type {
            TokenType::Minus => self.compiler.emit_byte(OpCode::OpNegate, line),
            TokenType::Bang => self.compiler.emit_byte(OpCode::OpNot, line),
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
        let arg = self.compiler.resolve_local(&name)?;
        let (get_op, set_op) = match arg {
            None => {
                let index = self.compiler.identifier_constant(name.lexeme);
                (OpCode::OpGetGlobal { index }, OpCode::OpSetGlobal { index })
            }
            Some(index) => (OpCode::OpGetLocal { index }, OpCode::OpSetLocal { index }),
        };
        if can_assign && self.match_token_type(TokenType::Equal) {
            self.advance()?;
            self.expression()?;
            self.compiler.emit_byte(set_op, name.line);
        } else {
            self.compiler.emit_byte(get_op, name.line);
        }
        Ok(())
    }

    fn number(&mut self) -> Result<(), InterpretError> {
        let token = self.previous.as_ref().unwrap();
        let value = token.lexeme.parse::<f64>().unwrap();
        let line = token.line;
        self.compiler.emit_constant(Value::Number(value), line);
        Ok(())
    }

    fn literal(&mut self) -> Result<(), InterpretError> {
        let token = self.previous.as_ref().unwrap();
        let token_type = token.token_type.clone();
        let line = token.line;
        match token_type {
            TokenType::False => self.compiler.emit_byte(OpCode::OpFalse, line),
            TokenType::Nil => self.compiler.emit_byte(OpCode::OpNil, line),
            TokenType::True => self.compiler.emit_byte(OpCode::OpTrue, line),
            _ => panic!("Expected literal"),
        }
        Ok(())
    }

    fn string(&mut self) -> Result<(), InterpretError> {
        let token = self.previous.as_ref().unwrap();
        let value = &token.lexeme;
        let line = token.line;
        self.compiler
            .emit_constant(Value::LString(value[1..value.len() - 1].to_string()), line);
        Ok(())
    }

    fn and(&mut self) -> Result<(), InterpretError> {
        let line = self.previous.as_ref().unwrap().line;
        let end_jump = self
            .compiler
            .emit_jump(OpCode::OpJumpIfFalse { offset: 0 }, line);

        self.compiler.emit_pop(line);
        self.parse_precedence(Precedence::And)?;

        self.compiler.patch_jump(end_jump);
        Ok(())
    }

    fn or(&mut self) -> Result<(), InterpretError> {
        let line = self.previous.as_ref().unwrap().line;
        let else_jump = self
            .compiler
            .emit_jump(OpCode::OpJumpIfFalse { offset: 0 }, line);
        let end_jump = self.compiler.emit_jump(OpCode::OpJump { offset: 0 }, line);

        self.compiler.patch_jump(else_jump);
        self.compiler.emit_pop(line);
        self.parse_precedence(Precedence::Or)?;
        self.compiler.patch_jump(end_jump);
        Ok(())
    }

    fn match_token_type(&self, token_type: TokenType) -> bool {
        self.current.as_ref().unwrap().token_type == token_type
    }
}

#[cfg(test)]
mod tests {

    use crate::compiler::FunctionType;

    use super::*;

    #[test]
    fn test_advance() {
        let source = Source::new("1 + 1".to_string());
        let compiler = Compiler::new(FunctionType::Script);
        let mut parser = Parser::new(source, compiler);
        let result = parser.advance().unwrap();
        assert_eq!(result, ());
    }

    #[test]
    fn test_expression() {
        let source = Source::new("1 + 1".to_string());
        let compiler = Compiler::new(FunctionType::Script);
        let mut parser = Parser::new(source, compiler);
        parser.advance().unwrap();
        let result = parser.expression().unwrap();
        assert_eq!(result, ());
    }

    #[test]
    fn test_expression_failure() {
        let source = Source::new("+ 1".to_string());
        let compiler = Compiler::new(FunctionType::Script);
        let mut parser = Parser::new(source, compiler);
        parser.advance().unwrap();
        let result = parser.expression();
        assert!(result.is_err());
    }
}
