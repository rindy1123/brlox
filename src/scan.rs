use crate::token::{Token, TokenType};

#[derive(Debug, PartialEq, Eq)]
pub struct Source {
    pub text: String,
    pub start: usize,
    pub current: usize,
    pub line: usize,
}

impl Source {
    pub fn new(text: String) -> Source {
        Source {
            text,
            start: 0,
            current: 0,
            line: 1,
        }
    }
}

pub fn scan_token(source: &mut Source) -> Token {
    skip_white_space(source);
    source.start = source.current;

    if is_at_end(source) {
        return make_token(source, TokenType::EOF);
    }

    let c = advance(source);
    if is_alpha(c) {
        return identifier(source);
    }

    if is_digit(c) {
        return number(source);
    }

    match c {
        '(' => make_token(source, TokenType::LeftParen),
        ')' => make_token(source, TokenType::RightParen),
        '{' => make_token(source, TokenType::LeftBrace),
        '}' => make_token(source, TokenType::RightBrace),
        ';' => make_token(source, TokenType::Semicolon),
        ',' => make_token(source, TokenType::Comma),
        '.' => make_token(source, TokenType::Dot),
        '-' => make_token(source, TokenType::Minus),
        '+' => make_token(source, TokenType::Plus),
        '*' => make_token(source, TokenType::Star),
        '/' => make_token(source, TokenType::Slash),
        '"' => string(source),
        '!' => {
            let token_type = if match_char(source, '=') {
                TokenType::BangEqual
            } else {
                TokenType::Bang
            };
            make_token(source, token_type)
        }
        '=' => {
            let token_type = if match_char(source, '=') {
                TokenType::EqualEqual
            } else {
                TokenType::Equal
            };
            make_token(source, token_type)
        }
        '<' => {
            let token_type = if match_char(source, '=') {
                TokenType::LessEqual
            } else {
                TokenType::Less
            };
            make_token(source, token_type)
        }
        '>' => {
            let token_type = if match_char(source, '=') {
                TokenType::GreaterEqual
            } else {
                TokenType::Greater
            };
            make_token(source, token_type)
        }
        _ => error_token(source.line, "Unexpected character."),
    }
}

fn number(source: &mut Source) -> Token {
    while is_digit(peek(source)) {
        advance(source);
    }

    if peek(source) == '.' && is_digit(peek_next(source)) {
        advance(source);
        while is_digit(peek(source)) {
            advance(source);
        }
    }

    make_token(source, TokenType::Number)
}

fn identifier(source: &mut Source) -> Token {
    while is_alpha(peek(source)) || is_digit(peek(source)) {
        advance(source);
    }

    make_token(source, identifier_type(source))
}

fn identifier_type(source: &Source) -> TokenType {
    match nth_char(source.text.clone(), source.start) {
        'a' => return check_keyword(source, 1, "nd", TokenType::And),
        'c' => return check_keyword(source, 1, "lass", TokenType::Class),
        'e' => return check_keyword(source, 1, "lse", TokenType::Else),
        'f' => match nth_char(source.text.clone(), source.start + 1) {
            'a' => return check_keyword(source, 2, "lse", TokenType::False),
            'o' => return check_keyword(source, 2, "r", TokenType::For),
            'u' => return check_keyword(source, 2, "n", TokenType::Fun),
            _ => TokenType::Identifier,
        },
        'i' => return check_keyword(source, 1, "f", TokenType::If),
        'n' => return check_keyword(source, 1, "il", TokenType::Nil),
        'o' => return check_keyword(source, 1, "r", TokenType::Or),
        'p' => return check_keyword(source, 1, "rint", TokenType::Print),
        'r' => return check_keyword(source, 1, "eturn", TokenType::Return),
        's' => return check_keyword(source, 1, "uper", TokenType::Super),
        't' => match nth_char(source.text.clone(), source.start + 1) {
            'h' => return check_keyword(source, 2, "is", TokenType::This),
            'r' => return check_keyword(source, 2, "ue", TokenType::True),
            _ => TokenType::Identifier,
        },
        'v' => return check_keyword(source, 1, "ar", TokenType::Var),
        'w' => return check_keyword(source, 1, "hile", TokenType::While),
        _ => TokenType::Identifier,
    }
}

