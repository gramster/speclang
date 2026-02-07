//! SPL name resolution and module/import handling.
//!
//! Walks a parsed SPL `Program` and:
//! 1. Builds a symbol table of all declarations.
//! 2. Resolves every `QualifiedName` / `TypeRef` against that table.
//! 3. Reports errors for undefined names, duplicate definitions,
//!    and invalid references (e.g., referencing a gen where a type is expected).

use crate::ast::*;
use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// Symbol kinds — what a name can refer to
// ---------------------------------------------------------------------------

/// The kind of entity a symbol names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Module,
    Type,
    Error,
    Function,
    Capability,
    Law,
    Req,
    Decision,
    Gen,
    Prop,
    Oracle,
}

// ---------------------------------------------------------------------------
// Symbol table
// ---------------------------------------------------------------------------

/// A resolved symbol entry.
#[derive(Debug, Clone)]
pub struct Symbol {
    /// What kind of declaration this is.
    pub kind: SymbolKind,
    /// The simple name of the declaration.
    pub name: String,
    /// Index into the corresponding item list in the program.
    pub item_index: usize,
}

/// Name resolution error.
#[derive(Debug, Clone)]
pub struct ResolveError {
    pub message: String,
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "resolve error: {}", self.message)
    }
}

impl std::error::Error for ResolveError {}

/// Result of name resolution: a resolved program plus the symbol table.
#[derive(Debug)]
pub struct ResolvedProgram<'a> {
    /// Reference to the original AST.
    pub program: &'a Program,
    /// The module name (if declared).
    pub module_name: Option<QualifiedName>,
    /// Symbols defined in this module, keyed by simple name.
    pub symbols: HashMap<String, Symbol>,
    /// Import aliases: alias → qualified name.
    pub imports: HashMap<String, QualifiedName>,
    /// Requirement tags defined by `req` declarations: tag → item index.
    pub req_tags: HashMap<String, usize>,
}

// ---------------------------------------------------------------------------
// Built-in type names (always in scope)
// ---------------------------------------------------------------------------

const BUILTIN_TYPES: &[&str] = &[
    "Int", "Bool", "String", "Bytes", "Unit",
    "U8", "U16", "U32", "U64", "U128",
    "I8", "I16", "I32", "I64", "I128",
    "F32", "F64",
    "Option", "Result", "Set", "List", "Map",
];

fn is_builtin_type(name: &str) -> bool {
    BUILTIN_TYPES.contains(&name)
}

// ---------------------------------------------------------------------------
// Phase 1: Build the symbol table
// ---------------------------------------------------------------------------

