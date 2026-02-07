//! IMPL lexer.
//!
//! Tokenizes `.impl` source files into a stream of tokens.

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

/// Token kinds for IMPL.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Module,
    Import,
    As,
    Impl,
    Fn,
    Let,
    Mut,
    If,
    Else,
    Match,
    Loop,
    While,
    Break,
    Continue,
    Return,
    Assert,
    Alloc,
    Borrow,
    BorrowMut,
    Own,
    Ref,
    MutRef,
    Slice,
    MutSlice,
    Cap,
    True,
    False,
    Region,

    // Literals
    IntLiteral(i128),
    FloatLiteral(f64),
    StringLiteral(String),

    // Identifiers
    Ident(String),

    // Punctuation
    LParen,      // (
    RParen,      // )
    LBrace,      // {
    RBrace,      // }
    LBracket,    // [
    RBracket,    // ]
    Comma,       // ,
    Colon,       // :
    Semicolon,   // ;
    Dot,         // .
    Arrow,       // ->
    FatArrow,    // =>
    Eq,          // =
    EqEq,        // ==
    Ne,          // !=
    Lt,          // <
    Gt,          // >
    Le,          // <=
    Ge,          // >=
    Plus,        // +
    Minus,       // -
    Star,        // *
    Slash,       // /
    Percent,     // %
    Amp,         // &
    Pipe,        // |
    Caret,       // ^
    Tilde,       // ~
    AmpAmp,      // &&
    PipePipe,    // ||
    LtLt,        // <<
    GtGt,        // >>
    Bang,        // !
    Question,    // ?
    Underscore,  // _ (wildcard)

    // Special
    Eof,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Module => write!(f, "module"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::As => write!(f, "as"),
            TokenKind::Impl => write!(f, "impl"),
            TokenKind::Fn => write!(f, "fn"),
            TokenKind::Let => write!(f, "let"),
            TokenKind::Mut => write!(f, "mut"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::Loop => write!(f, "loop"),
            TokenKind::While => write!(f, "while"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::Assert => write!(f, "assert"),
            TokenKind::Alloc => write!(f, "alloc"),
            TokenKind::Borrow => write!(f, "borrow"),
            TokenKind::BorrowMut => write!(f, "borrow_mut"),
            TokenKind::Own => write!(f, "own"),
            TokenKind::Ref => write!(f, "ref"),
            TokenKind::MutRef => write!(f, "mutref"),
            TokenKind::Slice => write!(f, "slice"),
            TokenKind::MutSlice => write!(f, "mutslice"),
            TokenKind::Cap => write!(f, "cap"),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::Region => write!(f, "region"),
            TokenKind::IntLiteral(n) => write!(f, "{n}"),
            TokenKind::FloatLiteral(n) => write!(f, "{n}"),
            TokenKind::StringLiteral(s) => write!(f, "\"{s}\""),
            TokenKind::Ident(s) => write!(f, "{s}"),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::FatArrow => write!(f, "=>"),
            TokenKind::Eq => write!(f, "="),
            TokenKind::EqEq => write!(f, "=="),
            TokenKind::Ne => write!(f, "!="),
            TokenKind::Lt => write!(f, "<"),
            TokenKind::Gt => write!(f, ">"),
            TokenKind::Le => write!(f, "<="),
            TokenKind::Ge => write!(f, ">="),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::Amp => write!(f, "&"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Caret => write!(f, "^"),
            TokenKind::Tilde => write!(f, "~"),
            TokenKind::AmpAmp => write!(f, "&&"),
            TokenKind::PipePipe => write!(f, "||"),
            TokenKind::LtLt => write!(f, "<<"),
            TokenKind::GtGt => write!(f, ">>"),
            TokenKind::Bang => write!(f, "!"),
            TokenKind::Question => write!(f, "?"),
            TokenKind::Underscore => write!(f, "_"),
            TokenKind::Eof => write!(f, "EOF"),
        }
    }
}

/// A token with span information.
#[derive(Debug, Clone)]
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
        write!(f, "lex error at {}: {}", self.span.start, self.message)
    }
}

