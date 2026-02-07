//! Contract insertion pass.
//!
//! Transforms a Core IR module by inserting runtime assertion statements
//! derived from function contracts (requires/ensures). The insertion
//! respects the `ContractPolicy` on each contract:
//! - `Always` → always insert
//! - `Debug` → insert only in debug mode
//! - `Sampled(ppm)` → insert with probabilistic guard
//!
//! Preconditions (`Requires`) are inserted at the start of the function body.
//! Postconditions (`Ensures`) are inserted before each `return` statement
//! and at the end of the body (for the trailing expression).

use speclang_ir::contract::{ContractKind, ContractPolicy};
use speclang_ir::expr::{BinOp, Block, Expr, Literal, Stmt};
use speclang_ir::module::{Function, Module};
use speclang_ir::types::Type;

/// Compilation mode that controls which contracts are inserted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationMode {
    /// Insert all contracts regardless of policy.
    Debug,
    /// Insert only `Always` contracts; skip `Debug` and `Sampled`.
    Release,
    /// Insert `Always` and `Sampled` (with probability guards); skip `Debug`.
    ReleaseSampled,
}

/// Insert contract assertions into a module according to the compilation mode.
///
/// Returns a new module with assertion statements injected.
pub fn insert_contracts(module: &Module, mode: CompilationMode) -> Module {
    let mut result = module.clone();
    for func in &mut result.functions {
        insert_function_contracts(func, mode);
    }
    result
}

/// Insert contract assertions into a single function.
fn insert_function_contracts(func: &mut Function, mode: CompilationMode) {
    let mut preconditions = Vec::new();
    let mut postconditions = Vec::new();

    for contract in &func.contracts {
        if !should_insert(&contract.policy, mode) {
            continue;
        }

        let tag_str = if contract.req_tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", contract.req_tags.join(", "))
        };

        match contract.kind {
            ContractKind::Requires => {
                let message = format!(
                    "precondition failed: {}{}",
                    func.name, tag_str
                );
                let assert_stmt = make_assert(
                    &contract.predicate,
                    &message,
                    &contract.policy,
                    mode,
                );
                preconditions.push(assert_stmt);
            }
            ContractKind::Ensures => {
                let message = format!(
                    "postcondition failed: {}{}",
                    func.name, tag_str
                );
                let assert_stmt = make_assert(
                    &contract.predicate,
                    &message,
                    &contract.policy,
                    mode,
                );
                postconditions.push(assert_stmt);
            }
            ContractKind::Invariant => {
                // Invariants are handled at type-construction time,
                // not in function bodies.
            }
        }
    }

    // Insert preconditions at the start of the body
    if !preconditions.is_empty() {
        let mut new_stmts = preconditions;
        new_stmts.append(&mut func.body.stmts);
        func.body.stmts = new_stmts;
    }

    // Insert postconditions before return statements and at the end
    if !postconditions.is_empty() {
        func.body.stmts = rewrite_returns_with_postconditions(
            &func.body.stmts,
            &postconditions,
        );

        // If the function has a trailing expression, bind it to `result`,
        // check postconditions, then return `result`.
        if let Some(trailing) = func.body.expr.take() {
            let mut final_stmts = Vec::new();
            final_stmts.push(Stmt::Let {
                name: "result".to_string(),
                ty: func.return_type.clone(),
                value: *trailing,
            });
            for post in &postconditions {
                final_stmts.push(post.clone());
            }
            func.body.stmts.append(&mut final_stmts);
            func.body.expr = Some(Box::new(Expr::Var("result".to_string())));
        }
    }
}

/// Determine whether a contract should be inserted based on policy and mode.
fn should_insert(policy: &ContractPolicy, mode: CompilationMode) -> bool {
    match (policy, mode) {
        (_, CompilationMode::Debug) => true,
        (ContractPolicy::Always, _) => true,
        (ContractPolicy::Debug, CompilationMode::Release) => false,
        (ContractPolicy::Debug, CompilationMode::ReleaseSampled) => false,
        (ContractPolicy::Sampled(_), CompilationMode::Release) => false,
        (ContractPolicy::Sampled(_), CompilationMode::ReleaseSampled) => true,
    }
}

/// Create an assertion statement, optionally wrapped in a sampling guard.
fn make_assert(
    predicate: &Expr,
    message: &str,
    policy: &ContractPolicy,
    mode: CompilationMode,
) -> Stmt {
    let assert_stmt = Stmt::Assert {
        cond: predicate.clone(),
        message: message.to_string(),
    };

    match (policy, mode) {
        (ContractPolicy::Sampled(ppm), CompilationMode::ReleaseSampled) => {
            // Wrap in: if random_sample() < ppm { assert(...) }
            // We model this as an If statement with the assert inside.
            Stmt::If {
                cond: Expr::BinOp {
                    op: BinOp::Lt,
                    lhs: Box::new(Expr::Call {
                        func: vec!["__runtime".to_string(), "sample_u32".to_string()],
                        args: vec![],
                    }),
                    rhs: Box::new(Expr::Literal(Literal::Int(*ppm as i128))),
                },
                then_block: Block {
                    stmts: vec![assert_stmt],
                    expr: None,
                },
                else_block: Block::empty(),
            }
        }
        _ => assert_stmt,
    }
}

