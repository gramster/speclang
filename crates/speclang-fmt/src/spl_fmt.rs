//! SPL source code formatter.
//!
//! Pretty-prints an SPL `Program` AST back to canonical source text.

use speclang_spl::ast::*;

/// Format an SPL program to canonical source text.
pub fn format_spl(program: &Program) -> String {
    let mut f = SplFormatter::new();
    f.format_program(program);
    f.buf
}

struct SplFormatter {
    buf: String,
    indent: usize,
}

impl SplFormatter {
    fn new() -> Self {
        SplFormatter {
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

    fn format_program(&mut self, program: &Program) {
        let mut prev_kind = ItemKind::None;
        for item in &program.items {
            let kind = item_kind(item);
            // Blank line between different kinds of items, or between functions.
            if prev_kind != ItemKind::None
                && (kind != prev_kind || kind == ItemKind::Function)
            {
                self.blank();
            }
            self.format_item(item);
            prev_kind = kind;
        }
    }

    fn format_item(&mut self, item: &ModuleItem) {
        match item {
            ModuleItem::Module(m) => self.format_module(m),
            ModuleItem::Import(i) => self.format_import(i),
            ModuleItem::Capability(c) => self.format_capability(c),
            ModuleItem::Type(t) => self.format_type_decl(t),
            ModuleItem::Error(e) => self.format_error(e),
            ModuleItem::FnSpec(f) => self.format_fn_spec(f),
            ModuleItem::Law(l) => self.format_law(l),
            ModuleItem::Req(r) => self.format_req(r),
            ModuleItem::Decision(d) => self.format_decision(d),
            ModuleItem::Gen(g) => self.format_gen(g),
            ModuleItem::Prop(p) => self.format_prop(p),
            ModuleItem::Oracle(o) => self.format_oracle(o),
            ModuleItem::Policy(po) => self.format_policy(po),
        }
    }

    fn format_module(&mut self, m: &ModuleDecl) {
        self.line(&format!("module {};", m.name.join(".")));
    }

    fn format_import(&mut self, i: &ImportDecl) {
        if let Some(alias) = &i.alias {
            self.line(&format!("import {} as {};", i.name.join("."), alias));
        } else {
            self.line(&format!("import {};", i.name.join(".")));
        }
    }

    fn format_capability(&mut self, c: &CapabilityDecl) {
        let params: Vec<String> = c
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, format_type_ref(&p.ty)))
            .collect();
        self.line(&format!("capability {}({});", c.name, params.join(", ")));
    }

    fn format_type_decl(&mut self, t: &TypeDecl) {
        match &t.body {
            TypeBody::Alias { ty, refine } => {
                let ty_s = format_type_ref(ty);
                if let Some(r) = refine {
                    self.line(&format!(
                        "type {} = {} where {};",
                        t.name,
                        ty_s,
                        format_refine(r)
                    ));
                } else {
                    self.line(&format!("type {} = {};", t.name, ty_s));
                }
            }
            TypeBody::Struct { fields, invariant } => {
                self.line(&format!("type {} struct {{", t.name));
                self.push();
                for f in fields {
                    self.line(&format!("{}: {};", f.name, format_type_ref(&f.ty)));
                }
                if let Some(invs) = invariant {
                    self.line("invariant {");
                    self.push();
                    for inv in invs {
                        self.line(&format!("{};", format_refine(inv)));
                    }
                    self.pop();
                    self.line("}");
                }
                self.pop();
                self.line("};");
            }
            TypeBody::Enum { variants } => {
                self.line(&format!("type {} enum {{", t.name));
                self.push();
                for v in variants {
                    if v.fields.is_empty() {
                        self.line(&format!("{};", v.name));
                    } else {
                        let fields: Vec<String> =
                            v.fields.iter().map(|f| format_type_ref(f)).collect();
                        self.line(&format!("{}({});", v.name, fields.join(", ")));
                    }
                }
                self.pop();
                self.line("};");
            }
        }
    }

    fn format_error(&mut self, e: &ErrorDecl) {
        self.line(&format!("error {} {{", e.name));
        self.push();
        for v in &e.variants {
            self.line(&format!("{}: \"{}\";", v.name, v.message));
        }
        self.pop();
        self.line("};");
    }

