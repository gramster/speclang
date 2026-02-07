//! Core IR type checking pass.
//!
//! Validates a Core IR `Module` for well-formedness:
//! - All named type references resolve within the module.
//! - No duplicate type or function definitions.
//! - Function parameter and return types are well-formed.
//! - Expressions are structurally valid.
//! - Contract predicates reference valid types.

use speclang_ir::capability::CapabilityDef;
use speclang_ir::contract::Contract;
use speclang_ir::expr::{Block, Expr, Stmt};
use speclang_ir::module::{Function, Module, TypeDef};
use speclang_ir::types::{QName, Type};
use std::collections::{HashMap, HashSet};
use std::fmt;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// A type-checking error in Core IR.
#[derive(Debug, Clone)]
pub struct VerifyError {
    pub message: String,
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "verify error: {}", self.message)
    }
}

impl std::error::Error for VerifyError {}

// ---------------------------------------------------------------------------
// Verifier
// ---------------------------------------------------------------------------

struct Verifier<'a> {
    module: &'a Module,
    /// Known type names (from TypeDef declarations).
    type_names: HashSet<String>,
    /// Known function names.
    fn_names: HashSet<String>,
    /// Known capability names.
    cap_names: HashSet<String>,
    errors: Vec<VerifyError>,
}

impl<'a> Verifier<'a> {
    fn new(module: &'a Module) -> Self {
        Verifier {
            module,
            type_names: HashSet::new(),
            fn_names: HashSet::new(),
            cap_names: HashSet::new(),
            errors: Vec::new(),
        }
    }

    fn err(&mut self, msg: impl Into<String>) {
        self.errors.push(VerifyError {
            message: msg.into(),
        });
    }

    // -----------------------------------------------------------------------
    // Phase 1: collect names, check for duplicates
    // -----------------------------------------------------------------------

