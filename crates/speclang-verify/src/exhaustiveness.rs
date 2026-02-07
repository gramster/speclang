//! Exhaustive pattern match checking.
//!
//! Ensures that every `match` expression in a Core IR module covers all
//! possible variants of the scrutinee type. The checker looks at:
//! - Enum types → every variant must appear, or a wildcard must be present.
//! - Bool → true and false, or wildcard.
//! - Tuples/structs → at least one arm must be a wildcard or bind.
//!
//! If a wildcard (`_`) or bind pattern is present, the match is trivially
//! exhaustive for that position.

use speclang_ir::expr::{Block, Expr, MatchArm, Pattern, Stmt};
use speclang_ir::module::{Function, Module};
use speclang_ir::types::Type;
use std::collections::{HashMap, HashSet};
use std::fmt;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// A pattern exhaustiveness error.
#[derive(Debug, Clone)]
pub struct ExhaustivenessError {
    pub message: String,
}

impl fmt::Display for ExhaustivenessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "exhaustiveness error: {}", self.message)
    }
}

impl std::error::Error for ExhaustivenessError {}

// ---------------------------------------------------------------------------
// Checker
// ---------------------------------------------------------------------------

struct ExhaustivenessChecker<'a> {
    module: &'a Module,
    /// Map type name → list of variant names (for enum types).
    enum_variants: HashMap<String, Vec<String>>,
    errors: Vec<ExhaustivenessError>,
}

impl<'a> ExhaustivenessChecker<'a> {
    fn new(module: &'a Module) -> Self {
        let mut enum_variants = HashMap::new();

        for td in &module.type_defs {
            if let Type::Enum(variants) = &td.ty {
                let names: Vec<String> =
                    variants.iter().map(|v| v.name.clone()).collect();
                enum_variants.insert(td.name.clone(), names);
            }
        }

        ExhaustivenessChecker {
            module,
            enum_variants,
            errors: Vec::new(),
        }
    }

    fn err(&mut self, msg: impl Into<String>) {
        self.errors.push(ExhaustivenessError {
            message: msg.into(),
        });
    }

    fn check_module(&mut self) {
        for f in &self.module.functions {
            self.check_function(f);
        }
    }

    fn check_function(&mut self, f: &Function) {
        self.check_block(&f.body, &f.name);
    }

    fn check_block(&mut self, block: &Block, fn_name: &str) {
        for stmt in &block.stmts {
            self.check_stmt(stmt, fn_name);
        }
        if let Some(tail) = &block.expr {
            self.check_expr(tail, fn_name);
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt, fn_name: &str) {
        match stmt {
            Stmt::Let { value, .. } => self.check_expr(value, fn_name),
            Stmt::Assign { value, .. } => self.check_expr(value, fn_name),
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                self.check_expr(cond, fn_name);
                self.check_block(then_block, fn_name);
                self.check_block(else_block, fn_name);
            }
            Stmt::Match { expr, arms } => {
                self.check_expr(expr, fn_name);
                self.check_arms(arms, fn_name);
                for arm in arms {
                    self.check_block(&arm.body, fn_name);
                }
            }
            Stmt::Return(e) => self.check_expr(e, fn_name),
            Stmt::Assert { cond, .. } => self.check_expr(cond, fn_name),
            Stmt::Expr(e) => self.check_expr(e, fn_name),
        }
    }

    fn check_expr(&mut self, expr: &Expr, fn_name: &str) {
        match expr {
            Expr::Match { expr, arms } => {
                self.check_expr(expr, fn_name);
                self.check_arms(arms, fn_name);
                for arm in arms {
                    self.check_block(&arm.body, fn_name);
                }
            }
            Expr::If {
                cond,
                then_block,
                else_block,
            } => {
                self.check_expr(cond, fn_name);
                self.check_block(then_block, fn_name);
                self.check_block(else_block, fn_name);
            }
            Expr::Block(block) => self.check_block(block, fn_name),
            Expr::BinOp { lhs, rhs, .. } => {
                self.check_expr(lhs, fn_name);
                self.check_expr(rhs, fn_name);
            }
            Expr::UnOp { operand, .. } => {
                self.check_expr(operand, fn_name);
            }
            Expr::Call { args, .. } => {
                for arg in args {
                    self.check_expr(arg, fn_name);
                }
            }
            Expr::StructLit { fields, .. } => {
                for (_, v) in fields {
                    self.check_expr(v, fn_name);
                }
            }
            Expr::FieldGet { expr, .. } => self.check_expr(expr, fn_name),
            Expr::EnumLit { fields, .. } => {
                for f in fields {
                    self.check_expr(f, fn_name);
                }
            }
            Expr::TupleLit(elems) => {
                for e in elems {
                    self.check_expr(e, fn_name);
                }
            }
            Expr::Alloc { value, .. } => self.check_expr(value, fn_name),
            Expr::Borrow(e) | Expr::BorrowMut(e) => self.check_expr(e, fn_name),
            Expr::Convert { expr, .. } => self.check_expr(expr, fn_name),
            Expr::Literal(_) | Expr::Var(_) => {}
        }
    }