fn check_keyword(source: &Source, start: usize, rest: &str, token_type: TokenType) -> TokenType {
    let length = rest.len();
    // check the length of token
    if source.current - source.start != start + length {
        return TokenType::Identifier;
    }
    let head_of_token = source.start + start;
    let tail_of_token = head_of_token + length;
    // check if the token matches brlox's keywords
    if source.text[head_of_token..tail_of_token].ne(rest) {
        return TokenType::Identifier;
    }
    return token_type;
}

fn skip_white_space(source: &mut Source) {
    loop {
        let c = peek(source);
        match c {
            ' ' | '\r' | '\t' => {
                advance(source);
            }
            '\n' => {
                source.line += 1;
                advance(source);
            }
            '/' => {
                if peek_next(source) != '/' {
                    return;
                }
                while peek(source) != '\n' && !is_at_end(source) {
                    advance(source);
                }
            }
            _ => return,
        }
    }
}

fn peek(source: &Source) -> char {
    nth_char(source.text.clone(), source.current)
}

fn peek_next(source: &Source) -> char {
    if is_at_end(source) {
        return '\0';
    }
    nth_char(source.text.clone(), source.current + 1)
}

fn match_char(source: &mut Source, c: char) -> bool {
    if is_at_end(source) {
        return false;
    }
    if nth_char(source.text.clone(), source.current) != c {
        return false;
    }
    source.current += 1;
    true
}

fn string(source: &mut Source) -> Token {
    while peek(source) != '"' && !is_at_end(source) {
        if peek(source) == '\n' {
            source.line += 1;
        }
        advance(source);
    }

    if is_at_end(source) {
        return error_token(source.line, "Unterminated string.");
    }
    advance(source);
    make_token(source, TokenType::LString)
}

fn make_token(source: &Source, token_type: TokenType) -> Token {
    Token::new(
        token_type,
        source.text[source.start..source.current].to_string(),
        source.line,
    )
}

fn error_token(line: usize, message: &str) -> Token {
    Token::new(TokenType::Error, message.to_string(), line)
}

fn advance(source: &mut Source) -> char {
    let ret = nth_char(source.text.clone(), source.current);
    source.current += 1;
    ret
}

fn is_at_end(source: &Source) -> bool {
    nth_char(source.text.clone(), source.current) == '\0'
}

fn nth_char(text: String, n: usize) -> char {
    if text.len() == n {
        return '\0';
    }
    text.chars().nth(n).unwrap()
}

fn is_digit(c: char) -> bool {
    c >= '0' && c <= '9'
}

