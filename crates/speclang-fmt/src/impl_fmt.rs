//! IMPL source code formatter.
//!
//! Pretty-prints an IMPL `ImplProgram` AST back to canonical source text.

use speclang_impl::ast::*;

/// Format an IMPL program to canonical source text.
pub fn format_impl(program: &ImplProgram) -> String {
    let mut f = ImplFormatter::new();
    f.format_program(program);
    f.buf
}

struct ImplFormatter {
    buf: String,
    indent: usize,
}

impl ImplFormatter {
    fn new() -> Self {
        ImplFormatter {
            buf: String::new(),
            indent: 0,
        }
    }

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

    fn format_program(&mut self, program: &ImplProgram) {
        let mut prev_is_fn = false;
        for (i, item) in program.items.iter().enumerate() {
            let is_fn = matches!(item, ImplItem::Function(_));
            if i > 0 && (is_fn || prev_is_fn) {
                self.blank();
            }
            self.format_item(item);
            prev_is_fn = is_fn;
        }
    }

    fn format_item(&mut self, item: &ImplItem) {
        match item {
            ImplItem::Module(m) => self.line(&format!("module {};", m.name.join("."))),
            ImplItem::Import(i) => {
                if let Some(alias) = &i.alias {
                    self.line(&format!("import {} as {};", i.name.join("."), alias));
                } else {
                    self.line(&format!("import {};", i.name.join(".")));
                }
            }
            ImplItem::Function(f) => self.format_function(f),
        }
    }

    fn format_function(&mut self, f: &ImplFunction) {
        let params: Vec<String> = f
            .params
            .iter()
            .map(|p| {
                if p.is_cap {
                    format!("{}: cap {}", p.name, format_type_ref(&p.ty))
                } else {
                    format!("{}: {}", p.name, format_type_ref(&p.ty))
                }
            })
            .collect();
        let ret = format_type_ref(&f.return_type);

        // Multi-line params if many.
        if params.len() <= 3 && params.iter().map(|p| p.len()).sum::<usize>() < 60 {
            self.line(&format!(
                "impl fn \"{}\" {}({}) -> {} {{",
                f.stable_id,
                f.name,
                params.join(", "),
                ret,
            ));
        } else {
            self.line(&format!("impl fn \"{}\" {}(", f.stable_id, f.name));
            self.push();
            for (i, p) in params.iter().enumerate() {
                let comma = if i + 1 < params.len() { "," } else { "," };
                self.line(&format!("{}{}", p, comma));
            }
            self.pop();
            self.line(&format!(") -> {} {{", ret));
        }

        self.push();
        self.format_block_body(&f.body);
        self.pop();
        self.line("}");
    }

    fn format_block_body(&mut self, block: &ImplBlock) {
        for stmt in &block.stmts {
            self.format_stmt(stmt);
        }
        if let Some(tail) = &block.expr {
            let s = format_expr(tail);
            self.line(&s);
        }
    }

    fn format_stmt(&mut self, stmt: &ImplStmt) {
        match stmt {
            ImplStmt::Let { name, ty, value } => {
                let val = format_expr(value);
                if let Some(t) = ty {
                    self.line(&format!("let {}: {} = {};", name, format_type_ref(t), val));
                } else {
                    self.line(&format!("let {} = {};", name, val));
                }
            }
            ImplStmt::LetMut { name, ty, value } => {
                let val = format_expr(value);
                if let Some(t) = ty {
                    self.line(&format!(
                        "let mut {}: {} = {};",
                        name,
                        format_type_ref(t),
                        val
                    ));
                } else {
                    self.line(&format!("let mut {} = {};", name, val));
                }
            }
            ImplStmt::Assign { target, value } => {
                let val = format_expr(value);
                self.line(&format!("{} = {};", target, val));
            }
            ImplStmt::If {
                cond,
                then_block,
                else_block,
            } => {
                let c = format_expr(cond);
                self.line(&format!("if {} {{", c));
                self.push();
                self.format_block_body(then_block);
                self.pop();
                if let Some(eb) = else_block {
                    self.line("} else {");
                    self.push();
                    self.format_block_body(eb);
                    self.pop();
                }
                self.line("}");
            }
            ImplStmt::Match { expr, arms } => {
                let e = format_expr(expr);
                self.line(&format!("match {} {{", e));
                self.push();
                for arm in arms {
                    self.format_match_arm(arm);
                }
                self.pop();
                self.line("}");
            }
            ImplStmt::Return(expr) => {
                if let Some(e) = expr {
                    self.line(&format!("return {};", format_expr(e)));
                } else {
                    self.line("return;");
                }
            }
            ImplStmt::Assert { cond, message } => {
                let c = format_expr(cond);
                if let Some(msg) = message {
                    self.line(&format!("assert({}, \"{}\");", c, msg));
                } else {
                    self.line(&format!("assert({});", c));
                }
            }
            ImplStmt::While { cond, body } => {
                let c = format_expr(cond);
                self.line(&format!("while {} {{", c));
                self.push();
                self.format_block_body(body);
                self.pop();
                self.line("}");
            }
            ImplStmt::Loop(body) => {
                self.line("loop {");
                self.push();
                self.format_block_body(body);
                self.pop();
                self.line("}");
            }
            ImplStmt::Break => self.line("break;"),
            ImplStmt::Continue => self.line("continue;"),
            ImplStmt::Expr(e) => {
                let s = format_expr(e);
                self.line(&format!("{};", s));
            }
        }
    }