impl std::error::Error for LexError {}

/// IMPL lexer.
pub struct Lexer<'a> {
    source: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Lexer {
            source: source.as_bytes(),
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

    fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace_and_comments();

        if self.pos >= self.source.len() {
            return Ok(Token {
                kind: TokenKind::Eof,
                span: Span::new(self.pos, self.pos),
            });
        }

        let start = self.pos;
        let ch = self.source[self.pos] as char;

        // String literals
        if ch == '"' {
            return self.lex_string(start);
        }

        // Number literals
        if ch.is_ascii_digit() {
            return self.lex_number(start);
        }

        // Identifiers and keywords
        if ch.is_ascii_alphabetic() || ch == '_' {
            return self.lex_ident_or_keyword(start);
        }

        // Punctuation
        self.lex_punct(start)
    }

    fn skip_whitespace_and_comments(&mut self) {
        while self.pos < self.source.len() {
            let ch = self.source[self.pos] as char;
            if ch.is_ascii_whitespace() {
                self.pos += 1;
            } else if ch == '/' && self.pos + 1 < self.source.len() {
                if self.source[self.pos + 1] == b'/' {
                    // Line comment
                    self.pos += 2;
                    while self.pos < self.source.len() && self.source[self.pos] != b'\n' {
                        self.pos += 1;
                    }
                } else if self.source[self.pos + 1] == b'*' {
                    // Block comment
                    self.pos += 2;
                    while self.pos + 1 < self.source.len() {
                        if self.source[self.pos] == b'*' && self.source[self.pos + 1] == b'/' {
                            self.pos += 2;
                            break;
                        }
                        self.pos += 1;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn lex_string(&mut self, start: usize) -> Result<Token, LexError> {
        self.pos += 1; // skip opening "
        let mut s = String::new();
        while self.pos < self.source.len() {
            let ch = self.source[self.pos] as char;
            if ch == '"' {
                self.pos += 1;
                return Ok(Token {
                    kind: TokenKind::StringLiteral(s),
                    span: Span::new(start, self.pos),
                });
            }
            if ch == '\\' && self.pos + 1 < self.source.len() {
                self.pos += 1;
                match self.source[self.pos] as char {
                    'n' => s.push('\n'),
                    't' => s.push('\t'),
                    '\\' => s.push('\\'),
                    '"' => s.push('"'),
                    other => {
                        s.push('\\');
                        s.push(other);
                    }
                }
                self.pos += 1;
            } else {
                s.push(ch);
                self.pos += 1;
            }
        }
        Err(LexError {
            message: "unterminated string literal".to_string(),
            span: Span::new(start, self.pos),
        })
    }

    fn lex_number(&mut self, start: usize) -> Result<Token, LexError> {
        let mut is_float = false;

        // Handle hex
        if self.pos + 1 < self.source.len()
            && self.source[self.pos] == b'0'
            && (self.source[self.pos + 1] == b'x' || self.source[self.pos + 1] == b'X')
        {
            self.pos += 2;
            while self.pos < self.source.len()
                && (self.source[self.pos] as char).is_ascii_hexdigit()
            {
                self.pos += 1;
            }
            let text = std::str::from_utf8(&self.source[start..self.pos]).unwrap();
            let value = i128::from_str_radix(&text[2..], 16).map_err(|_| LexError {
                message: format!("invalid hex literal: {text}"),
                span: Span::new(start, self.pos),
            })?;
            return Ok(Token {
                kind: TokenKind::IntLiteral(value),
                span: Span::new(start, self.pos),
            });
        }

        while self.pos < self.source.len() {
            let ch = self.source[self.pos] as char;
            if ch.is_ascii_digit() {
                self.pos += 1;
            } else if ch == '.' && !is_float {
                // Check it's not a method call like `x.foo`
                if self.pos + 1 < self.source.len()
                    && (self.source[self.pos + 1] as char).is_ascii_digit()
                {
                    is_float = true;
                    self.pos += 1;
                } else {
                    break;
                }
            } else if ch == '_' {
                self.pos += 1; // digit separator
            } else {
                break;
            }
        }

        let text: String = std::str::from_utf8(&self.source[start..self.pos])
            .unwrap()
            .chars()
            .filter(|c| *c != '_')
            .collect();

        if is_float {
            let value: f64 = text.parse().map_err(|_| LexError {
                message: format!("invalid float literal: {text}"),
                span: Span::new(start, self.pos),
            })?;
            Ok(Token {
                kind: TokenKind::FloatLiteral(value),
                span: Span::new(start, self.pos),
            })
        } else {
            let value: i128 = text.parse().map_err(|_| LexError {
                message: format!("invalid integer literal: {text}"),
                span: Span::new(start, self.pos),
            })?;
            Ok(Token {
                kind: TokenKind::IntLiteral(value),
                span: Span::new(start, self.pos),
            })
        }
    }

    fn lex_ident_or_keyword(&mut self, start: usize) -> Result<Token, LexError> {
        while self.pos < self.source.len() {
            let ch = self.source[self.pos] as char;
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.pos += 1;
            } else {
                break;
            }
        }

        let text = std::str::from_utf8(&self.source[start..self.pos]).unwrap();
        let span = Span::new(start, self.pos);

        // Check for multi-word keywords
        let kind = match text {
            "module" => TokenKind::Module,
            "import" => TokenKind::Import,
            "as" => TokenKind::As,
            "impl" => TokenKind::Impl,
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "match" => TokenKind::Match,
            "loop" => TokenKind::Loop,
            "while" => TokenKind::While,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "return" => TokenKind::Return,
            "assert" => TokenKind::Assert,
            "alloc" => TokenKind::Alloc,
            "borrow" => TokenKind::Borrow,
            "borrow_mut" => TokenKind::BorrowMut,
            "own" => TokenKind::Own,
            "ref" => TokenKind::Ref,
            "mutref" => TokenKind::MutRef,
            "slice" => TokenKind::Slice,
            "mutslice" => TokenKind::MutSlice,
            "cap" => TokenKind::Cap,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "region" => TokenKind::Region,
            "_" => TokenKind::Underscore,
            _ => TokenKind::Ident(text.to_string()),
        };

        Ok(Token { kind, span })
    }

    fn lex_punct(&mut self, start: usize) -> Result<Token, LexError> {
        let ch = self.source[self.pos] as char;
        let next = if self.pos + 1 < self.source.len() {
            Some(self.source[self.pos + 1] as char)
        } else {
            None
        };

        let (kind, len) = match (ch, next) {
            ('-', Some('>')) => (TokenKind::Arrow, 2),
            ('=', Some('>')) => (TokenKind::FatArrow, 2),
            ('=', Some('=')) => (TokenKind::EqEq, 2),
            ('!', Some('=')) => (TokenKind::Ne, 2),
            ('<', Some('=')) => (TokenKind::Le, 2),
            ('>', Some('=')) => (TokenKind::Ge, 2),
            ('<', Some('<')) => (TokenKind::LtLt, 2),
            ('>', Some('>')) => (TokenKind::GtGt, 2),
            ('&', Some('&')) => (TokenKind::AmpAmp, 2),
            ('|', Some('|')) => (TokenKind::PipePipe, 2),
            ('(', _) => (TokenKind::LParen, 1),
            (')', _) => (TokenKind::RParen, 1),
            ('{', _) => (TokenKind::LBrace, 1),
            ('}', _) => (TokenKind::RBrace, 1),
            ('[', _) => (TokenKind::LBracket, 1),
            (']', _) => (TokenKind::RBracket, 1),
            (',', _) => (TokenKind::Comma, 1),
            (':', _) => (TokenKind::Colon, 1),
            (';', _) => (TokenKind::Semicolon, 1),
            ('.', _) => (TokenKind::Dot, 1),
            ('=', _) => (TokenKind::Eq, 1),
            ('<', _) => (TokenKind::Lt, 1),
            ('>', _) => (TokenKind::Gt, 1),
            ('+', _) => (TokenKind::Plus, 1),
            ('-', _) => (TokenKind::Minus, 1),
            ('*', _) => (TokenKind::Star, 1),
            ('/', _) => (TokenKind::Slash, 1),
            ('%', _) => (TokenKind::Percent, 1),
            ('&', _) => (TokenKind::Amp, 1),
            ('|', _) => (TokenKind::Pipe, 1),
            ('^', _) => (TokenKind::Caret, 1),
            ('~', _) => (TokenKind::Tilde, 1),
            ('!', _) => (TokenKind::Bang, 1),
            ('?', _) => (TokenKind::Question, 1),
            _ => {
                return Err(LexError {
                    message: format!("unexpected character: {ch}"),
                    span: Span::new(start, start + 1),
                });
            }
        };

        self.pos += len;
        Ok(Token {
            kind,
            span: Span::new(start, self.pos),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(source: &str) -> Vec<TokenKind> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        tokens.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn test_keywords() {
        let tokens = lex("impl fn let mut if else match loop while break continue return");
        assert!(matches!(tokens[0], TokenKind::Impl));
        assert!(matches!(tokens[1], TokenKind::Fn));
        assert!(matches!(tokens[2], TokenKind::Let));
        assert!(matches!(tokens[3], TokenKind::Mut));
        assert!(matches!(tokens[4], TokenKind::If));
        assert!(matches!(tokens[5], TokenKind::Else));
        assert!(matches!(tokens[6], TokenKind::Match));
        assert!(matches!(tokens[7], TokenKind::Loop));
        assert!(matches!(tokens[8], TokenKind::While));
        assert!(matches!(tokens[9], TokenKind::Break));
        assert!(matches!(tokens[10], TokenKind::Continue));
        assert!(matches!(tokens[11], TokenKind::Return));
    }

    #[test]
    fn test_type_keywords() {
        let tokens = lex("own ref mutref slice mutslice cap region");
        assert!(matches!(tokens[0], TokenKind::Own));
        assert!(matches!(tokens[1], TokenKind::Ref));
        assert!(matches!(tokens[2], TokenKind::MutRef));
        assert!(matches!(tokens[3], TokenKind::Slice));
        assert!(matches!(tokens[4], TokenKind::MutSlice));
        assert!(matches!(tokens[5], TokenKind::Cap));
        assert!(matches!(tokens[6], TokenKind::Region));
    }

    #[test]
    fn test_literals() {
        let tokens = lex(r#"42 3.14 "hello" true false"#);
        assert_eq!(tokens[0], TokenKind::IntLiteral(42));
        assert_eq!(tokens[1], TokenKind::FloatLiteral(3.14));
        assert_eq!(tokens[2], TokenKind::StringLiteral("hello".to_string()));
        assert_eq!(tokens[3], TokenKind::True);
        assert_eq!(tokens[4], TokenKind::False);
    }

    #[test]
    fn test_hex_literal() {
        let tokens = lex("0xFF");
        assert_eq!(tokens[0], TokenKind::IntLiteral(255));
    }

    #[test]
    fn test_operators() {
        let tokens = lex("+ - * / % & | ^ ~ && || << >> == != <= >=");
        assert_eq!(tokens[0], TokenKind::Plus);
        assert_eq!(tokens[1], TokenKind::Minus);
        assert_eq!(tokens[2], TokenKind::Star);
        assert_eq!(tokens[3], TokenKind::Slash);
        assert_eq!(tokens[4], TokenKind::Percent);
        assert_eq!(tokens[5], TokenKind::Amp);
        assert_eq!(tokens[6], TokenKind::Pipe);
        assert_eq!(tokens[7], TokenKind::Caret);
        assert_eq!(tokens[8], TokenKind::Tilde);
        assert_eq!(tokens[9], TokenKind::AmpAmp);
        assert_eq!(tokens[10], TokenKind::PipePipe);
        assert_eq!(tokens[11], TokenKind::LtLt);
        assert_eq!(tokens[12], TokenKind::GtGt);
        assert_eq!(tokens[13], TokenKind::EqEq);
        assert_eq!(tokens[14], TokenKind::Ne);
        assert_eq!(tokens[15], TokenKind::Le);
        assert_eq!(tokens[16], TokenKind::Ge);
    }

    #[test]
    fn test_punctuation() {
        let tokens = lex("( ) { } [ ] , : ; . -> =>");
        assert_eq!(tokens[0], TokenKind::LParen);
        assert_eq!(tokens[1], TokenKind::RParen);
        assert_eq!(tokens[2], TokenKind::LBrace);
        assert_eq!(tokens[3], TokenKind::RBrace);
        assert_eq!(tokens[4], TokenKind::LBracket);
        assert_eq!(tokens[5], TokenKind::RBracket);
        assert_eq!(tokens[6], TokenKind::Comma);
        assert_eq!(tokens[7], TokenKind::Colon);
        assert_eq!(tokens[8], TokenKind::Semicolon);
        assert_eq!(tokens[9], TokenKind::Dot);
        assert_eq!(tokens[10], TokenKind::Arrow);
        assert_eq!(tokens[11], TokenKind::FatArrow);
    }

    #[test]
    fn test_impl_fn_header() {
        let tokens = lex(r#"impl fn "music.snap.v1" snap_to_scale(note: i32) -> i32"#);
        assert!(matches!(tokens[0], TokenKind::Impl));
        assert!(matches!(tokens[1], TokenKind::Fn));
        assert_eq!(
            tokens[2],
            TokenKind::StringLiteral("music.snap.v1".to_string())
        );
        assert_eq!(tokens[3], TokenKind::Ident("snap_to_scale".to_string()));
        assert_eq!(tokens[4], TokenKind::LParen);
        assert_eq!(tokens[5], TokenKind::Ident("note".to_string()));
        assert_eq!(tokens[6], TokenKind::Colon);
        assert_eq!(tokens[7], TokenKind::Ident("i32".to_string()));
        assert_eq!(tokens[8], TokenKind::RParen);
        assert_eq!(tokens[9], TokenKind::Arrow);
        assert_eq!(tokens[10], TokenKind::Ident("i32".to_string()));
    }

    #[test]
    fn test_comments() {
        let tokens = lex("42 // comment\n43 /* block */ 44");
        assert_eq!(tokens[0], TokenKind::IntLiteral(42));
        assert_eq!(tokens[1], TokenKind::IntLiteral(43));
        assert_eq!(tokens[2], TokenKind::IntLiteral(44));
    }

    #[test]
    fn test_digit_separator() {
        let tokens = lex("1_000_000");
        assert_eq!(tokens[0], TokenKind::IntLiteral(1_000_000));
    }

    #[test]
    fn test_string_escapes() {
        let tokens = lex(r#""hello\nworld""#);
        assert_eq!(
            tokens[0],
            TokenKind::StringLiteral("hello\nworld".to_string())
        );
    }

    #[test]
    fn test_builtin_functions() {
        let tokens = lex("alloc borrow borrow_mut assert");
        assert!(matches!(tokens[0], TokenKind::Alloc));
        assert!(matches!(tokens[1], TokenKind::Borrow));
        assert!(matches!(tokens[2], TokenKind::BorrowMut));
        assert!(matches!(tokens[3], TokenKind::Assert));
    }

    #[test]
    fn test_wildcard() {
        let tokens = lex("_ _x");
        assert!(matches!(tokens[0], TokenKind::Underscore));
        // `_x` is an identifier, not underscore
        assert_eq!(tokens[1], TokenKind::Ident("_x".to_string()));
    }

    #[test]
    fn test_empty() {
        let tokens = lex("");
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0], TokenKind::Eof));
    }

    #[test]
    fn test_unterminated_string() {
        let mut lexer = Lexer::new(r#""hello"#);
        assert!(lexer.tokenize().is_err());
    }
}