/// Resolve names in a parsed SPL program.
///
/// Returns a `ResolvedProgram` on success, or a list of errors.
pub fn resolve(program: &Program) -> Result<ResolvedProgram<'_>, Vec<ResolveError>> {
    let mut resolved = ResolvedProgram {
        program,
        module_name: None,
        symbols: HashMap::new(),
        imports: HashMap::new(),
        req_tags: HashMap::new(),
    };
    let mut errors: Vec<ResolveError> = Vec::new();

    // Phase 1: collect all declarations into the symbol table.
    for (index, item) in program.items.iter().enumerate() {
        match item {
            ModuleItem::Module(m) => {
                if resolved.module_name.is_some() {
                    errors.push(ResolveError {
                        message: format!(
                            "duplicate module declaration '{}'",
                            m.name.join(".")
                        ),
                    });
                } else {
                    resolved.module_name = Some(m.name.clone());
                }
            }
            ModuleItem::Import(i) => {
                let alias = i.alias.clone().unwrap_or_else(|| {
                    i.name.last().cloned().unwrap_or_default()
                });
                if resolved.imports.contains_key(&alias) {
                    errors.push(ResolveError {
                        message: format!("duplicate import alias '{alias}'"),
                    });
                } else {
                    resolved.imports.insert(alias, i.name.clone());
                }
            }
            ModuleItem::Capability(c) => {
                insert_symbol(
                    &mut resolved.symbols,
                    &mut errors,
                    &c.name,
                    SymbolKind::Capability,
                    index,
                );
            }
            ModuleItem::Type(t) => {
                insert_symbol(
                    &mut resolved.symbols,
                    &mut errors,
                    &t.name,
                    SymbolKind::Type,
                    index,
                );
            }
            ModuleItem::Error(e) => {
                insert_symbol(
                    &mut resolved.symbols,
                    &mut errors,
                    &e.name,
                    SymbolKind::Error,
                    index,
                );
            }
            ModuleItem::FnSpec(f) => {
                insert_symbol(
                    &mut resolved.symbols,
                    &mut errors,
                    &f.name,
                    SymbolKind::Function,
                    index,
                );
            }
            ModuleItem::Law(l) => {
                insert_symbol(
                    &mut resolved.symbols,
                    &mut errors,
                    &l.name,
                    SymbolKind::Law,
                    index,
                );
            }
            ModuleItem::Req(r) => {
                if resolved.req_tags.contains_key(&r.tag) {
                    errors.push(ResolveError {
                        message: format!("duplicate requirement tag '{}'", r.tag),
                    });
                } else {
                    resolved.req_tags.insert(r.tag.clone(), index);
                }
            }
            ModuleItem::Decision(d) => {
                insert_symbol(
                    &mut resolved.symbols,
                    &mut errors,
                    &d.name,
                    SymbolKind::Decision,
                    index,
                );
            }
            ModuleItem::Gen(g) => {
                insert_symbol(
                    &mut resolved.symbols,
                    &mut errors,
                    &g.name,
                    SymbolKind::Gen,
                    index,
                );
            }
            ModuleItem::Prop(p) => {
                insert_symbol(
                    &mut resolved.symbols,
                    &mut errors,
                    &p.name,
                    SymbolKind::Prop,
                    index,
                );
            }
            ModuleItem::Oracle(_) => {
                // Oracles reference existing functions, they don't
                // introduce new definitions. Checked in phase 2.
            }
            ModuleItem::Policy(_) => {
                // Policy blocks don't introduce names — nothing to register.
            }
        }
    }

    // Phase 2: resolve all name references.
    resolve_references(&resolved, &mut errors);

    if errors.is_empty() {
        Ok(resolved)
    } else {
        Err(errors)
    }
}

fn insert_symbol(
    symbols: &mut HashMap<String, Symbol>,
    errors: &mut Vec<ResolveError>,
    name: &str,
    kind: SymbolKind,
    item_index: usize,
) {
    if symbols.contains_key(name) {
        errors.push(ResolveError {
            message: format!("duplicate definition '{name}'"),
        });
    } else {
        symbols.insert(
            name.to_string(),
            Symbol {
                kind,
                name: name.to_string(),
                item_index,
            },
        );
    }
}

// ---------------------------------------------------------------------------
// Phase 2: Resolve references
// ---------------------------------------------------------------------------

/// Context for checking references. Holds import aliases + symbol table.
struct ResolveCtx<'a> {
    symbols: &'a HashMap<String, Symbol>,
    imports: &'a HashMap<String, QualifiedName>,
    req_tags: &'a HashMap<String, usize>,
}

impl<'a> ResolveCtx<'a> {
    /// Check that a type reference resolves to a valid type.
    fn check_type_ref(&self, ty: &TypeRef, errors: &mut Vec<ResolveError>) {
        let name = &ty.name;
        if name.len() == 1 {
            let simple = &name[0];
            if !is_builtin_type(simple)
                && !self.symbols.contains_key(simple)
                && !self.imports.contains_key(simple)
            {
                errors.push(ResolveError {
                    message: format!("undefined type '{simple}'"),
                });
            }
        } else if name.len() >= 2 {
            // Qualified: first component must be an import alias or known module.
            let prefix = &name[0];
            if !self.imports.contains_key(prefix)
                && !self.symbols.contains_key(prefix)
            {
                errors.push(ResolveError {
                    message: format!(
                        "undefined module or import '{prefix}' in '{}'",
                        name.join(".")
                    ),
                });
            }
        }
        // Recurse into type arguments.
        for arg in &ty.args {
            self.check_type_ref(arg, errors);
        }
    }