    fn format_match_arm(&mut self, arm: &ImplMatchArm) {
        let pat = format_pattern(&arm.pattern);
        self.line(&format!("{} => {{", pat));
        self.push();
        self.format_block_body(&arm.body);
        self.pop();
        self.line("}");
    }
}

// ---------------------------------------------------------------------------
// Expression formatting (returns inline string)
// ---------------------------------------------------------------------------

fn format_expr(expr: &ImplExpr) -> String {
    match expr {
        ImplExpr::Literal(lit) => format_literal(lit),
        ImplExpr::Var(name) => name.clone(),
        ImplExpr::BinOp { op, lhs, rhs } => {
            let l = format_expr(lhs);
            let r = format_expr(rhs);
            let op_s = format_binop(*op);
            format!("({} {} {})", l, op_s, r)
        }
        ImplExpr::UnOp { op, operand } => {
            let o = format_expr(operand);
            let op_s = format_unop(*op);
            format!("({}{})", op_s, o)
        }
        ImplExpr::Call { func, args } => {
            let f = func.join(".");
            let arg_strs: Vec<String> = args.iter().map(|a| format_expr(a)).collect();
            format!("{}({})", f, arg_strs.join(", "))
        }
        ImplExpr::StructLit { ty, fields } => {
            let name = ty.join(".");
            let field_strs: Vec<String> = fields
                .iter()
                .map(|(n, v)| format!("{}: {}", n, format_expr(v)))
                .collect();
            format!("{} {{ {} }}", name, field_strs.join(", "))
        }
        ImplExpr::FieldGet { expr, field } => {
            let e = format_expr(expr);
            format!("{}.{}", e, field)
        }
        ImplExpr::EnumLit { ty, variant, args } => {
            let name = ty.join(".");
            if args.is_empty() {
                format!("{}.{}", name, variant)
            } else {
                let arg_strs: Vec<String> = args.iter().map(|a| format_expr(a)).collect();
                format!("{}.{}({})", name, variant, arg_strs.join(", "))
            }
        }
        ImplExpr::TupleLit(elems) => {
            let parts: Vec<String> = elems.iter().map(|e| format_expr(e)).collect();
            format!("({})", parts.join(", "))
        }
        ImplExpr::If {
            cond,
            then_block,
            else_block,
        } => {
            let c = format_expr(cond);
            let then_s = format_block_inline(then_block);
            if let Some(eb) = else_block {
                let else_s = format_block_inline(eb);
                format!("if {} {{ {} }} else {{ {} }}", c, then_s, else_s)
            } else {
                format!("if {} {{ {} }}", c, then_s)
            }
        }
        ImplExpr::Match { expr, arms } => {
            let e = format_expr(expr);
            let arm_strs: Vec<String> = arms
                .iter()
                .map(|arm| {
                    let pat = format_pattern(&arm.pattern);
                    let body = format_block_inline(&arm.body);
                    format!("{} => {{ {} }}", pat, body)
                })
                .collect();
            format!("match {} {{ {} }}", e, arm_strs.join(", "))
        }
        ImplExpr::Block(block) => {
            let body = format_block_inline(block);
            format!("{{ {} }}", body)
        }
        ImplExpr::Alloc { region, value } => {
            let r = format_expr(region);
            let v = format_expr(value);
            format!("alloc({}, {})", r, v)
        }
        ImplExpr::Borrow(inner) => {
            let e = format_expr(inner);
            format!("borrow({})", e)
        }
        ImplExpr::BorrowMut(inner) => {
            let e = format_expr(inner);
            format!("borrow_mut({})", e)
        }
        ImplExpr::Convert { expr, target } => {
            let e = format_expr(expr);
            let t = format_type_ref(target);
            format!("{} as {}", e, t)
        }
        ImplExpr::Loop(body) => {
            let b = format_block_inline(body);
            format!("loop {{ {} }}", b)
        }
        ImplExpr::While { cond, body } => {
            let c = format_expr(cond);
            let b = format_block_inline(body);
            format!("while {} {{ {} }}", c, b)
        }
        ImplExpr::Break => "break".to_string(),
        ImplExpr::Continue => "continue".to_string(),
        ImplExpr::Return(expr) => {
            if let Some(e) = expr {
                format!("return {}", format_expr(e))
            } else {
                "return".to_string()
            }
        }
    }
}

