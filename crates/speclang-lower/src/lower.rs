//! SPL-to-Core IR lowering implementation.
//!
//! Converts a resolved, type-checked SPL `Program` into a Core IR `Module`.
//!
//! ## Lowering rules
//!
//! | SPL construct       | Core IR output                                     |
//! |---------------------|----------------------------------------------------|
//! | `type X = T`        | `TypeDef { name: X, ty: lower(T) }`               |
//! | `type X = T refine` | `TypeDef` + constructor fn with assert              |
//! | `type X struct`     | `TypeDef { ..Struct }` + invariant check fn         |
//! | `type X enum`       | `TypeDef { ..Enum }`                               |
//! | `capability C`      | `CapabilityDef { name: C, fields }`                |
//! | `error E`           | `TypeDef { ..Enum }` with string payloads          |
//! | `fn F`              | `Function` with contracts from requires/ensures     |
//! | `req R`             | Stored as metadata in contract `req_tags`          |
//! | `decision D`        | Compile-time marker (validated, no IR output)       |
//! | `gen G`             | Test-time metadata (no runtime IR output)           |
//! | `prop P`            | Generated test function                             |
//! | `oracle O`          | Annotation on the function definition               |
//! | `policy`            | Module-level metadata (no IR output)                |

use speclang_ir::capability::{CapabilityDef, CapabilityField, CapabilityType};
use speclang_ir::contract::{Contract, ContractKind, ContractPolicy};
use speclang_ir::expr::{BinOp, Block, Expr, Literal, Stmt, UnOp};
use speclang_ir::module::{
    Annotation, Compat, Function, Module, Param as IrParam, TypeDef,
};
use speclang_ir::types::{
    Field, PrimitiveType, QName, Type, Variant,
};
use speclang_spl::ast::*;
use speclang_spl::resolve::ResolvedProgram;
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
        write!(f, "lower error: {}", self.message)
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

    // -----------------------------------------------------------------------
    // Type lowering
    // -----------------------------------------------------------------------

    /// Lower an SPL `TypeRef` to a Core IR `Type`.
    fn lower_type_ref(&mut self, tr: &TypeRef) -> Type {
        let name = tr.name.join(".");

        let base = match name.as_str() {
            "Int" => Type::Primitive(PrimitiveType::Int),
            "Bool" => Type::Primitive(PrimitiveType::Bool),
            "String" => Type::Primitive(PrimitiveType::String),
            "Bytes" => Type::Primitive(PrimitiveType::Bytes),
            "Unit" => Type::Primitive(PrimitiveType::Unit),
            "U8" => Type::Primitive(PrimitiveType::U8),
            "U16" => Type::Primitive(PrimitiveType::U16),
            "U32" => Type::Primitive(PrimitiveType::U32),
            "U64" => Type::Primitive(PrimitiveType::U64),
            "U128" => Type::Primitive(PrimitiveType::U128),
            "I8" => Type::Primitive(PrimitiveType::I8),
            "I16" => Type::Primitive(PrimitiveType::I16),
            "I32" => Type::Primitive(PrimitiveType::I32),
            "I64" => Type::Primitive(PrimitiveType::I64),
            "I128" => Type::Primitive(PrimitiveType::I128),
            "F32" => Type::Primitive(PrimitiveType::F32),
            "F64" => Type::Primitive(PrimitiveType::F64),
            "Option" => {
                if tr.args.len() == 1 {
                    Type::Option(Box::new(self.lower_type_ref(&tr.args[0])))
                } else {
                    self.err(format!("Option expects 1 type argument, got {}", tr.args.len()));
                    Type::Primitive(PrimitiveType::Unit)
                }
            }
            "Result" => {
                if tr.args.len() == 2 {
                    Type::Result {
                        ok: Box::new(self.lower_type_ref(&tr.args[0])),
                        err: Box::new(self.lower_type_ref(&tr.args[1])),
                    }
                } else {
                    self.err(format!("Result expects 2 type arguments, got {}", tr.args.len()));
                    Type::Primitive(PrimitiveType::Unit)
                }
            }
            "Set" | "List" | "Map" => {
                let args: Vec<Type> = tr.args.iter().map(|a| self.lower_type_ref(a)).collect();
                Type::Generic {
                    name: vec![name],
                    args,
                }
            }
            _ => {
                if tr.args.is_empty() {
                    Type::Named(tr.name.clone())
                } else {
                    let args: Vec<Type> =
                        tr.args.iter().map(|a| self.lower_type_ref(a)).collect();
                    Type::Generic {
                        name: tr.name.clone(),
                        args,
                    }
                }
            }
        };

        if tr.nullable {
            Type::Option(Box::new(base))
        } else {
            base
        }
    }

    // -----------------------------------------------------------------------
    // Item lowering
    // -----------------------------------------------------------------------

    fn lower_program(&mut self, program: &Program) {
        for item in &program.items {
            match item {
                ModuleItem::Module(_) | ModuleItem::Import(_) => {}
                ModuleItem::Capability(c) => self.lower_capability(c),
                ModuleItem::Type(t) => self.lower_type_decl(t),
                ModuleItem::Error(e) => self.lower_error_decl(e),
                ModuleItem::FnSpec(f) => self.lower_fn_spec(f),
                ModuleItem::Law(_l) => {
                    // Laws are compile-time checks; no runtime output.
                }
                ModuleItem::Req(_) => {
                    // Req declarations are metadata; consumed by contracts.
                }
                ModuleItem::Decision(_) => {
                    // Decisions are compile-time resolutions; no IR output.
                }
                ModuleItem::Gen(_) => {
                    // Generators are test-time constructs; no runtime IR.
                }
                ModuleItem::Prop(p) => self.lower_prop(p),
                ModuleItem::Oracle(o) => self.lower_oracle(o),
                ModuleItem::Policy(_) => {
                    // Policy is module-level metadata; no IR output.
                }
            }
        }
    }

    fn lower_capability(&mut self, c: &CapabilityDecl) {
        let fields = c
            .params
            .iter()
            .map(|p| CapabilityField {
                name: p.name.clone(),
                ty: self.lower_type_ref(&p.ty),
            })
            .collect();
        self.module.cap_defs.push(CapabilityDef {
            name: c.name.clone(),
            fields,
        });
    }

    fn lower_type_decl(&mut self, t: &TypeDecl) {
        match &t.body {
            TypeBody::Alias { ty, refine } => {
                let ir_ty = self.lower_type_ref(ty);
                self.module.type_defs.push(TypeDef {
                    name: t.name.clone(),
                    ty: ir_ty.clone(),
                    annotations: vec![],
                });

                // If there's a refinement, generate a constructor function
                // that asserts the predicate.
                if let Some(refine_expr) = refine {
                    let pred = self.lower_refine_expr(refine_expr, "value");
                    let ctor = Function {
                        name: format!("new_{}", t.name),
                        params: vec![IrParam {
                            name: "value".into(),
                            ty: ir_ty.clone(),
                        }],
                        return_type: ir_ty,
                        effects: vec![],
                        contracts: vec![Contract {
                            kind: ContractKind::Requires,
                            predicate: pred,
                            policy: ContractPolicy::Always,
                            req_tags: vec![],
                        }],
                        body: Block::new(vec![], Some(Expr::Var("value".into()))),
                        annotations: vec![],
                    };
                    self.module.functions.push(ctor);
                }
            }
            TypeBody::Struct { fields, invariant } => {
                let ir_fields = fields
                    .iter()
                    .map(|f| Field {
                        name: f.name.clone(),
                        ty: self.lower_type_ref(&f.ty),
                    })
                    .collect();
                self.module.type_defs.push(TypeDef {
                    name: t.name.clone(),
                    ty: Type::Struct(ir_fields),
                    annotations: vec![],
                });

                // Generate invariant check function if present.
                if let Some(invs) = invariant {
                    let mut contracts = Vec::new();
                    for inv in invs {
                        let pred = self.lower_refine_expr(inv, "self");
                        contracts.push(Contract {
                            kind: ContractKind::Invariant,
                            predicate: pred,
                            policy: ContractPolicy::Debug,
                            req_tags: vec![],
                        });
                    }
                    let check_fn = Function {
                        name: format!("check_{}", t.name),
                        params: vec![IrParam {
                            name: "self".into(),
                            ty: Type::Named(vec![t.name.clone()]),
                        }],
                        return_type: Type::Primitive(PrimitiveType::Bool),
                        effects: vec![],
                        contracts,
                        body: Block::new(
                            vec![],
                            Some(Expr::Literal(Literal::Bool(true))),
                        ),
                        annotations: vec![],
                    };
                    self.module.functions.push(check_fn);
                }
            }
            TypeBody::Enum { variants } => {
                let ir_variants = variants
                    .iter()
                    .map(|v| Variant {
                        name: v.name.clone(),
                        fields: v
                            .fields
                            .iter()
                            .map(|f| self.lower_type_ref(f))
                            .collect(),
                    })
                    .collect();
                self.module.type_defs.push(TypeDef {
                    name: t.name.clone(),
                    ty: Type::Enum(ir_variants),
                    annotations: vec![],
                });
            }
        }
    }

    fn lower_error_decl(&mut self, e: &ErrorDecl) {
        // Lower error declarations as enum types with string payloads.
        let variants = e
            .variants
            .iter()
            .map(|v| Variant {
                name: v.name.clone(),
                fields: vec![Type::Primitive(PrimitiveType::String)],
            })
            .collect();
        self.module.type_defs.push(TypeDef {
            name: e.name.clone(),
            ty: Type::Enum(variants),
            annotations: vec![],
        });
    }

    fn lower_fn_spec(&mut self, f: &FnSpecDecl) {
        let params: Vec<IrParam> = f
            .params
            .iter()
            .map(|p| IrParam {
                name: p.name.clone(),
                ty: self.lower_type_ref(&p.ty),
            })
            .collect();

        let return_type = self.lower_type_ref(&f.return_type);

        let mut annotations = vec![Annotation::Id(f.stable_id.clone())];
        if let Some(compat) = &f.compat {
            annotations.push(Annotation::Compat(match compat {
                CompatKind::StableCall => Compat::StableCall,
                CompatKind::StableSemantics => Compat::StableSemantics,
                CompatKind::Unstable => Compat::Unstable,
            }));
        }

        let mut contracts = Vec::new();
        let mut effects = Vec::new();

        for block in &f.blocks {
            match block {
                FnBlock::Requires { req_tags, conditions } => {
                    for cond in conditions {
                        let pred = self.lower_refine_expr(cond, "");
                        contracts.push(Contract {
                            kind: ContractKind::Requires,
                            predicate: pred,
                            policy: ContractPolicy::Debug,
                            req_tags: req_tags.clone(),
                        });
                    }
                }
                FnBlock::Ensures { req_tags, conditions } => {
                    for cond in conditions {
                        let pred = self.lower_refine_expr(cond, "");
                        contracts.push(Contract {
                            kind: ContractKind::Ensures,
                            predicate: pred,
                            policy: ContractPolicy::Debug,
                            req_tags: req_tags.clone(),
                        });
                    }
                }
                FnBlock::Effects(eff_items) => {
                    for eff in eff_items {
                        effects.push(CapabilityType {
                            name: eff.name.clone(),
                        });
                    }
                }
                FnBlock::Raises(_) | FnBlock::Perf(_) | FnBlock::Notes(_) => {}
                FnBlock::Examples { req_tags, items } => {
                    // Generate test functions for examples.
                    for (i, ex) in items.iter().enumerate() {
                        let lhs = self.lower_spl_expr(&ex.lhs);
                        let rhs = self.lower_spl_expr(&ex.rhs);
                        let test_fn = Function {
                            name: format!("test_{}_{i}", f.name),
                            params: vec![],
                            return_type: Type::Primitive(PrimitiveType::Unit),
                            effects: vec![],
                            contracts: vec![],
                            body: Block::new(
                                vec![Stmt::Assert {
                                    cond: Expr::BinOp {
                                        op: BinOp::Eq,
                                        lhs: Box::new(lhs),
                                        rhs: Box::new(rhs),
                                    },
                                    message: ex.label.clone(),
                                }],
                                None,
                            ),
                            annotations: req_tags
                                .iter()
                                .map(|t| Annotation::ReqTag(t.clone()))
                                .collect(),
                        };
                        self.module.functions.push(test_fn);
                    }
                }
            }
        }

        let func = Function {
            name: f.name.clone(),
            params,
            return_type,
            effects,
            contracts,
            // SPL functions don't have implementation bodies —
            // those come from the IMPL layer. We use an empty body.
            body: Block::empty(),
            annotations,
        };
        self.module.functions.push(func);
    }

    fn lower_prop(&mut self, p: &PropDecl) {
        // Generate a test function for the property.
        // The actual property-based test generation would expand
        // the forall quantifiers with generators at test time.
        let pred = self.lower_refine_expr(&p.body, "");

        let params: Vec<IrParam> = p
            .quantifiers
            .iter()
            .map(|q| IrParam {
                name: q.name.clone(),
                ty: self.lower_type_ref(&q.ty),
            })
            .collect();

        let test_fn = Function {
            name: format!("prop_{}", p.name),
            params,
            return_type: Type::Primitive(PrimitiveType::Unit),
            effects: vec![],
            contracts: vec![],
            body: Block::new(
                vec![Stmt::Assert {
                    cond: pred,
                    message: format!("property '{}' violated", p.name),
                }],
                None,
            ),
            annotations: p
                .req_tags
                .iter()
                .map(|t| Annotation::ReqTag(t.clone()))
                .collect(),
        };
        self.module.functions.push(test_fn);
    }

    fn lower_oracle(&mut self, o: &OracleDecl) {
        // Add an annotation to the referenced function.
        // Since the function may already be lowered, we search for it.
        let fn_name = o.name.last().cloned().unwrap_or_default();
        if let Some(func) = self
            .module
            .functions
            .iter_mut()
            .find(|f| f.name == fn_name)
        {
            let kind_str = match o.kind {
                OracleKind::Reference => "reference",
                OracleKind::Optimized => "optimized",
            };
            func.annotations.push(Annotation::Id(format!(
                "oracle:{}",
                kind_str
            )));
        }
    }

    // -----------------------------------------------------------------------
    // Refinement expression lowering
    // -----------------------------------------------------------------------

    /// Lower a refinement expression to a Core IR expression.
    fn lower_refine_expr(&mut self, expr: &RefineExpr, self_name: &str) -> Expr {
        match expr {
            RefineExpr::And(a, b) => Expr::BinOp {
                op: BinOp::And,
                lhs: Box::new(self.lower_refine_expr(a, self_name)),
                rhs: Box::new(self.lower_refine_expr(b, self_name)),
            },
            RefineExpr::Or(a, b) => Expr::BinOp {
                op: BinOp::Or,
                lhs: Box::new(self.lower_refine_expr(a, self_name)),
                rhs: Box::new(self.lower_refine_expr(b, self_name)),
            },
            RefineExpr::Not(e) => Expr::UnOp {
                op: UnOp::Not,
                operand: Box::new(self.lower_refine_expr(e, self_name)),
            },
            RefineExpr::Compare { lhs, op, rhs } => {
                let ir_op = match op {
                    CompareOp::Eq => BinOp::Eq,
                    CompareOp::Ne => BinOp::Ne,
                    CompareOp::Lt => BinOp::Lt,
                    CompareOp::Le => BinOp::Le,
                    CompareOp::Gt => BinOp::Gt,
                    CompareOp::Ge => BinOp::Ge,
                };
                Expr::BinOp {
                    op: ir_op,
                    lhs: Box::new(self.lower_refine_atom(lhs, self_name)),
                    rhs: Box::new(self.lower_refine_atom(rhs, self_name)),
                }
            }
            RefineExpr::Atom(a) => self.lower_refine_atom(a, self_name),
        }
    }

    fn lower_refine_atom(&mut self, atom: &RefineAtom, self_name: &str) -> Expr {
        match atom {
            RefineAtom::SelfRef => {
                if self_name.is_empty() {
                    Expr::Var("self".into())
                } else {
                    Expr::Var(self_name.to_string())
                }
            }
            RefineAtom::Ident(name) => Expr::Var(name.clone()),
            RefineAtom::IntLit(n) => Expr::Literal(Literal::Int(*n as i128)),
            RefineAtom::StringLit(s) => Expr::Literal(Literal::String(s.clone())),
            RefineAtom::Call(name, args) => {
                let ir_args = args
                    .iter()
                    .map(|a| self.lower_refine_atom(a, self_name))
                    .collect();
                Expr::Call {
                    func: vec![name.clone()],
                    args: ir_args,
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // SPL expression lowering (for examples)
    // -----------------------------------------------------------------------

    fn lower_spl_expr(&mut self, expr: &SplExpr) -> Expr {
        match expr {
            SplExpr::IntLit(n) => Expr::Literal(Literal::Int(*n as i128)),
            SplExpr::StringLit(s) => Expr::Literal(Literal::String(s.clone())),
            SplExpr::Ident(name) => Expr::Var(name.clone()),
            SplExpr::Call(name, args) => {
                let ir_args = args.iter().map(|a| self.lower_spl_expr(a)).collect();
                Expr::Call {
                    func: vec![name.clone()],
                    args: ir_args,
                }
            }
            SplExpr::SetLit(elems) => {
                // Lower set literals as a call to a built-in set constructor.
                let ir_elems = elems.iter().map(|e| self.lower_spl_expr(e)).collect();
                Expr::Call {
                    func: vec!["set_of".into()],
                    args: ir_elems,
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Lower a resolved SPL program to a Core IR module.
pub fn lower(resolved: &ResolvedProgram<'_>) -> Result<Module, Vec<LowerError>> {
    let module_name = resolved
        .module_name
        .clone()
        .unwrap_or_else(|| vec!["unnamed".into()]);

    let mut ctx = LowerCtx::new(module_name);
    ctx.lower_program(resolved.program);

    if ctx.errors.is_empty() {
        Ok(ctx.module)
    } else {
        Err(ctx.errors)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use speclang_spl::parser::parse_program;
    use speclang_spl::resolve::resolve;

    fn lower_source(src: &str) -> Module {
        let program = Box::leak(Box::new(parse_program(src).unwrap()));
        let resolved = Box::leak(Box::new(resolve(program).unwrap()));
        lower(resolved).unwrap()
    }

    #[test]
    fn lower_simple_type_alias() {
        let m = lower_source(r#"
module test;
type Foo = Int;
"#);
        assert_eq!(m.type_defs.len(), 1);
        assert_eq!(m.type_defs[0].name, "Foo");
        assert_eq!(m.type_defs[0].ty, Type::Primitive(PrimitiveType::Int));
    }

    #[test]
    fn lower_refine_type_generates_constructor() {
        let m = lower_source(r#"
module test;
type MidiNote = Int refine (1 <= self and self <= 12);
"#);
        assert_eq!(m.type_defs.len(), 1);
        assert_eq!(m.type_defs[0].name, "MidiNote");
        // Should have a `new_MidiNote` constructor function.
        assert!(
            m.functions.iter().any(|f| f.name == "new_MidiNote"),
            "expected constructor function, got: {:?}",
            m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let ctor = m.find_function("new_MidiNote").unwrap();
        assert_eq!(ctor.contracts.len(), 1);
        assert_eq!(ctor.contracts[0].kind, ContractKind::Requires);
    }

    #[test]
    fn lower_struct_type() {
        let m = lower_source(r#"
module test;
type Point struct {
  x: Int;
  y: Int;
};
"#);
        assert_eq!(m.type_defs.len(), 1);
        match &m.type_defs[0].ty {
            Type::Struct(fields) => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].name, "x");
                assert_eq!(fields[1].name, "y");
            }
            other => panic!("expected Struct, got {other:?}"),
        }
    }

    #[test]
    fn lower_enum_type() {
        let m = lower_source(r#"
module test;
type Color enum {
  Red;
  Green;
  Blue;
};
"#);
        assert_eq!(m.type_defs.len(), 1);
        match &m.type_defs[0].ty {
            Type::Enum(variants) => {
                assert_eq!(variants.len(), 3);
                assert_eq!(variants[0].name, "Red");
            }
            other => panic!("expected Enum, got {other:?}"),
        }
    }

    #[test]
    fn lower_error_decl() {
        let m = lower_source(r#"
module test;
error ParseError {
  BadInput: "bad input";
  Overflow: "value overflow";
};
"#);
        assert_eq!(m.type_defs.len(), 1);
        match &m.type_defs[0].ty {
            Type::Enum(variants) => {
                assert_eq!(variants.len(), 2);
                assert_eq!(variants[0].name, "BadInput");
                // Each variant has a string payload.
                assert_eq!(variants[0].fields.len(), 1);
            }
            other => panic!("expected Enum, got {other:?}"),
        }
    }

    #[test]
    fn lower_fn_spec() {
        let m = lower_source(r#"
module test;
type Foo = Int;
fn identity @id("test.identity") @compat(stable_call)
  (x: Foo) -> Foo
{
  ensures { result == x; }
};
"#);
        let func = m.find_function("identity").unwrap();
        assert_eq!(func.params.len(), 1);
        assert_eq!(func.params[0].name, "x");
        assert_eq!(func.contracts.len(), 1);
        assert_eq!(func.contracts[0].kind, ContractKind::Ensures);
        // Check annotations.
        assert!(func
            .annotations
            .iter()
            .any(|a| matches!(a, Annotation::Id(id) if id == "test.identity")));
        assert!(func
            .annotations
            .iter()
            .any(|a| matches!(a, Annotation::Compat(Compat::StableCall))));
    }

    #[test]
    fn lower_fn_with_examples_generates_tests() {
        let m = lower_source(r#"
module test;
type Foo = Int;
fn add @id("test.add") (a: Foo, b: Foo) -> Foo {
  examples {
    "one plus one": add(1, 1) == 2;
    "zero": add(0, 0) == 0;
  }
};
"#);
        // Should have: `add` itself + 2 test functions.
        assert!(m.find_function("add").is_some());
        assert!(m.find_function("test_add_0").is_some());
        assert!(m.find_function("test_add_1").is_some());
    }

    #[test]
    fn lower_capability() {
        let m = lower_source(r#"
module test;
type Host = String;
capability Net(host: Host);
"#);
        assert_eq!(m.cap_defs.len(), 1);
        assert_eq!(m.cap_defs[0].name, "Net");
        assert_eq!(m.cap_defs[0].fields.len(), 1);
    }

    #[test]
    fn lower_fn_with_effects() {
        let m = lower_source(r#"
module test;
capability Net();
type Resp = String;
fn fetch @id("test.fetch") (url: String) -> Resp {
  effects { Net }
};
"#);
        let func = m.find_function("fetch").unwrap();
        assert_eq!(func.effects.len(), 1);
        assert_eq!(func.effects[0].name, "Net");
    }

    #[test]
    fn lower_prop_generates_test() {
        let m = lower_source(r#"
module test;
req REQ-1: "test prop";
type Foo = Int;
fn identity @id("test.id") (x: Foo) -> Foo {};
gen FooGen { range: 1..10; };
prop [REQ-1] id_prop:
  forall x: Foo from FooGen
  identity(x) == x;
"#);
        let prop_fn = m.find_function("prop_id_prop").unwrap();
        assert_eq!(prop_fn.params.len(), 1);
        assert_eq!(prop_fn.params[0].name, "x");
        assert!(prop_fn
            .annotations
            .iter()
            .any(|a| matches!(a, Annotation::ReqTag(t) if t == "REQ-1")));
    }

    #[test]
    fn lower_nullable_type() {
        let m = lower_source(r#"
module test;
type MaybeInt = Int?;
"#);
        assert_eq!(m.type_defs.len(), 1);
        match &m.type_defs[0].ty {
            Type::Option(inner) => {
                assert_eq!(**inner, Type::Primitive(PrimitiveType::Int));
            }
            other => panic!("expected Option, got {other:?}"),
        }
    }

    #[test]
    fn lower_full_example() {
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
        let m = lower_source(src);
        assert_eq!(m.name, vec!["music", "scale"]);
        assert!(m.find_type("MidiNote").is_some());
        assert!(m.find_function("snap_to_scale").is_some());
        assert!(m.find_function("new_MidiNote").is_some());
        assert!(m.find_function("prop_snap_in_scale").is_some());
        assert!(m.find_function("test_snap_to_scale_0").is_some());
        assert!(m.find_function("test_snap_to_scale_1").is_some());
    }
}
