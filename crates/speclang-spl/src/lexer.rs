//! SPL lexer.
//!
//! Tokenizes `.spl` source files into a stream of tokens.

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

/// Token kinds for SPL.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Module,
    Import,
    As,
    Type,
    Struct,
    Enum,
    Fn,
    Error,
    Capability,
    Law,
    Req,
    Decision,
    Gen,
    Prop,
    Oracle,
    Policy,
    Refine,
    Invariant,
    Requires,
    Ensures,
    Effects,
    Raises,
    Perf,
    Examples,
    Notes,
    When,
    Choose,
    Forall,
    From,
    Allow,
    Deny,
    Deterministic,
    And,
    Or,
    Not,
    SelfKw,
    Reference,
    Optimized,
    StableCall,
    StableSemantics,
    Unstable,

    // Literals
    IntLiteral(i64),
    StringLiteral(String),

    // Identifiers and REQ tags
    Ident(String),
    /// REQ-xxx identifier (used in req declarations and tags)
    ReqId(String),

    // Annotations
    AtId,       // @id
    AtCompat,   // @compat

    // Punctuation
    LParen,     // (
    RParen,     // )
    LBrace,     // {
    RBrace,     // }
    LBracket,   // [
    RBracket,   // ]
    LAngle,     // <
    RAngle,     // >
    Comma,      // ,
    Colon,      // :
    Semicolon,  // ;
    Dot,        // .
    DotDot,     // ..
    Arrow,      // ->
    Eq,         // =
    EqEq,       // ==
    Ne,         // !=
    Le,         // <=
    Ge,         // >=
    Question,   // ?

    // Special
    Eof,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Module => write!(f, "module"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::As => write!(f, "as"),
            TokenKind::Type => write!(f, "type"),
            TokenKind::Struct => write!(f, "struct"),
            TokenKind::Enum => write!(f, "enum"),
            TokenKind::Fn => write!(f, "fn"),
            TokenKind::Error => write!(f, "error"),
            TokenKind::Capability => write!(f, "capability"),
            TokenKind::Law => write!(f, "law"),
            TokenKind::Req => write!(f, "req"),
            TokenKind::Decision => write!(f, "decision"),
            TokenKind::Gen => write!(f, "gen"),
            TokenKind::Prop => write!(f, "prop"),
            TokenKind::Oracle => write!(f, "oracle"),
            TokenKind::Policy => write!(f, "policy"),
            TokenKind::Refine => write!(f, "refine"),
            TokenKind::Invariant => write!(f, "invariant"),
            TokenKind::Requires => write!(f, "requires"),
            TokenKind::Ensures => write!(f, "ensures"),
            TokenKind::Effects => write!(f, "effects"),
            TokenKind::Raises => write!(f, "raises"),
            TokenKind::Perf => write!(f, "perf"),
            TokenKind::Examples => write!(f, "examples"),
            TokenKind::Notes => write!(f, "notes"),
            TokenKind::When => write!(f, "when"),
            TokenKind::Choose => write!(f, "choose"),
            TokenKind::Forall => write!(f, "forall"),
            TokenKind::From => write!(f, "from"),
            TokenKind::Allow => write!(f, "allow"),
            TokenKind::Deny => write!(f, "deny"),
            TokenKind::Deterministic => write!(f, "deterministic"),
            TokenKind::And => write!(f, "and"),
            TokenKind::Or => write!(f, "or"),
            TokenKind::Not => write!(f, "not"),
            TokenKind::SelfKw => write!(f, "self"),
            TokenKind::Reference => write!(f, "reference"),
            TokenKind::Optimized => write!(f, "optimized"),
            TokenKind::StableCall => write!(f, "stable_call"),
            TokenKind::StableSemantics => write!(f, "stable_semantics"),
            TokenKind::Unstable => write!(f, "unstable"),
            TokenKind::IntLiteral(n) => write!(f, "{n}"),
            TokenKind::StringLiteral(s) => write!(f, "\"{s}\""),
            TokenKind::Ident(s) => write!(f, "{s}"),
            TokenKind::ReqId(s) => write!(f, "{s}"),
            TokenKind::AtId => write!(f, "@id"),
            TokenKind::AtCompat => write!(f, "@compat"),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::LAngle => write!(f, "<"),
            TokenKind::RAngle => write!(f, ">"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::DotDot => write!(f, ".."),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::Eq => write!(f, "="),
            TokenKind::EqEq => write!(f, "=="),
            TokenKind::Ne => write!(f, "!="),
            TokenKind::Le => write!(f, "<="),
            TokenKind::Ge => write!(f, ">="),
            TokenKind::Question => write!(f, "?"),
            TokenKind::Eof => write!(f, "EOF"),
        }
    }
}

