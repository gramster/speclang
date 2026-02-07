//! SPL type checker and semantic analysis.
//!
//! Operates on a `ResolvedProgram` (names already resolved) and checks:
//! - Type references are well-formed (correct arity, no cycles).
//! - Refinement expressions are well-typed (comparisons are on compatible types).
//! - Function specs are consistent (params, return types, examples type-check).
//! - Generator fields have valid types/ranges.
//! - Properties reference valid generators and produce boolean expressions.
//!
//! This is a *specification-level* type checker.  It does not generate code;
//! it just validates that the SPL program is semantically well-formed before
//! lowering to Core IR.

use crate::ast::*;
use crate::resolve::{ResolvedProgram, SymbolKind};
use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// Type-check errors
// ---------------------------------------------------------------------------

/// A type-checking error.
#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "type error: {}", self.message)
    }
}

impl std::error::Error for TypeError {}

// ---------------------------------------------------------------------------
// Inferred types (simplified for the spec layer)
// ---------------------------------------------------------------------------

/// A simplified type representation for SPL type checking.
///
/// SPL operates on a high-level type algebra.  The exact machine
/// representations live in Core IR; here we just check consistency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplType {
    /// A primitive type (Int, Bool, String, etc.).
    Primitive(String),
    /// A named user-defined type.
    Named(String),
    /// A generic instantiation, e.g. Set<MidiNote>.
    Generic(String, Vec<SplType>),
    /// The nullable wrapper (?).
    Nullable(Box<SplType>),
    /// A type that couldn't be resolved — already reported.
    Error,
}

impl fmt::Display for SplType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SplType::Primitive(name) | SplType::Named(name) => write!(f, "{name}"),
            SplType::Generic(name, args) => {
                write!(f, "{name}<")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{arg}")?;
                }
                write!(f, ">")
            }
            SplType::Nullable(inner) => write!(f, "{inner}?"),
            SplType::Error => write!(f, "<error>"),
        }
    }
}

// ---------------------------------------------------------------------------
// Built-in type knowledge
// ---------------------------------------------------------------------------

/// Returns true if the name is a known primitive type.
fn is_primitive(name: &str) -> bool {
    matches!(
        name,
        "Int" | "Bool" | "String" | "Bytes" | "Unit"
            | "U8" | "U16" | "U32" | "U64" | "U128"
            | "I8" | "I16" | "I32" | "I64" | "I128"
            | "F32" | "F64"
    )
}

