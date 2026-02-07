//! SPL parser.
//!
//! Parses SPL source into the SPL AST.

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

/// Parse SPL source text into a `Program`.
pub fn parse_program(source: &str) -> Result<Program, ParseError> {
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

    fn advance(&mut self) -> &TokenKind {
        let kind = self.tokens.get(self.pos).map(|t| &t.kind).unwrap_or(&TokenKind::Eof);
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
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            _ => Err(ParseError {
                message: format!("expected identifier, found {}", self.peek()),
                span: self.current_span(),
            }),
        }
    }

    fn expect_string(&mut self) -> Result<String, ParseError> {
        match self.peek().clone() {
            TokenKind::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            _ => Err(ParseError {
                message: format!("expected string literal, found {}", self.peek()),
                span: self.current_span(),
            }),
        }
    }

    fn expect_req_id(&mut self) -> Result<String, ParseError> {
        match self.peek().clone() {
            TokenKind::ReqId(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            _ => Err(ParseError {
                message: format!("expected REQ-xxx, found {}", self.peek()),
                span: self.current_span(),
            }),
        }
    }

    fn expect_int(&mut self) -> Result<i64, ParseError> {
        match self.peek().clone() {
            TokenKind::IntLiteral(n) => {
                self.advance();
                Ok(n)
            }
            _ => Err(ParseError {
                message: format!("expected integer, found {}", self.peek()),
                span: self.current_span(),
            }),
        }
    }

    // -----------------------------------------------------------------------
    // Program
    // -----------------------------------------------------------------------

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut items = Vec::new();
        while *self.peek() != TokenKind::Eof {
            items.push(self.parse_module_item()?);
        }
        Ok(Program { items })
    }

    fn parse_module_item(&mut self) -> Result<ModuleItem, ParseError> {
        match self.peek() {
            TokenKind::Module => self.parse_module_decl().map(ModuleItem::Module),
            TokenKind::Import => self.parse_import_decl().map(ModuleItem::Import),
            TokenKind::Capability => self.parse_capability_decl().map(ModuleItem::Capability),
            TokenKind::Type => self.parse_type_decl().map(ModuleItem::Type),
            TokenKind::Error => self.parse_error_decl().map(ModuleItem::Error),
            TokenKind::Fn => self.parse_fn_spec_decl().map(ModuleItem::FnSpec),
            TokenKind::Law => self.parse_law_decl().map(ModuleItem::Law),
            TokenKind::Req => self.parse_req_decl().map(ModuleItem::Req),
            TokenKind::Decision => self.parse_decision_decl().map(ModuleItem::Decision),
            TokenKind::Gen => self.parse_gen_decl().map(ModuleItem::Gen),
            TokenKind::Prop => self.parse_prop_decl().map(ModuleItem::Prop),
            TokenKind::Oracle => self.parse_oracle_decl().map(ModuleItem::Oracle),
            TokenKind::Policy => self.parse_policy_decl().map(ModuleItem::Policy),
            _ => Err(ParseError {
                message: format!("unexpected token {}", self.peek()),
                span: self.current_span(),
            }),
        }
    }

    // -----------------------------------------------------------------------
    // ModuleDecl: module QualifiedName ;
    // -----------------------------------------------------------------------

    fn parse_module_decl(&mut self) -> Result<ModuleDecl, ParseError> {
        self.expect(&TokenKind::Module)?;
        let name = self.parse_qualified_name()?;
        self.expect(&TokenKind::Semicolon)?;
        Ok(ModuleDecl { name })
    }

    // -----------------------------------------------------------------------
    // ImportDecl: import QualifiedName [ as Ident ] ;
    // -----------------------------------------------------------------------

    fn parse_import_decl(&mut self) -> Result<ImportDecl, ParseError> {
        self.expect(&TokenKind::Import)?;
        let name = self.parse_qualified_name()?;
        let alias = if *self.peek() == TokenKind::As {
            self.advance();
            Some(self.expect_ident()?)
        } else {
            None
        };
        self.expect(&TokenKind::Semicolon)?;
        Ok(ImportDecl { name, alias })
    }

    // -----------------------------------------------------------------------
    // CapabilityDecl: capability Ident ( [ CapParams ] ) ;
    // -----------------------------------------------------------------------

    fn parse_capability_decl(&mut self) -> Result<CapabilityDecl, ParseError> {
        self.expect(&TokenKind::Capability)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let mut params = Vec::new();
        while *self.peek() != TokenKind::RParen {
            if !params.is_empty() {
                self.expect(&TokenKind::Comma)?;
            }
            let pname = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type_ref()?;
            params.push(CapParam { name: pname, ty });
        }
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::Semicolon)?;
        Ok(CapabilityDecl { name, params })
    }

    // -----------------------------------------------------------------------
    // TypeDecl: type Ident TypeBody
    // -----------------------------------------------------------------------

    fn parse_type_decl(&mut self) -> Result<TypeDecl, ParseError> {
        self.expect(&TokenKind::Type)?;
        let name = self.expect_ident()?;
        let body = self.parse_type_body()?;
        Ok(TypeDecl { name, body })
    }

    fn parse_type_body(&mut self) -> Result<TypeBody, ParseError> {
        match self.peek() {
            TokenKind::Eq => self.parse_alias_type(),
            TokenKind::Struct => self.parse_struct_type(),
            TokenKind::Enum => self.parse_enum_type(),
            _ => Err(ParseError {
                message: format!("expected '=', 'struct', or 'enum' in type body, found {}", self.peek()),
                span: self.current_span(),
            }),
        }
    }

    fn parse_alias_type(&mut self) -> Result<TypeBody, ParseError> {
        self.expect(&TokenKind::Eq)?;
        let ty = self.parse_type_ref()?;
        let refine = if *self.peek() == TokenKind::Refine {
            self.advance();
            Some(self.parse_refine_expr()?)
        } else {
            None
        };
        self.expect(&TokenKind::Semicolon)?;
        Ok(TypeBody::Alias { ty, refine })
    }

    fn parse_struct_type(&mut self) -> Result<TypeBody, ParseError> {
        self.expect(&TokenKind::Struct)?;
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while *self.peek() != TokenKind::RBrace {
            let name = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type_ref()?;
            self.expect(&TokenKind::Semicolon)?;
            fields.push(FieldDecl { name, ty });
        }
        self.expect(&TokenKind::RBrace)?;
        let invariant = if *self.peek() == TokenKind::Invariant {
            self.advance();
            self.expect(&TokenKind::LBrace)?;
            let mut exprs = Vec::new();
            while *self.peek() != TokenKind::RBrace {
                exprs.push(self.parse_refine_expr()?);
                self.expect(&TokenKind::Semicolon)?;
            }
            self.expect(&TokenKind::RBrace)?;
            Some(exprs)
        } else {
            None
        };
        self.expect(&TokenKind::Semicolon)?;
        Ok(TypeBody::Struct { fields, invariant })
    }

    fn parse_enum_type(&mut self) -> Result<TypeBody, ParseError> {
        self.expect(&TokenKind::Enum)?;
        self.expect(&TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while *self.peek() != TokenKind::RBrace {
            let name = self.expect_ident()?;
            let mut fields = Vec::new();
            if *self.peek() == TokenKind::LParen {
                self.advance();
                while *self.peek() != TokenKind::RParen {
                    if !fields.is_empty() {
                        self.expect(&TokenKind::Comma)?;
                    }
                    fields.push(self.parse_type_ref()?);
                }
                self.expect(&TokenKind::RParen)?;
            }
            self.expect(&TokenKind::Semicolon)?;
            variants.push(VariantDecl { name, fields });
        }
        self.expect(&TokenKind::RBrace)?;
        self.expect(&TokenKind::Semicolon)?;
        Ok(TypeBody::Enum { variants })
    }

    // -----------------------------------------------------------------------
    // ErrorDecl: error Ident { ErrorVariant* } ;
    // -----------------------------------------------------------------------

    fn parse_error_decl(&mut self) -> Result<ErrorDecl, ParseError> {
        self.expect(&TokenKind::Error)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while *self.peek() != TokenKind::RBrace {
            let vname = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let message = self.expect_string()?;
            self.expect(&TokenKind::Semicolon)?;
            variants.push(ErrorVariant {
                name: vname,
                message,
            });
        }
        self.expect(&TokenKind::RBrace)?;
        self.expect(&TokenKind::Semicolon)?;
        Ok(ErrorDecl { name, variants })
    }

    // -----------------------------------------------------------------------
    // FnSpecDecl
    // -----------------------------------------------------------------------

    fn parse_fn_spec_decl(&mut self) -> Result<FnSpecDecl, ParseError> {
        self.expect(&TokenKind::Fn)?;
        let name = self.expect_ident()?;

        // @id("...")
        self.expect(&TokenKind::AtId)?;
        self.expect(&TokenKind::LParen)?;
        let stable_id = self.expect_string()?;
        self.expect(&TokenKind::RParen)?;

        // optional @compat(...)
        let compat = if *self.peek() == TokenKind::AtCompat {
            self.advance();
            self.expect(&TokenKind::LParen)?;
            let kind = match self.peek() {
                TokenKind::StableCall => { self.advance(); CompatKind::StableCall }
                TokenKind::StableSemantics => { self.advance(); CompatKind::StableSemantics }
                TokenKind::Unstable => { self.advance(); CompatKind::Unstable }
                _ => {
                    return Err(ParseError {
                        message: format!("expected compat kind, found {}", self.peek()),
                        span: self.current_span(),
                    });
                }
            };
            self.expect(&TokenKind::RParen)?;
            Some(kind)
        } else {
            None
        };

        // Parameters
        self.expect(&TokenKind::LParen)?;
        let mut params = Vec::new();
        while *self.peek() != TokenKind::RParen {
            if !params.is_empty() {
                self.expect(&TokenKind::Comma)?;
            }
            let pname = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type_ref()?;
            params.push(Param { name: pname, ty });
        }
        self.expect(&TokenKind::RParen)?;

        // -> ReturnType
        self.expect(&TokenKind::Arrow)?;
        let return_type = self.parse_type_ref()?;

        // { FnBlock* }
        self.expect(&TokenKind::LBrace)?;
        let mut blocks = Vec::new();
        while *self.peek() != TokenKind::RBrace {
            blocks.push(self.parse_fn_block()?);
        }
        self.expect(&TokenKind::RBrace)?;
        self.expect(&TokenKind::Semicolon)?;

        Ok(FnSpecDecl {
            name,
            stable_id,
            compat,
            params,
            return_type,
            blocks,
        })
    }

    fn parse_fn_block(&mut self) -> Result<FnBlock, ParseError> {
        match self.peek() {
            TokenKind::Requires => {
                self.advance();
                let req_tags = self.parse_optional_req_tags()?;
                let conditions = self.parse_block_of_exprs()?;
                Ok(FnBlock::Requires { req_tags, conditions })
            }
            TokenKind::Ensures => {
                self.advance();
                let req_tags = self.parse_optional_req_tags()?;
                let conditions = self.parse_block_of_exprs()?;
                Ok(FnBlock::Ensures { req_tags, conditions })
            }
            TokenKind::Effects => {
                self.advance();
                self.expect(&TokenKind::LBrace)?;
                let mut items = Vec::new();
                while *self.peek() != TokenKind::RBrace {
                    if !items.is_empty() {
                        self.expect(&TokenKind::Comma)?;
                    }
                    let ename = self.expect_ident()?;
                    let mut args = Vec::new();
                    if *self.peek() == TokenKind::LParen {
                        self.advance();
                        while *self.peek() != TokenKind::RParen {
                            if !args.is_empty() {
                                self.expect(&TokenKind::Comma)?;
                            }
                            args.push(self.expect_ident()?);
                        }
                        self.expect(&TokenKind::RParen)?;
                    }
                    items.push(EffectItem { name: ename, args });
                }
                self.expect(&TokenKind::RBrace)?;
                Ok(FnBlock::Effects(items))
            }
            TokenKind::Raises => {
                self.advance();
                self.expect(&TokenKind::LBrace)?;
                let mut items = Vec::new();
                while *self.peek() != TokenKind::RBrace {
                    let error = self.parse_qualified_name()?;
                    let description = if *self.peek() == TokenKind::Colon {
                        self.advance();
                        Some(self.expect_string()?)
                    } else {
                        None
                    };
                    self.expect(&TokenKind::Semicolon)?;
                    items.push(RaisesItem { error, description });
                }
                self.expect(&TokenKind::RBrace)?;
                Ok(FnBlock::Raises(items))
            }
            TokenKind::Perf => {
                self.advance();
                self.expect(&TokenKind::LBrace)?;
                let mut items = Vec::new();
                while *self.peek() != TokenKind::RBrace {
                    let key = self.expect_ident()?;
                    self.expect(&TokenKind::Colon)?;
                    let value = match self.peek() {
                        TokenKind::StringLiteral(_) => self.expect_string()?,
                        TokenKind::IntLiteral(n) => {
                            let n = *n;
                            self.advance();
                            n.to_string()
                        }
                        TokenKind::Ident(_) => {
                            // Could be a qualified name
                            let qn = self.parse_qualified_name()?;
                            qn.join(".")
                        }
                        _ => {
                            return Err(ParseError {
                                message: format!("expected perf value, found {}", self.peek()),
                                span: self.current_span(),
                            });
                        }
                    };
                    self.expect(&TokenKind::Semicolon)?;
                    items.push(PerfItem { key, value });
                }
                self.expect(&TokenKind::RBrace)?;
                Ok(FnBlock::Perf(items))
            }
            TokenKind::Examples => {
                self.advance();
                let req_tags = self.parse_optional_req_tags()?;
                self.expect(&TokenKind::LBrace)?;
                let mut items = Vec::new();
                while *self.peek() != TokenKind::RBrace {
                    let label = self.expect_string()?;
                    self.expect(&TokenKind::Colon)?;
                    let lhs = self.parse_spl_expr()?;
                    self.expect(&TokenKind::EqEq)?;
                    let rhs = self.parse_spl_expr()?;
                    self.expect(&TokenKind::Semicolon)?;
                    items.push(ExampleItem { label, lhs, rhs });
                }
                self.expect(&TokenKind::RBrace)?;
                Ok(FnBlock::Examples { req_tags, items })
            }
            TokenKind::Notes => {
                self.advance();
                self.expect(&TokenKind::LBrace)?;
                let mut notes = Vec::new();
                while *self.peek() != TokenKind::RBrace {
                    notes.push(self.expect_string()?);
                    self.expect(&TokenKind::Semicolon)?;
                }
                self.expect(&TokenKind::RBrace)?;
                Ok(FnBlock::Notes(notes))
            }
            _ => Err(ParseError {
                message: format!("expected fn block keyword, found {}", self.peek()),
                span: self.current_span(),
            }),
        }
    }

    // -----------------------------------------------------------------------
    // LawDecl: law Ident : RefineExpr ;
    // -----------------------------------------------------------------------

    fn parse_law_decl(&mut self) -> Result<LawDecl, ParseError> {
        self.expect(&TokenKind::Law)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let expr = self.parse_refine_expr()?;
        self.expect(&TokenKind::Semicolon)?;
        Ok(LawDecl { name, expr })
    }

    // -----------------------------------------------------------------------
    // ReqDecl: req ReqId : String ;
    // -----------------------------------------------------------------------

    fn parse_req_decl(&mut self) -> Result<ReqDecl, ParseError> {
        self.expect(&TokenKind::Req)?;
        let tag = self.expect_req_id()?;
        self.expect(&TokenKind::Colon)?;
        let description = self.expect_string()?;
        self.expect(&TokenKind::Semicolon)?;
        Ok(ReqDecl { tag, description })
    }

    // -----------------------------------------------------------------------
    // DecisionDecl: decision [ReqTag] Ident : when: String; choose: String;
    // -----------------------------------------------------------------------

    fn parse_decision_decl(&mut self) -> Result<DecisionDecl, ParseError> {
        self.expect(&TokenKind::Decision)?;
        let req_tags = self.parse_optional_req_tags()?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        self.expect(&TokenKind::When)?;
        self.expect(&TokenKind::Colon)?;
        let when = self.expect_string()?;
        self.expect(&TokenKind::Semicolon)?;
        self.expect(&TokenKind::Choose)?;
        self.expect(&TokenKind::Colon)?;
        let choose = self.expect_string()?;
        self.expect(&TokenKind::Semicolon)?;
        Ok(DecisionDecl {
            req_tags,
            name,
            when,
            choose,
        })
    }

    // -----------------------------------------------------------------------
    // GenDecl: gen Ident { GenField* } ;
    // -----------------------------------------------------------------------

    fn parse_gen_decl(&mut self) -> Result<GenDecl, ParseError> {
        self.expect(&TokenKind::Gen)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while *self.peek() != TokenKind::RBrace {
            let key = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let value = self.parse_gen_value()?;
            self.expect(&TokenKind::Semicolon)?;
            fields.push(GenField { key, value });
        }
        self.expect(&TokenKind::RBrace)?;
        self.expect(&TokenKind::Semicolon)?;
        Ok(GenDecl { name, fields })
    }

    fn parse_gen_value(&mut self) -> Result<GenValue, ParseError> {
        match self.peek() {
            TokenKind::StringLiteral(_) => {
                let s = self.expect_string()?;
                Ok(GenValue::StringLit(s))
            }
            TokenKind::IntLiteral(_) => {
                let lo = self.expect_int()?;
                if *self.peek() == TokenKind::DotDot {
                    self.advance();
                    let hi = self.expect_int()?;
                    Ok(GenValue::IntRange(lo, hi))
                } else {
                    // Just a single int — treat as ident-like? No, the grammar says IntRange.
                    // Fall back: single int not really in grammar, but be lenient.
                    Ok(GenValue::IntRange(lo, lo))
                }
            }
            TokenKind::LBracket => {
                self.advance();
                let mut items = Vec::new();
                while *self.peek() != TokenKind::RBracket {
                    if !items.is_empty() {
                        self.expect(&TokenKind::Comma)?;
                    }
                    items.push(self.parse_gen_value()?);
                }
                self.expect(&TokenKind::RBracket)?;
                Ok(GenValue::List(items))
            }
            TokenKind::Ident(_) => {
                let id = self.expect_ident()?;
                Ok(GenValue::Ident(id))
            }
            _ => Err(ParseError {
                message: format!("expected gen value, found {}", self.peek()),
                span: self.current_span(),
            }),
        }
    }

    // -----------------------------------------------------------------------
    // PropDecl: prop [ReqTag] Ident : { PropQuantifier } PropBody ;
    // -----------------------------------------------------------------------

    fn parse_prop_decl(&mut self) -> Result<PropDecl, ParseError> {
        self.expect(&TokenKind::Prop)?;
        let req_tags = self.parse_optional_req_tags()?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;

        let mut quantifiers = Vec::new();
        while *self.peek() == TokenKind::Forall {
            self.advance();
            let vname = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type_ref()?;
            let generator = if *self.peek() == TokenKind::From {
                self.advance();
                Some(self.expect_ident()?)
            } else {
                None
            };
            quantifiers.push(PropQuantifier {
                name: vname,
                ty,
                generator,
            });
        }

        let body = self.parse_refine_expr()?;
        self.expect(&TokenKind::Semicolon)?;

        Ok(PropDecl {
            req_tags,
            name,
            quantifiers,
            body,
        })
    }

    // -----------------------------------------------------------------------
    // OracleDecl: oracle QualifiedName : OracleKind ;
    // -----------------------------------------------------------------------

    fn parse_oracle_decl(&mut self) -> Result<OracleDecl, ParseError> {
        self.expect(&TokenKind::Oracle)?;
        let name = self.parse_qualified_name()?;
        self.expect(&TokenKind::Colon)?;
        let kind = match self.peek() {
            TokenKind::Reference => {
                self.advance();
                OracleKind::Reference
            }
            TokenKind::Optimized => {
                self.advance();
                OracleKind::Optimized
            }
            _ => {
                return Err(ParseError {
                    message: format!("expected 'reference' or 'optimized', found {}", self.peek()),
                    span: self.current_span(),
                });
            }
        };
        self.expect(&TokenKind::Semicolon)?;
        Ok(OracleDecl { name, kind })
    }

    // -----------------------------------------------------------------------
    // PolicyDecl: policy { PolicyRule* } ;
    // -----------------------------------------------------------------------

    fn parse_policy_decl(&mut self) -> Result<PolicyDecl, ParseError> {
        self.expect(&TokenKind::Policy)?;
        self.expect(&TokenKind::LBrace)?;
        let mut rules = Vec::new();
        while *self.peek() != TokenKind::RBrace {
            rules.push(self.parse_policy_rule()?);
        }
        self.expect(&TokenKind::RBrace)?;
        self.expect(&TokenKind::Semicolon)?;
        Ok(PolicyDecl { rules })
    }

    fn parse_policy_rule(&mut self) -> Result<PolicyRule, ParseError> {
        match self.peek() {
            TokenKind::Allow => {
                self.advance();
                let names = vec![self.expect_ident()?];
                // allow Ident [ "(" ... ")" ] ;
                // For simplicity, skip constraint parsing for now
                if *self.peek() == TokenKind::LParen {
                    // Skip constraint
                    self.advance();
                    let mut depth = 1;
                    while depth > 0 {
                        match self.peek() {
                            TokenKind::LParen => { depth += 1; self.advance(); }
                            TokenKind::RParen => { depth -= 1; self.advance(); }
                            TokenKind::Eof => break,
                            _ => { self.advance(); }
                        }
                    }
                }
                self.expect(&TokenKind::Semicolon)?;
                Ok(PolicyRule::Allow(names))
            }
            TokenKind::Deny => {
                self.advance();
                let mut names = vec![self.expect_ident()?];
                while *self.peek() == TokenKind::Comma {
                    self.advance();
                    names.push(self.expect_ident()?);
                }
                self.expect(&TokenKind::Semicolon)?;
                Ok(PolicyRule::Deny(names))
            }
            TokenKind::Deterministic => {
                self.advance();
                self.expect(&TokenKind::Semicolon)?;
                Ok(PolicyRule::Deterministic)
            }
            _ => Err(ParseError {
                message: format!("expected policy rule, found {}", self.peek()),
                span: self.current_span(),
            }),
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn parse_qualified_name(&mut self) -> Result<QualifiedName, ParseError> {
        let mut parts = vec![self.expect_ident()?];
        while *self.peek() == TokenKind::Dot {
            self.advance();
            parts.push(self.expect_ident()?);
        }
        Ok(parts)
    }

    fn parse_type_ref(&mut self) -> Result<TypeRef, ParseError> {
        let name = self.parse_qualified_name()?;
        let mut args = Vec::new();
        if *self.peek() == TokenKind::LAngle {
            self.advance();
            while *self.peek() != TokenKind::RAngle {
                if !args.is_empty() {
                    self.expect(&TokenKind::Comma)?;
                }
                args.push(self.parse_type_ref()?);
            }
            self.expect(&TokenKind::RAngle)?;
        }
        let nullable = if *self.peek() == TokenKind::Question {
            self.advance();
            true
        } else {
            false
        };
        Ok(TypeRef {
            name,
            args,
            nullable,
        })
    }

    fn parse_optional_req_tags(&mut self) -> Result<Vec<String>, ParseError> {
        if *self.peek() != TokenKind::LBracket {
            return Ok(vec![]);
        }
        self.advance();
        let mut tags = Vec::new();
        loop {
            tags.push(self.expect_req_id()?);
            if *self.peek() == TokenKind::Comma {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(&TokenKind::RBracket)?;
        Ok(tags)
    }

    fn parse_block_of_exprs(&mut self) -> Result<Vec<RefineExpr>, ParseError> {
        self.expect(&TokenKind::LBrace)?;
        let mut exprs = Vec::new();
        while *self.peek() != TokenKind::RBrace {
            exprs.push(self.parse_refine_expr()?);
            self.expect(&TokenKind::Semicolon)?;
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(exprs)
    }

    // -----------------------------------------------------------------------
    // Refinement expressions
    // -----------------------------------------------------------------------

    fn parse_refine_expr(&mut self) -> Result<RefineExpr, ParseError> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> Result<RefineExpr, ParseError> {
        let mut lhs = self.parse_and_expr()?;
        while *self.peek() == TokenKind::Or {
            self.advance();
            let rhs = self.parse_and_expr()?;
            lhs = RefineExpr::Or(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_and_expr(&mut self) -> Result<RefineExpr, ParseError> {
        let mut lhs = self.parse_pred_expr()?;
        while *self.peek() == TokenKind::And {
            self.advance();
            let rhs = self.parse_pred_expr()?;
            lhs = RefineExpr::And(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_pred_expr(&mut self) -> Result<RefineExpr, ParseError> {
        if *self.peek() == TokenKind::Not {
            self.advance();
            let inner = self.parse_pred_expr()?;
            return Ok(RefineExpr::Not(Box::new(inner)));
        }
        if *self.peek() == TokenKind::LParen {
            self.advance();
            let inner = self.parse_refine_expr()?;
            self.expect(&TokenKind::RParen)?;
            return Ok(inner);
        }
        self.parse_compare_expr()
    }

    fn parse_compare_expr(&mut self) -> Result<RefineExpr, ParseError> {
        let lhs = self.parse_refine_atom()?;
        let op = match self.peek() {
            TokenKind::EqEq => Some(CompareOp::Eq),
            TokenKind::Ne => Some(CompareOp::Ne),
            TokenKind::LAngle => Some(CompareOp::Lt),
            TokenKind::Le => Some(CompareOp::Le),
            TokenKind::RAngle => Some(CompareOp::Gt),
            TokenKind::Ge => Some(CompareOp::Ge),
            _ => None,
        };
        if let Some(op) = op {
            self.advance();
            let rhs = self.parse_refine_atom()?;
            Ok(RefineExpr::Compare {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
            })
        } else {
            Ok(RefineExpr::Atom(lhs))
        }
    }

    fn parse_refine_atom(&mut self) -> Result<RefineAtom, ParseError> {
        match self.peek().clone() {
            TokenKind::SelfKw => {
                self.advance();
                Ok(RefineAtom::SelfRef)
            }
            TokenKind::IntLiteral(n) => {
                let n = n;
                self.advance();
                Ok(RefineAtom::IntLit(n))
            }
            TokenKind::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Ok(RefineAtom::StringLit(s))
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                if *self.peek() == TokenKind::LParen {
                    // function call
                    self.advance();
                    let mut args = Vec::new();
                    while *self.peek() != TokenKind::RParen {
                        if !args.is_empty() {
                            self.expect(&TokenKind::Comma)?;
                        }
                        args.push(self.parse_refine_atom()?);
                    }
                    self.expect(&TokenKind::RParen)?;
                    Ok(RefineAtom::Call(name, args))
                } else {
                    Ok(RefineAtom::Ident(name))
                }
            }
            _ => Err(ParseError {
                message: format!("expected refinement atom, found {}", self.peek()),
                span: self.current_span(),
            }),
        }
    }

    // -----------------------------------------------------------------------
    // SPL Expressions (for examples)
    // -----------------------------------------------------------------------

    fn parse_spl_expr(&mut self) -> Result<SplExpr, ParseError> {
        match self.peek().clone() {
            TokenKind::IntLiteral(n) => {
                let n = n;
                self.advance();
                Ok(SplExpr::IntLit(n))
            }
            TokenKind::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Ok(SplExpr::StringLit(s))
            }
            TokenKind::LBrace => {
                // Set literal: {1, 5, 8}
                self.advance();
                let mut items = Vec::new();
                while *self.peek() != TokenKind::RBrace {
                    if !items.is_empty() {
                        self.expect(&TokenKind::Comma)?;
                    }
                    items.push(self.parse_spl_expr()?);
                }
                self.expect(&TokenKind::RBrace)?;
                Ok(SplExpr::SetLit(items))
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                if *self.peek() == TokenKind::LParen {
                    // function call
                    self.advance();
                    let mut args = Vec::new();
                    while *self.peek() != TokenKind::RParen {
                        if !args.is_empty() {
                            self.expect(&TokenKind::Comma)?;
                        }
                        args.push(self.parse_spl_expr()?);
                    }
                    self.expect(&TokenKind::RParen)?;
                    Ok(SplExpr::Call(name, args))
                } else {
                    Ok(SplExpr::Ident(name))
                }
            }
            _ => Err(ParseError {
                message: format!("expected expression, found {}", self.peek()),
                span: self.current_span(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_module() {
        let prog = parse_program("module music.scale;").unwrap();
        assert_eq!(prog.items.len(), 1);
        match &prog.items[0] {
            ModuleItem::Module(m) => assert_eq!(m.name, vec!["music", "scale"]),
            _ => panic!("expected module"),
        }
    }

    #[test]
    fn test_parse_import() {
        let prog = parse_program("import std.core as core;").unwrap();
        assert_eq!(prog.items.len(), 1);
        match &prog.items[0] {
            ModuleItem::Import(i) => {
                assert_eq!(i.name, vec!["std", "core"]);
                assert_eq!(i.alias.as_deref(), Some("core"));
            }
            _ => panic!("expected import"),
        }
    }

    #[test]
    fn test_parse_req_decl() {
        let prog = parse_program("req REQ-1: \"Notes must be in range\";").unwrap();
        assert_eq!(prog.items.len(), 1);
        match &prog.items[0] {
            ModuleItem::Req(r) => {
                assert_eq!(r.tag, "REQ-1");
                assert_eq!(r.description, "Notes must be in range");
            }
            _ => panic!("expected req"),
        }
    }

    #[test]
    fn test_parse_type_alias_with_refine() {
        let prog = parse_program("type MidiNote = Int refine (1 <= self and self <= 12);").unwrap();
        assert_eq!(prog.items.len(), 1);
        match &prog.items[0] {
            ModuleItem::Type(t) => {
                assert_eq!(t.name, "MidiNote");
                match &t.body {
                    TypeBody::Alias { ty, refine } => {
                        assert_eq!(ty.name, vec!["Int"]);
                        assert!(refine.is_some());
                    }
                    _ => panic!("expected alias"),
                }
            }
            _ => panic!("expected type"),
        }
    }

    #[test]
    fn test_parse_struct_type() {
        let prog = parse_program(
            "type Point struct { x: Int; y: Int; };",
        )
        .unwrap();
        match &prog.items[0] {
            ModuleItem::Type(t) => {
                assert_eq!(t.name, "Point");
                match &t.body {
                    TypeBody::Struct { fields, invariant } => {
                        assert_eq!(fields.len(), 2);
                        assert_eq!(fields[0].name, "x");
                        assert_eq!(fields[1].name, "y");
                        assert!(invariant.is_none());
                    }
                    _ => panic!("expected struct"),
                }
            }
            _ => panic!("expected type"),
        }
    }

    #[test]
    fn test_parse_enum_type() {
        let prog = parse_program(
            "type Color enum { Red; Green; Blue; };",
        )
        .unwrap();
        match &prog.items[0] {
            ModuleItem::Type(t) => {
                assert_eq!(t.name, "Color");
                match &t.body {
                    TypeBody::Enum { variants } => {
                        assert_eq!(variants.len(), 3);
                        assert_eq!(variants[0].name, "Red");
                    }
                    _ => panic!("expected enum"),
                }
            }
            _ => panic!("expected type"),
        }
    }

    #[test]
    fn test_parse_error_decl() {
        let prog = parse_program(
            r#"error ScaleError { EmptyScale: "scale must not be empty"; };"#,
        )
        .unwrap();
        match &prog.items[0] {
            ModuleItem::Error(e) => {
                assert_eq!(e.name, "ScaleError");
                assert_eq!(e.variants.len(), 1);
                assert_eq!(e.variants[0].name, "EmptyScale");
            }
            _ => panic!("expected error"),
        }
    }

    #[test]
    fn test_parse_capability_decl() {
        let prog = parse_program("capability Net(host: Host);").unwrap();
        match &prog.items[0] {
            ModuleItem::Capability(c) => {
                assert_eq!(c.name, "Net");
                assert_eq!(c.params.len(), 1);
                assert_eq!(c.params[0].name, "host");
            }
            _ => panic!("expected capability"),
        }
    }

    #[test]
    fn test_parse_fn_spec() {
        let src = r#"
fn snap_to_scale @id("music.snap.v1") @compat(stable_semantics)
  (note: MidiNote, scale: Set<MidiNote>) -> MidiNote
{
  requires [REQ-2] { scale_is_nonempty(scale); }
  ensures  [REQ-2] { set_contains(scale, result); }
  notes {
    "Distance is circular over 12.";
  }
  perf {
    time: "O(|scale|)";
    alloc: "none";
  }
  examples [REQ-3] {
    "octave edge": snap_to_scale(12, {1,5,8}) == 1;
    "in scale":    snap_to_scale(1,  {1,5,8}) == 1;
  }
};
"#;
        let prog = parse_program(src).unwrap();
        match &prog.items[0] {
            ModuleItem::FnSpec(f) => {
                assert_eq!(f.name, "snap_to_scale");
                assert_eq!(f.stable_id, "music.snap.v1");
                assert_eq!(f.compat, Some(CompatKind::StableSemantics));
                assert_eq!(f.params.len(), 2);
                assert_eq!(f.params[0].name, "note");
                assert_eq!(f.params[1].name, "scale");
                assert_eq!(f.params[1].ty.args.len(), 1);
                assert_eq!(f.return_type.name, vec!["MidiNote"]);
                assert_eq!(f.blocks.len(), 5);
                // Check requires
                match &f.blocks[0] {
                    FnBlock::Requires { req_tags, conditions } => {
                        assert_eq!(req_tags, &vec!["REQ-2"]);
                        assert_eq!(conditions.len(), 1);
                    }
                    _ => panic!("expected requires"),
                }
                // Check examples
                match &f.blocks[4] {
                    FnBlock::Examples { req_tags, items } => {
                        assert_eq!(req_tags, &vec!["REQ-3"]);
                        assert_eq!(items.len(), 2);
                        assert_eq!(items[0].label, "octave edge");
                    }
                    _ => panic!("expected examples"),
                }
            }
            _ => panic!("expected fn spec"),
        }
    }

    #[test]
    fn test_parse_gen_decl() {
        let prog = parse_program("gen MidiNoteGen { range: 1..12; };").unwrap();
        match &prog.items[0] {
            ModuleItem::Gen(g) => {
                assert_eq!(g.name, "MidiNoteGen");
                assert_eq!(g.fields.len(), 1);
                assert_eq!(g.fields[0].key, "range");
                match &g.fields[0].value {
                    GenValue::IntRange(1, 12) => {}
                    other => panic!("expected IntRange(1,12), got {:?}", other),
                }
            }
            _ => panic!("expected gen"),
        }
    }

    #[test]
    fn test_parse_decision_decl() {
        let src = r#"
decision [REQ-3] tie_break:
  when: "multiple notes at equal minimum distance";
  choose: "the smaller numeric note";
"#;
        let prog = parse_program(src).unwrap();
        match &prog.items[0] {
            ModuleItem::Decision(d) => {
                assert_eq!(d.req_tags, vec!["REQ-3"]);
                assert_eq!(d.name, "tie_break");
                assert_eq!(d.when, "multiple notes at equal minimum distance");
                assert_eq!(d.choose, "the smaller numeric note");
            }
            _ => panic!("expected decision"),
        }
    }

    #[test]
    fn test_parse_prop_decl() {
        let src = r#"
prop [REQ-2] snap_in_scale:
  forall n: MidiNote from MidiNoteGen
  forall s: Set<MidiNote> from ScaleGen
  set_contains(s, snap_to_scale(n, s));
"#;
        let prog = parse_program(src).unwrap();
        match &prog.items[0] {
            ModuleItem::Prop(p) => {
                assert_eq!(p.req_tags, vec!["REQ-2"]);
                assert_eq!(p.name, "snap_in_scale");
                assert_eq!(p.quantifiers.len(), 2);
                assert_eq!(p.quantifiers[0].name, "n");
                assert_eq!(p.quantifiers[0].ty.name, vec!["MidiNote"]);
                assert_eq!(p.quantifiers[0].generator.as_deref(), Some("MidiNoteGen"));
                assert_eq!(p.quantifiers[1].name, "s");
                assert_eq!(p.quantifiers[1].ty.args.len(), 1);
                assert_eq!(p.quantifiers[1].generator.as_deref(), Some("ScaleGen"));
            }
            _ => panic!("expected prop"),
        }
    }

    #[test]
    fn test_parse_oracle_decl() {
        let prog =
            parse_program("oracle music.scale.snap_to_scale: reference;").unwrap();
        match &prog.items[0] {
            ModuleItem::Oracle(o) => {
                assert_eq!(o.name, vec!["music", "scale", "snap_to_scale"]);
                assert_eq!(o.kind, OracleKind::Reference);
            }
            _ => panic!("expected oracle"),
        }
    }

    #[test]
    fn test_parse_policy_decl() {
        let src = r#"
policy {
  deny Net;
  deny FileWrite;
  deterministic;
};
"#;
        let prog = parse_program(src).unwrap();
        match &prog.items[0] {
            ModuleItem::Policy(p) => {
                assert_eq!(p.rules.len(), 3);
                match &p.rules[0] {
                    PolicyRule::Deny(names) => assert_eq!(names, &vec!["Net"]),
                    _ => panic!("expected deny"),
                }
                match &p.rules[2] {
                    PolicyRule::Deterministic => {}
                    _ => panic!("expected deterministic"),
                }
            }
            _ => panic!("expected policy"),
        }
    }

    #[test]
    fn test_parse_full_example() {
        let src = r#"
module music.scale;

req REQ-1: "Notes must be in range 1..12";
req REQ-2: "Snap result must be in the scale";
req REQ-3: "Tie-break chooses smaller numeric note";

type MidiNote = Int refine (1 <= self and self <= 12);

gen MidiNoteGen {
  range: 1..12;
};

gen ScaleGen {
  elements: MidiNoteGen;
  len: 1..12;
};

decision [REQ-3] tie_break:
  when: "multiple notes at equal minimum distance";
  choose: "the smaller numeric note";

fn snap_to_scale @id("music.snap.v1") @compat(stable_semantics)
  (note: MidiNote, scale: Set<MidiNote>) -> MidiNote
{
  requires [REQ-2] { scale_is_nonempty(scale); }
  ensures  [REQ-2] { set_contains(scale, result); }
  notes {
    "Distance is circular over 12. Tie-break chooses smaller numeric note.";
  }
  perf {
    time: "O(|scale|)";
    alloc: "none";
  }
  examples [REQ-3] {
    "octave edge goes to 1, not 0": snap_to_scale(12, {1,5,8}) == 1;
    "already in scale":            snap_to_scale(1,  {1,5,8}) == 1;
  }
};

prop [REQ-2] snap_in_scale:
  forall n: MidiNote from MidiNoteGen
  forall s: Set<MidiNote> from ScaleGen
  set_contains(s, snap_to_scale(n, s));

oracle music.scale.snap_to_scale: reference;

policy {
  deny Net;
  deny FileWrite;
  deterministic;
};
"#;
        let prog = parse_program(src).unwrap();
        // module + 3 reqs + type + 2 gens + decision + fn + prop + oracle + policy = 12
        assert_eq!(prog.items.len(), 12);
    }
}
