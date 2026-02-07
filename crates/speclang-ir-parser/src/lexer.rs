//! Lexer for Core IR textual form.
//!
//! Tokenizes the canonical textual representation of Core IR.

use std::fmt;

/// Source location for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }
}

/// Token kinds for Core IR.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Module,
    Type,
    Cap,
    Fn,
    Extern,
    Let,
    If,
    Else,
    Match,
    Return,
    Assert,
    Call,
    Struct,
    Enum,
    Own,
    Ref,
    MutRef,
    Slice,
    MutSlice,
    Effects,
    Requires,
    Ensures,
    Heap,

    // Primitive type keywords
    Bool,
    U8, U16, U32, U64, U128,
    I8, I16, I32, I64, I128,
    F32, F64,
    Unit,
    Int,
    StringKw,
    BytesKw,

    // Literals
    IntLiteral(i128),
    FloatLiteral(f64),
    StringLiteral(String),
    BoolLiteral(bool),

    // Identifier
    Ident(String),

    // Punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semicolon,
    Dot,
    Arrow,     // ->
    FatArrow,  // =>
    Eq,        // =
    EqEq,      // ==
    Ne,        // !=
    Lt,        // <
    Le,        // <=
    Gt,        // >
    Ge,        // >=
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Ampersand,
    Pipe,
    Caret,
    Tilde,
    Bang,
    AndAnd,    // &&
    PipePipe,  // ||
    Shl,       // <<
    Shr,       // >>
    At,        // @
    Underscore,

    // Special
    Eof,
}

/// A token with its kind and source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// Lexer error.
#[derive(Debug, Clone)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lex error at {}-{}: {}", self.span.start, self.span.end, self.message)
    }
}

impl std::error::Error for LexError {}

