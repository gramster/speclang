//! Capability threading verification.
//!
//! Ensures that Core IR functions with effects (capabilities) are only
//! called from functions that also declare those effects. This prevents
//! hidden I/O and side effects.
//!
//! Rules:
//! - If a function `f` calls function `g` and `g` declares effects `{Net, Fs}`,
//!   then `f` must also declare `{Net, Fs}` (or a superset).
//! - Pure functions (no declared effects) may not call effectful functions.

use speclang_ir::expr::{Block, Expr, Stmt};
use speclang_ir::module::Module;
use speclang_ir::types::QName;
use std::collections::{HashMap, HashSet};
use std::fmt;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// A capability threading error.
#[derive(Debug, Clone)]
pub struct CapabilityError {
    pub message: String,
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "capability error: {}", self.message)
    }
}

impl std::error::Error for CapabilityError {}

// ---------------------------------------------------------------------------
// Checker
// ---------------------------------------------------------------------------

struct CapabilityChecker<'a> {
    module: &'a Module,
    /// Map from function name → set of declared capability names.
    fn_effects: HashMap<QName, HashSet<String>>,
    errors: Vec<CapabilityError>,
}

impl<'a> CapabilityChecker<'a> {
    fn new(module: &'a Module) -> Self {
        let mut fn_effects = HashMap::new();

        // Collect declared effects for every function.
        for f in &module.functions {
            let caps: HashSet<String> = f.effects.iter().map(|c| c.name.clone()).collect();
            fn_effects.insert(vec![f.name.clone()], caps);
        }
        for e in &module.externs {
            let caps: HashSet<String> = e.effects.iter().map(|c| c.name.clone()).collect();
            fn_effects.insert(vec![e.name.clone()], caps);
        }

        CapabilityChecker {
            module,
            fn_effects,
            errors: Vec::new(),
        }
    }

    fn err(&mut self, msg: impl Into<String>) {
        self.errors.push(CapabilityError {
            message: msg.into(),
        });
    }

    fn check_module(&mut self) {
        for f in &self.module.functions {
            let caller_caps: HashSet<String> =
                f.effects.iter().map(|c| c.name.clone()).collect();
            self.check_block(&f.body, &caller_caps, &f.name);
        }
    }

    fn check_block(
        &mut self,
        block: &Block,
        caller_caps: &HashSet<String>,
        fn_name: &str,
    ) {
        for stmt in &block.stmts {
            self.check_stmt(stmt, caller_caps, fn_name);
        }
        if let Some(tail) = &block.expr {
            self.check_expr(tail, caller_caps, fn_name);
        }
    }

