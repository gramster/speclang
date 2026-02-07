//! Core IR → Rust source code generation.
//!
//! Produces a single Rust source string from a `Module`.

use speclang_ir::expr::{BinOp, Block, Expr, Literal, MatchArm, Pattern, Stmt, UnOp};
use speclang_ir::contract::{Contract, ContractKind, ContractPolicy};
use speclang_ir::module::{
    Annotation, Compat, ExternFunction, Function, Module,
    Param, TypeDef,
};
use speclang_ir::types::{PrimitiveType, Region, Type};
use speclang_ir::CapabilityDef;

// ---------------------------------------------------------------------------
// CodeGen state
// ---------------------------------------------------------------------------

/// Rust code generator.
pub struct RustCodeGen {
    buf: String,
    indent: usize,
}

impl RustCodeGen {
    pub fn new() -> Self {
        RustCodeGen {
            buf: String::new(),
            indent: 0,
        }
    }

    /// Generate Rust source from a Core IR module.
    pub fn generate(mut self, module: &Module) -> String {
        self.emit_header(module);
        self.emit_types(&module.type_defs);
        self.emit_capabilities(&module.cap_defs);
        self.emit_externs(&module.externs);
        self.emit_functions(&module.functions);
        self.buf
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn line(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.buf.push_str("    ");
        }
        self.buf.push_str(s);
        self.buf.push('\n');
    }

    fn blank(&mut self) {
        self.buf.push('\n');
    }

    fn push(&mut self) {
        self.indent += 1;
    }

    fn pop(&mut self) {
        assert!(self.indent > 0);
        self.indent -= 1;
    }

    // -----------------------------------------------------------------------
    // Header
    // -----------------------------------------------------------------------

    fn emit_header(&mut self, module: &Module) {
        let name = module.name.join("::");
        self.line(&format!("//! Generated from speclang module `{name}`"));
        self.line("#![allow(dead_code, unused_variables)]");
        self.blank();
    }

    // -----------------------------------------------------------------------
    // Types
    // -----------------------------------------------------------------------

    fn emit_types(&mut self, defs: &[TypeDef]) {
        for td in defs {
            self.emit_type_def(td);
            self.blank();
        }
    }

    fn emit_type_def(&mut self, td: &TypeDef) {
        self.emit_annotations(&td.annotations);
        match &td.ty {
            Type::Struct(fields) => {
                self.line(&format!("#[derive(Debug, Clone)]"));
                self.line(&format!("pub struct {} {{", td.name));
                self.push();
                for f in fields {
                    self.line(&format!(
                        "pub {}: {},",
                        f.name,
                        self.render_type(&f.ty)
                    ));
                }
                self.pop();
                self.line("}");
            }
            Type::Enum(variants) => {
                self.line(&format!("#[derive(Debug, Clone)]"));
                self.line(&format!("pub enum {} {{", td.name));
                self.push();
                for v in variants {
                    if v.fields.is_empty() {
                        self.line(&format!("{},", v.name));
                    } else {
                        let field_strs: Vec<String> = v
                            .fields
                            .iter()
                            .enumerate()
                            .map(|(_i, t)| self.render_type(t))
                            .collect();
                        self.line(&format!(
                            "{}({}),",
                            v.name,
                            field_strs.join(", ")
                        ));
                    }
                }
                self.pop();
                self.line("}");
            }
            Type::Tuple(elems) => {
                let inner: Vec<String> = elems.iter().map(|t| self.render_type(t)).collect();
                self.line(&format!(
                    "pub type {} = ({});",
                    td.name,
                    inner.join(", ")
                ));
            }
            _ => {
                // Type alias.
                self.line(&format!(
                    "pub type {} = {};",
                    td.name,
                    self.render_type(&td.ty)
                ));
            }
        }
    }