fn format_block_inline(block: &ImplBlock) -> String {
    let mut parts = Vec::new();
    for stmt in &block.stmts {
        parts.push(format_stmt_inline(stmt));
    }
    if let Some(tail) = &block.expr {
        parts.push(format_expr(tail));
    }
    parts.join("; ")
}

fn format_stmt_inline(stmt: &ImplStmt) -> String {
    match stmt {
        ImplStmt::Let { name, ty, value } => {
            let val = format_expr(value);
            if let Some(t) = ty {
                format!("let {}: {} = {}", name, format_type_ref(t), val)
            } else {
                format!("let {} = {}", name, val)
            }
        }
        ImplStmt::LetMut { name, ty, value } => {
            let val = format_expr(value);
            if let Some(t) = ty {
                format!("let mut {}: {} = {}", name, format_type_ref(t), val)
            } else {
                format!("let mut {} = {}", name, val)
            }
        }
        ImplStmt::Assign { target, value } => {
            format!("{} = {}", target, format_expr(value))
        }
        ImplStmt::Return(expr) => {
            if let Some(e) = expr {
                format!("return {}", format_expr(e))
            } else {
                "return".to_string()
            }
        }
        ImplStmt::Break => "break".to_string(),
        ImplStmt::Continue => "continue".to_string(),
        ImplStmt::Expr(e) => format_expr(e),
        _ => "/* complex stmt */".to_string(),
    }
}

fn format_literal(lit: &ImplLiteral) -> String {
    match lit {
        ImplLiteral::Bool(b) => b.to_string(),
        ImplLiteral::Int(n) => n.to_string(),
        ImplLiteral::Float(f) => format!("{}", f),
        ImplLiteral::String(s) => format!("\"{}\"", s),
        ImplLiteral::Unit => "()".to_string(),
    }
}

fn format_binop(op: ImplBinOp) -> &'static str {
    match op {
        ImplBinOp::Add => "+",
        ImplBinOp::Sub => "-",
        ImplBinOp::Mul => "*",
        ImplBinOp::Div => "/",
        ImplBinOp::Mod => "%",
        ImplBinOp::BitAnd => "&",
        ImplBinOp::BitOr => "|",
        ImplBinOp::BitXor => "^",
        ImplBinOp::Shl => "<<",
        ImplBinOp::Shr => ">>",
        ImplBinOp::Eq => "==",
        ImplBinOp::Ne => "!=",
        ImplBinOp::Lt => "<",
        ImplBinOp::Le => "<=",
        ImplBinOp::Gt => ">",
        ImplBinOp::Ge => ">=",
        ImplBinOp::And => "&&",
        ImplBinOp::Or => "||",
    }
}

fn format_unop(op: ImplUnOp) -> &'static str {
    match op {
        ImplUnOp::Neg => "-",
        ImplUnOp::Not => "!",
        ImplUnOp::BitNot => "~",
    }
}

fn format_pattern(pat: &ImplPattern) -> String {
    match pat {
        ImplPattern::Wildcard => "_".to_string(),
        ImplPattern::Bind(name) => name.clone(),
        ImplPattern::Literal(lit) => format_literal(lit),
        ImplPattern::Variant {
            ty,
            variant,
            fields,
        } => {
            let name = ty.join(".");
            if fields.is_empty() {
                format!("{}.{}", name, variant)
            } else {
                let pats: Vec<String> = fields.iter().map(|f| format_pattern(f)).collect();
                format!("{}.{}({})", name, variant, pats.join(", "))
            }
        }
        ImplPattern::Tuple(pats) => {
            let parts: Vec<String> = pats.iter().map(|p| format_pattern(p)).collect();
            format!("({})", parts.join(", "))
        }
        ImplPattern::Struct { ty, fields } => {
            let name = ty.join(".");
            let parts: Vec<String> = fields
                .iter()
                .map(|(n, p)| {
                    let ps = format_pattern(p);
                    if ps == *n {
                        n.clone()
                    } else {
                        format!("{}: {}", n, ps)
                    }
                })
                .collect();
            format!("{} {{ {} }}", name, parts.join(", "))
        }
    }
}