/// Lexer for Core IR textual form.
pub struct Lexer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer { input, pos: 0 }
    }

    /// Tokenize the entire input.
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            if tok.kind == TokenKind::Eof {
                tokens.push(tok);
                break;
            }
            tokens.push(tok);
        }
        Ok(tokens)
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace
            while let Some(ch) = self.peek_char() {
                if ch.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }
            // Skip line comments
            if self.input[self.pos..].starts_with('#') {
                while let Some(ch) = self.peek_char() {
                    self.advance();
                    if ch == '\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace_and_comments();

        let start = self.pos;

        let Some(ch) = self.advance() else {
            return Ok(Token {
                kind: TokenKind::Eof,
                span: Span::new(start, start),
            });
        };

        let kind = match ch {
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            ';' => TokenKind::Semicolon,
            '.' => TokenKind::Dot,
            '+' => TokenKind::Plus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '^' => TokenKind::Caret,
            '~' => TokenKind::Tilde,
            '@' => TokenKind::At,
            '_' if !self.peek_char().is_some_and(|c| c.is_alphanumeric() || c == '_') => {
                TokenKind::Underscore
            }
            '-' => {
                if self.peek_char() == Some('>') {
                    self.advance();
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            '=' => {
                if self.peek_char() == Some('=') {
                    self.advance();
                    TokenKind::EqEq
                } else if self.peek_char() == Some('>') {
                    self.advance();
                    TokenKind::FatArrow
                } else {
                    TokenKind::Eq
                }
            }
            '!' => {
                if self.peek_char() == Some('=') {
                    self.advance();
                    TokenKind::Ne
                } else {
                    TokenKind::Bang
                }
            }
            '<' => {
                if self.peek_char() == Some('=') {
                    self.advance();
                    TokenKind::Le
                } else if self.peek_char() == Some('<') {
                    self.advance();
                    TokenKind::Shl
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                if self.peek_char() == Some('=') {
                    self.advance();
                    TokenKind::Ge
                } else if self.peek_char() == Some('>') {
                    self.advance();
                    TokenKind::Shr
                } else {
                    TokenKind::Gt
                }
            }
            '&' => {
                if self.peek_char() == Some('&') {
                    self.advance();
                    TokenKind::AndAnd
                } else {
                    TokenKind::Ampersand
                }
            }
            '|' => {
                if self.peek_char() == Some('|') {
                    self.advance();
                    TokenKind::PipePipe
                } else {
                    TokenKind::Pipe
                }
            }
            '"' => {
                return self.lex_string(start);
            }
            c if c.is_ascii_digit() => {
                return self.lex_number(start, c);
            }
            c if c.is_alphabetic() || c == '_' => {
                return self.lex_ident_or_keyword(start, c);
            }
            _ => {
                return Err(LexError {
                    message: format!("unexpected character: '{ch}'"),
                    span: Span::new(start, self.pos),
                });
            }
        };

        Ok(Token {
            kind,
            span: Span::new(start, self.pos),
        })
    }

    fn lex_string(&mut self, start: usize) -> Result<Token, LexError> {
        let mut s = String::new();
        loop {
            match self.advance() {
                None => {
                    return Err(LexError {
                        message: "unterminated string literal".to_string(),
                        span: Span::new(start, self.pos),
                    });
                }
                Some('"') => break,
                Some('\\') => {
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('r') => s.push('\r'),
                        Some('\\') => s.push('\\'),
                        Some('"') => s.push('"'),
                        Some(c) => {
                            return Err(LexError {
                                message: format!("invalid escape: '\\{c}'"),
                                span: Span::new(self.pos - 2, self.pos),
                            });
                        }
                        None => {
                            return Err(LexError {
                                message: "unterminated escape in string".to_string(),
                                span: Span::new(start, self.pos),
                            });
                        }
                    }
                }
                Some(c) => s.push(c),
            }
        }
        Ok(Token {
            kind: TokenKind::StringLiteral(s),
            span: Span::new(start, self.pos),
        })
    }

    fn lex_number(&mut self, start: usize, first: char) -> Result<Token, LexError> {
        let mut num_str = String::new();
        num_str.push(first);

        // Check for hex
        if first == '0' && self.peek_char() == Some('x') {
            self.advance();
            num_str.push('x');
            while let Some(c) = self.peek_char() {
                if c.is_ascii_hexdigit() || c == '_' {
                    if c != '_' {
                        num_str.push(c);
                    }
                    self.advance();
                } else {
                    break;
                }
            }
            let val = i128::from_str_radix(&num_str[2..], 16).map_err(|_| LexError {
                message: format!("invalid hex literal: {num_str}"),
                span: Span::new(start, self.pos),
            })?;
            return Ok(Token {
                kind: TokenKind::IntLiteral(val),
                span: Span::new(start, self.pos),
            });
        }

        let mut is_float = false;
        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() || c == '_' {
                if c != '_' {
                    num_str.push(c);
                }
                self.advance();
            } else if c == '.' && !is_float {
                is_float = true;
                num_str.push(c);
                self.advance();
            } else {
                break;
            }
        }

        if is_float {
            let val: f64 = num_str.parse().map_err(|_| LexError {
                message: format!("invalid float literal: {num_str}"),
                span: Span::new(start, self.pos),
            })?;
            Ok(Token {
                kind: TokenKind::FloatLiteral(val),
                span: Span::new(start, self.pos),
            })
        } else {
            let val: i128 = num_str.parse().map_err(|_| LexError {
                message: format!("invalid integer literal: {num_str}"),
                span: Span::new(start, self.pos),
            })?;
            Ok(Token {
                kind: TokenKind::IntLiteral(val),
                span: Span::new(start, self.pos),
            })
        }
    }

    fn lex_ident_or_keyword(&mut self, start: usize, first: char) -> Result<Token, LexError> {
        let mut ident = String::new();
        ident.push(first);
        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' {
                ident.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let kind = match ident.as_str() {
            "module" => TokenKind::Module,
            "type" => TokenKind::Type,
            "cap" => TokenKind::Cap,
            "fn" => TokenKind::Fn,
            "extern" => TokenKind::Extern,
            "let" => TokenKind::Let,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "match" => TokenKind::Match,
            "return" => TokenKind::Return,
            "assert" => TokenKind::Assert,
            "call" => TokenKind::Call,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "own" => TokenKind::Own,
            "ref" => TokenKind::Ref,
            "mutref" => TokenKind::MutRef,
            "slice" => TokenKind::Slice,
            "mutslice" => TokenKind::MutSlice,
            "effects" => TokenKind::Effects,
            "heap" => TokenKind::Heap,
            "bool" => TokenKind::Bool,
            "u8" => TokenKind::U8,
            "u16" => TokenKind::U16,
            "u32" => TokenKind::U32,
            "u64" => TokenKind::U64,
            "u128" => TokenKind::U128,
            "i8" => TokenKind::I8,
            "i16" => TokenKind::I16,
            "i32" => TokenKind::I32,
            "i64" => TokenKind::I64,
            "i128" => TokenKind::I128,
            "f32" => TokenKind::F32,
            "f64" => TokenKind::F64,
            "unit" => TokenKind::Unit,
            "int" => TokenKind::Int,
            "string" => TokenKind::StringKw,
            "bytes" => TokenKind::BytesKw,
            "true" => TokenKind::BoolLiteral(true),
            "false" => TokenKind::BoolLiteral(false),
            "_" => TokenKind::Underscore,
            _ => TokenKind::Ident(ident),
        };

        Ok(Token {
            kind,
            span: Span::new(start, self.pos),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_simple_module() {
        let input = r#"module test.example { }"#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Module);
        assert!(matches!(tokens[1].kind, TokenKind::Ident(ref s) if s == "test"));
        assert_eq!(tokens[2].kind, TokenKind::Dot);
        assert!(matches!(tokens[3].kind, TokenKind::Ident(ref s) if s == "example"));
        assert_eq!(tokens[4].kind, TokenKind::LBrace);
        assert_eq!(tokens[5].kind, TokenKind::RBrace);
        assert_eq!(tokens[6].kind, TokenKind::Eof);
    }

    #[test]
    fn test_lex_function_sig() {
        let input = r#"fn add(x: i32, y: i32) -> i32"#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Fn);
        assert!(matches!(tokens[1].kind, TokenKind::Ident(ref s) if s == "add"));
        assert_eq!(tokens[2].kind, TokenKind::LParen);
    }

    #[test]
    fn test_lex_string_literal() {
        let input = r#""hello\nworld""#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "hello\nworld"));
    }

    #[test]
    fn test_lex_hex_literal() {
        let input = "0xFF";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::IntLiteral(255));
    }

    #[test]
    fn test_lex_comments() {
        let input = "fn # this is a comment\nadd";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Fn);
        assert!(matches!(tokens[1].kind, TokenKind::Ident(ref s) if s == "add"));
    }

    #[test]
    fn test_lex_arrows() {
        let input = "-> =>";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Arrow);
        assert_eq!(tokens[1].kind, TokenKind::FatArrow);
    }

    #[test]
    fn test_lex_requires_ensures() {
        let input = r#"@requires @ensures"#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::At);
        assert!(matches!(tokens[1].kind, TokenKind::Ident(ref s) if s == "requires"));
        assert_eq!(tokens[2].kind, TokenKind::At);
        assert!(matches!(tokens[3].kind, TokenKind::Ident(ref s) if s == "ensures"));
    }
}