    /// Check that a qualified name resolves (for oracles, raises, etc.).
    fn check_qualified_name(
        &self,
        name: &QualifiedName,
        errors: &mut Vec<ResolveError>,
    ) {
        if name.is_empty() {
            return;
        }
        // For fully qualified names we check the first component
        // against imports or module name.
        if name.len() == 1 {
            let simple = &name[0];
            if !self.symbols.contains_key(simple)
                && !self.imports.contains_key(simple)
            {
                errors.push(ResolveError {
                    message: format!("undefined name '{simple}'"),
                });
            }
        }
        // Multi-part qualified names are assumed to be externally
        // resolved (cross-module references) — we just verify the
        // first component.
        if name.len() >= 2 {
            let prefix = &name[0];
            if !self.imports.contains_key(prefix)
                && !self.symbols.contains_key(prefix)
                // Allow self-module references.
                && !self.is_self_module_prefix(prefix)
            {
                errors.push(ResolveError {
                    message: format!(
                        "undefined module or import '{prefix}' in '{}'",
                        name.join(".")
                    ),
                });
            }
        }
    }

    fn is_self_module_prefix(&self, _prefix: &str) -> bool {
        // If we have a module name, check if prefix matches the first
        // component. In practice oracle names often use the module path.
        // For now we allow anything since cross-module resolution isn't
        // done yet — only intra-module checks.
        // TODO: once multi-file resolution exists, tighten this.
        true
    }

    /// Check that a requirement tag has a corresponding `req` declaration.
    fn check_req_tag(&self, tag: &str, errors: &mut Vec<ResolveError>) {
        if !self.req_tags.contains_key(tag) {
            errors.push(ResolveError {
                message: format!(
                    "requirement tag '{tag}' used but no `req {tag}` declaration found"
                ),
            });
        }
    }

    /// Check all names in a `RefineExpr`.
    fn check_refine_expr(
        &self,
        expr: &RefineExpr,
        locals: &[&str],
        errors: &mut Vec<ResolveError>,
    ) {
        match expr {
            RefineExpr::And(a, b) | RefineExpr::Or(a, b) => {
                self.check_refine_expr(a, locals, errors);
                self.check_refine_expr(b, locals, errors);
            }
            RefineExpr::Not(e) => {
                self.check_refine_expr(e, locals, errors);
            }
            RefineExpr::Compare { lhs, rhs, .. } => {
                self.check_refine_atom(lhs, locals, errors);
                self.check_refine_atom(rhs, locals, errors);
            }
            RefineExpr::Atom(a) => {
                self.check_refine_atom(a, locals, errors);
            }
        }
    }

    /// Check an atom in a refinement expression.
    fn check_refine_atom(
        &self,
        atom: &RefineAtom,
        locals: &[&str],
        errors: &mut Vec<ResolveError>,
    ) {
        match atom {
            RefineAtom::SelfRef | RefineAtom::IntLit(_) | RefineAtom::StringLit(_) => {}
            RefineAtom::Ident(name) => {
                // Must be a local (param, quantifier) or a known symbol.
                if !locals.contains(&name.as_str())
                    && !self.symbols.contains_key(name)
                    && !is_builtin_type(name)
                    // "result" is implicitly bound in ensures blocks.
                    && name != "result"
                {
                    errors.push(ResolveError {
                        message: format!("undefined name '{name}' in refinement expression"),
                    });
                }
            }
            RefineAtom::Call(callee, args) => {
                // The callee might be a local, a module-level function,
                // or an imported/stdlib function. We only flag it if
                // it's clearly not resolvable; cross-module calls are
                // deferred to the type checker.
                // (For now, allow all function calls — they may come
                // from stdlib or imports that we haven't resolved yet.)
                let _ = callee;
                for arg in args {
                    self.check_refine_atom(arg, locals, errors);
                }
            }
        }
    }