fn is_alpha(c: char) -> bool {
    (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    mod helper {
        use super::*;

        #[test]
        fn test_is_digit() {
            assert!(is_digit('8'));
            assert!(!is_digit('a'));
        }
    }

    mod scan_token {
        use super::*;

        #[test]
        fn test_single_token() {
            let mut source = Source::new("(".to_string());
            let expected_token = Token::new(TokenType::LeftParen, "(".to_string(), 1);
            assert_eq!(scan_token(&mut source), expected_token);
        }

        #[test]
        fn test_pair_of_token() {
            let mut source = Source::new("!=".to_string());
            let expected_token = Token::new(TokenType::BangEqual, "!=".to_string(), 1);
            assert_eq!(scan_token(&mut source), expected_token);
        }

        #[test]
        fn test_error_token() {
            let mut source = Source::new("エラー".to_string());
            let expected_token =
                Token::new(TokenType::Error, "Unexpected character.".to_string(), 1);
            assert_eq!(scan_token(&mut source), expected_token);
        }
    }

    mod string {
        use super::*;

        #[test]
        fn test_string() {
            let mut source = Source {
                text: "\"abcd\nefg\"".to_string(),
                start: 0,
                current: 1,
                line: 1,
            };
            let token = string(&mut source);
            let expected_source = Source {
                text: "\"abcd\nefg\"".to_string(),
                start: 0,
                current: 10,
                line: 2,
            };
            let expected_token = Token {
                token_type: TokenType::LString,
                lexeme: "\"abcd\nefg\"".to_string(),
                line: 2,
            };
            assert_eq!(source, expected_source);
            assert_eq!(token, expected_token);
        }

        #[test]
        fn test_unterminated_string() {
            let mut source = Source {
                text: "\"abc".to_string(),
                start: 0,
                current: 1,
                line: 1,
            };
            let token = string(&mut source);
            let expected_token = Token {
                token_type: TokenType::Error,
                lexeme: "Unterminated string.".to_string(),
                line: 1,
            };
            assert_eq!(token, expected_token);
        }
    }

    #[test]
    fn test_peek() {
        let source = Source {
            text: "abcdefg".to_string(),
            start: 2,
            current: 5,
            line: 2,
        };
        assert_eq!(peek(&source), 'f');
    }

    mod peek_next {
        use super::*;

        #[test]
        fn test_peek_next() {
            let source = Source {
                text: "abcdefg".to_string(),
                start: 2,
                current: 5,
                line: 2,
            };
            assert_eq!(peek_next(&source), 'g');
        }

        #[test]
        fn test_peek_last() {
            let source = Source {
                text: "".to_string(),
                start: 0,
                current: 0,
                line: 1,
            };
            assert_eq!(peek_next(&source), '\0');
        }
    }

    mod match_char {
        use super::*;

        #[test]
        fn test_at_end() {
            let mut source = Source::new("\0".to_string());
            assert!(!match_char(&mut source, 'a'));
            assert_eq!(source.current, 0);
        }

        #[test]
        fn test_not_matched() {
            let mut source = Source::new("b".to_string());
            assert!(!match_char(&mut source, 'a'));
            assert_eq!(source.current, 0);
        }

        #[test]
        fn test_matched() {
            let mut source = Source::new("a".to_string());
            assert!(match_char(&mut source, 'a'));
            assert_eq!(source.current, 1);
        }
    }

    #[test]
    fn test_make_token() {
        let source = Source {
            text: "abcdefg".to_string(),
            start: 2,
            current: 5,
            line: 2,
        };
        let ret = make_token(&source, TokenType::LeftParen);
        let expected_token = Token {
            token_type: TokenType::LeftParen,
            lexeme: "cde".to_string(),
            line: 2,
        };
        assert_eq!(ret, expected_token);
    }

    #[test]
    fn test_error_token() {
        let ret = error_token(3, "error");
        let expected_token = Token {
            token_type: TokenType::Error,
            lexeme: "error".to_string(),
            line: 3,
        };
        assert_eq!(ret, expected_token);
    }

    #[test]
    fn test_advance() {
        let mut source = Source::new("abcde".to_string());
        let ret = advance(&mut source);
        assert_eq!(ret, 'a');
        assert_eq!(source.current, 1);
    }

    #[test]
    fn test_nth_char() {
        assert_eq!(nth_char("abcde".to_string(), 3), 'd');
        assert_eq!(nth_char("".to_string(), 0), '\0');
    }

    #[test]
    fn test_is_at_end() {
        let source = Source::new("\0".to_string());
        assert!(is_at_end(&source))
    }

    mod number {
        use super::*;
        #[test]
        fn test_integer() {
            let mut source = Source::new("123".to_string());
            let expected_token = Token::new(TokenType::Number, "123".to_string(), 1);
            assert_eq!(number(&mut source), expected_token);
        }

        #[test]
        fn test_float() {
            let mut source = Source::new("123.456".to_string());
            let expected_token = Token::new(TokenType::Number, "123.456".to_string(), 1);
            assert_eq!(number(&mut source), expected_token);
        }
    }

    mod test_skip_white_space {
        use super::*;

        #[test]
        fn test_white_space() {
            let mut source = Source::new(" \r\t".to_string());
            skip_white_space(&mut source);
            assert_eq!(source.current, 3);
        }

        #[test]
        fn test_new_line() {
            let mut source = Source::new("\n\n".to_string());
            skip_white_space(&mut source);
            assert_eq!(source.current, 2);
            assert_eq!(source.line, 3);
        }

        #[test]
        fn test_comment() {
            let mut source = Source::new("// comment".to_string());
            skip_white_space(&mut source);
            assert_eq!(source.current, 10);
            assert_eq!(source.line, 1);
        }
    }

    #[test]
    fn test_is_alpha() {
        assert!(is_alpha('a'));
        assert!(!is_alpha('1'));
    }

    #[test]
    fn test_identifier() {
        let mut source = Source::new("identifier123".to_string());
        let expected_token = Token::new(TokenType::Identifier, "identifier123".to_string(), 1);
        assert_eq!(identifier(&mut source), expected_token);
    }

    mod identifier_type {
        use super::*;

        #[test]
        fn test_and() {
            let source = Source {
                text: "and".to_string(),
                start: 0,
                current: 3,
                line: 1,
            };
            assert_eq!(identifier_type(&source), TokenType::And);
        }

        #[test]
        fn test_class() {
            let source = Source {
                text: "class".to_string(),
                start: 0,
                current: 5,
                line: 1,
            };
            assert_eq!(identifier_type(&source), TokenType::Class);
        }

        #[test]
        fn test_else() {
            let source = Source {
                text: "else".to_string(),
                start: 0,
                current: 4,
                line: 1,
            };
            assert_eq!(identifier_type(&source), TokenType::Else);
        }

        #[test]
        fn test_false() {
            let source = Source {
                text: "false".to_string(),
                start: 0,
                current: 5,
                line: 1,
            };
            assert_eq!(identifier_type(&source), TokenType::False);
        }

        #[test]
        fn test_for() {
            let source = Source {
                text: "for".to_string(),
                start: 0,
                current: 3,
                line: 1,
            };
            assert_eq!(identifier_type(&source), TokenType::For);
        }

        #[test]
        fn test_fun() {
            let source = Source {
                text: "fun".to_string(),
                start: 0,
                current: 3,
                line: 1,
            };
            assert_eq!(identifier_type(&source), TokenType::Fun);
        }

        #[test]
        fn test_if() {
            let source = Source {
                text: "if".to_string(),
                start: 0,
                current: 2,
                line: 1,
            };
            assert_eq!(identifier_type(&source), TokenType::If);
        }

        #[test]
        fn test_nil() {
            let source = Source {
                text: "nil".to_string(),
                start: 0,
                current: 3,
                line: 1,
            };
            assert_eq!(identifier_type(&source), TokenType::Nil);
        }

        #[test]
        fn test_or() {
            let source = Source {
                text: "or".to_string(),
                start: 0,
                current: 2,
                line: 1,
            };
            assert_eq!(identifier_type(&source), TokenType::Or);
        }

        #[test]
        fn test_print() {
            let source = Source {
                text: "print".to_string(),
                start: 0,
                current: 5,
                line: 1,
            };
            assert_eq!(identifier_type(&source), TokenType::Print);
        }

        #[test]
        fn test_return() {
            let source = Source {
                text: "return".to_string(),
                start: 0,
                current: 6,
                line: 1,
            };
            assert_eq!(identifier_type(&source), TokenType::Return);
        }

        #[test]
        fn test_super() {
            let source = Source {
                text: "super".to_string(),
                start: 0,
                current: 5,
                line: 1,
            };
            assert_eq!(identifier_type(&source), TokenType::Super);
        }

        #[test]
        fn test_this() {
            let source = Source {
                text: "this".to_string(),
                start: 0,
                current: 4,
                line: 1,
            };
            assert_eq!(identifier_type(&source), TokenType::This);
        }

        #[test]
        fn test_true() {
            let source = Source {
                text: "true".to_string(),
                start: 0,
                current: 4,
                line: 1,
            };
            assert_eq!(identifier_type(&source), TokenType::True);
        }

        #[test]
        fn test_var() {
            let source = Source {
                text: "var".to_string(),
                start: 0,
                current: 3,
                line: 1,
            };
            assert_eq!(identifier_type(&source), TokenType::Var);
        }

        #[test]
        fn test_while() {
            let source = Source {
                text: "while".to_string(),
                start: 0,
                current: 5,
                line: 1,
            };
            assert_eq!(identifier_type(&source), TokenType::While);
        }

        #[test]
        fn test_random_identifier() {
            let source = Source {
                text: "falsy".to_string(),
                start: 0,
                current: 5,
                line: 1,
            };
            assert_eq!(identifier_type(&source), TokenType::Identifier);
        }
    }

    mod check_keyword {
        use super::*;

        #[test]
        fn test_keyword() {
            let source = Source {
                text: "class".to_string(),
                start: 0,
                current: 5,
                line: 1,
            };
            let token_type = check_keyword(&source, 1, "lass", TokenType::Class);
            assert_eq!(token_type, TokenType::Class);
        }

        #[test]
        fn test_identifier() {
            let source = Source {
                text: "club".to_string(),
                start: 0,
                current: 4,
                line: 1,
            };
            let token_type = check_keyword(&source, 1, "lass", TokenType::Class);
            assert_eq!(token_type, TokenType::Identifier);
        }
    }
}