    fn render_type(&self, ty: &Type) -> String {
        match ty {
            Type::Primitive(prim) => self.render_primitive(prim).to_string(),
            Type::Named(name) => name.join("::"),
            Type::Struct(..) => "/* inline struct */".to_string(),
            Type::Enum(..) => "/* inline enum */".to_string(),
            Type::Tuple(elems) => {
                let inner: Vec<String> = elems.iter().map(|t| self.render_type(t)).collect();
                format!("({})", inner.join(", "))
            }
            Type::Own { region, inner } => {
                let inner_s = self.render_type(inner);
                match region {
                    Region::Heap => format!("Box<{inner_s}>"),
                    Region::Named(r) => format!("Box<{inner_s}> /* region {r} */"),
                }
            }
            Type::Ref(inner) => {
                let inner_s = self.render_type(inner);
                format!("&{inner_s}")
            }
            Type::MutRef(inner) => {
                let inner_s = self.render_type(inner);
                format!("&mut {inner_s}")
            }
            Type::Slice(inner) => {
                let inner_s = self.render_type(inner);
                format!("&[{inner_s}]")
            }
            Type::MutSlice(inner) => {
                let inner_s = self.render_type(inner);
                format!("&mut [{inner_s}]")
            }
            Type::Option(inner) => {
                let inner_s = self.render_type(inner);
                format!("Option<{inner_s}>")
            }
            Type::Result { ok, err } => {
                let ok_s = self.render_type(ok);
                let err_s = self.render_type(err);
                format!("Result<{ok_s}, {err_s}>")
            }
            Type::Generic { name, args } => {
                let name_s = name.join("::");
                let arg_strs: Vec<String> = args.iter().map(|a| self.render_type(a)).collect();
                format!("{name_s}<{}>", arg_strs.join(", "))
            }
            Type::Capability(_) => "()".to_string(), // capabilities are ZSTs in Rust
            Type::Region => "()".to_string(),
        }
    }

