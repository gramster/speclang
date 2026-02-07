//! SPL (Spec Layer) AST definitions.
//!
//! Represents the parsed structure of `.spl` files.

/// A qualified identifier (e.g., `music.scale`).
pub type QualifiedName = Vec<String>;

/// An SPL program (contents of one `.spl` file).
#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<ModuleItem>,
}

/// Top-level items in an SPL file.
#[derive(Debug, Clone)]
pub enum ModuleItem {
    Module(ModuleDecl),
    Import(ImportDecl),
    Capability(CapabilityDecl),
    Type(TypeDecl),
    Error(ErrorDecl),
    FnSpec(FnSpecDecl),
    Law(LawDecl),
}

/// Module declaration: `module music.scale;`
#[derive(Debug, Clone)]
pub struct ModuleDecl {
    pub name: QualifiedName,
}

/// Import declaration: `import std.core as core;`
#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub name: QualifiedName,
    pub alias: Option<String>,
}

/// Capability declaration: `capability Net(host: Host);`
#[derive(Debug, Clone)]
pub struct CapabilityDecl {
    pub name: String,
    pub params: Vec<CapParam>,
}

#[derive(Debug, Clone)]
pub struct CapParam {
    pub name: String,
    pub ty: TypeRef,
}

/// Type declaration (alias, struct, or enum).
#[derive(Debug, Clone)]
pub struct TypeDecl {
    pub name: String,
    pub body: TypeBody,
}

#[derive(Debug, Clone)]
pub enum TypeBody {
    Alias {
        ty: TypeRef,
        refine: Option<RefineExpr>,
    },
    Struct {
        fields: Vec<FieldDecl>,
        invariant: Option<Vec<RefineExpr>>,
    },
    Enum {
        variants: Vec<VariantDecl>,
    },
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub name: String,
    pub ty: TypeRef,
}

#[derive(Debug, Clone)]
pub struct VariantDecl {
    pub name: String,
    pub fields: Vec<TypeRef>,
}

/// A type reference in SPL.
#[derive(Debug, Clone)]
pub struct TypeRef {
    pub name: QualifiedName,
    pub args: Vec<TypeRef>,
    pub nullable: bool,
}

/// Error domain declaration.
#[derive(Debug, Clone)]
pub struct ErrorDecl {
    pub name: String,
    pub variants: Vec<ErrorVariant>,
}

#[derive(Debug, Clone)]
pub struct ErrorVariant {
    pub name: String,
    pub message: String,
}

/// Function spec declaration.
#[derive(Debug, Clone)]
pub struct FnSpecDecl {
    pub name: String,
    pub stable_id: String,
    pub compat: Option<CompatKind>,
    pub params: Vec<Param>,
    pub return_type: TypeRef,
    pub blocks: Vec<FnBlock>,
}

#[derive(Debug, Clone)]
pub enum CompatKind {
    StableCall,
    StableSemantics,
    Unstable,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeRef,
}

/// Blocks within a function spec.
#[derive(Debug, Clone)]
pub enum FnBlock {
    Requires(Vec<RefineExpr>),
    Ensures(Vec<RefineExpr>),
    Effects(Vec<EffectItem>),
    Raises(Vec<RaisesItem>),
    Perf(Vec<PerfItem>),
    Examples(Vec<ExampleItem>),
    Notes(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct EffectItem {
    pub name: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RaisesItem {
    pub error: QualifiedName,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PerfItem {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct ExampleItem {
    pub label: String,
    pub lhs: SplExpr,
    pub rhs: SplExpr,
}

/// Law/property declaration.
#[derive(Debug, Clone)]
pub struct LawDecl {
    pub name: String,
    pub expr: RefineExpr,
}

// ---------------------------------------------------------------------------
// Refinement expressions
// ---------------------------------------------------------------------------

/// Refinement/predicate expression (used in invariants, requires, ensures).
#[derive(Debug, Clone)]
pub enum RefineExpr {
    And(Box<RefineExpr>, Box<RefineExpr>),
    Or(Box<RefineExpr>, Box<RefineExpr>),
    Not(Box<RefineExpr>),
    Compare {
        lhs: Box<RefineAtom>,
        op: CompareOp,
        rhs: Box<RefineAtom>,
    },
    Atom(RefineAtom),
}

#[derive(Debug, Clone)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone)]
pub enum RefineAtom {
    SelfRef,
    Ident(String),
    IntLit(i64),
    StringLit(String),
    Call(String, Vec<RefineAtom>),
}

/// A simple expression in SPL (for examples).
#[derive(Debug, Clone)]
pub enum SplExpr {
    IntLit(i64),
    StringLit(String),
    Ident(String),
    Call(String, Vec<SplExpr>),
    SetLit(Vec<SplExpr>),
}