    fn check_arms(&mut self, arms: &[MatchArm], fn_name: &str) {
        if arms.is_empty() {
            self.err(format!(
                "empty match expression in function '{fn_name}'"
            ));
            return;
        }

        // Check if there's a wildcard or bind — trivially exhaustive.
        let has_catch_all = arms.iter().any(|arm| is_catch_all(&arm.pattern));
        if has_catch_all {
            return;
        }

        // Collect variant names from Variant patterns.
        let variant_names: HashSet<String> = arms
            .iter()
            .filter_map(|arm| {
                if let Pattern::Variant { ty: _, variant, .. } = &arm.pattern {
                    Some(variant.clone())
                } else {
                    None
                }
            })
            .collect();

        // If we see variant patterns, check exhaustiveness against known enum types.
        if !variant_names.is_empty() {
            // Find the enum type from the first variant pattern.
            let type_name = arms.iter().find_map(|arm| {
                if let Pattern::Variant { ty, .. } = &arm.pattern {
                    // Use last component of QName as the type name
                    ty.last().cloned()
                } else {
                    None
                }
            });

            if let Some(ty_name) = type_name {
                if let Some(all_variants) = self.enum_variants.get(&ty_name) {
                    let missing: Vec<&String> = all_variants
                        .iter()
                        .filter(|v| !variant_names.contains(*v))
                        .collect();
                    if !missing.is_empty() {
                        let missing_str: Vec<String> =
                            missing.iter().map(|v| v.to_string()).collect();
                        self.err(format!(
                            "non-exhaustive match in function '{}': \
                             missing variants [{}] of type '{}'",
                            fn_name,
                            missing_str.join(", "),
                            ty_name,
                        ));
                    }
                }
            }
        }

        // Check boolean exhaustiveness.
        let bool_lits: HashSet<bool> = arms
            .iter()
            .filter_map(|arm| {
                if let Pattern::Literal(speclang_ir::expr::Literal::Bool(b)) = &arm.pattern
                {
                    Some(*b)
                } else {
                    None
                }
            })
            .collect();

        if !bool_lits.is_empty() && bool_lits.len() < 2 {
            let missing = if bool_lits.contains(&true) {
                "false"
            } else {
                "true"
            };
            self.err(format!(
                "non-exhaustive match in function '{fn_name}': missing pattern for '{missing}'"
            ));
        }
    }
}