/// Returns the expected arity for built-in generic types.
fn builtin_generic_arity(name: &str) -> Option<usize> {
    match name {
        "Option" => Some(1),
        "Result" => Some(2),
        "Set" | "List" => Some(1),
        "Map" => Some(2),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Type-checker context
// ---------------------------------------------------------------------------

struct TypeChecker<'a> {
    resolved: &'a ResolvedProgram<'a>,
    errors: Vec<TypeError>,
    /// Caches the SplType for each user-defined type by name.
    type_cache: HashMap<String, SplType>,
}

impl<'a> TypeChecker<'a> {
    fn new(resolved: &'a ResolvedProgram<'a>) -> Self {
        TypeChecker {
            resolved,
            errors: Vec::new(),
            type_cache: HashMap::new(),
        }
    }

    fn err(&mut self, msg: impl Into<String>) {
        self.errors.push(TypeError {
            message: msg.into(),
        });
    }

    // -----------------------------------------------------------------------
    // TypeRef → SplType lowering
    // -----------------------------------------------------------------------

    /// Lower a `TypeRef` to an `SplType`, recording errors for bad references.
    fn lower_type_ref(&mut self, tr: &TypeRef) -> SplType {
        let name = tr.name.join(".");
        let base = if is_primitive(&name) {
            if !tr.args.is_empty() {
                self.err(format!("primitive type '{name}' does not take type arguments"));
            }
            SplType::Primitive(name)
        } else if let Some(expected_arity) = builtin_generic_arity(&name) {
            // Built-in generic type.
            if tr.args.len() != expected_arity {
                self.err(format!(
                    "'{name}' expects {expected_arity} type argument(s), got {}",
                    tr.args.len()
                ));
            }
            let args: Vec<SplType> = tr.args.iter().map(|a| self.lower_type_ref(a)).collect();
            SplType::Generic(name, args)
        } else if self.resolved.symbols.contains_key(&name) {
            if !tr.args.is_empty() {
                // User-defined types don't support generics yet.
                self.err(format!(
                    "type '{name}' does not take type arguments (user-defined generics not yet supported)"
                ));
            }
            SplType::Named(name)
        } else if self.resolved.imports.contains_key(&tr.name[0]) {
            // Qualified import reference — assume valid at this level.
            if tr.args.is_empty() {
                SplType::Named(name)
            } else {
                let args: Vec<SplType> = tr.args.iter().map(|a| self.lower_type_ref(a)).collect();
                SplType::Generic(name, args)
            }
        } else {
            // Unknown — already caught by resolver, just propagate error.
            SplType::Error
        };

        if tr.nullable {
            SplType::Nullable(Box::new(base))
        } else {
            base
        }
    }

    // -----------------------------------------------------------------------
    // Top-level check
    // -----------------------------------------------------------------------

    fn check_program(&mut self) {
        for item in &self.resolved.program.items {
            match item {
                ModuleItem::Module(_) | ModuleItem::Import(_) | ModuleItem::Req(_) => {}

                ModuleItem::Capability(c) => self.check_capability(c),
                ModuleItem::Type(t) => self.check_type_decl(t),
                ModuleItem::Error(e) => self.check_error_decl(e),
                ModuleItem::FnSpec(f) => self.check_fn_spec(f),
                ModuleItem::Law(l) => self.check_law(l),
                ModuleItem::Decision(_) => {} // just strings
                ModuleItem::Gen(g) => self.check_gen(g),
                ModuleItem::Prop(p) => self.check_prop(p),
                ModuleItem::Oracle(o) => self.check_oracle(o),
                ModuleItem::Policy(_) => {} // checked by resolver
            }
        }
    }

    // -----------------------------------------------------------------------
    // Individual checks
    // -----------------------------------------------------------------------

    fn check_capability(&mut self, c: &CapabilityDecl) {
        for p in &c.params {
            self.lower_type_ref(&p.ty);
        }
    }

    fn check_type_decl(&mut self, t: &TypeDecl) {
        match &t.body {
            TypeBody::Alias { ty, refine } => {
                let spl_ty = self.lower_type_ref(ty);
                self.type_cache.insert(t.name.clone(), spl_ty);

                if let Some(r) = refine {
                    self.check_refine_expr_is_bool(r, &t.name);
                }
            }
            TypeBody::Struct { fields, invariant } => {
                for f in fields {
                    self.lower_type_ref(&f.ty);
                }
                self.type_cache
                    .insert(t.name.clone(), SplType::Named(t.name.clone()));

                if let Some(invs) = invariant {
                    for inv in invs {
                        self.check_refine_expr_is_bool(inv, &t.name);
                    }
                }
            }
            TypeBody::Enum { variants } => {
                let mut seen = HashMap::new();
                for v in variants {
                    if let Some(_prev) = seen.insert(&v.name, ()) {
                        self.err(format!(
                            "duplicate variant '{}' in enum '{}'",
                            v.name, t.name
                        ));
                    }
                    for fty in &v.fields {
                        self.lower_type_ref(fty);
                    }
                }
                self.type_cache
                    .insert(t.name.clone(), SplType::Named(t.name.clone()));
            }
        }
    }

    fn check_error_decl(&mut self, e: &ErrorDecl) {
        let mut seen = HashMap::new();
        for v in &e.variants {
            if let Some(_prev) = seen.insert(&v.name, ()) {
                self.err(format!(
                    "duplicate error variant '{}' in error '{}'",
                    v.name, e.name
                ));
            }
        }
    }

    fn check_fn_spec(&mut self, f: &FnSpecDecl) {
        // Check params.
        let mut param_names: HashMap<&str, ()> = HashMap::new();
        for p in &f.params {
            if param_names.insert(&p.name, ()).is_some() {
                self.err(format!(
                    "duplicate parameter '{}' in function '{}'",
                    p.name, f.name
                ));
            }
            self.lower_type_ref(&p.ty);
        }

        // Check return type.
        let ret_ty = self.lower_type_ref(&f.return_type);

        // Check blocks.
        for block in &f.blocks {
            match block {
                FnBlock::Requires { conditions, .. } => {
                    for cond in conditions {
                        self.check_refine_expr_is_bool(cond, &f.name);
                    }
                }
                FnBlock::Ensures { conditions, .. } => {
                    for cond in conditions {
                        self.check_refine_expr_is_bool(cond, &f.name);
                    }
                }
                FnBlock::Effects(_) | FnBlock::Raises(_) | FnBlock::Perf(_) | FnBlock::Notes(_) => {}
                FnBlock::Examples { items, .. } => {
                    for ex in items {
                        // Each example is `expr == expr`.  The LHS should
                        // produce the return type; we do a basic check.
                        self.check_example_expr(&ex.lhs, &f.name, &ret_ty);
                        self.check_example_expr(&ex.rhs, &f.name, &ret_ty);
                    }
                }
            }
        }
    }

    fn check_law(&mut self, l: &LawDecl) {
        self.check_refine_expr_is_bool(&l.expr, &l.name);
    }

    fn check_gen(&mut self, g: &GenDecl) {
        for field in &g.fields {
            self.check_gen_field(field, &g.name);
        }
    }

    fn check_gen_field(&mut self, field: &GenField, gen_name: &str) {
        match &field.value {
            GenValue::IntRange(lo, hi) => {
                if lo > hi {
                    self.err(format!(
                        "invalid range {lo}..{hi} in generator '{gen_name}': low > high"
                    ));
                }
            }
            GenValue::Ident(name) => {
                // Should reference a gen or type — already checked by resolver.
                // But we can verify it's a Gen if used as `elements`.
                if field.key == "elements" {
                    if let Some(sym) = self.resolved.symbols.get(name) {
                        if sym.kind != SymbolKind::Gen {
                            self.err(format!(
                                "'{name}' used as 'elements' in generator '{gen_name}' but is not a generator"
                            ));
                        }
                    }
                }
            }
            GenValue::List(items) => {
                for item in items {
                    // Recurse for nested values.
                    let sub_field = GenField {
                        key: field.key.clone(),
                        value: item.clone(),
                    };
                    self.check_gen_field(&sub_field, gen_name);
                }
            }
            GenValue::StringLit(_) => {}
        }
    }

    fn check_prop(&mut self, p: &PropDecl) {
        // Check quantifier types.
        for q in &p.quantifiers {
            self.lower_type_ref(&q.ty);
        }

        // The body must be a boolean expression.
        self.check_refine_expr_is_bool(&p.body, &p.name);
    }

    fn check_oracle(&mut self, o: &OracleDecl) {
        // The oracle must reference a declared function.
        if let Some(fn_name) = o.name.last() {
            if let Some(sym) = self.resolved.symbols.get(fn_name) {
                if sym.kind != SymbolKind::Function {
                    self.err(format!(
                        "oracle '{}' references '{}' which is not a function",
                        o.name.join("."),
                        fn_name
                    ));
                }
            }
            // If not found, it might be cross-module — skip for now.
        }
    }

    // -----------------------------------------------------------------------
    // Expression checks
    // -----------------------------------------------------------------------

    /// Check that a refinement expression is well-formed (all sub-exprs valid).
    /// We don't fully type-infer here; we verify structural validity.
    fn check_refine_expr_is_bool(&mut self, _expr: &RefineExpr, _context: &str) {
        // Structural validation is mostly done by the parser and resolver.
        // Detailed type inference for refinement expressions would require
        // an SMT-style solver or at minimum a full expression type system.
        // For now, we accept any well-formed RefineExpr as "bool".
        //
        // TODO: Add deeper type inference for refinement expressions
        // once the type system supports it.
    }

    /// Basic check for example expressions.
    fn check_example_expr(&mut self, _expr: &SplExpr, _fn_name: &str, _expected_ty: &SplType) {
        // Example expressions have already been name-resolved.
        // Full type checking of example expressions would require
        // function signature lookup and argument type matching.
        // Deferred to a later phase.
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Type-check a resolved SPL program.
///
/// Returns `Ok(())` if the program is well-formed, or a list of type errors.
pub fn typecheck(resolved: &ResolvedProgram<'_>) -> Result<(), Vec<TypeError>> {
    let mut checker = TypeChecker::new(resolved);
    checker.check_program();

    if checker.errors.is_empty() {
        Ok(())
    } else {
        Err(checker.errors)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_program;
    use crate::resolve::resolve;

    fn check_source(src: &str) -> Result<(), Vec<TypeError>> {
        let program = Box::leak(Box::new(parse_program(src).unwrap()));
        let resolved = resolve(program).expect("resolve should succeed");
        let resolved = Box::leak(Box::new(resolved));
        typecheck(resolved)
    }

    fn check_source_fails(src: &str) -> Vec<TypeError> {
        let program = Box::leak(Box::new(parse_program(src).unwrap()));
        let resolved = resolve(program).expect("resolve should succeed");
        let resolved = Box::leak(Box::new(resolved));
        typecheck(resolved).expect_err("expected type errors")
    }

    #[test]
    fn typecheck_valid_full_example() {
        let src = r#"
module music.scale;

req REQ-1: "Notes in range";
req REQ-2: "Snap result in scale";
req REQ-3: "Tie-break rule";

type MidiNote = Int refine (1 <= self and self <= 12);

gen MidiNoteGen { range: 1..12; };
gen ScaleGen { elements: MidiNoteGen; len: 1..12; };

decision [REQ-3] tie_break:
  when: "equal distance";
  choose: "smaller note";

fn snap_to_scale @id("music.snap.v1") @compat(stable_semantics)
  (note: MidiNote, scale: Set<MidiNote>) -> MidiNote
{
  requires [REQ-2] { scale_is_nonempty(scale); }
  ensures  [REQ-2] { set_contains(scale, result); }
  examples [REQ-3] {
    "edge": snap_to_scale(12, {1,5,8}) == 1;
    "in":   snap_to_scale(1,  {1,5,8}) == 1;
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
        assert!(check_source(src).is_ok());
    }

    #[test]
    fn typecheck_primitive_with_type_args() {
        let src = r#"
module test;
type Bad = Int<Bool>;
"#;
        let errs = check_source_fails(src);
        assert!(
            errs.iter().any(|e| e.message.contains("does not take type arguments")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn typecheck_wrong_generic_arity() {
        let src = r#"
module test;
type Bad = Set<Int, Bool>;
"#;
        let errs = check_source_fails(src);
        assert!(
            errs.iter().any(|e| e.message.contains("expects 1 type argument")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn typecheck_duplicate_enum_variant() {
        let src = r#"
module test;
type Color enum {
  Red;
  Blue;
  Red;
};
"#;
        let errs = check_source_fails(src);
        assert!(
            errs.iter().any(|e| e.message.contains("duplicate variant 'Red'")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn typecheck_duplicate_error_variant() {
        let src = r#"
module test;
error ParseError {
  BadInput: "bad input";
  BadInput: "dupe";
};
"#;
        let errs = check_source_fails(src);
        assert!(
            errs.iter().any(|e| e.message.contains("duplicate error variant 'BadInput'")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn typecheck_duplicate_param() {
        let src = r#"
module test;
type Foo = Int;
fn bar @id("test.bar") (x: Foo, x: Foo) -> Foo {};
"#;
        let errs = check_source_fails(src);
        assert!(
            errs.iter().any(|e| e.message.contains("duplicate parameter 'x'")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn typecheck_invalid_gen_range() {
        let src = r#"
module test;
gen Bad { range: 10..1; };
"#;
        let errs = check_source_fails(src);
        assert!(
            errs.iter().any(|e| e.message.contains("low > high")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn typecheck_oracle_non_function() {
        let src = r#"
module test;
type Foo = Int;
oracle Foo: reference;
"#;
        let errs = check_source_fails(src);
        assert!(
            errs.iter().any(|e| e.message.contains("not a function")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn typecheck_nullable_type() {
        let src = r#"
module test;
type MaybeInt = Int?;
"#;
        assert!(check_source(src).is_ok());
    }

    #[test]
    fn typecheck_nested_generic() {
        let src = r#"
module test;
type Nested = Option<Set<Int>>;
"#;
        assert!(check_source(src).is_ok());
    }

    #[test]
    fn typecheck_gen_elements_non_gen() {
        let src = r#"
module test;
type Foo = Int;
gen Bad { elements: Foo; };
"#;
        let errs = check_source_fails(src);
        assert!(
            errs.iter().any(|e| e.message.contains("not a generator")),
            "got: {errs:?}"
        );
    }
}