/// Rewrite return statements to include postcondition checks.
///
/// For each `Return(expr)`, we generate:
/// ```text
/// let result = expr;
/// assert(postcondition_1);
/// assert(postcondition_2);
/// return result;
/// ```
fn rewrite_returns_with_postconditions(
    stmts: &[Stmt],
    postconditions: &[Stmt],
) -> Vec<Stmt> {
    let mut result = Vec::new();
    for stmt in stmts {
        match stmt {
            Stmt::Return(expr) => {
                result.push(Stmt::Let {
                    name: "result".to_string(),
                    ty: Type::unit(), // Type is approximate; verifier has already checked
                    value: expr.clone(),
                });
                for post in postconditions {
                    result.push(post.clone());
                }
                result.push(Stmt::Return(Expr::Var("result".to_string())));
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                result.push(Stmt::If {
                    cond: cond.clone(),
                    then_block: Block {
                        stmts: rewrite_returns_with_postconditions(
                            &then_block.stmts,
                            postconditions,
                        ),
                        expr: then_block.expr.clone(),
                    },
                    else_block: Block {
                        stmts: rewrite_returns_with_postconditions(
                            &else_block.stmts,
                            postconditions,
                        ),
                        expr: else_block.expr.clone(),
                    },
                });
            }
            Stmt::Match { expr, arms } => {
                let new_arms: Vec<_> = arms
                    .iter()
                    .map(|arm| speclang_ir::expr::MatchArm {
                        pattern: arm.pattern.clone(),
                        body: Block {
                            stmts: rewrite_returns_with_postconditions(
                                &arm.body.stmts,
                                postconditions,
                            ),
                            expr: arm.body.expr.clone(),
                        },
                    })
                    .collect();
                result.push(Stmt::Match {
                    expr: expr.clone(),
                    arms: new_arms,
                });
            }
            other => result.push(other.clone()),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use speclang_ir::contract::{Contract, ContractKind, ContractPolicy};
    use speclang_ir::expr::{BinOp, Expr, Literal};
    use speclang_ir::module::{Function, Module, Param};
    use speclang_ir::types::Type;

    fn make_test_module() -> Module {
        let mut m = Module::new(vec!["test".to_string()]);
        m.functions.push(Function {
            name: "add".to_string(),
            params: vec![
                Param {
                    name: "a".to_string(),
                    ty: Type::i32(),
                },
                Param {
                    name: "b".to_string(),
                    ty: Type::i32(),
                },
            ],
            return_type: Type::i32(),
            effects: vec![],
            contracts: vec![
                Contract {
                    kind: ContractKind::Requires,
                    predicate: Expr::BinOp {
                        op: BinOp::Ge,
                        lhs: Box::new(Expr::Var("a".to_string())),
                        rhs: Box::new(Expr::Literal(Literal::Int(0))),
                    },
                    policy: ContractPolicy::Always,
                    req_tags: vec!["REQ-001".to_string()],
                },
                Contract {
                    kind: ContractKind::Ensures,
                    predicate: Expr::BinOp {
                        op: BinOp::Ge,
                        lhs: Box::new(Expr::Var("result".to_string())),
                        rhs: Box::new(Expr::Var("a".to_string())),
                    },
                    policy: ContractPolicy::Debug,
                    req_tags: vec![],
                },
            ],
            body: Block {
                stmts: vec![],
                expr: Some(Box::new(Expr::BinOp {
                    op: BinOp::Add,
                    lhs: Box::new(Expr::Var("a".to_string())),
                    rhs: Box::new(Expr::Var("b".to_string())),
                })),
            },
            annotations: vec![],
        });
        m
    }

    #[test]
    fn debug_mode_inserts_all_contracts() {
        let m = make_test_module();
        let result = insert_contracts(&m, CompilationMode::Debug);
        let func = result.find_function("add").unwrap();
        // Should have precondition assert + postcondition let+assert
        assert!(func.body.stmts.len() >= 2);
        // First stmt should be the precondition assert
        match &func.body.stmts[0] {
            Stmt::Assert { message, .. } => {
                assert!(message.contains("precondition"));
            }
            other => panic!("Expected Assert, got {:?}", other),
        }
    }

    #[test]
    fn release_mode_skips_debug_contracts() {
        let m = make_test_module();
        let result = insert_contracts(&m, CompilationMode::Release);
        let func = result.find_function("add").unwrap();
        // Only the Always precondition should be present
        let asserts: Vec<_> = func
            .body
            .stmts
            .iter()
            .filter(|s| matches!(s, Stmt::Assert { .. }))
            .collect();
        assert_eq!(asserts.len(), 1);
        match &asserts[0] {
            Stmt::Assert { message, .. } => {
                assert!(message.contains("precondition"));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn postcondition_binds_result() {
        let m = make_test_module();
        let result = insert_contracts(&m, CompilationMode::Debug);
        let func = result.find_function("add").unwrap();
        // Should have a Let binding for "result"
        let has_result_binding = func.body.stmts.iter().any(|s| {
            matches!(s, Stmt::Let { name, .. } if name == "result")
        });
        assert!(has_result_binding);
    }

    #[test]
    fn trailing_expr_becomes_result_var() {
        let m = make_test_module();
        let result = insert_contracts(&m, CompilationMode::Debug);
        let func = result.find_function("add").unwrap();
        // Trailing expression should now be Var("result")
        match func.body.expr.as_deref() {
            Some(Expr::Var(name)) => assert_eq!(name, "result"),
            other => panic!("Expected Var(result), got {:?}", other),
        }
    }

    #[test]
    fn sampled_contract_wraps_in_if() {
        let mut m = Module::new(vec!["test".to_string()]);
        m.functions.push(Function {
            name: "f".to_string(),
            params: vec![],
            return_type: Type::unit(),
            effects: vec![],
            contracts: vec![Contract {
                kind: ContractKind::Requires,
                predicate: Expr::Literal(Literal::Bool(true)),
                policy: ContractPolicy::Sampled(1000),
                req_tags: vec![],
            }],
            body: Block::empty(),
            annotations: vec![],
        });
        let result = insert_contracts(&m, CompilationMode::ReleaseSampled);
        let func = result.find_function("f").unwrap();
        // The sampled contract should be wrapped in an If
        match &func.body.stmts[0] {
            Stmt::If { then_block, .. } => {
                assert!(matches!(&then_block.stmts[0], Stmt::Assert { .. }));
            }
            other => panic!("Expected If, got {:?}", other),
        }
    }

    #[test]
    fn no_contracts_no_change() {
        let mut m = Module::new(vec!["test".to_string()]);
        m.functions.push(Function {
            name: "noop".to_string(),
            params: vec![],
            return_type: Type::unit(),
            effects: vec![],
            contracts: vec![],
            body: Block::empty(),
            annotations: vec![],
        });
        let result = insert_contracts(&m, CompilationMode::Debug);
        let func = result.find_function("noop").unwrap();
        assert!(func.body.stmts.is_empty());
        assert!(func.body.expr.is_none());
    }

    #[test]
    fn return_stmt_gets_postcondition() {
        let mut m = Module::new(vec!["test".to_string()]);
        m.functions.push(Function {
            name: "ret".to_string(),
            params: vec![],
            return_type: Type::i32(),
            effects: vec![],
            contracts: vec![Contract {
                kind: ContractKind::Ensures,
                predicate: Expr::BinOp {
                    op: BinOp::Gt,
                    lhs: Box::new(Expr::Var("result".to_string())),
                    rhs: Box::new(Expr::Literal(Literal::Int(0))),
                },
                policy: ContractPolicy::Always,
                req_tags: vec![],
            }],
            body: Block {
                stmts: vec![Stmt::Return(Expr::Literal(Literal::Int(42)))],
                expr: None,
            },
            annotations: vec![],
        });
        let result = insert_contracts(&m, CompilationMode::Release);
        let func = result.find_function("ret").unwrap();
        // Should have: let result = 42; assert(...); return result;
        assert_eq!(func.body.stmts.len(), 3);
        assert!(matches!(&func.body.stmts[0], Stmt::Let { name, .. } if name == "result"));
        assert!(matches!(&func.body.stmts[1], Stmt::Assert { .. }));
        assert!(matches!(&func.body.stmts[2], Stmt::Return(_)));
    }

    #[test]
    fn should_insert_logic() {
        // Always inserts in all modes
        assert!(should_insert(&ContractPolicy::Always, CompilationMode::Debug));
        assert!(should_insert(&ContractPolicy::Always, CompilationMode::Release));
        assert!(should_insert(&ContractPolicy::Always, CompilationMode::ReleaseSampled));

        // Debug only inserts in Debug mode
        assert!(should_insert(&ContractPolicy::Debug, CompilationMode::Debug));
        assert!(!should_insert(&ContractPolicy::Debug, CompilationMode::Release));
        assert!(!should_insert(&ContractPolicy::Debug, CompilationMode::ReleaseSampled));

        // Sampled inserts in Debug and ReleaseSampled
        assert!(should_insert(&ContractPolicy::Sampled(100), CompilationMode::Debug));
        assert!(!should_insert(&ContractPolicy::Sampled(100), CompilationMode::Release));
        assert!(should_insert(&ContractPolicy::Sampled(100), CompilationMode::ReleaseSampled));
    }
}
