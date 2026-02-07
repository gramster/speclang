//! `std.contracts` — Pure boolean helpers for contract lowering.
//!
//! These functions simplify the SPL-to-IR contract lowering by providing
//! named boolean combinators. They're pure and always inlined by backends.

use speclang_ir::expr::{Block, Expr};
use speclang_ir::module::{Function, Module, Param};
use speclang_ir::types::{QName, Type};

fn qname(s: &str) -> QName {
    s.split('.').map(|p| p.to_string()).collect()
}

fn param(name: &str, ty: Type) -> Param {
    Param {
        name: name.to_string(),
        ty,
    }
}

/// Build the `std.contracts` module.
///
/// Unlike other stdlib modules that use `ExternFunction`, the contracts
/// module provides actual implementations since these are trivial pure
/// boolean functions that should be available immediately.
pub fn module() -> Module {
    let mut m = Module::new(qname("std.contracts"));

    // implies(a, b) = not a or b
    m.functions.push(Function {
        name: "implies".to_string(),
        params: vec![param("a", Type::bool()), param("b", Type::bool())],
        return_type: Type::bool(),
        effects: vec![],
        contracts: vec![],
        body: Block {
            stmts: vec![],
            expr: Some(Box::new(Expr::BinOp {
                op: speclang_ir::expr::BinOp::Or,
                lhs: Box::new(Expr::UnOp {
                    op: speclang_ir::expr::UnOp::Not,
                    operand: Box::new(Expr::Var("a".to_string())),
                }),
                rhs: Box::new(Expr::Var("b".to_string())),
            })),
        },
        annotations: vec![],
    });

    // and(a, b) = a and b
    m.functions.push(Function {
        name: "and".to_string(),
        params: vec![param("a", Type::bool()), param("b", Type::bool())],
        return_type: Type::bool(),
        effects: vec![],
        contracts: vec![],
        body: Block {
            stmts: vec![],
            expr: Some(Box::new(Expr::BinOp {
                op: speclang_ir::expr::BinOp::And,
                lhs: Box::new(Expr::Var("a".to_string())),
                rhs: Box::new(Expr::Var("b".to_string())),
            })),
        },
        annotations: vec![],
    });

    // or(a, b) = a or b
    m.functions.push(Function {
        name: "or".to_string(),
        params: vec![param("a", Type::bool()), param("b", Type::bool())],
        return_type: Type::bool(),
        effects: vec![],
        contracts: vec![],
        body: Block {
            stmts: vec![],
            expr: Some(Box::new(Expr::BinOp {
                op: speclang_ir::expr::BinOp::Or,
                lhs: Box::new(Expr::Var("a".to_string())),
                rhs: Box::new(Expr::Var("b".to_string())),
            })),
        },
        annotations: vec![],
    });

    // not(a) = !a
    m.functions.push(Function {
        name: "not".to_string(),
        params: vec![param("a", Type::bool())],
        return_type: Type::bool(),
        effects: vec![],
        contracts: vec![],
        body: Block {
            stmts: vec![],
            expr: Some(Box::new(Expr::UnOp {
                op: speclang_ir::expr::UnOp::Not,
                operand: Box::new(Expr::Var("a".to_string())),
            })),
        },
        annotations: vec![],
    });

    // forall_in_set — stub: backends expand quantifiers over iterators
    // (In v0, universal quantifiers are expanded at test-generation time,
    //  not at runtime.)

    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contracts_module_name() {
        let m = module();
        assert_eq!(m.name, vec!["std", "contracts"]);
    }

    #[test]
    fn contracts_has_implies() {
        let m = module();
        let f = m.find_function("implies").expect("implies function");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.return_type, Type::bool());
    }

    #[test]
    fn contracts_has_boolean_combinators() {
        let m = module();
        assert!(m.find_function("and").is_some());
        assert!(m.find_function("or").is_some());
        assert!(m.find_function("not").is_some());
    }

    #[test]
    fn implies_body_is_not_a_or_b() {
        let m = module();
        let f = m.find_function("implies").unwrap();
        // Body should have a trailing expression
        assert!(f.body.expr.is_some());
        match f.body.expr.as_deref().unwrap() {
            Expr::BinOp { op, .. } => {
                assert_eq!(*op, speclang_ir::expr::BinOp::Or);
            }
            other => panic!("Expected BinOp(Or), got {:?}", other),
        }
    }

    #[test]
    fn not_takes_one_param() {
        let m = module();
        let f = m.find_function("not").unwrap();
        assert_eq!(f.params.len(), 1);
    }
}