    fn render_primitive(&self, p: &PrimitiveType) -> &'static str {
        match p {
            PrimitiveType::Bool => "bool",
            PrimitiveType::U8 => "u8",
            PrimitiveType::U16 => "u16",
            PrimitiveType::U32 => "u32",
            PrimitiveType::U64 => "u64",
            PrimitiveType::U128 => "u128",
            PrimitiveType::I8 => "i8",
            PrimitiveType::I16 => "i16",
            PrimitiveType::I32 => "i32",
            PrimitiveType::I64 => "i64",
            PrimitiveType::I128 => "i128",
            PrimitiveType::F32 => "f32",
            PrimitiveType::F64 => "f64",
            PrimitiveType::Unit => "()",
            PrimitiveType::Int => "i64",    // default int
            PrimitiveType::String => "String",
            PrimitiveType::Bytes => "Vec<u8>",
        }
    }

    // -----------------------------------------------------------------------
    // Capabilities
    // -----------------------------------------------------------------------

    fn emit_capabilities(&mut self, caps: &[CapabilityDef]) {
        for cap in caps {
            self.line(&format!("/// Capability token: {}", cap.name));
            if cap.fields.is_empty() {
                self.line(&format!("pub struct {};", cap.name));
            } else {
                self.line(&format!("#[derive(Debug)]"));
                self.line(&format!("pub struct {} {{", cap.name));
                self.push();
                for f in &cap.fields {
                    self.line(&format!(
                        "pub {}: {},",
                        f.name,
                        self.render_type(&f.ty)
                    ));
                }
                self.pop();
                self.line("}");
            }
            self.blank();
        }
    }

    // -----------------------------------------------------------------------
    // Externs
    // -----------------------------------------------------------------------

    fn emit_externs(&mut self, externs: &[ExternFunction]) {
        if externs.is_empty() {
            return;
        }
        self.line("extern \"C\" {");
        self.push();
        for e in externs {
            self.emit_annotations(&e.annotations);
            let params = self.render_params(&e.params);
            let ret = self.render_return(&e.return_type);
            self.line(&format!("fn {name}({params}){ret};", name = e.name));
        }
        self.pop();
        self.line("}");
        self.blank();
    }

    // -----------------------------------------------------------------------
    // Functions
    // -----------------------------------------------------------------------

    fn emit_functions(&mut self, functions: &[Function]) {
        for f in functions {
            self.emit_function(f);
            self.blank();
        }
    }

    fn emit_function(&mut self, f: &Function) {
        self.emit_annotations(&f.annotations);

        // Build signature.
        let params = self.render_params(&f.params);
        let ret = self.render_return(&f.return_type);

        // Effects comment.
        if !f.effects.is_empty() {
            let eff_names: Vec<String> = f.effects.iter().map(|e| e.name.clone()).collect();
            self.line(&format!("// effects: {}", eff_names.join(", ")));
        }

        // Test attribute.
        let is_test = f
            .annotations
            .iter()
            .any(|a| matches!(a, Annotation::Id(id) if id.starts_with("test_") || id.starts_with("prop_")));

        if is_test {
            self.line("#[test]");
        }

        self.line(&format!("pub fn {name}({params}){ret} {{", name = f.name));
        self.push();

        // Emit requires contracts as debug_assert! at function entry.
        for c in &f.contracts {
            if c.kind == ContractKind::Requires {
                self.emit_contract_assert(c);
            }
        }

        // Body.
        self.emit_block_body(&f.body);

        // Emit ensures contracts.
        // In practice, ensures would capture the return value, but for now
        // we just emit them as comments.
        for c in &f.contracts {
            if c.kind == ContractKind::Ensures {
                let expr_str = self.render_expr(&c.predicate);
                self.line(&format!("// ensures: {expr_str}"));
            }
        }

        self.pop();
        self.line("}");
    }

    fn emit_contract_assert(&mut self, c: &Contract) {
        let pred = self.render_expr(&c.predicate);
        match c.policy {
            ContractPolicy::Always => {
                self.line(&format!("assert!({pred});"));
            }
            ContractPolicy::Debug => {
                self.line(&format!("debug_assert!({pred});"));
            }
            ContractPolicy::Sampled(n) => {
                self.line(&format!(
                    "// sampled({n}): assert!({pred});"
                ));
            }
        }
    }

    fn render_params(&self, params: &[Param]) -> String {
        params
            .iter()
            .map(|p| format!("{}: {}", p.name, self.render_type(&p.ty)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn render_return(&self, ty: &Type) -> String {
        match ty {
            Type::Primitive(PrimitiveType::Unit) => String::new(),
            _ => format!(" -> {}", self.render_type(ty)),
        }
    }

    // -----------------------------------------------------------------------
    // Annotations
    // -----------------------------------------------------------------------

    fn emit_annotations(&mut self, annotations: &[Annotation]) {
        for ann in annotations {
            match ann {
                Annotation::Id(id) => {
                    self.line(&format!("// id: {id}"));
                }
                Annotation::Compat(compat) => {
                    let s = match compat {
                        Compat::StableCall => "stable-call",
                        Compat::StableSemantics => "stable-semantics",
                        Compat::Unstable => "unstable",
                    };
                    self.line(&format!("// compat: {s}"));
                }
                Annotation::ReqTag(tag) => {
                    self.line(&format!("// req: {tag}"));
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Block / Statements
    // -----------------------------------------------------------------------

    fn emit_block_body(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.emit_stmt(stmt);
        }
        if let Some(tail) = &block.expr {
            let s = self.render_expr(tail);
            self.line(&s);
        }
    }

    fn emit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, ty, value } => {
                let val = self.render_expr(value);
                let ty_s = self.render_type(ty);
                self.line(&format!("let {name}: {ty_s} = {val};"));
            }
            Stmt::Assign { target, value } => {
                let val = self.render_expr(value);
                self.line(&format!("{target} = {val};"));
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                let cond_s = self.render_expr(cond);
                self.line(&format!("if {cond_s} {{"));
                self.push();
                self.emit_block_body(then_block);
                self.pop();
                self.line("} else {");
                self.push();
                self.emit_block_body(else_block);
                self.pop();
                self.line("}");
            }
            Stmt::Match { expr, arms } => {
                let scrut = self.render_expr(expr);
                self.line(&format!("match {scrut} {{"));
                self.push();
                for arm in arms {
                    self.emit_match_arm(arm);
                }
                self.pop();
                self.line("}");
            }
            Stmt::Return(e) => {
                let s = self.render_expr(e);
                self.line(&format!("return {s};"));
            }
            Stmt::Assert { cond, message } => {
                let c = self.render_expr(cond);
                if message.is_empty() {
                    self.line(&format!("assert!({c});"));
                } else {
                    self.line(&format!("assert!({c}, \"{message}\");"));
                }
            }
            Stmt::Expr(e) => {
                let s = self.render_expr(e);
                self.line(&format!("{s};"));
            }
        }
    }

    fn emit_match_arm(&mut self, arm: &MatchArm) {
        let pat = self.render_pattern(&arm.pattern);
        self.line(&format!("{pat} => {{"));
        self.push();
        self.emit_block_body(&arm.body);
        self.pop();
        self.line("}");
    }

    // -----------------------------------------------------------------------
    // Patterns
    // -----------------------------------------------------------------------

    fn render_pattern(&self, pat: &Pattern) -> String {
        match pat {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Bind(name) => name.clone(),
            Pattern::Literal(lit) => self.render_literal(lit),
            Pattern::Variant {
                ty,
                variant,
                fields,
            } => {
                let type_name = ty.join("::");
                if fields.is_empty() {
                    format!("{type_name}::{variant}")
                } else {
                    let field_pats: Vec<String> =
                        fields.iter().map(|f| self.render_pattern(f)).collect();
                    format!(
                        "{type_name}::{variant}({})",
                        field_pats.join(", ")
                    )
                }
            }
            Pattern::Tuple(pats) => {
                let inner: Vec<String> =
                    pats.iter().map(|p| self.render_pattern(p)).collect();
                format!("({})", inner.join(", "))
            }
            Pattern::Struct { ty, fields } => {
                let type_name = ty.join("::");
                let field_pats: Vec<String> = fields
                    .iter()
                    .map(|(name, pat)| {
                        let p = self.render_pattern(pat);
                        if p == *name {
                            name.clone()
                        } else {
                            format!("{name}: {p}")
                        }
                    })
                    .collect();
                format!("{type_name} {{ {} }}", field_pats.join(", "))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Expressions
    // -----------------------------------------------------------------------

    fn render_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Literal(lit) => self.render_literal(lit),
            Expr::Var(name) => name.clone(),
            Expr::BinOp { op, lhs, rhs } => {
                let l = self.render_expr(lhs);
                let r = self.render_expr(rhs);
                let op_s = self.render_binop(op);
                format!("({l} {op_s} {r})")
            }
            Expr::UnOp { op, operand } => {
                let o = self.render_expr(operand);
                let op_s = self.render_unop(op);
                format!("({op_s}{o})")
            }
            Expr::Call { func, args } => {
                let func_s = func.join("::");
                let arg_strs: Vec<String> =
                    args.iter().map(|a| self.render_expr(a)).collect();
                format!("{func_s}({})", arg_strs.join(", "))
            }
            Expr::StructLit { ty, fields } => {
                let type_name = ty.join("::");
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(name, val)| {
                        let v = self.render_expr(val);
                        format!("{name}: {v}")
                    })
                    .collect();
                format!("{type_name} {{ {} }}", field_strs.join(", "))
            }
            Expr::FieldGet { expr, field } => {
                let e = self.render_expr(expr);
                format!("{e}.{field}")
            }
            Expr::EnumLit {
                ty,
                variant,
                fields,
            } => {
                let type_name = ty.join("::");
                if fields.is_empty() {
                    format!("{type_name}::{variant}")
                } else {
                    let field_strs: Vec<String> =
                        fields.iter().map(|f| self.render_expr(f)).collect();
                    format!(
                        "{type_name}::{variant}({})",
                        field_strs.join(", ")
                    )
                }
            }
            Expr::TupleLit(elems) => {
                let inner: Vec<String> =
                    elems.iter().map(|e| self.render_expr(e)).collect();
                format!("({})", inner.join(", "))
            }
            Expr::If {
                cond,
                then_block,
                else_block,
            } => {
                let c = self.render_expr(cond);
                let then_s = self.render_block_inline(then_block);
                let else_s = self.render_block_inline(else_block);
                format!("if {c} {{ {then_s} }} else {{ {else_s} }}")
            }
            Expr::Match { expr, arms } => {
                let scrut = self.render_expr(expr);
                let arm_strs: Vec<String> = arms
                    .iter()
                    .map(|arm| {
                        let pat = self.render_pattern(&arm.pattern);
                        let body = self.render_block_inline(&arm.body);
                        format!("{pat} => {{ {body} }}")
                    })
                    .collect();
                format!("match {scrut} {{ {} }}", arm_strs.join(", "))
            }
            Expr::Block(block) => {
                let body = self.render_block_inline(block);
                format!("{{ {body} }}")
            }
            Expr::Alloc { value, .. } => {
                let v = self.render_expr(value);
                format!("Box::new({v})")
            }
            Expr::Borrow(inner) => {
                let e = self.render_expr(inner);
                format!("&{e}")
            }
            Expr::BorrowMut(inner) => {
                let e = self.render_expr(inner);
                format!("&mut {e}")
            }
            Expr::Convert { expr, target } => {
                let e = self.render_expr(expr);
                let t = self.render_type(target);
                format!("({e} as {t})")
            }
        }
    }

    fn render_literal(&self, lit: &Literal) -> String {
        match lit {
            Literal::Bool(b) => b.to_string(),
            Literal::Int(n) => n.to_string(),
            Literal::BigInt(s) => s.clone(),
            Literal::F32(f) => format!("{f}_f32"),
            Literal::F64(f) => format!("{f}_f64"),
            Literal::String(s) => format!("\"{s}\".to_string()"),
            Literal::Bytes(b) => format!("vec!{b:?}"),
            Literal::Unit => "()".to_string(),
        }
    }

    fn render_binop(&self, op: &BinOp) -> &'static str {
        match op {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Mod => "%",
            BinOp::BitAnd => "&",
            BinOp::BitOr => "|",
            BinOp::BitXor => "^",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
        }
    }

    fn render_unop(&self, op: &UnOp) -> &'static str {
        match op {
            UnOp::Neg => "-",
            UnOp::Not => "!",
            UnOp::BitNot => "!",
        }
    }

    fn render_block_inline(&self, block: &Block) -> String {
        let mut parts = Vec::new();
        for stmt in &block.stmts {
            parts.push(self.render_stmt_inline(stmt));
        }
        if let Some(tail) = &block.expr {
            parts.push(self.render_expr(tail));
        }
        parts.join("; ")
    }

    fn render_stmt_inline(&self, stmt: &Stmt) -> String {
        match stmt {
            Stmt::Let { name, ty, value } => {
                let val = self.render_expr(value);
                let ty_s = self.render_type(ty);
                format!("let {name}: {ty_s} = {val}")
            }
            Stmt::Assign { target, value } => {
                let val = self.render_expr(value);
                format!("{target} = {val}")
            }
            Stmt::Return(e) => {
                let s = self.render_expr(e);
                format!("return {s}")
            }
            Stmt::Assert { cond, message } => {
                let c = self.render_expr(cond);
                if message.is_empty() {
                    format!("assert!({c})")
                } else {
                    format!("assert!({c}, \"{message}\")")
                }
            }
            Stmt::Expr(e) => self.render_expr(e),
            _ => "/* complex stmt */".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Generate Rust source code from a Core IR module.
pub fn generate_rust(module: &Module) -> String {
    RustCodeGen::new().generate(module)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use speclang_ir::capability::CapabilityField;
    use speclang_ir::contract::{Contract, ContractKind, ContractPolicy};
    use speclang_ir::expr::{BinOp, Block, Expr, Literal, MatchArm, Pattern, Stmt};
    use speclang_ir::module::{Function, Module, Param, TypeDef};
    use speclang_ir::types::{Field, PrimitiveType, Region, Type, Variant};
    use speclang_ir::CapabilityType;

    fn make_module(name: &str) -> Module {
        Module::new(vec![name.to_string()])
    }

    #[test]
    fn codegen_empty_module() {
        let m = make_module("empty");
        let code = generate_rust(&m);
        assert!(code.contains("Generated from speclang module"));
        assert!(code.contains("#![allow(dead_code"));
    }

    #[test]
    fn codegen_struct_type() {
        let mut m = make_module("test");
        m.type_defs.push(TypeDef {
            name: "Point".into(),
            ty: Type::Struct(vec![
                Field {
                    name: "x".into(),
                    ty: Type::i32(),
                },
                Field {
                    name: "y".into(),
                    ty: Type::i32(),
                },
            ]),
            annotations: vec![],
        });
        let code = generate_rust(&m);
        assert!(code.contains("pub struct Point {"), "got:\n{code}");
        assert!(code.contains("pub x: i32"), "got:\n{code}");
        assert!(code.contains("pub y: i32"), "got:\n{code}");
    }

    #[test]
    fn codegen_enum_type() {
        let mut m = make_module("test");
        m.type_defs.push(TypeDef {
            name: "Color".into(),
            ty: Type::Enum(vec![
                Variant {
                    name: "Red".into(),
                    fields: vec![],
                },
                Variant {
                    name: "Green".into(),
                    fields: vec![],
                },
                Variant {
                    name: "Blue".into(),
                    fields: vec![],
                },
            ]),
            annotations: vec![],
        });
        let code = generate_rust(&m);
        assert!(code.contains("pub enum Color {"), "got:\n{code}");
        assert!(code.contains("Red,"), "got:\n{code}");
        assert!(code.contains("Green,"), "got:\n{code}");
        assert!(code.contains("Blue,"), "got:\n{code}");
    }

    #[test]
    fn codegen_function_simple() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "add".into(),
            params: vec![
                Param {
                    name: "a".into(),
                    ty: Type::i32(),
                },
                Param {
                    name: "b".into(),
                    ty: Type::i32(),
                },
            ],
            return_type: Type::i32(),
            effects: vec![],
            contracts: vec![],
            body: Block::new(
                vec![],
                Some(Expr::BinOp {
                    op: BinOp::Add,
                    lhs: Box::new(Expr::Var("a".into())),
                    rhs: Box::new(Expr::Var("b".into())),
                }),
            ),
            annotations: vec![],
        });
        let code = generate_rust(&m);
        assert!(code.contains("pub fn add(a: i32, b: i32) -> i32"), "got:\n{code}");
        assert!(code.contains("(a + b)"), "got:\n{code}");
    }

    #[test]
    fn codegen_function_with_contract() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "positive".into(),
            params: vec![Param {
                name: "x".into(),
                ty: Type::i32(),
            }],
            return_type: Type::i32(),
            effects: vec![],
            contracts: vec![Contract {
                kind: ContractKind::Requires,
                predicate: Expr::BinOp {
                    op: BinOp::Gt,
                    lhs: Box::new(Expr::Var("x".into())),
                    rhs: Box::new(Expr::Literal(Literal::Int(0))),
                },
                policy: ContractPolicy::Debug,
                req_tags: vec![],
            }],
            body: Block::new(vec![], Some(Expr::Var("x".into()))),
            annotations: vec![],
        });
        let code = generate_rust(&m);
        assert!(code.contains("debug_assert!((x > 0))"), "got:\n{code}");
    }

    #[test]
    fn codegen_owned_type() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "make_box".into(),
            params: vec![],
            return_type: Type::own(Region::Heap, Type::i32()),
            effects: vec![],
            contracts: vec![],
            body: Block::new(
                vec![],
                Some(Expr::Alloc {
                    region: Box::new(Expr::Literal(Literal::Unit)),
                    ty: Type::i32(),
                    value: Box::new(Expr::Literal(Literal::Int(42))),
                }),
            ),
            annotations: vec![],
        });
        let code = generate_rust(&m);
        assert!(code.contains("-> Box<i32>"), "got:\n{code}");
        assert!(code.contains("Box::new(42)"), "got:\n{code}");
    }

    #[test]
    fn codegen_match_expr() {
        let mut m = make_module("test");
        m.type_defs.push(TypeDef {
            name: "Dir".into(),
            ty: Type::Enum(vec![
                Variant { name: "Up".into(), fields: vec![] },
                Variant { name: "Down".into(), fields: vec![] },
            ]),
            annotations: vec![],
        });
        m.functions.push(Function {
            name: "to_int".into(),
            params: vec![Param { name: "d".into(), ty: Type::named("Dir") }],
            return_type: Type::i32(),
            effects: vec![],
            contracts: vec![],
            body: Block::new(
                vec![],
                Some(Expr::Match {
                    expr: Box::new(Expr::Var("d".into())),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::Variant { ty: vec!["Dir".into()], variant: "Up".into(), fields: vec![] },
                            body: Block::new(vec![], Some(Expr::Literal(Literal::Int(1)))),
                        },
                        MatchArm {
                            pattern: Pattern::Variant { ty: vec!["Dir".into()], variant: "Down".into(), fields: vec![] },
                            body: Block::new(vec![], Some(Expr::Literal(Literal::Int(-1)))),
                        },
                    ],
                }),
            ),
            annotations: vec![],
        });
        let code = generate_rust(&m);
        assert!(code.contains("match d"), "got:\n{code}");
        assert!(code.contains("Dir::Up"), "got:\n{code}");
        assert!(code.contains("Dir::Down"), "got:\n{code}");
    }

    #[test]
    fn codegen_capability_token() {
        let mut m = make_module("test");
        m.cap_defs.push(CapabilityDef {
            name: "Net".into(),
            fields: vec![],
        });
        let code = generate_rust(&m);
        assert!(code.contains("pub struct Net;"), "got:\n{code}");
        assert!(code.contains("Capability token"), "got:\n{code}");
    }

    #[test]
    fn codegen_effects_comment() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "fetch".into(),
            params: vec![],
            return_type: Type::string(),
            effects: vec![CapabilityType { name: "Net".into() }],
            contracts: vec![],
            body: Block::new(
                vec![],
                Some(Expr::Literal(Literal::String("data".into()))),
            ),
            annotations: vec![],
        });
        let code = generate_rust(&m);
        assert!(code.contains("// effects: Net"), "got:\n{code}");
    }

    #[test]
    fn codegen_let_and_return() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "compute".into(),
            params: vec![Param { name: "x".into(), ty: Type::i32() }],
            return_type: Type::i32(),
            effects: vec![],
            contracts: vec![],
            body: Block::new(
                vec![
                    Stmt::Let {
                        name: "y".into(),
                        ty: Type::i32(),
                        value: Expr::BinOp {
                            op: BinOp::Mul,
                            lhs: Box::new(Expr::Var("x".into())),
                            rhs: Box::new(Expr::Literal(Literal::Int(2))),
                        },
                    },
                    Stmt::Return(Expr::Var("y".into())),
                ],
                None,
            ),
            annotations: vec![],
        });
        let code = generate_rust(&m);
        assert!(code.contains("let y: i32 = (x * 2);"), "got:\n{code}");
        assert!(code.contains("return y;"), "got:\n{code}");
    }

    #[test]
    fn codegen_option_result_types() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "maybe".into(),
            params: vec![],
            return_type: Type::result(
                Type::option(Type::i32()),
                Type::string(),
            ),
            effects: vec![],
            contracts: vec![],
            body: Block::new(vec![], Some(Expr::Literal(Literal::Unit))),
            annotations: vec![],
        });
        let code = generate_rust(&m);
        assert!(
            code.contains("-> Result<Option<i32>, String>"),
            "got:\n{code}"
        );
    }

    #[test]
    fn codegen_borrow_and_slice() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "len".into(),
            params: vec![Param {
                name: "data".into(),
                ty: Type::slice(Type::Primitive(PrimitiveType::U8)),
            }],
            return_type: Type::Primitive(PrimitiveType::U64),
            effects: vec![],
            contracts: vec![],
            body: Block::new(vec![], Some(Expr::Literal(Literal::Int(0)))),
            annotations: vec![],
        });
        let code = generate_rust(&m);
        assert!(code.contains("data: &[u8]"), "got:\n{code}");
        assert!(code.contains("-> u64"), "got:\n{code}");
    }
}
