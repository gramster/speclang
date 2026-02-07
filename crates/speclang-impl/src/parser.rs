//! IMPL parser.
//!
//! Parses `.impl` source into the IMPL AST.

use crate::ast::*;
use crate::lexer::{Lexer, Span, Token, TokenKind};
use std::fmt;

/// Parse error.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error at {}: {}", self.span.start, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parse IMPL source text into an `ImplProgram`.
pub fn parse_impl(source: &str) -> Result<ImplProgram, ParseError> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().map_err(|e| ParseError {
        message: e.message,
        span: e.span,
    })?;
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &TokenKind {
        self.tokens
            .get(self.pos)
            .map(|t| &t.kind)
            .unwrap_or(&TokenKind::Eof)
    }

    fn current_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|t| t.span)
            .unwrap_or(Span::new(0, 0))
    }

    fn advance(&mut self) -> TokenKind {
        let kind = self
            .tokens
            .get(self.pos)
            .map(|t| t.kind.clone())
            .unwrap_or(TokenKind::Eof);
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        kind
    }

    fn expect(&mut self, expected: &TokenKind) -> Result<(), ParseError> {
        if self.peek() == expected {
            self.advance();
            Ok(())
        } else {
            Err(ParseError {
                message: format!("expected {expected}, found {}", self.peek()),
                span: self.current_span(),
            })
        }
    }

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        match self.peek().clone() {
            TokenKind::Ident(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(ParseError {
                message: format!("expected identifier, found {other}"),
                span: self.current_span(),
            }),
        }
    }

    fn expect_string(&mut self) -> Result<String, ParseError> {
        match self.peek().clone() {
            TokenKind::StringLiteral(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(ParseError {
                message: format!("expected string literal, found {other}"),
                span: self.current_span(),
            }),
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(kind)
    }

    // -----------------------------------------------------------------------
    // Program
    // -----------------------------------------------------------------------

    fn parse_program(&mut self) -> Result<ImplProgram, ParseError> {
        let mut items = Vec::new();
        while *self.peek() != TokenKind::Eof {
            items.push(self.parse_item()?);
        }
        Ok(ImplProgram { items })
    }

    fn parse_item(&mut self) -> Result<ImplItem, ParseError> {
        match self.peek() {
            TokenKind::Module => self.parse_module_decl(),
            TokenKind::Import => self.parse_import_decl(),
            TokenKind::Impl => self.parse_impl_fn(),
            _ => Err(ParseError {
                message: format!(
                    "expected `module`, `import`, or `impl`, found {}",
                    self.peek()
                ),
                span: self.current_span(),
            }),
        }
    }

    // -----------------------------------------------------------------------
    // Module / Import
    // -----------------------------------------------------------------------

    fn parse_module_decl(&mut self) -> Result<ImplItem, ParseError> {
        self.expect(&TokenKind::Module)?;
        let name = self.parse_qualified_name()?;
        self.expect(&TokenKind::Semicolon)?;
        Ok(ImplItem::Module(ModuleDecl { name }))
    }

    fn parse_import_decl(&mut self) -> Result<ImplItem, ParseError> {
        self.expect(&TokenKind::Import)?;
        let name = self.parse_qualified_name()?;
        let alias = if *self.peek() == TokenKind::As {
            self.advance();
            Some(self.expect_ident()?)
        } else {
            None
        };
        self.expect(&TokenKind::Semicolon)?;
        Ok(ImplItem::Import(ImportDecl { name, alias }))
    }

    fn parse_qualified_name(&mut self) -> Result<QualifiedName, ParseError> {
        let mut parts = vec![self.expect_ident()?];
        while *self.peek() == TokenKind::Dot {
            self.advance();
            parts.push(self.expect_ident()?);
        }
        Ok(parts)
    }

    // -----------------------------------------------------------------------
    // impl fn
    // -----------------------------------------------------------------------

    fn parse_impl_fn(&mut self) -> Result<ImplItem, ParseError> {
        self.expect(&TokenKind::Impl)?;
        self.expect(&TokenKind::Fn)?;

        let stable_id = self.expect_string()?;
        let name = self.expect_ident()?;

        self.expect(&TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(&TokenKind::RParen)?;

        let return_type = if *self.peek() == TokenKind::Arrow {
            self.advance();
            self.parse_type()?
        } else {
            ImplTypeRef::Named("unit".to_string())
        };

        let body = self.parse_block()?;

        Ok(ImplItem::Function(ImplFunction {
            stable_id,
            name,
            params,
            return_type,
            body,
        }))
    }

    fn parse_params(&mut self) -> Result<Vec<ImplParam>, ParseError> {
        let mut params = Vec::new();
        while *self.peek() != TokenKind::RParen {
            if !params.is_empty() {
                self.expect(&TokenKind::Comma)?;
                if *self.peek() == TokenKind::RParen {
                    break; // trailing comma
                }
            }
            params.push(self.parse_param()?);
        }
        Ok(params)
    }

    fn parse_param(&mut self) -> Result<ImplParam, ParseError> {
        let name = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;

        // Check for `cap Name` pattern
        if *self.peek() == TokenKind::Cap {
            self.advance();
            let cap_name = self.expect_ident()?;
            return Ok(ImplParam {
                name,
                ty: ImplTypeRef::Capability(cap_name),
                is_cap: true,
            });
        }

        let ty = self.parse_type()?;
        Ok(ImplParam {
            name,
            ty,
            is_cap: false,
        })
    }

    // -----------------------------------------------------------------------
    // Types
    // -----------------------------------------------------------------------

    fn parse_type(&mut self) -> Result<ImplTypeRef, ParseError> {
        let base = self.parse_base_type()?;

        // Check for `?` (option shorthand)
        if *self.peek() == TokenKind::Question {
            self.advance();
            return Ok(ImplTypeRef::Option(Box::new(base)));
        }

        Ok(base)
    }

    fn parse_base_type(&mut self) -> Result<ImplTypeRef, ParseError> {
        match self.peek().clone() {
            TokenKind::Own => {
                self.advance();
                self.expect(&TokenKind::LBracket)?;
                let region = self.expect_ident()?;
                self.expect(&TokenKind::Comma)?;
                let inner = self.parse_type()?;
                self.expect(&TokenKind::RBracket)?;
                Ok(ImplTypeRef::Own {
                    region,
                    inner: Box::new(inner),
                })
            }
            TokenKind::Ref => {
                self.advance();
                self.expect(&TokenKind::LBracket)?;
                let inner = self.parse_type()?;
                self.expect(&TokenKind::RBracket)?;
                Ok(ImplTypeRef::Ref(Box::new(inner)))
            }
            TokenKind::MutRef => {
                self.advance();
                self.expect(&TokenKind::LBracket)?;
                let inner = self.parse_type()?;
                self.expect(&TokenKind::RBracket)?;
                Ok(ImplTypeRef::MutRef(Box::new(inner)))
            }
            TokenKind::Slice => {
                self.advance();
                self.expect(&TokenKind::LBracket)?;
                let inner = self.parse_type()?;
                self.expect(&TokenKind::RBracket)?;
                Ok(ImplTypeRef::Slice(Box::new(inner)))
            }
            TokenKind::MutSlice => {
                self.advance();
                self.expect(&TokenKind::LBracket)?;
                let inner = self.parse_type()?;
                self.expect(&TokenKind::RBracket)?;
                Ok(ImplTypeRef::MutSlice(Box::new(inner)))
            }
            TokenKind::Region => {
                self.advance();
                Ok(ImplTypeRef::Region)
            }
            TokenKind::LParen => {
                self.advance();
                if *self.peek() == TokenKind::RParen {
                    self.advance();
                    return Ok(ImplTypeRef::Named("unit".to_string()));
                }
                let mut types = vec![self.parse_type()?];
                while *self.peek() == TokenKind::Comma {
                    self.advance();
                    if *self.peek() == TokenKind::RParen {
                        break;
                    }
                    types.push(self.parse_type()?);
                }
                self.expect(&TokenKind::RParen)?;
                if types.len() == 1 {
                    Ok(types.into_iter().next().unwrap())
                } else {
                    Ok(ImplTypeRef::Tuple(types))
                }
            }
            TokenKind::Ident(name) => {
                self.advance();
                let mut qname = vec![name];
                while *self.peek() == TokenKind::Dot {
                    self.advance();
                    qname.push(self.expect_ident()?);
                }

                // Check for generic args: Name[T1, T2]
                if *self.peek() == TokenKind::LBracket {
                    self.advance();
                    let mut args = vec![self.parse_type()?];
                    while *self.peek() == TokenKind::Comma {
                        self.advance();
                        args.push(self.parse_type()?);
                    }
                    self.expect(&TokenKind::RBracket)?;

                    // Special case: Result[T, E]
                    if qname.len() == 1 && qname[0] == "Result" && args.len() == 2 {
                        let mut args = args.into_iter();
                        return Ok(ImplTypeRef::Result {
                            ok: Box::new(args.next().unwrap()),
                            err: Box::new(args.next().unwrap()),
                        });
                    }

                    Ok(ImplTypeRef::Generic { name: qname, args })
                } else if qname.len() == 1 {
                    Ok(ImplTypeRef::Named(qname.into_iter().next().unwrap()))
                } else {
                    Ok(ImplTypeRef::Qualified(qname))
                }
            }
            other => Err(ParseError {
                message: format!("expected type, found {other}"),
                span: self.current_span(),
            }),
        }
    }

    // -----------------------------------------------------------------------
    // Blocks
    // -----------------------------------------------------------------------

    fn parse_block(&mut self) -> Result<ImplBlock, ParseError> {
        self.expect(&TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        let mut trailing_expr = None;

        while *self.peek() != TokenKind::RBrace {
            // Check if this could be a trailing expression (if/match/block at end)
            match self.peek() {
                TokenKind::If | TokenKind::Match | TokenKind::LBrace => {
                    let expr = self.parse_expr()?;
                    if *self.peek() == TokenKind::RBrace {
                        // Trailing expression
                        trailing_expr = Some(expr);
                        break;
                    }
                    // Not trailing — it's a statement, possibly needs semicolon
                    if *self.peek() == TokenKind::Semicolon {
                        self.advance();
                    }
                    stmts.push(ImplStmt::Expr(expr));
                    continue;
                }
                _ => {}
            }

            // Try to parse a statement
            let stmt = self.parse_stmt()?;

            // Check if this is a trailing expression (no semicolon before `}`)
            match &stmt {
                ImplStmt::Expr(expr) if *self.peek() == TokenKind::RBrace => {
                    trailing_expr = Some(expr.clone());
                }
                _ => {
                    stmts.push(stmt);
                }
            }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(ImplBlock::new(stmts, trailing_expr))
    }

    // -----------------------------------------------------------------------
    // Statements
    // -----------------------------------------------------------------------

    fn parse_stmt(&mut self) -> Result<ImplStmt, ParseError> {
        match self.peek() {
            TokenKind::Let => self.parse_let_stmt(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::Match => self.parse_match_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::Assert => self.parse_assert_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::Loop => self.parse_loop_stmt(),
            TokenKind::Break => {
                self.advance();
                self.expect(&TokenKind::Semicolon)?;
                Ok(ImplStmt::Break)
            }
            TokenKind::Continue => {
                self.advance();
                self.expect(&TokenKind::Semicolon)?;
                Ok(ImplStmt::Continue)
            }
            _ => self.parse_expr_or_assign_stmt(),
        }
    }

    fn parse_let_stmt(&mut self) -> Result<ImplStmt, ParseError> {
        self.expect(&TokenKind::Let)?;
        let is_mut = if *self.peek() == TokenKind::Mut {
            self.advance();
            true
        } else {
            false
        };

        let name = self.expect_ident()?;

        let ty = if *self.peek() == TokenKind::Colon {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(&TokenKind::Eq)?;
        let value = self.parse_expr()?;
        self.expect(&TokenKind::Semicolon)?;

        if is_mut {
            Ok(ImplStmt::LetMut { name, ty, value })
        } else {
            Ok(ImplStmt::Let { name, ty, value })
        }
    }

    fn parse_if_stmt(&mut self) -> Result<ImplStmt, ParseError> {
        self.expect(&TokenKind::If)?;
        let cond = self.parse_expr()?;
        let then_block = self.parse_block()?;
        let else_block = if *self.peek() == TokenKind::Else {
            self.advance();
            if *self.peek() == TokenKind::If {
                // else if chain → wrap in block
                let nested = self.parse_if_stmt()?;
                Some(ImplBlock::new(vec![nested], None))
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };
        Ok(ImplStmt::If {
            cond,
            then_block,
            else_block,
        })
    }

    fn parse_match_stmt(&mut self) -> Result<ImplStmt, ParseError> {
        self.expect(&TokenKind::Match)?;
        let expr = self.parse_expr()?;
        self.expect(&TokenKind::LBrace)?;
        let mut arms = Vec::new();
        while *self.peek() != TokenKind::RBrace {
            arms.push(self.parse_match_arm()?);
            // Optional comma between arms
            if *self.peek() == TokenKind::Comma {
                self.advance();
            }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(ImplStmt::Match { expr, arms })
    }

    fn parse_match_arm(&mut self) -> Result<ImplMatchArm, ParseError> {
        let pattern = self.parse_pattern()?;
        self.expect(&TokenKind::FatArrow)?;
        let body = if *self.peek() == TokenKind::LBrace {
            self.parse_block()?
        } else {
            let expr = self.parse_expr()?;
            ImplBlock::new(vec![], Some(expr))
        };
        Ok(ImplMatchArm { pattern, body })
    }

    fn parse_return_stmt(&mut self) -> Result<ImplStmt, ParseError> {
        self.expect(&TokenKind::Return)?;
        if *self.peek() == TokenKind::Semicolon {
            self.advance();
            return Ok(ImplStmt::Return(None));
        }
        let expr = self.parse_expr()?;
        self.expect(&TokenKind::Semicolon)?;
        Ok(ImplStmt::Return(Some(expr)))
    }

    fn parse_assert_stmt(&mut self) -> Result<ImplStmt, ParseError> {
        self.expect(&TokenKind::Assert)?;
        self.expect(&TokenKind::LParen)?;
        let cond = self.parse_expr()?;
        let message = if *self.peek() == TokenKind::Comma {
            self.advance();
            match self.peek().clone() {
                TokenKind::StringLiteral(s) => {
                    self.advance();
                    Some(s)
                }
                _ => None,
            }
        } else {
            None
        };
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::Semicolon)?;
        Ok(ImplStmt::Assert { cond, message })
    }

    fn parse_while_stmt(&mut self) -> Result<ImplStmt, ParseError> {
        self.expect(&TokenKind::While)?;
        let cond = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(ImplStmt::While { cond, body })
    }

    fn parse_loop_stmt(&mut self) -> Result<ImplStmt, ParseError> {
        self.expect(&TokenKind::Loop)?;
        let body = self.parse_block()?;
        Ok(ImplStmt::Loop(body))
    }

    fn parse_expr_or_assign_stmt(&mut self) -> Result<ImplStmt, ParseError> {
        let expr = self.parse_expr()?;

        // Check for assignment: `x = expr;`
        if *self.peek() == TokenKind::Eq {
            self.advance();
            if let ImplExpr::Var(name) = expr {
                let value = self.parse_expr()?;
                self.expect(&TokenKind::Semicolon)?;
                return Ok(ImplStmt::Assign {
                    target: name,
                    value,
                });
            } else {
                return Err(ParseError {
                    message: "invalid assignment target".to_string(),
                    span: self.current_span(),
                });
            }
        }

        // Expression statement — semicolon required unless before `}`
        if *self.peek() != TokenKind::RBrace {
            self.expect(&TokenKind::Semicolon)?;
        }
        Ok(ImplStmt::Expr(expr))
    }

    // -----------------------------------------------------------------------
    // Patterns
    // -----------------------------------------------------------------------

    fn parse_pattern(&mut self) -> Result<ImplPattern, ParseError> {
        match self.peek().clone() {
            TokenKind::Underscore => {
                self.advance();
                Ok(ImplPattern::Wildcard)
            }
            TokenKind::True => {
                self.advance();
                Ok(ImplPattern::Literal(ImplLiteral::Bool(true)))
            }
            TokenKind::False => {
                self.advance();
                Ok(ImplPattern::Literal(ImplLiteral::Bool(false)))
            }
            TokenKind::IntLiteral(n) => {
                self.advance();
                Ok(ImplPattern::Literal(ImplLiteral::Int(n)))
            }
            TokenKind::StringLiteral(s) => {
                self.advance();
                Ok(ImplPattern::Literal(ImplLiteral::String(s)))
            }
            TokenKind::LParen => {
                self.advance();
                let mut pats = Vec::new();
                while *self.peek() != TokenKind::RParen {
                    if !pats.is_empty() {
                        self.expect(&TokenKind::Comma)?;
                    }
                    pats.push(self.parse_pattern()?);
                }
                self.expect(&TokenKind::RParen)?;
                Ok(ImplPattern::Tuple(pats))
            }
            TokenKind::Ident(name) => {
                self.advance();

                // Check: Ident.Ident (enum variant)
                if *self.peek() == TokenKind::Dot {
                    self.advance();
                    let variant = self.expect_ident()?;

                    // Variant(fields)
                    if *self.peek() == TokenKind::LParen {
                        self.advance();
                        let mut fields = Vec::new();
                        while *self.peek() != TokenKind::RParen {
                            if !fields.is_empty() {
                                self.expect(&TokenKind::Comma)?;
                            }
                            fields.push(self.parse_pattern()?);
                        }
                        self.expect(&TokenKind::RParen)?;
                        return Ok(ImplPattern::Variant {
                            ty: vec![name],
                            variant,
                            fields,
                        });
                    }
                    // Unit variant
                    return Ok(ImplPattern::Variant {
                        ty: vec![name],
                        variant,
                        fields: vec![],
                    });
                }

                // Check: Ident { fields } (struct pattern)
                if *self.peek() == TokenKind::LBrace {
                    self.advance();
                    let mut fields = Vec::new();
                    while *self.peek() != TokenKind::RBrace {
                        if !fields.is_empty() {
                            self.expect(&TokenKind::Comma)?;
                        }
                        let field_name = self.expect_ident()?;
                        if *self.peek() == TokenKind::Colon {
                            self.advance();
                            let pat = self.parse_pattern()?;
                            fields.push((field_name, pat));
                        } else {
                            // shorthand: `{ x }` means `{ x: x }`
                            fields.push((
                                field_name.clone(),
                                ImplPattern::Bind(field_name),
                            ));
                        }
                    }
                    self.expect(&TokenKind::RBrace)?;
                    return Ok(ImplPattern::Struct {
                        ty: vec![name],
                        fields,
                    });
                }

                // Simple binding
                Ok(ImplPattern::Bind(name))
            }
            other => Err(ParseError {
                message: format!("expected pattern, found {other}"),
                span: self.current_span(),
            }),
        }
    }

    // -----------------------------------------------------------------------
    // Expressions (precedence climbing)
    // -----------------------------------------------------------------------

    fn parse_expr(&mut self) -> Result<ImplExpr, ParseError> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> Result<ImplExpr, ParseError> {
        let mut lhs = self.parse_and_expr()?;
        while *self.peek() == TokenKind::PipePipe {
            self.advance();
            let rhs = self.parse_and_expr()?;
            lhs = ImplExpr::BinOp {
                op: ImplBinOp::Or,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_and_expr(&mut self) -> Result<ImplExpr, ParseError> {
        let mut lhs = self.parse_comparison_expr()?;
        while *self.peek() == TokenKind::AmpAmp {
            self.advance();
            let rhs = self.parse_comparison_expr()?;
            lhs = ImplExpr::BinOp {
                op: ImplBinOp::And,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_comparison_expr(&mut self) -> Result<ImplExpr, ParseError> {
        let mut lhs = self.parse_bitor_expr()?;
        loop {
            let op = match self.peek() {
                TokenKind::EqEq => ImplBinOp::Eq,
                TokenKind::Ne => ImplBinOp::Ne,
                TokenKind::Lt => ImplBinOp::Lt,
                TokenKind::Le => ImplBinOp::Le,
                TokenKind::Gt => ImplBinOp::Gt,
                TokenKind::Ge => ImplBinOp::Ge,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_bitor_expr()?;
            lhs = ImplExpr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_bitor_expr(&mut self) -> Result<ImplExpr, ParseError> {
        let mut lhs = self.parse_bitxor_expr()?;
        while *self.peek() == TokenKind::Pipe {
            self.advance();
            let rhs = self.parse_bitxor_expr()?;
            lhs = ImplExpr::BinOp {
                op: ImplBinOp::BitOr,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_bitxor_expr(&mut self) -> Result<ImplExpr, ParseError> {
        let mut lhs = self.parse_bitand_expr()?;
        while *self.peek() == TokenKind::Caret {
            self.advance();
            let rhs = self.parse_bitand_expr()?;
            lhs = ImplExpr::BinOp {
                op: ImplBinOp::BitXor,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_bitand_expr(&mut self) -> Result<ImplExpr, ParseError> {
        let mut lhs = self.parse_shift_expr()?;
        while *self.peek() == TokenKind::Amp {
            self.advance();
            let rhs = self.parse_shift_expr()?;
            lhs = ImplExpr::BinOp {
                op: ImplBinOp::BitAnd,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_shift_expr(&mut self) -> Result<ImplExpr, ParseError> {
        let mut lhs = self.parse_add_expr()?;
        loop {
            let op = match self.peek() {
                TokenKind::LtLt => ImplBinOp::Shl,
                TokenKind::GtGt => ImplBinOp::Shr,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_add_expr()?;
            lhs = ImplExpr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_add_expr(&mut self) -> Result<ImplExpr, ParseError> {
        let mut lhs = self.parse_mul_expr()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => ImplBinOp::Add,
                TokenKind::Minus => ImplBinOp::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_mul_expr()?;
            lhs = ImplExpr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_mul_expr(&mut self) -> Result<ImplExpr, ParseError> {
        let mut lhs = self.parse_cast_expr()?;
        loop {
            let op = match self.peek() {
                TokenKind::Star => ImplBinOp::Mul,
                TokenKind::Slash => ImplBinOp::Div,
                TokenKind::Percent => ImplBinOp::Mod,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_cast_expr()?;
            lhs = ImplExpr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_cast_expr(&mut self) -> Result<ImplExpr, ParseError> {
        let mut expr = self.parse_unary_expr()?;
        while *self.peek() == TokenKind::As {
            self.advance();
            let target = self.parse_type()?;
            expr = ImplExpr::Convert {
                expr: Box::new(expr),
                target,
            };
        }
        Ok(expr)
    }

    fn parse_unary_expr(&mut self) -> Result<ImplExpr, ParseError> {
        match self.peek() {
            TokenKind::Minus => {
                self.advance();
                let operand = self.parse_unary_expr()?;
                Ok(ImplExpr::UnOp {
                    op: ImplUnOp::Neg,
                    operand: Box::new(operand),
                })
            }
            TokenKind::Bang => {
                self.advance();
                let operand = self.parse_unary_expr()?;
                Ok(ImplExpr::UnOp {
                    op: ImplUnOp::Not,
                    operand: Box::new(operand),
                })
            }
            TokenKind::Tilde => {
                self.advance();
                let operand = self.parse_unary_expr()?;
                Ok(ImplExpr::UnOp {
                    op: ImplUnOp::BitNot,
                    operand: Box::new(operand),
                })
            }
            _ => self.parse_postfix_expr(),
        }
    }

    fn parse_postfix_expr(&mut self) -> Result<ImplExpr, ParseError> {
        let mut expr = self.parse_primary_expr()?;

        loop {
            match self.peek() {
                TokenKind::Dot => {
                    self.advance();
                    let field = self.expect_ident()?;
                    // Check for method-like enum construction: `Ty.Variant(args)`
                    if *self.peek() == TokenKind::LParen {
                        if let ImplExpr::Var(ty_name) = &expr {
                            self.advance();
                            let mut args = Vec::new();
                            while *self.peek() != TokenKind::RParen {
                                if !args.is_empty() {
                                    self.expect(&TokenKind::Comma)?;
                                }
                                args.push(self.parse_expr()?);
                            }
                            self.expect(&TokenKind::RParen)?;
                            expr = ImplExpr::EnumLit {
                                ty: vec![ty_name.clone()],
                                variant: field,
                                args,
                            };
                            continue;
                        }
                    }
                    expr = ImplExpr::FieldGet {
                        expr: Box::new(expr),
                        field,
                    };
                }
                TokenKind::LParen => {
                    // Function call
                    if let ImplExpr::Var(name) = &expr {
                        let func_name = vec![name.clone()];
                        self.advance();
                        let mut args = Vec::new();
                        while *self.peek() != TokenKind::RParen {
                            if !args.is_empty() {
                                self.expect(&TokenKind::Comma)?;
                            }
                            args.push(self.parse_expr()?);
                        }
                        self.expect(&TokenKind::RParen)?;
                        expr = ImplExpr::Call {
                            func: func_name,
                            args,
                        };
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary_expr(&mut self) -> Result<ImplExpr, ParseError> {
        match self.peek().clone() {
            TokenKind::IntLiteral(n) => {
                self.advance();
                Ok(ImplExpr::Literal(ImplLiteral::Int(n)))
            }
            TokenKind::FloatLiteral(n) => {
                self.advance();
                Ok(ImplExpr::Literal(ImplLiteral::Float(n)))
            }
            TokenKind::StringLiteral(s) => {
                self.advance();
                Ok(ImplExpr::Literal(ImplLiteral::String(s)))
            }
            TokenKind::True => {
                self.advance();
                Ok(ImplExpr::Literal(ImplLiteral::Bool(true)))
            }
            TokenKind::False => {
                self.advance();
                Ok(ImplExpr::Literal(ImplLiteral::Bool(false)))
            }
            TokenKind::Alloc => {
                self.advance();
                self.expect(&TokenKind::LParen)?;
                let region = self.parse_expr()?;
                self.expect(&TokenKind::Comma)?;
                let value = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(ImplExpr::Alloc {
                    region: Box::new(region),
                    value: Box::new(value),
                })
            }
            TokenKind::Borrow => {
                self.advance();
                self.expect(&TokenKind::LParen)?;
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(ImplExpr::Borrow(Box::new(expr)))
            }
            TokenKind::BorrowMut => {
                self.advance();
                self.expect(&TokenKind::LParen)?;
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(ImplExpr::BorrowMut(Box::new(expr)))
            }
            TokenKind::Return => {
                self.advance();
                if *self.peek() == TokenKind::Semicolon || *self.peek() == TokenKind::RBrace {
                    Ok(ImplExpr::Return(None))
                } else {
                    let expr = self.parse_expr()?;
                    Ok(ImplExpr::Return(Some(Box::new(expr))))
                }
            }
            TokenKind::If => {
                self.advance();
                let cond = self.parse_expr()?;
                let then_block = self.parse_block()?;
                let else_block = if *self.peek() == TokenKind::Else {
                    self.advance();
                    if *self.peek() == TokenKind::If {
                        // Wrap else-if in block
                        let nested = self.parse_primary_expr()?;
                        Some(ImplBlock::new(vec![], Some(nested)))
                    } else {
                        Some(self.parse_block()?)
                    }
                } else {
                    None
                };
                Ok(ImplExpr::If {
                    cond: Box::new(cond),
                    then_block,
                    else_block,
                })
            }
            TokenKind::Match => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::LBrace)?;
                let mut arms = Vec::new();
                while *self.peek() != TokenKind::RBrace {
                    arms.push(self.parse_match_arm()?);
                    if *self.peek() == TokenKind::Comma {
                        self.advance();
                    }
                }
                self.expect(&TokenKind::RBrace)?;
                Ok(ImplExpr::Match {
                    expr: Box::new(expr),
                    arms,
                })
            }
            TokenKind::Loop => {
                self.advance();
                let body = self.parse_block()?;
                Ok(ImplExpr::Loop(body))
            }
            TokenKind::While => {
                self.advance();
                let cond = self.parse_expr()?;
                let body = self.parse_block()?;
                Ok(ImplExpr::While {
                    cond: Box::new(cond),
                    body,
                })
            }
            TokenKind::Break => {
                self.advance();
                Ok(ImplExpr::Break)
            }
            TokenKind::Continue => {
                self.advance();
                Ok(ImplExpr::Continue)
            }
            TokenKind::LBrace => {
                let block = self.parse_block()?;
                Ok(ImplExpr::Block(block))
            }
            TokenKind::LParen => {
                self.advance();
                if *self.peek() == TokenKind::RParen {
                    self.advance();
                    return Ok(ImplExpr::Literal(ImplLiteral::Unit));
                }
                let expr = self.parse_expr()?;
                if *self.peek() == TokenKind::Comma {
                    // Tuple literal
                    let mut exprs = vec![expr];
                    while *self.peek() == TokenKind::Comma {
                        self.advance();
                        if *self.peek() == TokenKind::RParen {
                            break;
                        }
                        exprs.push(self.parse_expr()?);
                    }
                    self.expect(&TokenKind::RParen)?;
                    return Ok(ImplExpr::TupleLit(exprs));
                }
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::Ident(name) => {
                self.advance();
                // Check for struct literal: Ident { field: expr, ... }
                // Only if next token is `{` and it looks like field initializers
                if *self.peek() == TokenKind::LBrace {
                    // Lookahead: if next is `ident :` then it's a struct literal
                    if self.is_struct_literal_start() {
                        self.advance(); // consume `{`
                        let mut fields = Vec::new();
                        while *self.peek() != TokenKind::RBrace {
                            if !fields.is_empty() {
                                self.expect(&TokenKind::Comma)?;
                                if *self.peek() == TokenKind::RBrace {
                                    break;
                                }
                            }
                            let field_name = self.expect_ident()?;
                            self.expect(&TokenKind::Colon)?;
                            let field_value = self.parse_expr()?;
                            fields.push((field_name, field_value));
                        }
                        self.expect(&TokenKind::RBrace)?;
                        return Ok(ImplExpr::StructLit {
                            ty: vec![name],
                            fields,
                        });
                    }
                }
                Ok(ImplExpr::Var(name))
            }
            other => Err(ParseError {
                message: format!("expected expression, found {other}"),
                span: self.current_span(),
            }),
        }
    }

    /// Lookahead to distinguish struct literal `Name { field: ... }` from
    /// block expression `name { stmt; ... }`.
    fn is_struct_literal_start(&self) -> bool {
        // After `{`, check if we have `ident :` pattern
        let after_brace = self.pos + 1;
        if after_brace + 1 < self.tokens.len() {
            if let TokenKind::Ident(_) = &self.tokens[after_brace].kind {
                if self.tokens[after_brace + 1].kind == TokenKind::Colon {
                    return true;
                }
            }
            // Empty struct: `Name {}`
            if self.tokens[after_brace].kind == TokenKind::RBrace {
                return true;
            }
        }
        false
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_module() {
        let prog = parse_impl("module music.scale;").unwrap();
        assert_eq!(prog.items.len(), 1);
        match &prog.items[0] {
            ImplItem::Module(m) => assert_eq!(m.name, vec!["music", "scale"]),
            _ => panic!("expected module"),
        }
    }

    #[test]
    fn test_parse_import() {
        let prog = parse_impl("import std.core as core;").unwrap();
        match &prog.items[0] {
            ImplItem::Import(i) => {
                assert_eq!(i.name, vec!["std", "core"]);
                assert_eq!(i.alias.as_deref(), Some("core"));
            }
            _ => panic!("expected import"),
        }
    }

    #[test]
    fn test_parse_simple_fn() {
        let src = r#"
            impl fn "test.add.v1" add(a: i32, b: i32) -> i32 {
                a + b
            }
        "#;
        let prog = parse_impl(src).unwrap();
        match &prog.items[0] {
            ImplItem::Function(f) => {
                assert_eq!(f.stable_id, "test.add.v1");
                assert_eq!(f.name, "add");
                assert_eq!(f.params.len(), 2);
                assert_eq!(f.params[0].name, "a");
                assert_eq!(f.params[1].name, "b");
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_cap_param() {
        let src = r#"
            impl fn "test.fetch.v1" fetch(url: string, net: cap Net) -> string {
                url
            }
        "#;
        let prog = parse_impl(src).unwrap();
        match &prog.items[0] {
            ImplItem::Function(f) => {
                assert_eq!(f.params.len(), 2);
                assert!(!f.params[0].is_cap);
                assert!(f.params[1].is_cap);
                assert_eq!(f.params[1].ty, ImplTypeRef::Capability("Net".to_string()));
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_let_and_return() {
        let src = r#"
            impl fn "test.v1" foo(x: i32) -> i32 {
                let y: i32 = x + 1;
                return y;
            }
        "#;
        let prog = parse_impl(src).unwrap();
        match &prog.items[0] {
            ImplItem::Function(f) => {
                assert_eq!(f.body.stmts.len(), 2);
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_let_mut() {
        let src = r#"
            impl fn "test.v1" foo() -> i32 {
                let mut x: i32 = 0;
                x = 42;
                x
            }
        "#;
        let prog = parse_impl(src).unwrap();
        match &prog.items[0] {
            ImplItem::Function(f) => {
                assert!(matches!(&f.body.stmts[0], ImplStmt::LetMut { .. }));
                assert!(matches!(&f.body.stmts[1], ImplStmt::Assign { .. }));
                assert!(f.body.expr.is_some());
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_if_else() {
        let src = r#"
            impl fn "test.v1" abs(x: i32) -> i32 {
                if x >= 0 {
                    x
                } else {
                    -x
                }
            }
        "#;
        let prog = parse_impl(src).unwrap();
        match &prog.items[0] {
            ImplItem::Function(f) => {
                assert!(f.body.expr.is_some());
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_match() {
        let src = r#"
            impl fn "test.v1" classify(x: i32) -> string {
                match x {
                    0 => "zero",
                    _ => "other",
                }
            }
        "#;
        let prog = parse_impl(src).unwrap();
        match &prog.items[0] {
            ImplItem::Function(f) => {
                assert!(f.body.expr.is_some());
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_while_loop() {
        let src = r#"
            impl fn "test.v1" countdown(n: i32) -> i32 {
                let mut x: i32 = n;
                while x > 0 {
                    x = x - 1;
                }
                x
            }
        "#;
        let prog = parse_impl(src).unwrap();
        match &prog.items[0] {
            ImplItem::Function(f) => {
                assert!(matches!(&f.body.stmts[1], ImplStmt::While { .. }));
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_struct_literal() {
        let src = r#"
            impl fn "test.v1" make_point() -> Point {
                Point { x: 1, y: 2 }
            }
        "#;
        let prog = parse_impl(src).unwrap();
        match &prog.items[0] {
            ImplItem::Function(f) => {
                assert!(f.body.expr.is_some());
                match f.body.expr.as_deref() {
                    Some(ImplExpr::StructLit { ty, fields }) => {
                        assert_eq!(ty, &vec!["Point".to_string()]);
                        assert_eq!(fields.len(), 2);
                    }
                    _ => panic!("expected struct literal"),
                }
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_alloc_borrow() {
        let src = r#"
            impl fn "test.v1" make_box(r: region) -> own[heap, i32] {
                let b: own[heap, i32] = alloc(r, 42);
                let x: ref[i32] = borrow(b);
                b
            }
        "#;
        let prog = parse_impl(src).unwrap();
        match &prog.items[0] {
            ImplItem::Function(f) => {
                assert_eq!(f.body.stmts.len(), 2);
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_enum_construction() {
        let src = r#"
            impl fn "test.v1" wrap(x: i32) -> Option[i32] {
                Option.Some(x)
            }
        "#;
        let prog = parse_impl(src).unwrap();
        match &prog.items[0] {
            ImplItem::Function(f) => {
                assert!(f.body.expr.is_some());
                match f.body.expr.as_deref() {
                    Some(ImplExpr::EnumLit { variant, .. }) => {
                        assert_eq!(variant, "Some");
                    }
                    _ => panic!("expected enum literal"),
                }
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_field_access() {
        let src = r#"
            impl fn "test.v1" get_x(p: Point) -> i32 {
                p.x
            }
        "#;
        let prog = parse_impl(src).unwrap();
        match &prog.items[0] {
            ImplItem::Function(f) => {
                assert!(f.body.expr.is_some());
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_assert() {
        let src = r#"
            impl fn "test.v1" checked_div(a: i32, b: i32) -> i32 {
                assert(b != 0, "division by zero");
                a / b
            }
        "#;
        let prog = parse_impl(src).unwrap();
        match &prog.items[0] {
            ImplItem::Function(f) => {
                assert!(matches!(&f.body.stmts[0], ImplStmt::Assert { .. }));
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_type_conversion() {
        let src = r#"
            impl fn "test.v1" widen(x: i32) -> i64 {
                x as i64
            }
        "#;
        let prog = parse_impl(src).unwrap();
        match &prog.items[0] {
            ImplItem::Function(f) => {
                assert!(f.body.expr.is_some());
                match f.body.expr.as_deref() {
                    Some(ImplExpr::Convert { target, .. }) => {
                        assert_eq!(*target, ImplTypeRef::Named("i64".to_string()));
                    }
                    _ => panic!("expected convert"),
                }
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_complex_program() {
        let src = r#"
            module music.scale;
            import std.core;

            impl fn "music.snap.v1" snap_to_scale(
                note: i32,
                scale: ref[Set[i32]],
            ) -> i32 {
                let mut best: i32 = -1;
                let mut best_dist: i32 = 13;
                let mut i: i32 = 0;
                while i < 12 {
                    if contains(scale, i + 1) {
                        let d: i32 = distance_mod12(note, i + 1);
                        if d < best_dist || (d == best_dist && i + 1 < best) {
                            best = i + 1;
                            best_dist = d;
                        }
                    }
                    i = i + 1;
                }
                best
            }
        "#;
        let prog = parse_impl(src).unwrap();
        assert_eq!(prog.items.len(), 3);
        assert!(matches!(&prog.items[0], ImplItem::Module(_)));
        assert!(matches!(&prog.items[1], ImplItem::Import(_)));
        assert!(matches!(&prog.items[2], ImplItem::Function(_)));
    }

    #[test]
    fn test_parse_ownership_types() {
        let src = r#"
            impl fn "test.v1" own_types(
                a: own[heap, i32],
                b: ref[string],
                c: mutref[i32],
                d: slice[u8],
                e: mutslice[u8],
            ) -> unit {
                ()
            }
        "#;
        let prog = parse_impl(src).unwrap();
        match &prog.items[0] {
            ImplItem::Function(f) => {
                assert_eq!(f.params.len(), 5);
                assert!(matches!(f.params[0].ty, ImplTypeRef::Own { .. }));
                assert!(matches!(f.params[1].ty, ImplTypeRef::Ref(_)));
                assert!(matches!(f.params[2].ty, ImplTypeRef::MutRef(_)));
                assert!(matches!(f.params[3].ty, ImplTypeRef::Slice(_)));
                assert!(matches!(f.params[4].ty, ImplTypeRef::MutSlice(_)));
            }
            _ => panic!("expected function"),
        }
    }
}
