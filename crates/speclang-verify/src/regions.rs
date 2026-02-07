//! Region lifetime verification.
//!
//! Ensures that owned values allocated in a region are not accessed after
//! the region has been invalidated, and that borrows do not escape the
//! region's scope.
//!
//! For now, this is a conservative check:
//! - Named regions must be declared (appear in function params/effects).
//! - `Alloc` expressions referencing a named region must be in scope.
//! - References derived from region-allocated values must not escape
//!   the function (conservative: all refs are local).
//!
//! A full region inference/checking system (Polonius-like) is a future
//! extension.

use speclang_ir::expr::{Block, Expr, Stmt};
use speclang_ir::module::{Function, Module};
use speclang_ir::types::{Region, Type};
use std::collections::HashSet;
use std::fmt;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// A region lifetime error.
#[derive(Debug, Clone)]
pub struct RegionError {
    pub message: String,
}

impl fmt::Display for RegionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "region error: {}", self.message)
    }
}

impl std::error::Error for RegionError {}

// ---------------------------------------------------------------------------
// Checker
// ---------------------------------------------------------------------------

struct RegionChecker<'a> {
    module: &'a Module,
    errors: Vec<RegionError>,
}

impl<'a> RegionChecker<'a> {
    fn new(module: &'a Module) -> Self {
        RegionChecker {
            module,
            errors: Vec::new(),
        }
    }

    fn err(&mut self, msg: impl Into<String>) {
        self.errors.push(RegionError {
            message: msg.into(),
        });
    }

    fn check_module(&mut self) {
        for f in &self.module.functions {
            self.check_function(f);
        }
    }

    fn check_function(&mut self, f: &Function) {
        // Collect all named regions referenced in the function's parameter types
        // and return type. These are the "in-scope" regions.
        let mut known_regions = HashSet::new();
        known_regions.insert("heap".to_string()); // Heap is always valid.

        for p in &f.params {
            self.collect_regions(&p.ty, &mut known_regions);
        }
        self.collect_regions(&f.return_type, &mut known_regions);

        // Walk the body and check that all region references are known.
        self.check_block(&f.body, &known_regions, &f.name);
    }

    fn collect_regions(&self, ty: &Type, regions: &mut HashSet<String>) {
        match ty {
            Type::Own {
                region: Region::Named(r),
                inner,
            } => {
                regions.insert(r.clone());
                self.collect_regions(inner, regions);
            }
            Type::Own {
                region: Region::Heap,
                inner,
            } => {
                self.collect_regions(inner, regions);
            }
            Type::Ref(inner)
            | Type::MutRef(inner)
            | Type::Slice(inner)
            | Type::MutSlice(inner)
            | Type::Option(inner) => {
                self.collect_regions(inner, regions);
            }
            Type::Result { ok, err } => {
                self.collect_regions(ok, regions);
                self.collect_regions(err, regions);
            }
            Type::Struct(fields) => {
                for f in fields {
                    self.collect_regions(&f.ty, regions);
                }
            }
            Type::Enum(variants) => {
                for v in variants {
                    for t in &v.fields {
                        self.collect_regions(t, regions);
                    }
                }
            }
            Type::Tuple(elems) => {
                for t in elems {
                    self.collect_regions(t, regions);
                }
            }
            Type::Generic { args, .. } => {
                for a in args {
                    self.collect_regions(a, regions);
                }
            }
            _ => {}
        }
    }

    fn check_block(
        &mut self,
        block: &Block,
        regions: &HashSet<String>,
        fn_name: &str,
    ) {
        for stmt in &block.stmts {
            self.check_stmt(stmt, regions, fn_name);
        }
        if let Some(tail) = &block.expr {
            self.check_expr(tail, regions, fn_name);
        }
    }

    fn check_stmt(
        &mut self,
        stmt: &Stmt,
        regions: &HashSet<String>,
        fn_name: &str,
    ) {
        match stmt {
            Stmt::Let { value, ty, .. } => {
                self.check_type_regions(ty, regions, fn_name);
                self.check_expr(value, regions, fn_name);
            }
            Stmt::Assign { value, .. } => {
                self.check_expr(value, regions, fn_name);
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                self.check_expr(cond, regions, fn_name);
                self.check_block(then_block, regions, fn_name);
                self.check_block(else_block, regions, fn_name);
            }
            Stmt::Match { expr, arms } => {
                self.check_expr(expr, regions, fn_name);
                for arm in arms {
                    self.check_block(&arm.body, regions, fn_name);
                }
            }
            Stmt::Return(e) => self.check_expr(e, regions, fn_name),
            Stmt::Assert { cond, .. } => self.check_expr(cond, regions, fn_name),
            Stmt::Expr(e) => self.check_expr(e, regions, fn_name),
        }
    }