    fn format_fn_spec(&mut self, f: &FnSpecDecl) {
        let params: Vec<String> = f
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, format_type_ref(&p.ty)))
            .collect();
        let ret = format_type_ref(&f.return_type);

        let mut header = format!("fn {} @id(\"{}\")", f.name, f.stable_id);
        if let Some(compat) = &f.compat {
            let c = match compat {
                CompatKind::StableCall => "stable-call",
                CompatKind::StableSemantics => "stable-semantics",
                CompatKind::Unstable => "unstable",
            };
            header.push_str(&format!(" @compat(\"{}\")", c));
        }
        header.push_str(&format!("({}) -> {} {{", params.join(", "), ret));
        self.line(&header);
        self.push();

        for block in &f.blocks {
            self.format_fn_block(block);
        }

        self.pop();
        self.line("};");
    }

    fn format_fn_block(&mut self, block: &FnBlock) {
        match block {
            FnBlock::Requires { req_tags, conditions } => {
                let tags = format_req_tags(req_tags);
                self.line(&format!("requires{} {{", tags));
                self.push();
                for c in conditions {
                    self.line(&format!("{};", format_refine(c)));
                }
                self.pop();
                self.line("}");
            }
            FnBlock::Ensures { req_tags, conditions } => {
                let tags = format_req_tags(req_tags);
                self.line(&format!("ensures{} {{", tags));
                self.push();
                for c in conditions {
                    self.line(&format!("{};", format_refine(c)));
                }
                self.pop();
                self.line("}");
            }
            FnBlock::Effects(effects) => {
                self.line("effects {");
                self.push();
                let eff_strs: Vec<String> = effects
                    .iter()
                    .map(|e| {
                        if e.args.is_empty() {
                            e.name.clone()
                        } else {
                            format!("{}({})", e.name, e.args.join(", "))
                        }
                    })
                    .collect();
                self.line(&format!("{}", eff_strs.join(", ")));
                self.pop();
                self.line("}");
            }
            FnBlock::Raises(raises) => {
                self.line("raises {");
                self.push();
                for r in raises {
                    let name = r.error.join(".");
                    if let Some(desc) = &r.description {
                        self.line(&format!("{}: \"{}\",", name, desc));
                    } else {
                        self.line(&format!("{},", name));
                    }
                }
                self.pop();
                self.line("}");
            }
            FnBlock::Perf(items) => {
                self.line("perf {");
                self.push();
                for p in items {
                    self.line(&format!("{}: {},", p.key, p.value));
                }
                self.pop();
                self.line("}");
            }
            FnBlock::Examples { req_tags, items } => {
                let tags = format_req_tags(req_tags);
                self.line(&format!("examples{} {{", tags));
                self.push();
                for ex in items {
                    self.line(&format!(
                        "\"{}\": {} == {};",
                        ex.label,
                        format_spl_expr(&ex.lhs),
                        format_spl_expr(&ex.rhs)
                    ));
                }
                self.pop();
                self.line("}");
            }
            FnBlock::Notes(notes) => {
                self.line("notes {");
                self.push();
                for n in notes {
                    self.line(&format!("\"{}\";", n));
                }
                self.pop();
                self.line("}");
            }
        }
    }

    fn format_law(&mut self, l: &LawDecl) {
        self.line(&format!("law {}: {};", l.name, format_refine(&l.expr)));
    }

    fn format_req(&mut self, r: &ReqDecl) {
        self.line(&format!("req {}: \"{}\";", r.tag, r.description));
    }

    fn format_decision(&mut self, d: &DecisionDecl) {
        let tags = format_req_tags(&d.req_tags);
        self.line(&format!("decision{} {} {{", tags, d.name));
        self.push();
        self.line(&format!("when: \"{}\";", d.when));
        self.line(&format!("choose: \"{}\";", d.choose));
        self.pop();
        self.line("}");
    }

    fn format_gen(&mut self, g: &GenDecl) {
        self.line(&format!("gen {} {{", g.name));
        self.push();
        for f in &g.fields {
            self.line(&format!("{}: {};", f.key, format_gen_value(&f.value)));
        }
        self.pop();
        self.line("};");
    }

    fn format_prop(&mut self, p: &PropDecl) {
        let tags = format_req_tags(&p.req_tags);
        let quants: Vec<String> = p
            .quantifiers
            .iter()
            .map(|q| {
                let ty = format_type_ref(&q.ty);
                if let Some(generator) = &q.generator {
                    format!("forall {}: {} from {}", q.name, ty, generator)
                } else {
                    format!("forall {}: {}", q.name, ty)
                }
            })
            .collect();
        self.line(&format!(
            "prop{} {}: {} {};",
            tags,
            p.name,
            quants.join(", "),
            format_refine(&p.body)
        ));
    }

    fn format_oracle(&mut self, o: &OracleDecl) {
        let kind = match o.kind {
            OracleKind::Reference => "reference",
            OracleKind::Optimized => "optimized",
        };
        self.line(&format!("oracle {}: {};", o.name.join("."), kind));
    }

    fn format_policy(&mut self, po: &PolicyDecl) {
        self.line("policy {");
        self.push();
        for rule in &po.rules {
            match rule {
                PolicyRule::Allow(caps) => {
                    self.line(&format!("allow {};", caps.join(", ")));
                }
                PolicyRule::Deny(caps) => {
                    self.line(&format!("deny {};", caps.join(", ")));
                }
                PolicyRule::Deterministic => {
                    self.line("deterministic;");
                }
            }
        }
        self.pop();
        self.line("}");
    }
}

