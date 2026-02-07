//! Parser for Core IR textual form.
//!
//! Parses the canonical textual representation as defined in ir-grammar.md
//! into Core IR AST nodes.

use crate::lexer::{Lexer, Token, TokenKind, Span};
use speclang_ir::module::Module;

use std::fmt;

/// Parse error.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error at {}-{}: {}", self.span.start, self.span.end, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parse a Core IR module from source text.
pub fn parse_module(input: &str) -> Result<Module, ParseError> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().map_err(|e| ParseError {
        message: e.message,
        span: e.span,
    })?;
    let mut parser = Parser::new(&tokens);
    parser.parse_module()
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
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

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos];
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &TokenKind) -> Result<&Token, ParseError> {
        if self.peek() == expected {
            Ok(self.advance())
        } else {
            Err(ParseError {
                message: format!("expected {:?}, found {:?}", expected, self.peek()),
                span: self.current_span(),
            })
        }
    }

    fn parse_module(&mut self) -> Result<Module, ParseError> {
        self.expect(&TokenKind::Module)?;
        let name = self.parse_qname()?;
        self.expect(&TokenKind::LBrace)?;

        let mut module = Module::new(name);

        while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
            match self.peek() {
                TokenKind::Type => {
                    let type_def = self.parse_type_def()?;
                    module.type_defs.push(type_def);
                }
                TokenKind::Cap => {
                    let cap_def = self.parse_cap_def()?;
                    module.cap_defs.push(cap_def);
                }
                TokenKind::Fn => {
                    let func = self.parse_function()?;
                    module.functions.push(func);
                }
                TokenKind::Extern => {
                    let ext = self.parse_extern()?;
                    module.externs.push(ext);
                }
                _ => {
                    return Err(ParseError {
                        message: format!("unexpected token in module: {:?}", self.peek()),
                        span: self.current_span(),
                    });
                }
            }
        }

        self.expect(&TokenKind::RBrace)?;
        Ok(module)
    }

    fn parse_qname(&mut self) -> Result<Vec<String>, ParseError> {
        let mut parts = vec![self.parse_ident()?];
        while self.peek() == &TokenKind::Dot {
            self.advance();
            parts.push(self.parse_ident()?);
        }
        Ok(parts)
    }

    fn parse_ident(&mut self) -> Result<String, ParseError> {
        match self.peek().clone() {
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => Err(ParseError {
                message: format!("expected identifier, found {:?}", self.peek()),
                span: self.current_span(),
            }),
        }
    }

    fn parse_type_def(&mut self) -> Result<speclang_ir::module::TypeDef, ParseError> {
        self.expect(&TokenKind::Type)?;
        let name = self.parse_ident()?;
        self.expect(&TokenKind::Eq)?;
        let ty = self.parse_type()?;
        self.expect(&TokenKind::Semicolon)?;
        Ok(speclang_ir::module::TypeDef {
            name,
            ty,
            annotations: vec![],
        })
    }

    fn parse_cap_def(&mut self) -> Result<speclang_ir::capability::CapabilityDef, ParseError> {
        self.expect(&TokenKind::Cap)?;
        let name = self.parse_ident()?;
        let mut fields = vec![];
        if self.peek() == &TokenKind::LParen {
            self.advance();
            while self.peek() != &TokenKind::RParen {
                let fname = self.parse_ident()?;
                self.expect(&TokenKind::Colon)?;
                let fty = self.parse_type()?;
                fields.push(speclang_ir::capability::CapabilityField {
                    name: fname,
                    ty: fty,
                });
                if self.peek() == &TokenKind::Comma {
                    self.advance();
                }
            }
            self.expect(&TokenKind::RParen)?;
        }
        self.expect(&TokenKind::Semicolon)?;
        Ok(speclang_ir::capability::CapabilityDef { name, fields })
    }

    fn parse_function(&mut self) -> Result<speclang_ir::module::Function, ParseError> {
        self.expect(&TokenKind::Fn)?;
        let name = self.parse_ident()?;
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::Arrow)?;
        let return_type = self.parse_type()?;

        let mut effects = vec![];
        if self.peek() == &TokenKind::Effects {
            self.advance();
            self.expect(&TokenKind::LParen)?;
            while self.peek() != &TokenKind::RParen {
                let eff_name = self.parse_ident()?;
                effects.push(speclang_ir::capability::CapabilityType { name: eff_name });
                if self.peek() == &TokenKind::Comma {
                    self.advance();
                }
            }
            self.expect(&TokenKind::RParen)?;
        }

        // Parse optional contract metadata
        let contracts = vec![];
        let mut annotations = vec![];
        while self.peek() == &TokenKind::At {
            self.advance();
            let attr_name = self.parse_ident()?;
            match attr_name.as_str() {
                "id" => {
                    if let TokenKind::StringLiteral(id) = self.peek().clone() {
                        let id = id.clone();
                        self.advance();
                        annotations.push(speclang_ir::module::Annotation::Id(id));
                    }
                }
                "req_tag" => {
                    if let TokenKind::StringLiteral(tag) = self.peek().clone() {
                        let tag = tag.clone();
                        self.advance();
                        annotations.push(speclang_ir::module::Annotation::ReqTag(tag));
                    }
                }
                "requires" | "ensures" => {
                    // TODO: parse predicate expressions
                    // For now skip to semicolon
                    while self.peek() != &TokenKind::Semicolon
                        && self.peek() != &TokenKind::At
                        && self.peek() != &TokenKind::LBrace
                    {
                        self.advance();
                    }
                    if self.peek() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }
                _ => {}
            }
        }

        let body = self.parse_block()?;

        Ok(speclang_ir::module::Function {
            name,
            params,
            return_type,
            effects,
            contracts,
            body,
            annotations,
        })
    }

    fn parse_extern(&mut self) -> Result<speclang_ir::module::ExternFunction, ParseError> {
        self.expect(&TokenKind::Extern)?;
        self.expect(&TokenKind::Fn)?;
        let name = self.parse_ident()?;
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::Arrow)?;
        let return_type = self.parse_type()?;

        let mut effects = vec![];
        if self.peek() == &TokenKind::Effects {
            self.advance();
            self.expect(&TokenKind::LParen)?;
            while self.peek() != &TokenKind::RParen {
                let eff_name = self.parse_ident()?;
                effects.push(speclang_ir::capability::CapabilityType { name: eff_name });
                if self.peek() == &TokenKind::Comma {
                    self.advance();
                }
            }
            self.expect(&TokenKind::RParen)?;
        }
        self.expect(&TokenKind::Semicolon)?;

        Ok(speclang_ir::module::ExternFunction {
            name,
            params,
            return_type,
            effects,
            annotations: vec![],
        })
    }

    fn parse_params(&mut self) -> Result<Vec<speclang_ir::module::Param>, ParseError> {
        let mut params = vec![];
        while self.peek() != &TokenKind::RParen {
            let name = self.parse_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type()?;
            params.push(speclang_ir::module::Param { name, ty });
            if self.peek() == &TokenKind::Comma {
                self.advance();
            }
        }
        Ok(params)
    }

    fn parse_type(&mut self) -> Result<speclang_ir::Type, ParseError> {
        match self.peek().clone() {
            TokenKind::Bool => { self.advance(); Ok(speclang_ir::Type::Primitive(speclang_ir::types::PrimitiveType::Bool)) }
            TokenKind::U8 => { self.advance(); Ok(speclang_ir::Type::Primitive(speclang_ir::types::PrimitiveType::U8)) }
            TokenKind::U16 => { self.advance(); Ok(speclang_ir::Type::Primitive(speclang_ir::types::PrimitiveType::U16)) }
            TokenKind::U32 => { self.advance(); Ok(speclang_ir::Type::Primitive(speclang_ir::types::PrimitiveType::U32)) }
            TokenKind::U64 => { self.advance(); Ok(speclang_ir::Type::Primitive(speclang_ir::types::PrimitiveType::U64)) }
            TokenKind::U128 => { self.advance(); Ok(speclang_ir::Type::Primitive(speclang_ir::types::PrimitiveType::U128)) }
            TokenKind::I8 => { self.advance(); Ok(speclang_ir::Type::Primitive(speclang_ir::types::PrimitiveType::I8)) }
            TokenKind::I16 => { self.advance(); Ok(speclang_ir::Type::Primitive(speclang_ir::types::PrimitiveType::I16)) }
            TokenKind::I32 => { self.advance(); Ok(speclang_ir::Type::Primitive(speclang_ir::types::PrimitiveType::I32)) }
            TokenKind::I64 => { self.advance(); Ok(speclang_ir::Type::Primitive(speclang_ir::types::PrimitiveType::I64)) }
            TokenKind::I128 => { self.advance(); Ok(speclang_ir::Type::Primitive(speclang_ir::types::PrimitiveType::I128)) }
            TokenKind::F32 => { self.advance(); Ok(speclang_ir::Type::Primitive(speclang_ir::types::PrimitiveType::F32)) }
            TokenKind::F64 => { self.advance(); Ok(speclang_ir::Type::Primitive(speclang_ir::types::PrimitiveType::F64)) }
            TokenKind::Unit => { self.advance(); Ok(speclang_ir::Type::Primitive(speclang_ir::types::PrimitiveType::Unit)) }
            TokenKind::Int => { self.advance(); Ok(speclang_ir::Type::Primitive(speclang_ir::types::PrimitiveType::Int)) }
            TokenKind::StringKw => { self.advance(); Ok(speclang_ir::Type::Primitive(speclang_ir::types::PrimitiveType::String)) }
            TokenKind::BytesKw => { self.advance(); Ok(speclang_ir::Type::Primitive(speclang_ir::types::PrimitiveType::Bytes)) }
            TokenKind::Struct => {
                self.advance();
                self.expect(&TokenKind::LBrace)?;
                let mut fields = vec![];
                while self.peek() != &TokenKind::RBrace {
                    let name = self.parse_ident()?;
                    self.expect(&TokenKind::Colon)?;
                    let ty = self.parse_type()?;
                    fields.push(speclang_ir::types::Field { name, ty });
                    if self.peek() == &TokenKind::Comma {
                        self.advance();
                    }
                }
                self.expect(&TokenKind::RBrace)?;
                Ok(speclang_ir::Type::Struct(fields))
            }
            TokenKind::Enum => {
                self.advance();
                self.expect(&TokenKind::LBrace)?;
                let mut variants = vec![];
                while self.peek() != &TokenKind::RBrace {
                    let name = self.parse_ident()?;
                    let mut vfields = vec![];
                    if self.peek() == &TokenKind::LParen {
                        self.advance();
                        while self.peek() != &TokenKind::RParen {
                            vfields.push(self.parse_type()?);
                            if self.peek() == &TokenKind::Comma {
                                self.advance();
                            }
                        }
                        self.expect(&TokenKind::RParen)?;
                    }
                    variants.push(speclang_ir::types::Variant { name, fields: vfields });
                    if self.peek() == &TokenKind::Comma {
                        self.advance();
                    }
                }
                self.expect(&TokenKind::RBrace)?;
                Ok(speclang_ir::Type::Enum(variants))
            }
            TokenKind::Own => {
                self.advance();
                self.expect(&TokenKind::LBracket)?;
                let region = self.parse_region()?;
                self.expect(&TokenKind::Comma)?;
                let inner = self.parse_type()?;
                self.expect(&TokenKind::RBracket)?;
                Ok(speclang_ir::Type::Own { region, inner: Box::new(inner) })
            }
            TokenKind::Ref => {
                self.advance();
                self.expect(&TokenKind::LBracket)?;
                let inner = self.parse_type()?;
                self.expect(&TokenKind::RBracket)?;
                Ok(speclang_ir::Type::Ref(Box::new(inner)))
            }
            TokenKind::MutRef => {
                self.advance();
                self.expect(&TokenKind::LBracket)?;
                let inner = self.parse_type()?;
                self.expect(&TokenKind::RBracket)?;
                Ok(speclang_ir::Type::MutRef(Box::new(inner)))
            }
            TokenKind::Slice => {
                self.advance();
                self.expect(&TokenKind::LBracket)?;
                let inner = self.parse_type()?;
                self.expect(&TokenKind::RBracket)?;
                Ok(speclang_ir::Type::Slice(Box::new(inner)))
            }
            TokenKind::MutSlice => {
                self.advance();
                self.expect(&TokenKind::LBracket)?;
                let inner = self.parse_type()?;
                self.expect(&TokenKind::RBracket)?;
                Ok(speclang_ir::Type::MutSlice(Box::new(inner)))
            }
            TokenKind::Ident(_) => {
                let name = self.parse_qname()?;
                Ok(speclang_ir::Type::Named(name))
            }
            _ => Err(ParseError {
                message: format!("expected type, found {:?}", self.peek()),
                span: self.current_span(),
            }),
        }
    }

    fn parse_region(&mut self) -> Result<speclang_ir::types::Region, ParseError> {
        if self.peek() == &TokenKind::Heap {
            self.advance();
            Ok(speclang_ir::types::Region::Heap)
        } else {
            let name = self.parse_ident()?;
            Ok(speclang_ir::types::Region::Named(name))
        }
    }

    fn parse_block(&mut self) -> Result<speclang_ir::expr::Block, ParseError> {
        self.expect(&TokenKind::LBrace)?;
        let mut stmts = vec![];

        while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
            let stmt = self.parse_stmt()?;
            stmts.push(stmt);
        }

        self.expect(&TokenKind::RBrace)?;
        Ok(speclang_ir::expr::Block::new(stmts, None))
    }

    fn parse_stmt(&mut self) -> Result<speclang_ir::expr::Stmt, ParseError> {
        match self.peek() {
            TokenKind::Let => {
                self.advance();
                let name = self.parse_ident()?;
                self.expect(&TokenKind::Colon)?;
                let ty = self.parse_type()?;
                self.expect(&TokenKind::Eq)?;
                let value = self.parse_expr()?;
                self.expect(&TokenKind::Semicolon)?;
                Ok(speclang_ir::expr::Stmt::Let { name, ty, value })
            }
            TokenKind::Return => {
                self.advance();
                let value = self.parse_expr()?;
                self.expect(&TokenKind::Semicolon)?;
                Ok(speclang_ir::expr::Stmt::Return(value))
            }
            TokenKind::Assert => {
                self.advance();
                self.expect(&TokenKind::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(&TokenKind::Comma)?;
                let message = match self.peek().clone() {
                    TokenKind::StringLiteral(s) => {
                        let s = s.clone();
                        self.advance();
                        s
                    }
                    _ => {
                        return Err(ParseError {
                            message: "expected string literal for assert message".to_string(),
                            span: self.current_span(),
                        });
                    }
                };
                self.expect(&TokenKind::RParen)?;
                self.expect(&TokenKind::Semicolon)?;
                Ok(speclang_ir::expr::Stmt::Assert { cond, message })
            }
            _ => {
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::Semicolon)?;
                Ok(speclang_ir::expr::Stmt::Expr(expr))
            }
        }
    }

    fn parse_expr(&mut self) -> Result<speclang_ir::expr::Expr, ParseError> {
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<speclang_ir::expr::Expr, ParseError> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                TokenKind::EqEq => speclang_ir::expr::BinOp::Eq,
                TokenKind::Ne => speclang_ir::expr::BinOp::Ne,
                TokenKind::Lt => speclang_ir::expr::BinOp::Lt,
                TokenKind::Le => speclang_ir::expr::BinOp::Le,
                TokenKind::Gt => speclang_ir::expr::BinOp::Gt,
                TokenKind::Ge => speclang_ir::expr::BinOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = speclang_ir::expr::Expr::BinOp {
                op,
                lhs: Box::new(left),
                rhs: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<speclang_ir::expr::Expr, ParseError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => speclang_ir::expr::BinOp::Add,
                TokenKind::Minus => speclang_ir::expr::BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = speclang_ir::expr::Expr::BinOp {
                op,
                lhs: Box::new(left),
                rhs: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<speclang_ir::expr::Expr, ParseError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                TokenKind::Star => speclang_ir::expr::BinOp::Mul,
                TokenKind::Slash => speclang_ir::expr::BinOp::Div,
                TokenKind::Percent => speclang_ir::expr::BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = speclang_ir::expr::Expr::BinOp {
                op,
                lhs: Box::new(left),
                rhs: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<speclang_ir::expr::Expr, ParseError> {
        match self.peek() {
            TokenKind::Minus => {
                self.advance();
                let operand = self.parse_primary()?;
                Ok(speclang_ir::expr::Expr::UnOp {
                    op: speclang_ir::expr::UnOp::Neg,
                    operand: Box::new(operand),
                })
            }
            TokenKind::Bang => {
                self.advance();
                let operand = self.parse_primary()?;
                Ok(speclang_ir::expr::Expr::UnOp {
                    op: speclang_ir::expr::UnOp::Not,
                    operand: Box::new(operand),
                })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<speclang_ir::expr::Expr, ParseError> {
        match self.peek().clone() {
            TokenKind::IntLiteral(v) => {
                self.advance();
                Ok(speclang_ir::expr::Expr::Literal(speclang_ir::expr::Literal::Int(v)))
            }
            TokenKind::FloatLiteral(v) => {
                self.advance();
                Ok(speclang_ir::expr::Expr::Literal(speclang_ir::expr::Literal::F64(v)))
            }
            TokenKind::BoolLiteral(v) => {
                self.advance();
                Ok(speclang_ir::expr::Expr::Literal(speclang_ir::expr::Literal::Bool(v)))
            }
            TokenKind::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Ok(speclang_ir::expr::Expr::Literal(speclang_ir::expr::Literal::String(s)))
            }
            TokenKind::Call => {
                self.advance();
                let func = self.parse_qname()?;
                self.expect(&TokenKind::LParen)?;
                let mut args = vec![];
                while self.peek() != &TokenKind::RParen {
                    args.push(self.parse_expr()?);
                    if self.peek() == &TokenKind::Comma {
                        self.advance();
                    }
                }
                self.expect(&TokenKind::RParen)?;
                Ok(speclang_ir::expr::Expr::Call { func, args })
            }
            TokenKind::Ident(_) => {
                let name = self.parse_ident()?;
                Ok(speclang_ir::expr::Expr::Var(name))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            _ => Err(ParseError {
                message: format!("expected expression, found {:?}", self.peek()),
                span: self.current_span(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_module() {
        let input = "module test.empty {}";
        let module = parse_module(input).unwrap();
        assert_eq!(module.name, vec!["test", "empty"]);
        assert!(module.functions.is_empty());
        assert!(module.type_defs.is_empty());
    }

    #[test]
    fn test_parse_type_def() {
        let input = r#"module test {
            type MyInt = i32;
        }"#;
        let module = parse_module(input).unwrap();
        assert_eq!(module.type_defs.len(), 1);
        assert_eq!(module.type_defs[0].name, "MyInt");
    }

    #[test]
    fn test_parse_cap_def() {
        let input = r#"module test {
            cap Net(host: string);
            cap Clock;
        }"#;
        let module = parse_module(input).unwrap();
        assert_eq!(module.cap_defs.len(), 2);
        assert_eq!(module.cap_defs[0].name, "Net");
        assert_eq!(module.cap_defs[0].fields.len(), 1);
        assert_eq!(module.cap_defs[1].name, "Clock");
        assert!(module.cap_defs[1].fields.is_empty());
    }

    #[test]
    fn test_parse_pure_function() {
        let input = r#"module test {
            fn add(x: i32, y: i32) -> i32 {
                return x;
            }
        }"#;
        let module = parse_module(input).unwrap();
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "add");
        assert!(module.functions[0].is_pure());
        assert_eq!(module.functions[0].params.len(), 2);
    }

    #[test]
    fn test_parse_effectful_function() {
        let input = r#"module test {
            fn fetch(net: Net, url: string) -> string effects(Net) {
                return url;
            }
        }"#;
        let module = parse_module(input).unwrap();
        assert_eq!(module.functions[0].effects.len(), 1);
        assert_eq!(module.functions[0].effects[0].name, "Net");
    }

    #[test]
    fn test_parse_own_ref_types() {
        let input = r#"module test {
            fn borrow_it(p: own[heap, i32]) -> ref[i32] {
                return p;
            }
        }"#;
        let module = parse_module(input).unwrap();
        let param_ty = &module.functions[0].params[0].ty;
        assert!(matches!(param_ty, speclang_ir::Type::Own { .. }));
        let ret_ty = &module.functions[0].return_type;
        assert!(matches!(ret_ty, speclang_ir::Type::Ref(_)));
    }

    #[test]
    fn test_parse_req_tag_annotation() {
        let input = r#"module test {
            fn f(x: i32) -> i32
            @id "test.f.v1"
            @req_tag "REQ-001" {
                return x;
            }
        }"#;
        let module = parse_module(input).unwrap();
        assert_eq!(module.functions.len(), 1);
        let has_req_tag = module.functions[0].annotations.iter().any(|a| {
            matches!(a, speclang_ir::module::Annotation::ReqTag(t) if t == "REQ-001")
        });
        assert!(has_req_tag);
    }

    /// Round-trip: parse → print → re-parse → compare
    fn assert_roundtrip(input: &str) {
        let module1 = parse_module(input).unwrap();
        let printed = crate::print_module(&module1);
        let module2 = parse_module(&printed).unwrap_or_else(|e| {
            panic!("Round-trip re-parse failed:\n{e}\n\nPrinted:\n{printed}");
        });
        // Compare key structural properties
        assert_eq!(module1.name, module2.name, "module name mismatch");
        assert_eq!(module1.type_defs.len(), module2.type_defs.len(), "type def count mismatch");
        assert_eq!(module1.cap_defs.len(), module2.cap_defs.len(), "cap def count mismatch");
        assert_eq!(module1.functions.len(), module2.functions.len(), "function count mismatch");
        assert_eq!(module1.externs.len(), module2.externs.len(), "extern count mismatch");
        for (f1, f2) in module1.functions.iter().zip(module2.functions.iter()) {
            assert_eq!(f1.name, f2.name, "function name mismatch");
            assert_eq!(f1.params.len(), f2.params.len(), "param count mismatch for {}", f1.name);
            assert_eq!(f1.effects.len(), f2.effects.len(), "effect count mismatch for {}", f1.name);
            assert_eq!(f1.annotations.len(), f2.annotations.len(), "annotation count mismatch for {}", f1.name);
        }
    }

    #[test]
    fn test_roundtrip_empty_module() {
        assert_roundtrip("module test.empty {}");
    }

    #[test]
    fn test_roundtrip_types() {
        assert_roundtrip(r#"module test {
            type MyInt = i32;
            type Name = string;
        }"#);
    }

    #[test]
    fn test_roundtrip_capabilities() {
        assert_roundtrip(r#"module test {
            cap Net(host: string);
            cap Clock;
        }"#);
    }

    #[test]
    fn test_roundtrip_pure_function() {
        assert_roundtrip(r#"module test {
            fn add(x: i32, y: i32) -> i32 {
                return x;
            }
        }"#);
    }

    #[test]
    fn test_roundtrip_effectful_function() {
        assert_roundtrip(r#"module test {
            fn fetch(net: Net, url: string) -> string effects(Net) {
                return url;
            }
        }"#);
    }

    #[test]
    fn test_roundtrip_annotated_function() {
        assert_roundtrip(r#"module test {
            fn f(x: i32) -> i32
            @id "test.f.v1"
            @req_tag "REQ-001" {
                return x;
            }
        }"#);
    }

    #[test]
    fn test_roundtrip_complex_module() {
        assert_roundtrip(r#"module music.scale {
            type Midi = i32;
            cap Net(host: string);
            cap Clock;
            fn snap(note: i32, scale: i32) -> i32
            @id "music.snap.v1" {
                return note;
            }
            fn fetch(net: Net, url: string) -> string effects(Net) {
                return url;
            }
        }"#);
    }
}