    /// Check a `SplExpr` (used in examples).
    fn check_spl_expr(
        &self,
        expr: &SplExpr,
        locals: &[&str],
        errors: &mut Vec<ResolveError>,
    ) {
        match expr {
            SplExpr::IntLit(_) | SplExpr::StringLit(_) => {}
            SplExpr::Ident(name) => {
                if !locals.contains(&name.as_str())
                    && !self.symbols.contains_key(name)
                    && !is_builtin_type(name)
                {
                    errors.push(ResolveError {
                        message: format!(
                            "undefined name '{name}' in expression"
                        ),
                    });
                }
            }
            SplExpr::Call(callee, args) => {
                // Function calls may reference imported/stdlib
                // functions. Defer full resolution to type checker.
                let _ = callee;
                for arg in args {
                    self.check_spl_expr(arg, locals, errors);
                }
            }
            SplExpr::SetLit(elems) => {
                for e in elems {
                    self.check_spl_expr(e, locals, errors);
                }
            }
        }
    }
}

fn resolve_references(
    resolved: &ResolvedProgram<'_>,
    errors: &mut Vec<ResolveError>,
) {
    let ctx = ResolveCtx {
        symbols: &resolved.symbols,
        imports: &resolved.imports,
        req_tags: &resolved.req_tags,
    };

    for item in &resolved.program.items {
        match item {
            ModuleItem::Module(_) | ModuleItem::Import(_) | ModuleItem::Req(_) => {}

            ModuleItem::Capability(c) => {
                for p in &c.params {
                    ctx.check_type_ref(&p.ty, errors);
                }
            }

            ModuleItem::Type(t) => {
                resolve_type_body(&ctx, &t.body, errors);
            }

            ModuleItem::Error(_) => {
                // Error variants have string messages, no type refs.
            }

            ModuleItem::FnSpec(f) => {
                resolve_fn_spec(&ctx, f, errors);
            }

            ModuleItem::Law(l) => {
                ctx.check_refine_expr(&l.expr, &[], errors);
            }

            ModuleItem::Decision(d) => {
                for tag in &d.req_tags {
                    ctx.check_req_tag(tag, errors);
                }
            }

            ModuleItem::Gen(g) => {
                resolve_gen(&ctx, g, errors);
            }

            ModuleItem::Prop(p) => {
                resolve_prop(&ctx, p, errors);
            }

            ModuleItem::Oracle(o) => {
                ctx.check_qualified_name(&o.name, errors);
            }

            ModuleItem::Policy(pol) => {
                for rule in &pol.rules {
                    match rule {
                        PolicyRule::Allow(caps) | PolicyRule::Deny(caps) => {
                            for cap in caps {
                                // Capability names in policy should
                                // reference a declared capability.
                                if !ctx.symbols.contains_key(cap) {
                                    // Allow built-in capability names like
                                    // Net, FileWrite etc.  For now we just
                                    // warn-free — the type checker will
                                    // enforce this.
                                }
                            }
                        }
                        PolicyRule::Deterministic => {}
                    }
                }
            }
        }
    }
}

fn resolve_type_body(
    ctx: &ResolveCtx<'_>,
    body: &TypeBody,
    errors: &mut Vec<ResolveError>,
) {
    match body {
        TypeBody::Alias { ty, refine } => {
            ctx.check_type_ref(ty, errors);
            if let Some(r) = refine {
                ctx.check_refine_expr(r, &["self"], errors);
            }
        }
        TypeBody::Struct { fields, invariant } => {
            let field_names: Vec<&str> =
                fields.iter().map(|f| f.name.as_str()).collect();
            for f in fields {
                ctx.check_type_ref(&f.ty, errors);
            }
            if let Some(invs) = invariant {
                for inv in invs {
                    ctx.check_refine_expr(inv, &field_names, errors);
                }
            }
        }
        TypeBody::Enum { variants } => {
            for v in variants {
                for fty in &v.fields {
                    ctx.check_type_ref(fty, errors);
                }
            }
        }
    }
}