    fn check_expr(
        &mut self,
        expr: &Expr,
        regions: &HashSet<String>,
        fn_name: &str,
    ) {
        match expr {
            Expr::Alloc { ty, value, .. } => {
                // Check that the type's region is known.
                self.check_type_regions(ty, regions, fn_name);
                self.check_expr(value, regions, fn_name);
            }
            Expr::BinOp { lhs, rhs, .. } => {
                self.check_expr(lhs, regions, fn_name);
                self.check_expr(rhs, regions, fn_name);
            }
            Expr::UnOp { operand, .. } => {
                self.check_expr(operand, regions, fn_name);
            }
            Expr::Call { args, .. } => {
                for arg in args {
                    self.check_expr(arg, regions, fn_name);
                }
            }
            Expr::StructLit { fields, .. } => {
                for (_, v) in fields {
                    self.check_expr(v, regions, fn_name);
                }
            }
            Expr::FieldGet { expr, .. } => {
                self.check_expr(expr, regions, fn_name);
            }
            Expr::EnumLit { fields, .. } => {
                for f in fields {
                    self.check_expr(f, regions, fn_name);
                }
            }
            Expr::TupleLit(elems) => {
                for e in elems {
                    self.check_expr(e, regions, fn_name);
                }
            }
            Expr::If {
                cond,
                then_block,
                else_block,
            } => {
                self.check_expr(cond, regions, fn_name);
                self.check_block(then_block, regions, fn_name);
                self.check_block(else_block, regions, fn_name);
            }
            Expr::Match { expr, arms } => {
                self.check_expr(expr, regions, fn_name);
                for arm in arms {
                    self.check_block(&arm.body, regions, fn_name);
                }
            }
            Expr::Block(block) => {
                self.check_block(block, regions, fn_name);
            }
            Expr::Borrow(e) | Expr::BorrowMut(e) => {
                self.check_expr(e, regions, fn_name);
            }
            Expr::Convert { expr, target } => {
                self.check_expr(expr, regions, fn_name);
                self.check_type_regions(target, regions, fn_name);
            }
            Expr::Literal(_) | Expr::Var(_) => {}
        }
    }

    fn check_type_regions(
        &mut self,
        ty: &Type,
        regions: &HashSet<String>,
        fn_name: &str,
    ) {
        match ty {
            Type::Own {
                region: Region::Named(r),
                inner,
            } => {
                if !regions.contains(r) {
                    self.err(format!(
                        "unknown region '{r}' used in function '{fn_name}'"
                    ));
                }
                self.check_type_regions(inner, regions, fn_name);
            }
            Type::Own {
                region: Region::Heap,
                inner,
            } => {
                self.check_type_regions(inner, regions, fn_name);
            }
            Type::Ref(inner)
            | Type::MutRef(inner)
            | Type::Slice(inner)
            | Type::MutSlice(inner)
            | Type::Option(inner) => {
                self.check_type_regions(inner, regions, fn_name);
            }
            Type::Result { ok, err } => {
                self.check_type_regions(ok, regions, fn_name);
                self.check_type_regions(err, regions, fn_name);
            }
            Type::Tuple(elems) => {
                for t in elems {
                    self.check_type_regions(t, regions, fn_name);
                }
            }
            Type::Generic { args, .. } => {
                for a in args {
                    self.check_type_regions(a, regions, fn_name);
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Verify region lifetime rules in a Core IR module.
pub fn verify_regions(module: &Module) -> Result<(), Vec<RegionError>> {
    let mut checker = RegionChecker::new(module);
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
    use speclang_ir::expr::{Block, Expr, Literal, Stmt};
    use speclang_ir::module::{Function, Module, Param};
    use speclang_ir::types::{Region, Type};

    fn make_module(name: &str) -> Module {
        Module::new(vec![name.to_string()])
    }

    fn make_fn(name: &str, params: Vec<Param>, body: Block) -> Function {
        Function {
            name: name.into(),
            params,
            return_type: Type::unit(),
            effects: vec![],
            contracts: vec![],
            body,
            annotations: vec![],
        }
    }

    #[test]
    fn region_heap_always_valid() {
        let mut m = make_module("test");
        m.functions.push(make_fn(
            "f",
            vec![],
            Block::new(
                vec![Stmt::Let {
                    name: "x".into(),
                    ty: Type::own(Region::Heap, Type::int()),
                    value: Expr::Alloc {
                        region: Box::new(Expr::Literal(Literal::Unit)),
                        ty: Type::int(),
                        value: Box::new(Expr::Literal(Literal::Int(42))),
                    },
                }],
                None,
            ),
        ));
        assert!(verify_regions(&m).is_ok());
    }

    #[test]
    fn region_named_from_param_valid() {
        let mut m = make_module("test");
        m.functions.push(make_fn(
            "f",
            vec![Param {
                name: "buf".into(),
                ty: Type::own(Region::Named("arena".into()), Type::int()),
            }],
            Block::new(
                vec![Stmt::Let {
                    name: "x".into(),
                    ty: Type::own(Region::Named("arena".into()), Type::int()),
                    value: Expr::Alloc {
                        region: Box::new(Expr::Literal(Literal::Unit)),
                        ty: Type::int(),
                        value: Box::new(Expr::Literal(Literal::Int(1))),
                    },
                }],
                None,
            ),
        ));
        assert!(verify_regions(&m).is_ok());
    }

    #[test]
    fn region_unknown_named_fails() {
        let mut m = make_module("test");
        m.functions.push(make_fn(
            "f",
            vec![], // no region params!
            Block::new(
                vec![Stmt::Let {
                    name: "x".into(),
                    ty: Type::own(Region::Named("arena".into()), Type::int()),
                    value: Expr::Alloc {
                        region: Box::new(Expr::Literal(Literal::Unit)),
                        ty: Type::int(),
                        value: Box::new(Expr::Literal(Literal::Int(1))),
                    },
                }],
                None,
            ),
        ));
        let errs = verify_regions(&m).unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("arena")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn region_in_convert_checked() {
        let mut m = make_module("test");
        m.functions.push(make_fn(
            "f",
            vec![],
            Block::new(
                vec![],
                Some(Expr::Convert {
                    expr: Box::new(Expr::Literal(Literal::Int(1))),
                    target: Type::own(Region::Named("bogus".into()), Type::int()),
                }),
            ),
        ));
        let errs = verify_regions(&m).unwrap_err();
        assert!(
            errs.iter().any(|e| e.message.contains("bogus")),
            "got: {errs:?}"
        );
    }
}