fn format_type_ref(ty: &ImplTypeRef) -> String {
    match ty {
        ImplTypeRef::Named(name) => name.clone(),
        ImplTypeRef::Qualified(qname) => qname.join("."),
        ImplTypeRef::Own { region, inner } => {
            format!("own[{}, {}]", region, format_type_ref(inner))
        }
        ImplTypeRef::Ref(inner) => format!("ref[{}]", format_type_ref(inner)),
        ImplTypeRef::MutRef(inner) => format!("mutref[{}]", format_type_ref(inner)),
        ImplTypeRef::Slice(inner) => format!("slice[{}]", format_type_ref(inner)),
        ImplTypeRef::MutSlice(inner) => format!("mutslice[{}]", format_type_ref(inner)),
        ImplTypeRef::Tuple(elems) => {
            let parts: Vec<String> = elems.iter().map(|e| format_type_ref(e)).collect();
            format!("({})", parts.join(", "))
        }
        ImplTypeRef::Generic { name, args } => {
            let n = name.join(".");
            let arg_strs: Vec<String> = args.iter().map(|a| format_type_ref(a)).collect();
            format!("{}[{}]", n, arg_strs.join(", "))
        }
        ImplTypeRef::Option(inner) => format!("Option[{}]", format_type_ref(inner)),
        ImplTypeRef::Result { ok, err } => {
            format!("Result[{}, {}]", format_type_ref(ok), format_type_ref(err))
        }
        ImplTypeRef::Capability(name) => name.clone(),
        ImplTypeRef::Region => "region".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use speclang_impl::parser::parse_impl;

    /// Parse IMPL source, format it, and return the formatted text.
    fn roundtrip(src: &str) -> String {
        let program = parse_impl(src).expect("parse failed");
        format_impl(&program)
    }

    #[test]
    fn format_module_and_import() {
        let src = "module test;\nimport std.core;\n";
        let out = roundtrip(src);
        assert!(out.contains("module test;"), "got:\n{out}");
        assert!(out.contains("import std.core;"), "got:\n{out}");
    }

    #[test]
    fn format_simple_fn() {
        let src = r#"
impl fn "math.add.v1" add(a: I32, b: I32) -> I32 {
    (a + b)
}
"#;
        let out = roundtrip(src);
        assert!(
            out.contains("impl fn \"math.add.v1\" add("),
            "got:\n{out}"
        );
        assert!(out.contains("(a + b)"), "got:\n{out}");
    }

    #[test]
    fn format_let_and_return() {
        let src = r#"
impl fn "test.compute.v1" compute(x: I32) -> I32 {
    let y: I32 = (x * 2);
    return y;
}
"#;
        let out = roundtrip(src);
        assert!(out.contains("let y: I32 = (x * 2);"), "got:\n{out}");
        assert!(out.contains("return y;"), "got:\n{out}");
    }

    #[test]
    fn format_if_else() {
        let src = r#"
impl fn "test.max.v1" max(a: I32, b: I32) -> I32 {
    if (a > b) {
        a
    } else {
        b
    }
}
"#;
        let out = roundtrip(src);
        assert!(out.contains("if (a > b) {"), "got:\n{out}");
        assert!(out.contains("} else {"), "got:\n{out}");
    }

    #[test]
    fn format_match() {
        let src = r#"
impl fn "test.dir.v1" to_int(d: Dir) -> I32 {
    match d {
        Dir.Up => {
            1
        }
        Dir.Down => {
            -1
        }
    }
}
"#;
        let out = roundtrip(src);
        assert!(out.contains("match d {"), "got:\n{out}");
        assert!(out.contains("Dir.Up => {"), "got:\n{out}");
    }

    #[test]
    fn format_while_loop() {
        let src = r#"
impl fn "test.count.v1" count(n: I32) -> I32 {
    let mut i: I32 = 0;
    while (i < n) {
        i = (i + 1);
    }
    i
}
"#;
        let out = roundtrip(src);
        assert!(out.contains("let mut i: I32 = 0;"), "got:\n{out}");
        assert!(out.contains("while (i < n) {"), "got:\n{out}");
    }

    #[test]
    fn format_cap_param() {
        let src = r#"
impl fn "net.fetch.v1" fetch(url: String, _net: cap Net) -> String {
    url
}
"#;
        let out = roundtrip(src);
        assert!(out.contains("_net: cap Net"), "got:\n{out}");
    }

    #[test]
    fn format_ownership_types() {
        let src = r#"
impl fn "test.box.v1" make(arena: region) -> own[arena, I32] {
    alloc(arena, 42)
}
"#;
        let out = roundtrip(src);
        assert!(out.contains("arena: region"), "got:\n{out}");
        assert!(out.contains("own[arena, I32]"), "got:\n{out}");
        assert!(out.contains("alloc(arena, 42)"), "got:\n{out}");
    }

    #[test]
    fn format_assert_stmt() {
        let src = r#"
impl fn "test.check.v1" check(x: I32) -> I32 {
    assert((x > 0), "must be positive");
    x
}
"#;
        let out = roundtrip(src);
        assert!(
            out.contains("assert((x > 0), \"must be positive\");"),
            "got:\n{out}"
        );
    }
}
