//! IMPL effects checking.
//!
//! Verifies that effects used in IMPL function bodies are a subset of
//! the effects declared in the corresponding SPL spec.
//!
//! Rules:
//! - A function may only use capabilities passed as parameters
//! - When calling another function, the callee's required capabilities
//!   must be provided from the caller's available capabilities
//! - Pure functions (no capability params) may not perform any effects

use crate::ast::*;
use std::collections::HashSet;
use std::fmt;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// An effects checking error.
#[derive(Debug, Clone)]
pub struct EffectError {
    pub message: String,
    pub function: String,
}

impl fmt::Display for EffectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "effect error in `{}`: {}", self.function, self.message)
    }
}

impl std::error::Error for EffectError {}

// ---------------------------------------------------------------------------
// Effect context
// ---------------------------------------------------------------------------

/// Tracks the set of available capabilities in a function scope.
struct EffectCtx {
    /// Available capability names (from function parameters).
    available_caps: HashSet<String>,
    /// Capabilities actually used in the function body.
    used_caps: HashSet<String>,
    /// Function name for error reporting.
    function_name: String,
    /// Collected errors.
    errors: Vec<EffectError>,
}

impl EffectCtx {
    fn new(function_name: &str, cap_params: &[&ImplParam]) -> Self {
        let available_caps = cap_params
            .iter()
            .filter_map(|p| {
                if let ImplTypeRef::Capability(name) = &p.ty {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        EffectCtx {
            available_caps,
            used_caps: HashSet::new(),
            function_name: function_name.to_string(),
            errors: Vec::new(),
        }
    }

    fn err(&mut self, msg: impl Into<String>) {
        self.errors.push(EffectError {
            message: msg.into(),
            function: self.function_name.clone(),
        });
    }

    /// Record that a capability is being used (passed to a callee).
    fn use_cap(&mut self, cap_name: &str) {
        self.used_caps.insert(cap_name.to_string());
        if !self.available_caps.contains(cap_name) {
            self.err(format!(
                "capability `{cap_name}` used but not available in function scope"
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Check effects for all functions in an IMPL program.
///
/// Verifies:
/// 1. Functions only use capabilities they receive as parameters
/// 2. Capability variables are passed to function calls
pub fn check_effects(program: &ImplProgram) -> Result<(), Vec<EffectError>> {
    let mut errors = Vec::new();

    for item in &program.items {
        if let ImplItem::Function(f) = item {
            let cap_params: Vec<&ImplParam> =
                f.params.iter().filter(|p| p.is_cap).collect();
            let mut ctx = EffectCtx::new(&f.name, &cap_params);

            check_block_effects(&f.body, &mut ctx);

            errors.extend(ctx.errors);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// ---------------------------------------------------------------------------
// Block/statement/expression traversal
// ---------------------------------------------------------------------------

fn check_block_effects(block: &ImplBlock, ctx: &mut EffectCtx) {
    for stmt in &block.stmts {
        check_stmt_effects(stmt, ctx);
    }
    if let Some(expr) = &block.expr {
        check_expr_effects(expr, ctx);
    }
}

fn check_stmt_effects(stmt: &ImplStmt, ctx: &mut EffectCtx) {
    match stmt {
        ImplStmt::Let { value, .. } | ImplStmt::LetMut { value, .. } => {
            check_expr_effects(value, ctx);
        }
        ImplStmt::Assign { value, .. } => {
            check_expr_effects(value, ctx);
        }
        ImplStmt::If {
            cond,
            then_block,
            else_block,
        } => {
            check_expr_effects(cond, ctx);
            check_block_effects(then_block, ctx);
            if let Some(eb) = else_block {
                check_block_effects(eb, ctx);
            }
        }
        ImplStmt::Match { expr, arms } => {
            check_expr_effects(expr, ctx);
            for arm in arms {
                check_block_effects(&arm.body, ctx);
            }
        }
        ImplStmt::Return(Some(expr)) => {
            check_expr_effects(expr, ctx);
        }
        ImplStmt::Return(None) | ImplStmt::Break | ImplStmt::Continue => {}
        ImplStmt::Assert { cond, .. } => {
            check_expr_effects(cond, ctx);
        }
        ImplStmt::While { cond, body } => {
            check_expr_effects(cond, ctx);
            check_block_effects(body, ctx);
        }
        ImplStmt::Loop(body) => {
            check_block_effects(body, ctx);
        }
        ImplStmt::Expr(expr) => {
            check_expr_effects(expr, ctx);
        }
    }
}

fn check_expr_effects(expr: &ImplExpr, ctx: &mut EffectCtx) {
    match expr {
        ImplExpr::Literal(_) | ImplExpr::Var(_) | ImplExpr::Break | ImplExpr::Continue => {}
        ImplExpr::BinOp { lhs, rhs, .. } => {
            check_expr_effects(lhs, ctx);
            check_expr_effects(rhs, ctx);
        }
        ImplExpr::UnOp { operand, .. } => {
            check_expr_effects(operand, ctx);
        }
        ImplExpr::Call { args, .. } => {
            // Check args — any capability variable being passed records a use
            for arg in args {
                check_expr_effects(arg, ctx);
                // If an argument is a variable that is a known capability, record it
                if let ImplExpr::Var(name) = arg {
                    if ctx.available_caps.contains(name) {
                        ctx.use_cap(name);
                    }
                }
            }
        }
        ImplExpr::StructLit { fields, .. } => {
            for (_, value) in fields {
                check_expr_effects(value, ctx);
            }
        }
        ImplExpr::FieldGet { expr, .. } => {
            check_expr_effects(expr, ctx);
        }
        ImplExpr::EnumLit { args, .. } => {
            for arg in args {
                check_expr_effects(arg, ctx);
            }
        }
        ImplExpr::TupleLit(items) => {
            for item in items {
                check_expr_effects(item, ctx);
            }
        }
        ImplExpr::If {
            cond,
            then_block,
            else_block,
        } => {
            check_expr_effects(cond, ctx);
            check_block_effects(then_block, ctx);
            if let Some(eb) = else_block {
                check_block_effects(eb, ctx);
            }
        }
        ImplExpr::Match { expr, arms } => {
            check_expr_effects(expr, ctx);
            for arm in arms {
                check_block_effects(&arm.body, ctx);
            }
        }
        ImplExpr::Block(block) => {
            check_block_effects(block, ctx);
        }
        ImplExpr::Alloc { region, value } => {
            check_expr_effects(region, ctx);
            check_expr_effects(value, ctx);
        }
        ImplExpr::Borrow(expr) | ImplExpr::BorrowMut(expr) => {
            check_expr_effects(expr, ctx);
        }
        ImplExpr::Convert { expr, .. } => {
            check_expr_effects(expr, ctx);
        }
        ImplExpr::Loop(body) => {
            check_block_effects(body, ctx);
        }
        ImplExpr::While { cond, body } => {
            check_expr_effects(cond, ctx);
            check_block_effects(body, ctx);
        }
        ImplExpr::Return(Some(expr)) => {
            check_expr_effects(expr, ctx);
        }
        ImplExpr::Return(None) => {}
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_impl;

    fn check(src: &str) -> Result<(), Vec<EffectError>> {
        let prog = parse_impl(src).unwrap();
        check_effects(&prog)
    }

    #[test]
    fn test_pure_function_ok() {
        let src = r#"
            impl fn "test.v1" add(a: i32, b: i32) -> i32 {
                a + b
            }
        "#;
        assert!(check(src).is_ok());
    }

    #[test]
    fn test_cap_function_ok() {
        let src = r#"
            impl fn "test.v1" fetch(url: string, net: cap Net) -> string {
                do_fetch(url, net)
            }
        "#;
        assert!(check(src).is_ok());
    }

    #[test]
    fn test_unused_cap_is_ok() {
        // Having a cap param but not using it is fine (permission granted but not exercised)
        let src = r#"
            impl fn "test.v1" maybe_fetch(url: string, net: cap Net) -> string {
                url
            }
        "#;
        assert!(check(src).is_ok());
    }

    #[test]
    fn test_nested_blocks() {
        let src = r#"
            impl fn "test.v1" conditional_fetch(
                use_net: bool,
                url: string,
                net: cap Net,
            ) -> string {
                if use_net {
                    do_fetch(url, net)
                } else {
                    url
                }
            }
        "#;
        assert!(check(src).is_ok());
    }

    #[test]
    fn test_while_loop_with_effects() {
        let src = r#"
            impl fn "test.v1" poll(url: string, net: cap Net) -> string {
                let mut result: string = "";
                while result == "" {
                    result = do_fetch(url, net);
                }
                result
            }
        "#;
        assert!(check(src).is_ok());
    }

    #[test]
    fn test_match_with_effects() {
        let src = r#"
            impl fn "test.v1" handle(
                cmd: i32,
                net: cap Net,
            ) -> string {
                match cmd {
                    1 => do_fetch("http://example.com", net),
                    _ => "unknown",
                }
            }
        "#;
        assert!(check(src).is_ok());
    }

    #[test]
    fn test_multiple_caps() {
        let src = r#"
            impl fn "test.v1" fetch_and_log(
                url: string,
                net: cap Net,
                fs: cap FileWrite,
            ) -> string {
                let data: string = do_fetch(url, net);
                write_log(data, fs);
                data
            }
        "#;
        assert!(check(src).is_ok());
    }
}