/// A token with its kind and source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// Lexer error.
#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lex error at {}: {}", self.span.start, self.message)
    }
}

impl std::error::Error for LexError {}

/// SPL Lexer.
pub struct Lexer<'a> {
    _source: &'a str,
    chars: Vec<char>,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Lexer {
            _source: source,
            chars: source.chars().collect(),
            pos: 0,
        }
    }

    /// Tokenize the entire source into a vector of tokens.
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
        self.chars.get(self.pos).copied()
    }

    fn advance_char(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace
            while let Some(ch) = self.peek_char() {
                if ch.is_whitespace() {
                    self.advance_char();
                } else {
                    break;
                }
            }
            // Skip line comments
            if self.peek_char() == Some('#') {
                while let Some(ch) = self.advance_char() {
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

        let Some(ch) = self.advance_char() else {
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
            '?' => TokenKind::Question,

            '.' => {
                if self.peek_char() == Some('.') {
                    self.advance_char();
                    TokenKind::DotDot
                } else {
                    TokenKind::Dot
                }
            }

            '-' => {
                if self.peek_char() == Some('>') {
                    self.advance_char();
                    TokenKind::Arrow
                } else {
                    return Err(LexError {
                        message: "unexpected character '-'".to_string(),
                        span: Span::new(start, self.pos),
                    });
                }
            }

            '=' => {
                if self.peek_char() == Some('=') {
                    self.advance_char();
                    TokenKind::EqEq
                } else {
                    TokenKind::Eq
                }
            }

            '!' => {
                if self.peek_char() == Some('=') {
                    self.advance_char();
                    TokenKind::Ne
                } else {
                    return Err(LexError {
                        message: "expected '=' after '!'".to_string(),
                        span: Span::new(start, self.pos),
                    });
                }
            }

            '<' => {
                if self.peek_char() == Some('=') {
                    self.advance_char();
                    TokenKind::Le
                } else {
                    TokenKind::LAngle
                }
            }

            '>' => {
                if self.peek_char() == Some('=') {
                    self.advance_char();
                    TokenKind::Ge
                } else {
                    TokenKind::RAngle
                }
            }

            '@' => {
                // Parse @id or @compat
                let ident_start = self.pos;
                while let Some(c) = self.peek_char() {
                    if c.is_alphanumeric() || c == '_' {
                        self.advance_char();
                    } else {
                        break;
                    }
                }
                let ident: String = self.chars[ident_start..self.pos].iter().collect();
                match ident.as_str() {
                    "id" => TokenKind::AtId,
                    "compat" => TokenKind::AtCompat,
                    _ => {
                        return Err(LexError {
                            message: format!("unknown annotation @{ident}"),
                            span: Span::new(start, self.pos),
                        });
                    }
                }
            }

            '"' => self.lex_string(start)?,

            _ if ch.is_ascii_digit() => self.lex_number(start, ch)?,

            _ if ch.is_alphabetic() || ch == '_' => self.lex_ident_or_keyword(start, ch)?,

            _ => {
                return Err(LexError {
                    message: format!("unexpected character '{ch}'"),
                    span: Span::new(start, self.pos),
                });
            }
        };

        Ok(Token {
            kind,
            span: Span::new(start, self.pos),
        })
    }

    fn lex_string(&mut self, start: usize) -> Result<TokenKind, LexError> {
        let mut s = String::new();
        loop {
            match self.advance_char() {
                Some('"') => break,
                Some('\\') => {
                    match self.advance_char() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('\\') => s.push('\\'),
                        Some('"') => s.push('"'),
                        Some(c) => {
                            return Err(LexError {
                                message: format!("unknown escape sequence '\\{c}'"),
                                span: Span::new(start, self.pos),
                            });
                        }
                        None => {
                            return Err(LexError {
                                message: "unterminated string literal".to_string(),
                                span: Span::new(start, self.pos),
                            });
                        }
                    }
                }
                Some(c) => s.push(c),
                None => {
                    return Err(LexError {
                        message: "unterminated string literal".to_string(),
                        span: Span::new(start, self.pos),
                    });
                }
            }
        }
        Ok(TokenKind::StringLiteral(s))
    }

    fn lex_number(&mut self, _start: usize, first: char) -> Result<TokenKind, LexError> {
        let mut digits = String::new();
        digits.push(first);

        // Check for hex
        if first == '0' && self.peek_char() == Some('x') {
            self.advance_char();
            digits.push('x');
            while let Some(c) = self.peek_char() {
                if c.is_ascii_hexdigit() {
                    digits.push(c);
                    self.advance_char();
                } else {
                    break;
                }
            }
            let val = i64::from_str_radix(&digits[2..], 16).unwrap_or(0);
            return Ok(TokenKind::IntLiteral(val));
        }

        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() {
                digits.push(c);
                self.advance_char();
            } else {
                break;
            }
        }
        let val: i64 = digits.parse().unwrap_or(0);
        Ok(TokenKind::IntLiteral(val))
    }

    fn lex_ident_or_keyword(&mut self, _start: usize, first: char) -> Result<TokenKind, LexError> {
        let mut ident = String::new();
        ident.push(first);

        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' {
                ident.push(c);
                self.advance_char();
            } else {
                break;
            }
        }

        // Check for REQ-xxx pattern
        if ident.starts_with("REQ") && self.peek_char() == Some('-') {
            self.advance_char();
            ident.push('-');
            while let Some(c) = self.peek_char() {
                if c.is_alphanumeric() || c == '_' || c == '-' {
                    ident.push(c);
                    self.advance_char();
                } else {
                    break;
                }
            }
            return Ok(TokenKind::ReqId(ident));
        }

        let kind = match ident.as_str() {
            "module" => TokenKind::Module,
            "import" => TokenKind::Import,
            "as" => TokenKind::As,
            "type" => TokenKind::Type,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "fn" => TokenKind::Fn,
            "error" => TokenKind::Error,
            "capability" => TokenKind::Capability,
            "law" => TokenKind::Law,
            "req" => TokenKind::Req,
            "decision" => TokenKind::Decision,
            "gen" => TokenKind::Gen,
            "prop" => TokenKind::Prop,
            "oracle" => TokenKind::Oracle,
            "policy" => TokenKind::Policy,
            "refine" => TokenKind::Refine,
            "invariant" => TokenKind::Invariant,
            "requires" => TokenKind::Requires,
            "ensures" => TokenKind::Ensures,
            "effects" => TokenKind::Effects,
            "raises" => TokenKind::Raises,
            "perf" => TokenKind::Perf,
            "examples" => TokenKind::Examples,
            "notes" => TokenKind::Notes,
            "when" => TokenKind::When,
            "choose" => TokenKind::Choose,
            "forall" => TokenKind::Forall,
            "from" => TokenKind::From,
            "allow" => TokenKind::Allow,
            "deny" => TokenKind::Deny,
            "deterministic" => TokenKind::Deterministic,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "self" => TokenKind::SelfKw,
            "reference" => TokenKind::Reference,
            "optimized" => TokenKind::Optimized,
            "stable_call" => TokenKind::StableCall,
            "stable_semantics" => TokenKind::StableSemantics,
            "unstable" => TokenKind::Unstable,
            _ => TokenKind::Ident(ident),
        };

        Ok(kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(input: &str) -> Vec<TokenKind> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        tokens.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn test_module_decl() {
        let tokens = lex("module music.scale;");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Module,
                TokenKind::Ident("music".into()),
                TokenKind::Dot,
                TokenKind::Ident("scale".into()),
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_import_with_alias() {
        let tokens = lex("import std.core as core;");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Import,
                TokenKind::Ident("std".into()),
                TokenKind::Dot,
                TokenKind::Ident("core".into()),
                TokenKind::As,
                TokenKind::Ident("core".into()),
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_req_declaration() {
        let tokens = lex("req REQ-1: \"Notes must be in range\";");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Req,
                TokenKind::ReqId("REQ-1".into()),
                TokenKind::Colon,
                TokenKind::StringLiteral("Notes must be in range".into()),
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_type_with_refine() {
        let tokens = lex("type MidiNote = Int refine (1 <= self and self <= 12);");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Type,
                TokenKind::Ident("MidiNote".into()),
                TokenKind::Eq,
                TokenKind::Ident("Int".into()),
                TokenKind::Refine,
                TokenKind::LParen,
                TokenKind::IntLiteral(1),
                TokenKind::Le,
                TokenKind::SelfKw,
                TokenKind::And,
                TokenKind::SelfKw,
                TokenKind::Le,
                TokenKind::IntLiteral(12),
                TokenKind::RParen,
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_fn_header() {
        let tokens = lex("fn snap @id(\"music.snap.v1\") @compat(stable_semantics)");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Fn,
                TokenKind::Ident("snap".into()),
                TokenKind::AtId,
                TokenKind::LParen,
                TokenKind::StringLiteral("music.snap.v1".into()),
                TokenKind::RParen,
                TokenKind::AtCompat,
                TokenKind::LParen,
                TokenKind::StableSemantics,
                TokenKind::RParen,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_gen_block() {
        let tokens = lex("gen MidiNoteGen { range: 1..12; };");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Gen,
                TokenKind::Ident("MidiNoteGen".into()),
                TokenKind::LBrace,
                TokenKind::Ident("range".into()),
                TokenKind::Colon,
                TokenKind::IntLiteral(1),
                TokenKind::DotDot,
                TokenKind::IntLiteral(12),
                TokenKind::Semicolon,
                TokenKind::RBrace,
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_decision() {
        let tokens = lex("decision [REQ-3] tie_break:");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Decision,
                TokenKind::LBracket,
                TokenKind::ReqId("REQ-3".into()),
                TokenKind::RBracket,
                TokenKind::Ident("tie_break".into()),
                TokenKind::Colon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_prop_forall() {
        let tokens = lex("prop [REQ-2] snap_in_scale: forall n: MidiNote from MidiNoteGen");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Prop,
                TokenKind::LBracket,
                TokenKind::ReqId("REQ-2".into()),
                TokenKind::RBracket,
                TokenKind::Ident("snap_in_scale".into()),
                TokenKind::Colon,
                TokenKind::Forall,
                TokenKind::Ident("n".into()),
                TokenKind::Colon,
                TokenKind::Ident("MidiNote".into()),
                TokenKind::From,
                TokenKind::Ident("MidiNoteGen".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_policy() {
        let tokens = lex("policy { deny Net; deterministic; };");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Policy,
                TokenKind::LBrace,
                TokenKind::Deny,
                TokenKind::Ident("Net".into()),
                TokenKind::Semicolon,
                TokenKind::Deterministic,
                TokenKind::Semicolon,
                TokenKind::RBrace,
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_oracle() {
        let tokens = lex("oracle music.scale.snap: reference;");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Oracle,
                TokenKind::Ident("music".into()),
                TokenKind::Dot,
                TokenKind::Ident("scale".into()),
                TokenKind::Dot,
                TokenKind::Ident("snap".into()),
                TokenKind::Colon,
                TokenKind::Reference,
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_comments_skipped() {
        let tokens = lex("# this is a comment\nmodule test;");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Module,
                TokenKind::Ident("test".into()),
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_string_escapes() {
        let tokens = lex(r#""hello\nworld""#);
        assert_eq!(
            tokens,
            vec![
                TokenKind::StringLiteral("hello\nworld".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_hex_literal() {
        let tokens = lex("0xFF");
        assert_eq!(
            tokens,
            vec![TokenKind::IntLiteral(255), TokenKind::Eof]
        );
    }

    #[test]
    fn test_comparison_ops() {
        let tokens = lex("== != <= >=");
        assert_eq!(
            tokens,
            vec![
                TokenKind::EqEq,
                TokenKind::Ne,
                TokenKind::Le,
                TokenKind::Ge,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_set_literal_braces() {
        let tokens = lex("{1,5,8}");
        assert_eq!(
            tokens,
            vec![
                TokenKind::LBrace,
                TokenKind::IntLiteral(1),
                TokenKind::Comma,
                TokenKind::IntLiteral(5),
                TokenKind::Comma,
                TokenKind::IntLiteral(8),
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_type_args() {
        let tokens = lex("Set<MidiNote>");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Ident("Set".into()),
                TokenKind::LAngle,
                TokenKind::Ident("MidiNote".into()),
                TokenKind::RAngle,
                TokenKind::Eof,
            ]
        );
    }
}