    fn collect_names(&mut self) {
        for td in &self.module.type_defs {
            if !self.type_names.insert(td.name.clone()) {
                self.err(format!("duplicate type definition '{}'", td.name));
            }
        }
        for f in &self.module.functions {
            if !self.fn_names.insert(f.name.clone()) {
                self.err(format!("duplicate function definition '{}'", f.name));
            }
        }
        for e in &self.module.externs {
            if !self.fn_names.insert(e.name.clone()) {
                self.err(format!(
                    "duplicate function/extern definition '{}'",
                    e.name
                ));
            }
        }
        for c in &self.module.cap_defs {
            if !self.cap_names.insert(c.name.clone()) {
                self.err(format!(
                    "duplicate capability definition '{}'",
                    c.name
                ));
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 2: check all types, functions, capabilities
    // -----------------------------------------------------------------------

    fn check_module(&mut self) {
        self.collect_names();

        for td in &self.module.type_defs {
            self.check_type_def(td);
        }

        for cd in &self.module.cap_defs {
            self.check_capability_def(cd);
        }

        for f in &self.module.functions {
            self.check_function(f);
        }

        for e in &self.module.externs {
            self.check_extern_function(e);
        }
    }

    fn check_type_def(&mut self, td: &TypeDef) {
        self.check_type(&td.ty, &td.name);
    }

    fn check_capability_def(&mut self, cd: &CapabilityDef) {
        for field in &cd.fields {
            self.check_type(&field.ty, &cd.name);
        }
    }

    fn check_function(&mut self, f: &Function) {
        // Check parameter types.
        let mut param_names: HashMap<&str, ()> = HashMap::new();
        for p in &f.params {
            if param_names.insert(&p.name, ()).is_some() {
                self.err(format!(
                    "duplicate parameter '{}' in function '{}'",
                    p.name, f.name
                ));
            }
            self.check_type(&p.ty, &f.name);
        }

        // Check return type.
        self.check_type(&f.return_type, &f.name);

        // Check effect references.
        for eff in &f.effects {
            if !self.cap_names.contains(&eff.name) {
                self.err(format!(
                    "function '{}' references undefined capability '{}'",
                    f.name, eff.name
                ));
            }
        }

        // Check contracts.
        for contract in &f.contracts {
            self.check_contract(contract, &f.name);
        }

        // Check body.
        let scope: HashMap<String, ()> = f
            .params
            .iter()
            .map(|p| (p.name.clone(), ()))
            .collect();
        self.check_block(&f.body, &scope, &f.name);
    }

    fn check_extern_function(&mut self, e: &speclang_ir::module::ExternFunction) {
        for p in &e.params {
            self.check_type(&p.ty, &e.name);
        }
        self.check_type(&e.return_type, &e.name);
        for eff in &e.effects {
            if !self.cap_names.contains(&eff.name) {
                self.err(format!(
                    "extern function '{}' references undefined capability '{}'",
                    e.name, eff.name
                ));
            }
        }
    }

    fn check_contract(&mut self, contract: &Contract, fn_name: &str) {
        // Contract predicates reference the function params and possibly "result".
        // We just check well-formedness of the expression here.
        self.check_expr(&contract.predicate, &HashMap::new(), fn_name);
    }

    // -----------------------------------------------------------------------
    // Type well-formedness
    // -----------------------------------------------------------------------

    fn check_type(&mut self, ty: &Type, context: &str) {
        match ty {
            Type::Primitive(_) | Type::Region => {}

            Type::Struct(fields) => {
                let mut seen = HashSet::new();
                for f in fields {
                    if !seen.insert(&f.name) {
                        self.err(format!(
                            "duplicate field '{}' in struct type (context: {context})",
                            f.name
                        ));
                    }
                    self.check_type(&f.ty, context);
                }
            }

            Type::Enum(variants) => {
                let mut seen = HashSet::new();
                for v in variants {
                    if !seen.insert(&v.name) {
                        self.err(format!(
                            "duplicate variant '{}' in enum type (context: {context})",
                            v.name
                        ));
                    }
                    for fty in &v.fields {
                        self.check_type(fty, context);
                    }
                }
            }

            Type::Tuple(elements) => {
                for e in elements {
                    self.check_type(e, context);
                }
            }

            Type::Own { inner, .. } => self.check_type(inner, context),
            Type::Ref(inner) | Type::MutRef(inner) => self.check_type(inner, context),
            Type::Slice(inner) | Type::MutSlice(inner) => self.check_type(inner, context),
            Type::Option(inner) => self.check_type(inner, context),
            Type::Result { ok, err } => {
                self.check_type(ok, context);
                self.check_type(err, context);
            }

            Type::Named(qname) => {
                self.check_named_type(qname, context);
            }

            Type::Generic { name, args } => {
                self.check_named_type(name, context);
                for arg in args {
                    self.check_type(arg, context);
                }
            }

            Type::Capability(name) => {
                if !self.cap_names.contains(name) {
                    self.err(format!(
                        "undefined capability type '{name}' (context: {context})"
                    ));
                }
            }
        }
    }

    fn check_named_type(&mut self, qname: &QName, context: &str) {
        if qname.is_empty() {
            self.err(format!("empty type name (context: {context})"));
            return;
        }
        // For single-component names, check against known type defs.
        if qname.len() == 1 {
            let name = &qname[0];
            if !self.type_names.contains(name) {
                // May be a built-in or cross-module reference.
                // Built-in types are represented as Primitive, so
                // Named("Int") would be an error.
                self.err(format!(
                    "undefined type '{name}' (context: {context})"
                ));
            }
        }
        // Multi-segment names are cross-module references;
        // skip for now until we have multi-module support.
    }

    // -----------------------------------------------------------------------
    // Expression checking (structural validity)
    // -----------------------------------------------------------------------

    fn check_expr(
        &mut self,
        expr: &Expr,
        scope: &HashMap<String, ()>,
        context: &str,
    ) {
        match expr {
            Expr::Literal(_) => {}

            Expr::Var(name) => {
                if !scope.contains_key(name) {
                    // Might be a module-level name or param — relaxed check.
                }
            }

            Expr::BinOp { lhs, rhs, .. } => {
                self.check_expr(lhs, scope, context);
                self.check_expr(rhs, scope, context);
            }

            Expr::UnOp { operand, .. } => {
                self.check_expr(operand, scope, context);
            }

            Expr::Call { func, args } => {
                // Check the function exists.
                if func.len() == 1 && !self.fn_names.contains(&func[0]) {
                    self.err(format!(
                        "call to undefined function '{}' (context: {context})",
                        func[0]
                    ));
                }
                for arg in args {
                    self.check_expr(arg, scope, context);
                }
            }

            Expr::StructLit { ty, fields } => {
                self.check_named_type(ty, context);
                for (_, val) in fields {
                    self.check_expr(val, scope, context);
                }
            }

            Expr::FieldGet { expr, .. } => {
                self.check_expr(expr, scope, context);
            }

            Expr::EnumLit { ty, fields, .. } => {
                self.check_named_type(ty, context);
                for f in fields {
                    self.check_expr(f, scope, context);
                }
            }

            Expr::TupleLit(elements) => {
                for e in elements {
                    self.check_expr(e, scope, context);
                }
            }

            Expr::If {
                cond,
                then_block,
                else_block,
            } => {
                self.check_expr(cond, scope, context);
                self.check_block(then_block, scope, context);
                self.check_block(else_block, scope, context);
            }

            Expr::Match { expr, arms } => {
                self.check_expr(expr, scope, context);
                for arm in arms {
                    self.check_block(&arm.body, scope, context);
                }
            }

            Expr::Block(block) => {
                self.check_block(block, scope, context);
            }

            Expr::Alloc { region, value, .. } => {
                self.check_expr(region, scope, context);
                self.check_expr(value, scope, context);
            }

            Expr::Borrow(inner) | Expr::BorrowMut(inner) => {
                self.check_expr(inner, scope, context);
            }

            Expr::Convert { expr, target } => {
                self.check_expr(expr, scope, context);
                self.check_type(target, context);
            }
        }
    }

    fn check_block(
        &mut self,
        block: &Block,
        scope: &HashMap<String, ()>,
        context: &str,
    ) {
        let mut local_scope = scope.clone();

        for stmt in &block.stmts {
            match stmt {
                Stmt::Let { name, ty, value } => {
                    self.check_type(ty, context);
                    self.check_expr(value, &local_scope, context);
                    local_scope.insert(name.clone(), ());
                }
                Stmt::Assign { value, .. } => {
                    self.check_expr(value, &local_scope, context);
                }
                Stmt::If {
                    cond,
                    then_block,
                    else_block,
                } => {
                    self.check_expr(cond, &local_scope, context);
                    self.check_block(then_block, &local_scope, context);
                    self.check_block(else_block, &local_scope, context);
                }
                Stmt::Match { expr, arms } => {
                    self.check_expr(expr, &local_scope, context);
                    for arm in arms {
                        self.check_block(&arm.body, &local_scope, context);
                    }
                }
                Stmt::Return(e) => {
                    self.check_expr(e, &local_scope, context);
                }
                Stmt::Assert { cond, .. } => {
                    self.check_expr(cond, &local_scope, context);
                }
                Stmt::Expr(e) => {
                    self.check_expr(e, &local_scope, context);
                }
            }
        }

        if let Some(tail) = &block.expr {
            self.check_expr(tail, &local_scope, context);
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Type-check a Core IR module.
///
/// Returns `Ok(())` if the module is well-formed, or a list of errors.
pub fn verify_module(module: &Module) -> Result<(), Vec<VerifyError>> {
    let mut verifier = Verifier::new(module);
    verifier.check_module();

    if verifier.errors.is_empty() {
        Ok(())
    } else {
        Err(verifier.errors)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use speclang_ir::capability::CapabilityDef;
    use speclang_ir::expr::{Block, Expr, Stmt};
    use speclang_ir::module::{Function, Module, Param, TypeDef};
    use speclang_ir::types::{Field, PrimitiveType, Type, Variant};

    fn make_module(name: &str) -> Module {
        Module::new(vec![name.to_string()])
    }

    #[test]
    fn verify_empty_module() {
        let m = make_module("test");
        assert!(verify_module(&m).is_ok());
    }

    #[test]
    fn verify_valid_type_def() {
        let mut m = make_module("test");
        m.type_defs.push(TypeDef {
            name: "MidiNote".into(),
            ty: Type::Primitive(PrimitiveType::Int),
            annotations: vec![],
        });
        assert!(verify_module(&m).is_ok());
    }

    #[test]
    fn verify_duplicate_type_def() {
        let mut m = make_module("test");
        m.type_defs.push(TypeDef {
            name: "Foo".into(),
            ty: Type::int(),
            annotations: vec![],
        });
        m.type_defs.push(TypeDef {
            name: "Foo".into(),
            ty: Type::bool(),
            annotations: vec![],
        });
        let errs = verify_module(&m).unwrap_err();
        assert!(errs.iter().any(|e| e.message.contains("duplicate type")));
    }

    #[test]
    fn verify_duplicate_function() {
        let mut m = make_module("test");
        let f = Function {
            name: "foo".into(),
            params: vec![],
            return_type: Type::unit(),
            effects: vec![],
            contracts: vec![],
            body: Block::empty(),
            annotations: vec![],
        };
        m.functions.push(f.clone());
        m.functions.push(f);
        let errs = verify_module(&m).unwrap_err();
        assert!(errs.iter().any(|e| e.message.contains("duplicate function")));
    }

    #[test]
    fn verify_function_with_valid_named_type() {
        let mut m = make_module("test");
        m.type_defs.push(TypeDef {
            name: "MidiNote".into(),
            ty: Type::int(),
            annotations: vec![],
        });
        m.functions.push(Function {
            name: "identity".into(),
            params: vec![Param {
                name: "x".into(),
                ty: Type::named("MidiNote"),
            }],
            return_type: Type::named("MidiNote"),
            effects: vec![],
            contracts: vec![],
            body: Block::new(
                vec![],
                Some(Expr::Var("x".into())),
            ),
            annotations: vec![],
        });
        assert!(verify_module(&m).is_ok());
    }

    #[test]
    fn verify_function_with_undefined_type() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "bad".into(),
            params: vec![Param {
                name: "x".into(),
                ty: Type::named("NoSuchType"),
            }],
            return_type: Type::unit(),
            effects: vec![],
            contracts: vec![],
            body: Block::empty(),
            annotations: vec![],
        });
        let errs = verify_module(&m).unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("undefined type 'NoSuchType'")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn verify_function_with_undefined_capability() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "bad".into(),
            params: vec![],
            return_type: Type::unit(),
            effects: vec![speclang_ir::capability::CapabilityType {
                name: "Net".into(),
            }],
            contracts: vec![],
            body: Block::empty(),
            annotations: vec![],
        });
        let errs = verify_module(&m).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.message.contains("undefined capability 'Net'")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn verify_duplicate_param() {
        let mut m = make_module("test");
        m.type_defs.push(TypeDef {
            name: "Foo".into(),
            ty: Type::int(),
            annotations: vec![],
        });
        m.functions.push(Function {
            name: "bad".into(),
            params: vec![
                Param {
                    name: "x".into(),
                    ty: Type::named("Foo"),
                },
                Param {
                    name: "x".into(),
                    ty: Type::named("Foo"),
                },
            ],
            return_type: Type::unit(),
            effects: vec![],
            contracts: vec![],
            body: Block::empty(),
            annotations: vec![],
        });
        let errs = verify_module(&m).unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("duplicate parameter 'x'")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn verify_struct_duplicate_field() {
        let mut m = make_module("test");
        m.type_defs.push(TypeDef {
            name: "Bad".into(),
            ty: Type::Struct(vec![
                Field {
                    name: "x".into(),
                    ty: Type::int(),
                },
                Field {
                    name: "x".into(),
                    ty: Type::bool(),
                },
            ]),
            annotations: vec![],
        });
        let errs = verify_module(&m).unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("duplicate field 'x'")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn verify_enum_duplicate_variant() {
        let mut m = make_module("test");
        m.type_defs.push(TypeDef {
            name: "Bad".into(),
            ty: Type::Enum(vec![
                Variant {
                    name: "A".into(),
                    fields: vec![],
                },
                Variant {
                    name: "A".into(),
                    fields: vec![],
                },
            ]),
            annotations: vec![],
        });
        let errs = verify_module(&m).unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("duplicate variant 'A'")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn verify_call_undefined_function() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "caller".into(),
            params: vec![],
            return_type: Type::unit(),
            effects: vec![],
            contracts: vec![],
            body: Block::new(
                vec![Stmt::Expr(Expr::Call {
                    func: vec!["nonexistent".into()],
                    args: vec![],
                })],
                None,
            ),
            annotations: vec![],
        });
        let errs = verify_module(&m).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.message.contains("undefined function 'nonexistent'")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn verify_valid_capability() {
        let mut m = make_module("test");
        m.cap_defs.push(CapabilityDef {
            name: "Net".into(),
            fields: vec![],
        });
        m.functions.push(Function {
            name: "do_net".into(),
            params: vec![],
            return_type: Type::unit(),
            effects: vec![speclang_ir::capability::CapabilityType {
                name: "Net".into(),
            }],
            contracts: vec![],
            body: Block::empty(),
            annotations: vec![],
        });
        assert!(verify_module(&m).is_ok());
    }
}
