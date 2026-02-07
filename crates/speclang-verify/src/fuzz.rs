//! Fuzz harness generation from SPL contracts.
//!
//! Generates fuzz targets from SPL function contracts and property tests.
//! Each fuzz target:
//! 1. Takes arbitrary bytes as input
//! 2. Derives structured inputs from the bytes
//! 3. Checks preconditions (skip if not satisfied)
//! 4. Calls the function under test
//! 5. Checks postconditions
//!
//! This module generates Core IR functions that can be lowered to Rust
//! (for cargo-fuzz/libFuzzer) or other fuzz engines.

use speclang_ir::contract::ContractKind;
use speclang_ir::expr::{Block, Expr, Literal, Stmt, UnOp};
use speclang_ir::module::{Function, Module, Param};
use speclang_ir::types::{PrimitiveType, Type};

/// A generated fuzz target.
#[derive(Debug, Clone)]
pub struct FuzzTarget {
    /// Name of the fuzz target.
    pub name: String,
    /// The source function being fuzzed.
    pub source_function: String,
    /// The generated fuzz harness function.
    pub function: Function,
}

/// Generate fuzz targets for all functions in a module that have contracts.
pub fn generate_fuzz_targets(module: &Module) -> Vec<FuzzTarget> {
    let mut targets = Vec::new();

    for func in &module.functions {
        // Skip test/prop functions (they ARE tests, not fuzz targets)
        if func.name.starts_with("test_") || func.name.starts_with("prop_") {
            continue;
        }

        // Only generate fuzz targets for functions with contracts
        let has_requires = func
            .contracts
            .iter()
            .any(|c| c.kind == ContractKind::Requires);
        let has_ensures = func
            .contracts
            .iter()
            .any(|c| c.kind == ContractKind::Ensures);

        if !has_requires && !has_ensures {
            continue;
        }

        targets.push(generate_fuzz_target(func));
    }

    targets
}

/// Generate a fuzz target for a single function.
fn generate_fuzz_target(func: &Function) -> FuzzTarget {
    let fuzz_name = format!("fuzz_{}", func.name);

    // The fuzz harness takes raw bytes and derives structured inputs.
    // For simplicity in v0, we generate a function that takes the same
    // params as the original (the fuzz engine provides structured inputs).
    let mut body_stmts = Vec::new();

    // 1. Check preconditions — if any fails, return early (skip this input)
    for contract in &func.contracts {
        if contract.kind == ContractKind::Requires {
            // if !precondition { return; }
            body_stmts.push(Stmt::If {
                cond: Expr::UnOp {
                    op: UnOp::Not,
                    operand: Box::new(contract.predicate.clone()),
                },
                then_block: Block {
                    stmts: vec![Stmt::Return(Expr::Literal(Literal::Unit))],
                    expr: None,
                },
                else_block: Block::empty(),
            });
        }
    }

    // 2. Call the function under test and bind result
    let args: Vec<Expr> = func
        .params
        .iter()
        .map(|p| Expr::Var(p.name.clone()))
        .collect();

    body_stmts.push(Stmt::Let {
        name: "result".to_string(),
        ty: func.return_type.clone(),
        value: Expr::Call {
            func: vec![func.name.clone()],
            args,
        },
    });

    // 3. Check postconditions
    for contract in &func.contracts {
        if contract.kind == ContractKind::Ensures {
            let tag_str = if contract.req_tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", contract.req_tags.join(", "))
            };
            body_stmts.push(Stmt::Assert {
                cond: contract.predicate.clone(),
                message: format!(
                    "fuzz: postcondition failed: {}{}",
                    func.name, tag_str
                ),
            });
        }
    }

    let harness = Function {
        name: fuzz_name.clone(),
        params: func.params.clone(),
        return_type: Type::unit(),
        effects: func.effects.clone(),
        contracts: vec![],
        body: Block {
            stmts: body_stmts,
            expr: None,
        },
        annotations: vec![],
    };

    FuzzTarget {
        name: fuzz_name,
        source_function: func.name.clone(),
        function: harness,
    }
}

/// Generate a fuzz harness module from fuzz targets.
pub fn generate_fuzz_module(
    module_name: &[String],
    targets: &[FuzzTarget],
) -> Module {
    let mut fuzz_mod_name: Vec<String> = module_name.to_vec();
    fuzz_mod_name.push("fuzz".to_string());
    let mut fuzz_module = Module::new(fuzz_mod_name);

    for target in targets {
        fuzz_module.functions.push(target.function.clone());
    }

    fuzz_module
}

/// Estimate the "fuzzability" of a function based on its parameter types.
///
/// Returns a score from 0.0 (hard to fuzz) to 1.0 (easy to fuzz).
/// Primitives are easy; complex nested types are harder.
pub fn fuzzability_score(params: &[Param]) -> f64 {
    if params.is_empty() {
        return 0.0; // Nothing to fuzz
    }

    let total: f64 = params.iter().map(|p| type_fuzzability(&p.ty)).sum();
    total / params.len() as f64
}