    fn check_stmt(
        &mut self,
        stmt: &Stmt,
        caller_caps: &HashSet<String>,
        fn_name: &str,
    ) {
        match stmt {
            Stmt::Let { value, .. } => self.check_expr(value, caller_caps, fn_name),
            Stmt::Assign { value, .. } => self.check_expr(value, caller_caps, fn_name),
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                self.check_expr(cond, caller_caps, fn_name);
                self.check_block(then_block, caller_caps, fn_name);
                self.check_block(else_block, caller_caps, fn_name);
            }
            Stmt::Match { expr, arms } => {
                self.check_expr(expr, caller_caps, fn_name);
                for arm in arms {
                    self.check_block(&arm.body, caller_caps, fn_name);
                }
            }
            Stmt::Return(e) => self.check_expr(e, caller_caps, fn_name),
            Stmt::Assert { cond, .. } => self.check_expr(cond, caller_caps, fn_name),
            Stmt::Expr(e) => self.check_expr(e, caller_caps, fn_name),
        }
    }

    fn check_expr(
        &mut self,
        expr: &Expr,
        caller_caps: &HashSet<String>,
        fn_name: &str,
    ) {
        match expr {
            Expr::Call { func, args } => {
                // Check that the caller has all effects of the callee.
                if let Some(callee_caps) = self.fn_effects.get(func) {
                    let missing: Vec<&String> = callee_caps
                        .iter()
                        .filter(|c| !caller_caps.contains(*c))
                        .collect();
                    if !missing.is_empty() {
                        let missing_str: Vec<String> =
                            missing.iter().map(|c| c.to_string()).collect();
                        let func_str = func.join(".");
                        self.err(format!(
                            "function '{}' calls '{}' which requires capabilities [{}], \
                             but '{}' does not declare them",
                            fn_name,
                            func_str,
                            missing_str.join(", "),
                            fn_name,
                        ));
                    }
                }
                for arg in args {
                    self.check_expr(arg, caller_caps, fn_name);
                }
            }
            Expr::BinOp { lhs, rhs, .. } => {
                self.check_expr(lhs, caller_caps, fn_name);
                self.check_expr(rhs, caller_caps, fn_name);
            }
            Expr::UnOp { operand, .. } => {
                self.check_expr(operand, caller_caps, fn_name);
            }
            Expr::StructLit { fields, .. } => {
                for (_, v) in fields {
                    self.check_expr(v, caller_caps, fn_name);
                }
            }
            Expr::FieldGet { expr, .. } => {
                self.check_expr(expr, caller_caps, fn_name);
            }
            Expr::EnumLit { fields, .. } => {
                for f in fields {
                    self.check_expr(f, caller_caps, fn_name);
                }
            }
            Expr::TupleLit(elems) => {
                for e in elems {
                    self.check_expr(e, caller_caps, fn_name);
                }
            }
            Expr::If {
                cond,
                then_block,
                else_block,
            } => {
                self.check_expr(cond, caller_caps, fn_name);
                self.check_block(then_block, caller_caps, fn_name);
                self.check_block(else_block, caller_caps, fn_name);
            }
            Expr::Match { expr, arms } => {
                self.check_expr(expr, caller_caps, fn_name);
                for arm in arms {
                    self.check_block(&arm.body, caller_caps, fn_name);
                }
            }
            Expr::Block(block) => {
                self.check_block(block, caller_caps, fn_name);
            }
            Expr::Alloc { value, .. } => {
                self.check_expr(value, caller_caps, fn_name);
            }
            Expr::Borrow(e) | Expr::BorrowMut(e) => {
                self.check_expr(e, caller_caps, fn_name);
            }
            Expr::Convert { expr, .. } => {
                self.check_expr(expr, caller_caps, fn_name);
            }
            Expr::Literal(_) | Expr::Var(_) => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Verify capability threading rules in a Core IR module.
pub fn verify_capabilities(module: &Module) -> Result<(), Vec<CapabilityError>> {
    let mut checker = CapabilityChecker::new(module);
    checker.check_module();

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
    use speclang_ir::expr::{Block, Expr, Literal};
    use speclang_ir::module::{ExternFunction, Function, Module, Param};
    use speclang_ir::types::Type;
    use speclang_ir::CapabilityType;

    fn make_module(name: &str) -> Module {
        Module::new(vec![name.to_string()])
    }

    fn make_fn_with_effects(
        name: &str,
        effects: Vec<&str>,
        body: Block,
    ) -> Function {
        Function {
            name: name.into(),
            params: vec![],
            return_type: Type::unit(),
            effects: effects
                .into_iter()
                .map(|e| CapabilityType { name: e.into() })
                .collect(),
            contracts: vec![],
            body,
            annotations: vec![],
        }
    }

    #[test]
    fn capability_pure_calling_pure_ok() {
        let mut m = make_module("test");
        m.functions.push(make_fn_with_effects(
            "helper",
            vec![],
            Block::new(vec![], Some(Expr::Literal(Literal::Unit))),
        ));
        m.functions.push(make_fn_with_effects(
            "main",
            vec![],
            Block::new(
                vec![],
                Some(Expr::Call {
                    func: vec!["helper".into()],
                    args: vec![],
                }),
            ),
        ));
        assert!(verify_capabilities(&m).is_ok());
    }

    #[test]
    fn capability_effectful_calling_effectful_ok() {
        let mut m = make_module("test");
        m.functions.push(make_fn_with_effects(
            "do_io",
            vec!["Net"],
            Block::new(vec![], Some(Expr::Literal(Literal::Unit))),
        ));
        m.functions.push(make_fn_with_effects(
            "main",
            vec!["Net"],
            Block::new(
                vec![],
                Some(Expr::Call {
                    func: vec!["do_io".into()],
                    args: vec![],
                }),
            ),
        ));
        assert!(verify_capabilities(&m).is_ok());
    }

    #[test]
    fn capability_pure_calling_effectful_fails() {
        let mut m = make_module("test");
        m.functions.push(make_fn_with_effects(
            "do_io",
            vec!["Net"],
            Block::new(vec![], Some(Expr::Literal(Literal::Unit))),
        ));
        m.functions.push(make_fn_with_effects(
            "main",
            vec![], // pure!
            Block::new(
                vec![],
                Some(Expr::Call {
                    func: vec!["do_io".into()],
                    args: vec![],
                }),
            ),
        ));
        let errs = verify_capabilities(&m).unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("Net")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn capability_missing_subset_fails() {
        let mut m = make_module("test");
        m.functions.push(make_fn_with_effects(
            "full_io",
            vec!["Net", "Fs"],
            Block::new(vec![], Some(Expr::Literal(Literal::Unit))),
        ));
        m.functions.push(make_fn_with_effects(
            "partial",
            vec!["Net"], // has Net but not Fs
            Block::new(
                vec![],
                Some(Expr::Call {
                    func: vec!["full_io".into()],
                    args: vec![],
                }),
            ),
        ));
        let errs = verify_capabilities(&m).unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("Fs")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn capability_extern_effects_checked() {
        let mut m = make_module("test");
        m.externs.push(ExternFunction {
            name: "syscall".into(),
            params: vec![],
            return_type: Type::unit(),
            effects: vec![CapabilityType { name: "Fs".into() }],
            annotations: vec![],
        });
        m.functions.push(make_fn_with_effects(
            "main",
            vec![], // pure
            Block::new(
                vec![],
                Some(Expr::Call {
                    func: vec!["syscall".into()],
                    args: vec![],
                }),
            ),
        ));
        let errs = verify_capabilities(&m).unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("Fs")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn capability_superset_ok() {
        let mut m = make_module("test");
        m.functions.push(make_fn_with_effects(
            "net_only",
            vec!["Net"],
            Block::new(vec![], Some(Expr::Literal(Literal::Unit))),
        ));
        m.functions.push(make_fn_with_effects(
            "main",
            vec!["Net", "Fs", "Rand"], // superset — OK
            Block::new(
                vec![],
                Some(Expr::Call {
                    func: vec!["net_only".into()],
                    args: vec![],
                }),
            ),
        ));
        assert!(verify_capabilities(&m).is_ok());
    }
}