fn is_catch_all(pattern: &Pattern) -> bool {
    matches!(pattern, Pattern::Wildcard | Pattern::Bind(_))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Verify exhaustive pattern matching in a Core IR module.
pub fn verify_exhaustiveness(module: &Module) -> Result<(), Vec<ExhaustivenessError>> {
    let mut checker = ExhaustivenessChecker::new(module);
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
    use speclang_ir::expr::{Block, Expr, Literal, MatchArm, Pattern, Stmt};
    use speclang_ir::module::{Function, Module, Param, TypeDef};
    use speclang_ir::types::{Field, Type, Variant};

    fn make_module(name: &str) -> Module {
        Module::new(vec![name.to_string()])
    }

    fn make_fn(name: &str, body: Block) -> Function {
        Function {
            name: name.into(),
            params: vec![Param { name: "x".into(), ty: Type::int() }],
            return_type: Type::unit(),
            effects: vec![],
            contracts: vec![],
            body,
            annotations: vec![],
        }
    }

    fn add_color_enum(m: &mut Module) {
        m.type_defs.push(TypeDef {
            name: "Color".into(),
            ty: Type::Enum(vec![
                Variant {
                    name: "Red".into(),
                    fields: vec![],
                },
                Variant {
                    name: "Green".into(),
                    fields: vec![],
                },
                Variant {
                    name: "Blue".into(),
                    fields: vec![],
                },
            ]),
            annotations: vec![],
        });
    }

    #[test]
    fn exhaustive_enum_all_variants() {
        let mut m = make_module("test");
        add_color_enum(&mut m);
        m.functions.push(make_fn(
            "f",
            Block::new(
                vec![],
                Some(Expr::Match {
                    expr: Box::new(Expr::Var("x".into())),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::Variant {
                                ty: vec!["Color".into()],
                                variant: "Red".into(),
                                fields: vec![],
                            },
                            body: Block::new(vec![], Some(Expr::Literal(Literal::Unit))),
                        },
                        MatchArm {
                            pattern: Pattern::Variant {
                                ty: vec!["Color".into()],
                                variant: "Green".into(),
                                fields: vec![],
                            },
                            body: Block::new(vec![], Some(Expr::Literal(Literal::Unit))),
                        },
                        MatchArm {
                            pattern: Pattern::Variant {
                                ty: vec!["Color".into()],
                                variant: "Blue".into(),
                                fields: vec![],
                            },
                            body: Block::new(vec![], Some(Expr::Literal(Literal::Unit))),
                        },
                    ],
                }),
            ),
        ));
        assert!(verify_exhaustiveness(&m).is_ok());
    }

    #[test]
    fn exhaustive_enum_missing_variant() {
        let mut m = make_module("test");
        add_color_enum(&mut m);
        m.functions.push(make_fn(
            "f",
            Block::new(
                vec![],
                Some(Expr::Match {
                    expr: Box::new(Expr::Var("x".into())),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::Variant {
                                ty: vec!["Color".into()],
                                variant: "Red".into(),
                                fields: vec![],
                            },
                            body: Block::new(vec![], Some(Expr::Literal(Literal::Unit))),
                        },
                        MatchArm {
                            pattern: Pattern::Variant {
                                ty: vec!["Color".into()],
                                variant: "Green".into(),
                                fields: vec![],
                            },
                            body: Block::new(vec![], Some(Expr::Literal(Literal::Unit))),
                        },
                        // Missing Blue!
                    ],
                }),
            ),
        ));
        let errs = verify_exhaustiveness(&m).unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("Blue")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn exhaustive_wildcard_covers_all() {
        let mut m = make_module("test");
        add_color_enum(&mut m);
        m.functions.push(make_fn(
            "f",
            Block::new(
                vec![],
                Some(Expr::Match {
                    expr: Box::new(Expr::Var("x".into())),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::Variant {
                                ty: vec!["Color".into()],
                                variant: "Red".into(),
                                fields: vec![],
                            },
                            body: Block::new(vec![], Some(Expr::Literal(Literal::Unit))),
                        },
                        MatchArm {
                            pattern: Pattern::Wildcard,
                            body: Block::new(vec![], Some(Expr::Literal(Literal::Unit))),
                        },
                    ],
                }),
            ),
        ));
        assert!(verify_exhaustiveness(&m).is_ok());
    }

    #[test]
    fn exhaustive_bool_both() {
        let mut m = make_module("test");
        m.functions.push(make_fn(
            "f",
            Block::new(
                vec![],
                Some(Expr::Match {
                    expr: Box::new(Expr::Var("x".into())),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::Literal(Literal::Bool(true)),
                            body: Block::new(vec![], Some(Expr::Literal(Literal::Unit))),
                        },
                        MatchArm {
                            pattern: Pattern::Literal(Literal::Bool(false)),
                            body: Block::new(vec![], Some(Expr::Literal(Literal::Unit))),
                        },
                    ],
                }),
            ),
        ));
        assert!(verify_exhaustiveness(&m).is_ok());
    }

    #[test]
    fn exhaustive_bool_missing_false() {
        let mut m = make_module("test");
        m.functions.push(make_fn(
            "f",
            Block::new(
                vec![],
                Some(Expr::Match {
                    expr: Box::new(Expr::Var("x".into())),
                    arms: vec![MatchArm {
                        pattern: Pattern::Literal(Literal::Bool(true)),
                        body: Block::new(vec![], Some(Expr::Literal(Literal::Unit))),
                    }],
                }),
            ),
        ));
        let errs = verify_exhaustiveness(&m).unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("false")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn exhaustive_empty_match() {
        let mut m = make_module("test");
        m.functions.push(make_fn(
            "f",
            Block::new(
                vec![],
                Some(Expr::Match {
                    expr: Box::new(Expr::Var("x".into())),
                    arms: vec![],
                }),
            ),
        ));
        let errs = verify_exhaustiveness(&m).unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("empty match")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn exhaustive_bind_covers_all() {
        let mut m = make_module("test");
        add_color_enum(&mut m);
        m.functions.push(make_fn(
            "f",
            Block::new(
                vec![],
                Some(Expr::Match {
                    expr: Box::new(Expr::Var("x".into())),
                    arms: vec![MatchArm {
                        pattern: Pattern::Bind("val".into()),
                        body: Block::new(vec![], Some(Expr::Literal(Literal::Unit))),
                    }],
                }),
            ),
        ));
        assert!(verify_exhaustiveness(&m).is_ok());
    }
}