fn resolve_fn_spec(
    ctx: &ResolveCtx<'_>,
    f: &FnSpecDecl,
    errors: &mut Vec<ResolveError>,
) {
    // Build local scope from params.
    let mut locals: Vec<&str> = f.params.iter().map(|p| p.name.as_str()).collect();

    // Check param types.
    for p in &f.params {
        ctx.check_type_ref(&p.ty, errors);
    }

    // Check return type.
    ctx.check_type_ref(&f.return_type, errors);

    // Check all blocks.
    for block in &f.blocks {
        match block {
            FnBlock::Requires { req_tags, conditions } => {
                for tag in req_tags {
                    ctx.check_req_tag(tag, errors);
                }
                for cond in conditions {
                    ctx.check_refine_expr(cond, &locals, errors);
                }
            }
            FnBlock::Ensures { req_tags, conditions } => {
                for tag in req_tags {
                    ctx.check_req_tag(tag, errors);
                }
                // "result" is in scope for ensures.
                locals.push("result");
                for cond in conditions {
                    ctx.check_refine_expr(cond, &locals, errors);
                }
                locals.pop(); // remove "result"
            }
            FnBlock::Effects(effects) => {
                for eff in effects {
                    // Effect names should be declared capabilities.
                    if !ctx.symbols.contains_key(&eff.name) {
                        // Could be a built-in effect, or imported.
                        // Allow for now; type checker enforces.
                    }
                }
            }
            FnBlock::Raises(raises) => {
                for r in raises {
                    ctx.check_qualified_name(&r.error, errors);
                }
            }
            FnBlock::Perf(_) | FnBlock::Notes(_) => {}
            FnBlock::Examples { req_tags, items } => {
                for tag in req_tags {
                    ctx.check_req_tag(tag, errors);
                }
                for ex in items {
                    ctx.check_spl_expr(&ex.lhs, &locals, errors);
                    ctx.check_spl_expr(&ex.rhs, &locals, errors);
                }
            }
        }
    }
}

fn resolve_gen(
    ctx: &ResolveCtx<'_>,
    g: &GenDecl,
    errors: &mut Vec<ResolveError>,
) {
    for field in &g.fields {
        resolve_gen_value(ctx, &field.value, errors);
    }
}

fn resolve_gen_value(
    ctx: &ResolveCtx<'_>,
    value: &GenValue,
    errors: &mut Vec<ResolveError>,
) {
    match value {
        GenValue::Ident(name) => {
            // Should reference a gen or type.
            if !ctx.symbols.contains_key(name)
                && !is_builtin_type(name)
                && !ctx.imports.contains_key(name)
            {
                errors.push(ResolveError {
                    message: format!("undefined name '{name}' in generator"),
                });
            }
        }
        GenValue::List(items) => {
            for item in items {
                resolve_gen_value(ctx, item, errors);
            }
        }
        GenValue::StringLit(_) | GenValue::IntRange(_, _) => {}
    }
}

fn resolve_prop(
    ctx: &ResolveCtx<'_>,
    p: &PropDecl,
    errors: &mut Vec<ResolveError>,
) {
    // Check req tags.
    for tag in &p.req_tags {
        ctx.check_req_tag(tag, errors);
    }

    // Collect quantifier variables as locals.
    let mut locals: Vec<&str> = Vec::new();

    for q in &p.quantifiers {
        ctx.check_type_ref(&q.ty, errors);

        // If a generator is specified, check it exists.
        if let Some(gen_name) = &q.generator {
            match ctx.symbols.get(gen_name) {
                Some(sym) if sym.kind == SymbolKind::Gen => {}
                Some(_) => {
                    errors.push(ResolveError {
                        message: format!(
                            "'{gen_name}' is not a generator (used in `from` clause)"
                        ),
                    });
                }
                None => {
                    errors.push(ResolveError {
                        message: format!(
                            "undefined generator '{gen_name}' in `from` clause"
                        ),
                    });
                }
            }
        }

        locals.push(q.name.as_str());
    }

    // Check the body expression.
    ctx.check_refine_expr(&p.body, &locals, errors);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_program;

    fn resolve_source(src: &str) -> Result<ResolvedProgram<'_>, Vec<ResolveError>> {
        // We need the program to live long enough.
        // Since resolve borrows Program, we must be careful.
        // For tests, we leak to simplify lifetimes.
        let program = Box::leak(Box::new(parse_program(src).unwrap()));
        resolve(program)
    }

    #[test]
    fn resolve_valid_full_example() {
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
  examples [REQ-3] {
    "octave edge": snap_to_scale(12, {1,5,8}) == 1;
    "already in":  snap_to_scale(1,  {1,5,8}) == 1;
  }
};