// ---------------------------------------------------------------------------
// Helper formatting functions
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Clone, Copy)]
enum ItemKind {
    None,
    Module,
    Import,
    Capability,
    Type,
    Error,
    Function,
    Law,
    Req,
    Decision,
    Gen,
    Prop,
    Oracle,
    Policy,
}

fn item_kind(item: &ModuleItem) -> ItemKind {
    match item {
        ModuleItem::Module(_) => ItemKind::Module,
        ModuleItem::Import(_) => ItemKind::Import,
        ModuleItem::Capability(_) => ItemKind::Capability,
        ModuleItem::Type(_) => ItemKind::Type,
        ModuleItem::Error(_) => ItemKind::Error,
        ModuleItem::FnSpec(_) => ItemKind::Function,
        ModuleItem::Law(_) => ItemKind::Law,
        ModuleItem::Req(_) => ItemKind::Req,
        ModuleItem::Decision(_) => ItemKind::Decision,
        ModuleItem::Gen(_) => ItemKind::Gen,
        ModuleItem::Prop(_) => ItemKind::Prop,
        ModuleItem::Oracle(_) => ItemKind::Oracle,
        ModuleItem::Policy(_) => ItemKind::Policy,
    }
}

fn format_req_tags(tags: &[String]) -> String {
    if tags.is_empty() {
        String::new()
    } else {
        format!(" [{}]", tags.join(", "))
    }
}

fn format_type_ref(ty: &TypeRef) -> String {
    let base = ty.name.join(".");
    let s = if ty.args.is_empty() {
        base
    } else {
        let args: Vec<String> = ty.args.iter().map(|a| format_type_ref(a)).collect();
        format!("{}<{}>", base, args.join(", "))
    };
    if ty.nullable {
        format!("{}?", s)
    } else {
        s
    }
}

fn format_refine(expr: &RefineExpr) -> String {
    match expr {
        RefineExpr::And(lhs, rhs) => {
            format!("{} && {}", format_refine(lhs), format_refine(rhs))
        }
        RefineExpr::Or(lhs, rhs) => {
            format!("{} || {}", format_refine(lhs), format_refine(rhs))
        }
        RefineExpr::Not(inner) => {
            format!("!{}", format_refine(inner))
        }
        RefineExpr::Compare { lhs, op, rhs } => {
            let op_s = match op {
                CompareOp::Eq => "==",
                CompareOp::Ne => "!=",
                CompareOp::Lt => "<",
                CompareOp::Le => "<=",
                CompareOp::Gt => ">",
                CompareOp::Ge => ">=",
            };
            format!("{} {} {}", format_atom(lhs), op_s, format_atom(rhs))
        }
        RefineExpr::Atom(a) => format_atom(a),
    }
}

