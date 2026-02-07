//! IMPL-to-Core IR lowering.
//!
//! Converts a parsed, verified IMPL program into a Core IR `Module`.
//! This is the final compilation step before backend code generation.

use crate::ast::*;
use speclang_ir::capability::CapabilityType;
use speclang_ir::expr::{
    BinOp, Block, Expr, Literal, MatchArm, Pattern, Stmt, UnOp,
};
use speclang_ir::module::{
    Annotation, Function, Module, Param as IrParam,
};
use speclang_ir::types::{
    PrimitiveType, QName, Region, Type,
};
use std::fmt;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// A lowering error.
#[derive(Debug, Clone)]
pub struct LowerError {
    pub message: String,
}

impl fmt::Display for LowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "impl lower error: {}", self.message)
    }
}

impl std::error::Error for LowerError {}

// ---------------------------------------------------------------------------
// Lowering context
// ---------------------------------------------------------------------------

struct LowerCtx {
    module: Module,
    errors: Vec<LowerError>,
}

impl LowerCtx {
    fn new(name: QName) -> Self {
        LowerCtx {
            module: Module::new(name),
            errors: Vec::new(),
        }
    }

    fn err(&mut self, msg: impl Into<String>) {
        self.errors.push(LowerError {
            message: msg.into(),
        });
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Lower an IMPL program to a Core IR module.
pub fn lower_impl(program: &ImplProgram) -> Result<Module, Vec<LowerError>> {
    // Extract module name from the program
    let module_name = program
        .items
        .iter()
        .find_map(|item| match item {
            ImplItem::Module(m) => Some(m.name.clone()),
            _ => None,
        })
        .unwrap_or_else(|| vec!["main".to_string()]);

    let mut ctx = LowerCtx::new(module_name);

    for item in &program.items {
        match item {
            ImplItem::Module(_) | ImplItem::Import(_) => {
                // Module and import declarations are handled structurally
            }
            ImplItem::Function(f) => {
                lower_function(&mut ctx, f);
            }
        }
    }

    if ctx.errors.is_empty() {
        Ok(ctx.module)
    } else {
        Err(ctx.errors)
    }
}

// ---------------------------------------------------------------------------
// Function lowering
// ---------------------------------------------------------------------------

fn lower_function(ctx: &mut LowerCtx, f: &ImplFunction) {
    let params: Vec<IrParam> = f
        .params
        .iter()
        .map(|p| IrParam {
            name: p.name.clone(),
            ty: lower_type(&p.ty),
        })
        .collect();

    let return_type = lower_type(&f.return_type);

    // Extract capability effects from params
    let effects: Vec<CapabilityType> = f
        .params
        .iter()
        .filter(|p| p.is_cap)
        .filter_map(|p| {
            if let ImplTypeRef::Capability(name) = &p.ty {
                Some(CapabilityType {
                    name: name.clone(),
                })
            } else {
                None
            }
        })
        .collect();

    let body = lower_block(&f.body, ctx);

    let annotations = vec![Annotation::Id(f.stable_id.clone())];

    let function = Function {
        name: f.name.clone(),
        params,
        return_type,
        effects,
        contracts: vec![], // Contracts come from SPL, not IMPL
        body,
        annotations,
    };

    ctx.module.functions.push(function);
}

// ---------------------------------------------------------------------------
// Type lowering
// ---------------------------------------------------------------------------

fn lower_type(ty: &ImplTypeRef) -> Type {
    match ty {
        ImplTypeRef::Named(name) => match name.as_str() {
            "bool" => Type::Primitive(PrimitiveType::Bool),
            "i8" => Type::Primitive(PrimitiveType::I8),
            "i16" => Type::Primitive(PrimitiveType::I16),
            "i32" => Type::Primitive(PrimitiveType::I32),
            "i64" => Type::Primitive(PrimitiveType::I64),
            "i128" => Type::Primitive(PrimitiveType::I128),
            "u8" => Type::Primitive(PrimitiveType::U8),
            "u16" => Type::Primitive(PrimitiveType::U16),
            "u32" => Type::Primitive(PrimitiveType::U32),
            "u64" => Type::Primitive(PrimitiveType::U64),
            "u128" => Type::Primitive(PrimitiveType::U128),
            "f32" => Type::Primitive(PrimitiveType::F32),
            "f64" => Type::Primitive(PrimitiveType::F64),
            "int" => Type::Primitive(PrimitiveType::Int),
            "string" => Type::Primitive(PrimitiveType::String),
            "bytes" => Type::Primitive(PrimitiveType::Bytes),
            "unit" => Type::Primitive(PrimitiveType::Unit),
            other => Type::Named(vec![other.to_string()]),
        },
        ImplTypeRef::Qualified(parts) => Type::Named(parts.clone()),
        ImplTypeRef::Own { region, inner } => {
            let r = if region == "heap" {
                Region::Heap
            } else {
                Region::Named(region.clone())
            };
            Type::Own {
                region: r,
                inner: Box::new(lower_type(inner)),
            }
        }
        ImplTypeRef::Ref(inner) => Type::Ref(Box::new(lower_type(inner))),
        ImplTypeRef::MutRef(inner) => Type::MutRef(Box::new(lower_type(inner))),
        ImplTypeRef::Slice(inner) => Type::Slice(Box::new(lower_type(inner))),
        ImplTypeRef::MutSlice(inner) => Type::MutSlice(Box::new(lower_type(inner))),
        ImplTypeRef::Tuple(items) => {
            Type::Tuple(items.iter().map(|t| lower_type(t)).collect())
        }
        ImplTypeRef::Generic { name, args } => Type::Generic {
            name: name.clone(),
            args: args.iter().map(|t| lower_type(t)).collect(),
        },
        ImplTypeRef::Option(inner) => Type::Option(Box::new(lower_type(inner))),
        ImplTypeRef::Result { ok, err } => Type::Result {
            ok: Box::new(lower_type(ok)),
            err: Box::new(lower_type(err)),
        },
        ImplTypeRef::Capability(name) => Type::Capability(name.clone()),
        ImplTypeRef::Region => Type::Region,
    }
}

// ---------------------------------------------------------------------------
// Block lowering
// ---------------------------------------------------------------------------

fn lower_block(block: &ImplBlock, ctx: &mut LowerCtx) -> Block {
    let stmts: Vec<Stmt> = block
        .stmts
        .iter()
        .map(|s| lower_stmt(s, ctx))
        .collect();
    let expr = block.expr.as_ref().map(|e| lower_expr(e, ctx));
    Block::new(stmts, expr)
}

// ---------------------------------------------------------------------------
// Statement lowering
// ---------------------------------------------------------------------------

fn lower_stmt(stmt: &ImplStmt, ctx: &mut LowerCtx) -> Stmt {
    match stmt {
        ImplStmt::Let { name, ty, value } => {
            let ir_ty = ty
                .as_ref()
                .map(|t| lower_type(t))
                .unwrap_or(Type::Primitive(PrimitiveType::Unit));
            Stmt::Let {
                name: name.clone(),
                ty: ir_ty,
                value: lower_expr(value, ctx),
            }
        }
        ImplStmt::LetMut { name, ty, value } => {
            // Core IR doesn't distinguish let vs let mut at IR level;
            // mutability is tracked by the ownership/borrow checker
            let ir_ty = ty
                .as_ref()
                .map(|t| lower_type(t))
                .unwrap_or(Type::Primitive(PrimitiveType::Unit));
            Stmt::Let {
                name: name.clone(),
                ty: ir_ty,
                value: lower_expr(value, ctx),
            }
        }
        ImplStmt::Assign { target, value } => Stmt::Assign {
            target: target.clone(),
            value: lower_expr(value, ctx),
        },
        ImplStmt::If {
            cond,
            then_block,
            else_block,
        } => Stmt::If {
            cond: lower_expr(cond, ctx),
            then_block: lower_block(then_block, ctx),
            else_block: else_block
                .as_ref()
                .map(|b| lower_block(b, ctx))
                .unwrap_or_else(Block::empty),
        },
        ImplStmt::Match { expr, arms } => Stmt::Match {
            expr: lower_expr(expr, ctx),
            arms: arms.iter().map(|a| lower_match_arm(a, ctx)).collect(),
        },
        ImplStmt::Return(expr) => {
            let value = expr
                .as_ref()
                .map(|e| lower_expr(e, ctx))
                .unwrap_or(Expr::Literal(Literal::Unit));
            Stmt::Return(value)
        }
        ImplStmt::Assert { cond, message } => Stmt::Assert {
            cond: lower_expr(cond, ctx),
            message: message.clone().unwrap_or_else(|| "assertion failed".to_string()),
        },
        ImplStmt::While { cond, body } => {
            // Lower while to: loop { if !cond { break; } body }
            let break_stmt = Stmt::If {
                cond: Expr::UnOp {
                    op: UnOp::Not,
                    operand: Box::new(lower_expr(cond, ctx)),
                },
                then_block: Block::new(
                    vec![Stmt::Return(Expr::Literal(Literal::Unit))],
                    None,
                ),
                else_block: Block::empty(),
            };
            let mut loop_stmts = vec![break_stmt];
            let body_block = lower_block(body, ctx);
            loop_stmts.extend(body_block.stmts);
            let loop_body = Block::new(loop_stmts, body_block.expr.map(|e| *e));
            // Represent as a match on true with loop body
            // For now, emit as an if-based loop pattern
            Stmt::If {
                cond: Expr::Literal(Literal::Bool(true)),
                then_block: loop_body,
                else_block: Block::empty(),
            }
        }
        ImplStmt::Loop(body) => {
            // Loop → same pattern
            Stmt::If {
                cond: Expr::Literal(Literal::Bool(true)),
                then_block: lower_block(body, ctx),
                else_block: Block::empty(),
            }
        }
        ImplStmt::Break => {
            // Break/Continue need loop IR support; for now map to return unit
            Stmt::Return(Expr::Literal(Literal::Unit))
        }
        ImplStmt::Continue => {
            Stmt::Return(Expr::Literal(Literal::Unit))
        }
        ImplStmt::Expr(expr) => Stmt::Expr(lower_expr(expr, ctx)),
    }
}

// ---------------------------------------------------------------------------
// Expression lowering
// ---------------------------------------------------------------------------

fn lower_expr(expr: &ImplExpr, ctx: &mut LowerCtx) -> Expr {
    match expr {
        ImplExpr::Literal(lit) => Expr::Literal(lower_literal(lit)),
        ImplExpr::Var(name) => Expr::Var(name.clone()),
        ImplExpr::BinOp { op, lhs, rhs } => Expr::BinOp {
            op: lower_binop(*op),
            lhs: Box::new(lower_expr(lhs, ctx)),
            rhs: Box::new(lower_expr(rhs, ctx)),
        },
        ImplExpr::UnOp { op, operand } => Expr::UnOp {
            op: lower_unop(*op),
            operand: Box::new(lower_expr(operand, ctx)),
        },
        ImplExpr::Call { func, args } => Expr::Call {
            func: func.clone(),
            args: args.iter().map(|a| lower_expr(a, ctx)).collect(),
        },
        ImplExpr::StructLit { ty, fields } => Expr::StructLit {
            ty: ty.clone(),
            fields: fields
                .iter()
                .map(|(name, value)| (name.clone(), lower_expr(value, ctx)))
                .collect(),
        },
        ImplExpr::FieldGet { expr, field } => Expr::FieldGet {
            expr: Box::new(lower_expr(expr, ctx)),
            field: field.clone(),
        },
        ImplExpr::EnumLit { ty, variant, args } => Expr::EnumLit {
            ty: ty.clone(),
            variant: variant.clone(),
            fields: args.iter().map(|a| lower_expr(a, ctx)).collect(),
        },
        ImplExpr::TupleLit(items) => {
            Expr::TupleLit(items.iter().map(|i| lower_expr(i, ctx)).collect())
        }
        ImplExpr::If {
            cond,
            then_block,
            else_block,
        } => Expr::If {
            cond: Box::new(lower_expr(cond, ctx)),
            then_block: lower_block(then_block, ctx),
            else_block: else_block
                .as_ref()
                .map(|b| lower_block(b, ctx))
                .unwrap_or_else(Block::empty),
        },
        ImplExpr::Match { expr, arms } => Expr::Match {
            expr: Box::new(lower_expr(expr, ctx)),
            arms: arms.iter().map(|a| lower_match_arm(a, ctx)).collect(),
        },
        ImplExpr::Block(block) => Expr::Block(lower_block(block, ctx)),
        ImplExpr::Alloc { region, value } => Expr::Alloc {
            region: Box::new(lower_expr(region, ctx)),
            ty: Type::Primitive(PrimitiveType::Unit), // Type inference fills this in
            value: Box::new(lower_expr(value, ctx)),
        },
        ImplExpr::Borrow(expr) => Expr::Borrow(Box::new(lower_expr(expr, ctx))),
        ImplExpr::BorrowMut(expr) => Expr::BorrowMut(Box::new(lower_expr(expr, ctx))),
        ImplExpr::Convert { expr, target } => Expr::Convert {
            expr: Box::new(lower_expr(expr, ctx)),
            target: lower_type(target),
        },
        ImplExpr::Loop(body) => {
            // Lower loop to block (simplified — full loop IR not in Core IR yet)
            Expr::Block(lower_block(body, ctx))
        }
        ImplExpr::While { cond, body } => {
            // Simplified — lower to block with conditional
            Expr::If {
                cond: Box::new(lower_expr(cond, ctx)),
                then_block: lower_block(body, ctx),
                else_block: Block::empty(),
            }
        }
        ImplExpr::Break | ImplExpr::Continue => {
            // Placeholder — Core IR needs loop support for proper lowering
            Expr::Literal(Literal::Unit)
        }
        ImplExpr::Return(expr) => {
            // Return in expression position — lower the value
            expr.as_ref()
                .map(|e| lower_expr(e, ctx))
                .unwrap_or(Expr::Literal(Literal::Unit))
        }
    }
}

// ---------------------------------------------------------------------------
// Match arm lowering
// ---------------------------------------------------------------------------

fn lower_match_arm(arm: &ImplMatchArm, ctx: &mut LowerCtx) -> MatchArm {
    MatchArm {
        pattern: lower_pattern(&arm.pattern),
        body: lower_block(&arm.body, ctx),
    }
}

fn lower_pattern(pat: &ImplPattern) -> Pattern {
    match pat {
        ImplPattern::Wildcard => Pattern::Wildcard,
        ImplPattern::Bind(name) => Pattern::Bind(name.clone()),
        ImplPattern::Literal(lit) => Pattern::Literal(lower_literal(lit)),
        ImplPattern::Variant {
            ty,
            variant,
            fields,
        } => Pattern::Variant {
            ty: ty.clone(),
            variant: variant.clone(),
            fields: fields.iter().map(|p| lower_pattern(p)).collect(),
        },
        ImplPattern::Tuple(items) => {
            Pattern::Tuple(items.iter().map(|p| lower_pattern(p)).collect())
        }
        ImplPattern::Struct { ty, fields } => Pattern::Struct {
            ty: ty.clone(),
            fields: fields
                .iter()
                .map(|(name, pat)| (name.clone(), lower_pattern(pat)))
                .collect(),
        },
    }
}

// ---------------------------------------------------------------------------
// Literal / operator lowering
// ---------------------------------------------------------------------------

fn lower_literal(lit: &ImplLiteral) -> Literal {
    match lit {
        ImplLiteral::Bool(b) => Literal::Bool(*b),
        ImplLiteral::Int(n) => Literal::Int(*n),
        ImplLiteral::Float(f) => Literal::F64(*f),
        ImplLiteral::String(s) => Literal::String(s.clone()),
        ImplLiteral::Unit => Literal::Unit,
    }
}

fn lower_binop(op: ImplBinOp) -> BinOp {
    match op {
        ImplBinOp::Add => BinOp::Add,
        ImplBinOp::Sub => BinOp::Sub,
        ImplBinOp::Mul => BinOp::Mul,
        ImplBinOp::Div => BinOp::Div,
        ImplBinOp::Mod => BinOp::Mod,
        ImplBinOp::BitAnd => BinOp::BitAnd,
        ImplBinOp::BitOr => BinOp::BitOr,
        ImplBinOp::BitXor => BinOp::BitXor,
        ImplBinOp::Shl => BinOp::Shl,
        ImplBinOp::Shr => BinOp::Shr,
        ImplBinOp::Eq => BinOp::Eq,
        ImplBinOp::Ne => BinOp::Ne,
        ImplBinOp::Lt => BinOp::Lt,
        ImplBinOp::Le => BinOp::Le,
        ImplBinOp::Gt => BinOp::Gt,
        ImplBinOp::Ge => BinOp::Ge,
        ImplBinOp::And => BinOp::And,
        ImplBinOp::Or => BinOp::Or,
    }
}

fn lower_unop(op: ImplUnOp) -> UnOp {
    match op {
        ImplUnOp::Neg => UnOp::Neg,
        ImplUnOp::Not => UnOp::Not,
        ImplUnOp::BitNot => UnOp::BitNot,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_impl;

    fn lower(src: &str) -> Module {
        let prog = parse_impl(src).unwrap();
        lower_impl(&prog).unwrap()
    }

    #[test]
    fn test_lower_simple_function() {
        let module = lower(r#"
            impl fn "test.add.v1" add(a: i32, b: i32) -> i32 {
                a + b
            }
        "#);
        assert_eq!(module.functions.len(), 1);
        let f = &module.functions[0];
        assert_eq!(f.name, "add");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.return_type, Type::Primitive(PrimitiveType::I32));
        assert_eq!(f.stable_id(), Some("test.add.v1"));
    }

    #[test]
    fn test_lower_module_name() {
        let module = lower(r#"
            module music.scale;
            impl fn "test.v1" foo() -> unit { () }
        "#);
        assert_eq!(module.name, vec!["music", "scale"]);
    }

    #[test]
    fn test_lower_default_module_name() {
        let module = lower(r#"
            impl fn "test.v1" foo() -> unit { () }
        "#);
        assert_eq!(module.name, vec!["main"]);
    }

    #[test]
    fn test_lower_capability_effects() {
        let module = lower(r#"
            impl fn "test.v1" fetch(url: string, net: cap Net) -> string {
                url
            }
        "#);
        let f = &module.functions[0];
        assert_eq!(f.effects.len(), 1);
        assert_eq!(f.effects[0].name, "Net");
    }

    #[test]
    fn test_lower_ownership_types() {
        let module = lower(r#"
            impl fn "test.v1" boxed(x: own[heap, i32]) -> ref[i32] {
                borrow(x)
            }
        "#);
        let f = &module.functions[0];
        assert!(matches!(&f.params[0].ty, Type::Own { .. }));
        assert!(matches!(&f.return_type, Type::Ref(_)));
    }

    #[test]
    fn test_lower_let_and_assign() {
        let module = lower(r#"
            impl fn "test.v1" foo() -> i32 {
                let mut x: i32 = 0;
                x = 42;
                x
            }
        "#);
        let f = &module.functions[0];
        assert_eq!(f.body.stmts.len(), 2);
        assert!(matches!(&f.body.stmts[0], Stmt::Let { .. }));
        assert!(matches!(&f.body.stmts[1], Stmt::Assign { .. }));
        assert!(f.body.expr.is_some());
    }

    #[test]
    fn test_lower_if_else() {
        let module = lower(r#"
            impl fn "test.v1" abs(x: i32) -> i32 {
                if x >= 0 { x } else { -x }
            }
        "#);
        let f = &module.functions[0];
        assert!(f.body.expr.is_some());
        assert!(matches!(f.body.expr.as_deref(), Some(Expr::If { .. })));
    }

    #[test]
    fn test_lower_match() {
        let module = lower(r#"
            impl fn "test.v1" classify(x: i32) -> string {
                match x {
                    0 => "zero",
                    _ => "other",
                }
            }
        "#);
        let f = &module.functions[0];
        assert!(f.body.expr.is_some());
        if let Some(Expr::Match { arms, .. }) = f.body.expr.as_deref() {
            assert_eq!(arms.len(), 2);
        } else {
            panic!("expected match expression");
        }
    }

    #[test]
    fn test_lower_struct_literal() {
        let module = lower(r#"
            impl fn "test.v1" make() -> Point {
                Point { x: 1, y: 2 }
            }
        "#);
        let f = &module.functions[0];
        assert!(f.body.expr.is_some());
        assert!(matches!(
            f.body.expr.as_deref(),
            Some(Expr::StructLit { .. })
        ));
    }

    #[test]
    fn test_lower_alloc_borrow() {
        let module = lower(r#"
            impl fn "test.v1" make_box(r: region) -> own[heap, i32] {
                let b: own[heap, i32] = alloc(r, 42);
                b
            }
        "#);
        let f = &module.functions[0];
        if let Stmt::Let { value, .. } = &f.body.stmts[0] {
            assert!(matches!(value, Expr::Alloc { .. }));
        } else {
            panic!("expected let with alloc");
        }
    }

    #[test]
    fn test_lower_type_convert() {
        let module = lower(r#"
            impl fn "test.v1" widen(x: i32) -> i64 {
                x as i64
            }
        "#);
        let f = &module.functions[0];
        assert!(matches!(
            f.body.expr.as_deref(),
            Some(Expr::Convert { .. })
        ));
    }

    #[test]
    fn test_lower_assert() {
        let module = lower(r#"
            impl fn "test.v1" checked(x: i32) -> i32 {
                assert(x > 0, "must be positive");
                x
            }
        "#);
        let f = &module.functions[0];
        assert!(matches!(&f.body.stmts[0], Stmt::Assert { .. }));
    }

    #[test]
    fn test_lower_enum_construction() {
        let module = lower(r#"
            impl fn "test.v1" wrap(x: i32) -> Option[i32] {
                Option.Some(x)
            }
        "#);
        let f = &module.functions[0];
        assert!(matches!(
            f.body.expr.as_deref(),
            Some(Expr::EnumLit { .. })
        ));
    }

    #[test]
    fn test_lower_complex_function() {
        let module = lower(r#"
            module test;
            impl fn "test.snap.v1" snap(note: i32, scale: ref[Set[i32]]) -> i32 {
                let mut best: i32 = -1;
                let mut best_dist: i32 = 13;
                let mut i: i32 = 0;
                while i < 12 {
                    i = i + 1;
                }
                best
            }
        "#);
        assert_eq!(module.name, vec!["test"]);
        assert_eq!(module.functions.len(), 1);
        let f = &module.functions[0];
        assert_eq!(f.stable_id(), Some("test.snap.v1"));
    }
}