fn type_fuzzability(ty: &Type) -> f64 {
    match ty {
        Type::Primitive(p) => match p {
            PrimitiveType::Bool => 1.0,
            PrimitiveType::U8
            | PrimitiveType::I8
            | PrimitiveType::U16
            | PrimitiveType::I16
            | PrimitiveType::U32
            | PrimitiveType::I32
            | PrimitiveType::U64
            | PrimitiveType::I64 => 0.9,
            PrimitiveType::U128 | PrimitiveType::I128 => 0.8,
            PrimitiveType::F32 | PrimitiveType::F64 => 0.7,
            PrimitiveType::Int => 0.6,
            PrimitiveType::String => 0.5,
            PrimitiveType::Bytes => 0.6,
            PrimitiveType::Unit => 0.0,
        },
        Type::Option(inner) => type_fuzzability(inner) * 0.9,
        Type::Ref(inner) | Type::MutRef(inner) => type_fuzzability(inner),
        Type::Slice(inner) | Type::MutSlice(inner) => type_fuzzability(inner) * 0.7,
        Type::Tuple(elems) => {
            if elems.is_empty() {
                0.0
            } else {
                let sum: f64 = elems.iter().map(|t| type_fuzzability(t)).sum();
                sum / elems.len() as f64
            }
        }
        Type::Generic { .. } => 0.4,
        Type::Named(_) => 0.3,
        _ => 0.2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use speclang_ir::contract::{Contract, ContractPolicy};
    use speclang_ir::expr::{BinOp, Expr, Literal};

    fn make_contracted_function() -> Function {
        Function {
            name: "clamp".to_string(),
            params: vec![
                Param {
                    name: "x".to_string(),
                    ty: Type::i32(),
                },
                Param {
                    name: "lo".to_string(),
                    ty: Type::i32(),
                },
                Param {
                    name: "hi".to_string(),
                    ty: Type::i32(),
                },
            ],
            return_type: Type::i32(),
            effects: vec![],
            contracts: vec![
                Contract {
                    kind: ContractKind::Requires,
                    predicate: Expr::BinOp {
                        op: BinOp::Le,
                        lhs: Box::new(Expr::Var("lo".to_string())),
                        rhs: Box::new(Expr::Var("hi".to_string())),
                    },
                    policy: ContractPolicy::Always,
                    req_tags: vec![],
                },
                Contract {
                    kind: ContractKind::Ensures,
                    predicate: Expr::BinOp {
                        op: BinOp::And,
                        lhs: Box::new(Expr::BinOp {
                            op: BinOp::Ge,
                            lhs: Box::new(Expr::Var("result".to_string())),
                            rhs: Box::new(Expr::Var("lo".to_string())),
                        }),
                        rhs: Box::new(Expr::BinOp {
                            op: BinOp::Le,
                            lhs: Box::new(Expr::Var("result".to_string())),
                            rhs: Box::new(Expr::Var("hi".to_string())),
                        }),
                    },
                    policy: ContractPolicy::Always,
                    req_tags: vec!["REQ-CLAMP".to_string()],
                },
            ],
            body: Block {
                stmts: vec![],
                expr: Some(Box::new(Expr::Var("x".to_string()))),
            },
            annotations: vec![],
        }
    }

    #[test]
    fn generates_fuzz_target_for_contracted_function() {
        let mut m = Module::new(vec!["test".to_string()]);
        m.functions.push(make_contracted_function());
        let targets = generate_fuzz_targets(&m);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].name, "fuzz_clamp");
        assert_eq!(targets[0].source_function, "clamp");
    }

    #[test]
    fn fuzz_target_skips_on_precondition_failure() {
        let func = make_contracted_function();
        let target = generate_fuzz_target(&func);
        // First statement should be If (precondition check)
        match &target.function.body.stmts[0] {
            Stmt::If { then_block, .. } => {
                // Then block should return early
                assert!(matches!(&then_block.stmts[0], Stmt::Return(_)));
            }
            other => panic!("Expected If, got {:?}", other),
        }
    }

    #[test]
    fn fuzz_target_calls_function_and_checks_postcondition() {
        let func = make_contracted_function();
        let target = generate_fuzz_target(&func);
        let stmts = &target.function.body.stmts;
        // Should have: If (precondition), Let result = call, Assert (postcondition)
        assert!(stmts.len() >= 3);
        assert!(matches!(&stmts[1], Stmt::Let { name, .. } if name == "result"));
        assert!(matches!(&stmts[2], Stmt::Assert { .. }));
    }

    #[test]
    fn no_fuzz_target_for_uncontraced_function() {
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
        let targets = generate_fuzz_targets(&m);
        assert!(targets.is_empty());
    }

    #[test]
    fn skips_test_and_prop_functions() {
        let mut m = Module::new(vec!["test".to_string()]);
        let mut contracted = make_contracted_function();
        contracted.name = "test_something".to_string();
        m.functions.push(contracted);
        let targets = generate_fuzz_targets(&m);
        assert!(targets.is_empty());
    }

    #[test]
    fn generate_fuzz_module_creates_module() {
        let mut m = Module::new(vec!["my".to_string(), "mod".to_string()]);
        m.functions.push(make_contracted_function());
        let targets = generate_fuzz_targets(&m);
        let fuzz_mod = generate_fuzz_module(&m.name, &targets);
        assert_eq!(fuzz_mod.name, vec!["my", "mod", "fuzz"]);
        assert_eq!(fuzz_mod.functions.len(), 1);
    }

    #[test]
    fn fuzzability_primitives_scored_high() {
        let params = vec![
            Param { name: "a".to_string(), ty: Type::i32() },
            Param { name: "b".to_string(), ty: Type::i32() },
        ];
        let score = fuzzability_score(&params);
        assert!(score > 0.8);
    }

    #[test]
    fn fuzzability_empty_params_zero() {
        assert_eq!(fuzzability_score(&[]), 0.0);
    }

    #[test]
    fn fuzzability_complex_types_scored_lower() {
        let params = vec![
            Param {
                name: "m".to_string(),
                ty: Type::Generic {
                    name: vec!["Map".to_string()],
                    args: vec![Type::string(), Type::int()],
                },
            },
        ];
        let score = fuzzability_score(&params);
        assert!(score < 0.5);
    }
}