fn format_atom(atom: &RefineAtom) -> String {
    match atom {
        RefineAtom::SelfRef => "self".to_string(),
        RefineAtom::Ident(s) => s.clone(),
        RefineAtom::IntLit(n) => n.to_string(),
        RefineAtom::StringLit(s) => format!("\"{}\"", s),
        RefineAtom::Call(name, args) => {
            let arg_strs: Vec<String> = args.iter().map(|a| format_atom(a)).collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
    }
}

fn format_spl_expr(expr: &SplExpr) -> String {
    match expr {
        SplExpr::IntLit(n) => n.to_string(),
        SplExpr::StringLit(s) => format!("\"{}\"", s),
        SplExpr::Ident(s) => s.clone(),
        SplExpr::Call(name, args) => {
            let arg_strs: Vec<String> = args.iter().map(|a| format_spl_expr(a)).collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
        SplExpr::SetLit(elems) => {
            let elem_strs: Vec<String> = elems.iter().map(|e| format_spl_expr(e)).collect();
            format!("{{{}}}", elem_strs.join(", "))
        }
    }
}

fn format_gen_value(v: &GenValue) -> String {
    match v {
        GenValue::StringLit(s) => format!("\"{}\"", s),
        GenValue::IntRange(lo, hi) => format!("{}..{}", lo, hi),
        GenValue::Ident(s) => s.clone(),
        GenValue::List(items) => {
            let parts: Vec<String> = items.iter().map(|i| format_gen_value(i)).collect();
            format!("[{}]", parts.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use speclang_spl::parser::parse_program;

    /// Parse SPL source, format it, and return the formatted text.
    fn roundtrip(src: &str) -> String {
        let program = parse_program(src).expect("parse failed");
        format_spl(&program)
    }

    #[test]
    fn format_module_and_imports() {
        let src = "module music.scale;\nimport std.core;\nimport std.math as m;\n";
        let out = roundtrip(src);
        assert!(out.contains("module music.scale;"), "got:\n{out}");
        assert!(out.contains("import std.core;"), "got:\n{out}");
        assert!(out.contains("import std.math as m;"), "got:\n{out}");
    }

    #[test]
    fn format_type_alias() {
        let src = "module test;\ntype MidiNote = Int;\n";
        let out = roundtrip(src);
        assert!(out.contains("type MidiNote = Int;"), "got:\n{out}");
    }

    #[test]
    fn format_type_struct() {
        let src = "module test;\ntype Point struct { x: Int; y: Int; };\n";
        let out = roundtrip(src);
        assert!(out.contains("type Point struct {"), "got:\n{out}");
        assert!(out.contains("x: Int;"), "got:\n{out}");
        assert!(out.contains("y: Int;"), "got:\n{out}");
    }

    #[test]
    fn format_type_enum() {
        let src = "module test;\ntype Color enum { Red; Green; Blue; };\n";
        let out = roundtrip(src);
        assert!(out.contains("type Color enum {"), "got:\n{out}");
        assert!(out.contains("Red;"), "got:\n{out}");
    }

    #[test]
    fn format_capability() {
        let src = "module test;\ncapability Net();\n";
        let out = roundtrip(src);
        assert!(out.contains("capability Net();"), "got:\n{out}");
    }

    #[test]
    fn format_fn_spec() {
        let src = r#"module test;
fn add @id("math.add.v1") (a: Int, b: Int) -> Int {
    ensures {
        is_sum(result, a, b);
    }
};
"#;
        let out = roundtrip(src);
        assert!(
            out.contains("fn add @id(\"math.add.v1\")"),
            "got:\n{out}"
        );
        assert!(out.contains("ensures {"), "got:\n{out}");
        assert!(out.contains("is_sum(result, a, b);"), "got:\n{out}");
    }

    #[test]
    fn format_req() {
        let src = "module test;\nreq REQ-001: \"Data must be validated\";\n";
        let out = roundtrip(src);
        assert!(
            out.contains("req REQ-001: \"Data must be validated\";"),
            "got:\n{out}"
        );
    }

    #[test]
    fn format_gen() {
        let src = "module test;\ngen NoteGen { range: 0..127; };\n";
        let out = roundtrip(src);
        assert!(out.contains("gen NoteGen {"), "got:\n{out}");
        assert!(out.contains("range: 0..127;"), "got:\n{out}");
    }

    #[test]
    fn format_error_decl() {
        let src = "module test;\nerror ParseErr { BadToken: \"unexpected token\"; };\n";
        let out = roundtrip(src);
        assert!(out.contains("error ParseErr {"), "got:\n{out}");
        assert!(out.contains("BadToken:"), "got:\n{out}");
    }

    #[test]
    fn format_preserves_blank_lines_between_sections() {
        let src = "module test;\nimport std.core;\ntype X = Int;\n";
        let out = roundtrip(src);
        // Should have blank line between import and type sections
        assert!(out.contains("import std.core;\n\ntype X"), "got:\n{out}");
    }
}