prop [REQ-2] snap_in_scale:
  forall n: MidiNote from MidiNoteGen
  forall s: Set<MidiNote> from ScaleGen
  set_contains(s, snap_to_scale(n, s));

oracle music.scale.snap_to_scale: reference;

policy {
  deny Net;
  deterministic;
};
"#;
        let resolved = resolve_source(src);
        assert!(resolved.is_ok(), "errors: {:?}", resolved.err());
        let res = resolved.unwrap();
        assert_eq!(res.module_name.as_deref(), Some(&["music", "scale"].map(String::from)[..]));
        assert!(res.symbols.contains_key("MidiNote"));
        assert!(res.symbols.contains_key("snap_to_scale"));
        assert!(res.symbols.contains_key("MidiNoteGen"));
        assert!(res.symbols.contains_key("ScaleGen"));
        assert_eq!(res.req_tags.len(), 3);
    }

    #[test]
    fn resolve_undefined_type() {
        let src = r#"
module test;
type Foo = UnknownType;
"#;
        let result = resolve_source(src);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("undefined type 'UnknownType'")),
            "expected undefined type error, got: {errs:?}"
        );
    }

    #[test]
    fn resolve_duplicate_definition() {
        let src = r#"
module test;
type Foo = Int;
type Foo = Bool;
"#;
        let result = resolve_source(src);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("duplicate definition 'Foo'")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn resolve_duplicate_module() {
        let src = r#"
module a.b;
module c.d;
"#;
        let result = resolve_source(src);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("duplicate module declaration")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn resolve_undefined_req_tag() {
        let src = r#"
module test;
type Foo = Int;
fn bar @id("test.bar") (x: Foo) -> Foo {
  requires [REQ-99] { x == x; }
};
"#;
        let result = resolve_source(src);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("REQ-99")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn resolve_undefined_generator_in_prop() {
        let src = r#"
module test;
type Foo = Int;
fn identity @id("test.identity") (x: Foo) -> Foo {};
prop p1:
  forall x: Foo from NonExistentGen
  identity(x) == x;
"#;
        let result = resolve_source(src);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("undefined generator 'NonExistentGen'")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn resolve_non_gen_in_from_clause() {
        let src = r#"
module test;
type Foo = Int;
type Bar = Int;
fn identity @id("test.identity") (x: Foo) -> Foo {};
prop p1:
  forall x: Foo from Bar
  identity(x) == x;
"#;
        let result = resolve_source(src);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("is not a generator")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn resolve_undefined_fn_in_refine() {
        // Function calls in refinement expressions are deferred to the
        // type checker (they may be from imports/stdlib). But bare ident
        // references that aren't params/known symbols should still error.
        let src = r#"
module test;
type Foo = Int;
fn bar @id("test.bar") (x: Foo) -> Foo {
  requires { unknown_var == x; }
};
"#;
        let result = resolve_source(src);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("undefined name 'unknown_var'")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn resolve_result_in_ensures() {
        // "result" should be valid in an ensures block.
        let src = r#"
module test;
type Foo = Int;
fn bar @id("test.bar") (x: Foo) -> Foo {
  ensures { result == x; }
};
"#;
        let result = resolve_source(src);
        assert!(result.is_ok(), "errors: {:?}", result.err());
    }

    #[test]
    fn resolve_import_alias() {
        let src = r#"
module test;
import std.core as core;
type Foo = core.Option<Int>;
"#;
        let result = resolve_source(src);
        assert!(result.is_ok(), "errors: {:?}", result.err());
        let res = result.unwrap();
        assert!(res.imports.contains_key("core"));
    }

    #[test]
    fn resolve_gen_referencing_another_gen() {
        let src = r#"
module test;
gen Inner { range: 1..10; };
gen Outer { elements: Inner; len: 1..5; };
"#;
        let result = resolve_source(src);
        assert!(result.is_ok(), "errors: {:?}", result.err());
    }

    #[test]
    fn resolve_duplicate_import_alias() {
        let src = r#"
module test;
import a.b as x;
import c.d as x;
"#;
        let result = resolve_source(src);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("duplicate import alias 'x'")),
            "got: {errs:?}"
        );
    }
}
