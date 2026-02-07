//! IMPL (Implementation Layer) AST definitions.
//!
//! Represents the parsed structure of `.impl` files.
//! IMPL is a systems language with ownership, regions, effects,
//! and explicit memory management that binds to SPL specs via stable IDs.

/// A qualified identifier (e.g., `std.collections`).
pub type QualifiedName = Vec<String>;

/// An IMPL program (contents of one `.impl` file).
#[derive(Debug, Clone)]
pub struct ImplProgram {
    pub items: Vec<ImplItem>,
}

/// Top-level items in an `.impl` file.
#[derive(Debug, Clone)]
pub enum ImplItem {
    /// Module declaration: `module music.scale;`
    Module(ModuleDecl),
    /// Import: `import std.core;`
    Import(ImportDecl),
    /// Function implementation bound to an SPL spec.
    Function(ImplFunction),
}

/// Module declaration.
#[derive(Debug, Clone)]
pub struct ModuleDecl {
    pub name: QualifiedName,
}

/// Import declaration.
#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub name: QualifiedName,
    pub alias: Option<String>,
}

/// A function implementation bound to an SPL stable ID.
///
/// ```text
/// impl fn "music.snap.v1" snap_to_scale(
///     note: I32,
///     scale: ref[Set[I32]],
///     cap_net: cap Net,
/// ) -> I32 {
///     ...
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ImplFunction {
    /// The SPL stable ID this implementation binds to.
    pub stable_id: String,
    /// Local function name.
    pub name: String,
    /// Parameters (including capability parameters).
    pub params: Vec<ImplParam>,
    /// Return type.
    pub return_type: ImplTypeRef,
    /// Function body.
    pub body: ImplBlock,
}

/// A function parameter.
#[derive(Debug, Clone)]
pub struct ImplParam {
    pub name: String,
    pub ty: ImplTypeRef,
    /// Whether this is a capability parameter (`cap Net`).
    pub is_cap: bool,
}

/// A type reference in IMPL.
#[derive(Debug, Clone, PartialEq)]
pub enum ImplTypeRef {
    /// Primitive: `bool`, `i32`, `u64`, `int`, `string`, `bytes`, `unit`.
    Named(String),
    /// Qualified name: `std.core.Option`.
    Qualified(QualifiedName),
    /// Owning pointer: `own[R, T]`.
    Own {
        region: String,
        inner: Box<ImplTypeRef>,
    },
    /// Immutable borrow: `ref[T]`.
    Ref(Box<ImplTypeRef>),
    /// Mutable borrow: `mutref[T]`.
    MutRef(Box<ImplTypeRef>),
    /// Immutable slice: `slice[T]`.
    Slice(Box<ImplTypeRef>),
    /// Mutable slice: `mutslice[T]`.
    MutSlice(Box<ImplTypeRef>),
    /// Tuple: `(T1, T2, ...)`.
    Tuple(Vec<ImplTypeRef>),
    /// Generic application: `Set[T]`, `Option[T]`.
    Generic {
        name: QualifiedName,
        args: Vec<ImplTypeRef>,
    },
    /// Option type shorthand: `T?`.
    Option(Box<ImplTypeRef>),
    /// Result type: `Result[T, E]`.
    Result {
        ok: Box<ImplTypeRef>,
        err: Box<ImplTypeRef>,
    },
    /// Capability type: `cap Net`.
    Capability(String),
    /// Region token type.
    Region,
}

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

/// IMPL expression.
#[derive(Debug, Clone)]
pub enum ImplExpr {
    /// Literal value.
    Literal(ImplLiteral),
    /// Variable reference.
    Var(String),
    /// Binary operation: `a + b`, `a == b`, etc.
    BinOp {
        op: ImplBinOp,
        lhs: Box<ImplExpr>,
        rhs: Box<ImplExpr>,
    },
    /// Unary operation: `-x`, `!x`, `~x`.
    UnOp {
        op: ImplUnOp,
        operand: Box<ImplExpr>,
    },
    /// Function call: `f(args...)`.
    Call {
        func: QualifiedName,
        args: Vec<ImplExpr>,
    },
    /// Struct literal: `Point { x: 1, y: 2 }`.
    StructLit {
        ty: QualifiedName,
        fields: Vec<(String, ImplExpr)>,
    },
    /// Field access: `expr.field`.
    FieldGet {
        expr: Box<ImplExpr>,
        field: String,
    },
    /// Enum variant construction: `Option.Some(value)`.
    EnumLit {
        ty: QualifiedName,
        variant: String,
        args: Vec<ImplExpr>,
    },
    /// Tuple construction: `(a, b, c)`.
    TupleLit(Vec<ImplExpr>),
    /// If-else expression: `if cond { ... } else { ... }`.
    If {
        cond: Box<ImplExpr>,
        then_block: ImplBlock,
        else_block: Option<ImplBlock>,
    },
    /// Match expression: `match expr { ... }`.
    Match {
        expr: Box<ImplExpr>,
        arms: Vec<ImplMatchArm>,
    },
    /// Block expression: `{ ... }`.
    Block(ImplBlock),
    /// Allocation: `alloc(region, value)`.
    Alloc {
        region: Box<ImplExpr>,
        value: Box<ImplExpr>,
    },
    /// Borrow: `borrow(expr)`.
    Borrow(Box<ImplExpr>),
    /// Mutable borrow: `borrow_mut(expr)`.
    BorrowMut(Box<ImplExpr>),
    /// Explicit type conversion: `expr as T`.
    Convert {
        expr: Box<ImplExpr>,
        target: ImplTypeRef,
    },
    /// Loop: `loop { ... }`.
    Loop(ImplBlock),
    /// While loop: `while cond { ... }`.
    While {
        cond: Box<ImplExpr>,
        body: ImplBlock,
    },
    /// Break out of loop.
    Break,
    /// Continue the loop.
    Continue,
    /// Return expression: `return expr`.
    Return(Option<Box<ImplExpr>>),
}

/// IMPL literal values.
#[derive(Debug, Clone)]
pub enum ImplLiteral {
    Bool(bool),
    Int(i128),
    Float(f64),
    String(String),
    Unit,
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImplBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImplUnOp {
    Neg,
    Not,
    BitNot,
}

// ---------------------------------------------------------------------------
// Patterns
// ---------------------------------------------------------------------------

/// Pattern for match arms.
#[derive(Debug, Clone)]
pub enum ImplPattern {
    /// Wildcard: `_`.
    Wildcard,
    /// Bind to a variable: `x`.
    Bind(String),
    /// Literal pattern.
    Literal(ImplLiteral),
    /// Enum variant: `Option.Some(x)`.
    Variant {
        ty: QualifiedName,
        variant: String,
        fields: Vec<ImplPattern>,
    },
    /// Tuple pattern: `(a, b)`.
    Tuple(Vec<ImplPattern>),
    /// Struct pattern: `Point { x, y }`.
    Struct {
        ty: QualifiedName,
        fields: Vec<(String, ImplPattern)>,
    },
}

/// A match arm: `pattern => block`.
#[derive(Debug, Clone)]
pub struct ImplMatchArm {
    pub pattern: ImplPattern,
    pub body: ImplBlock,
}

// ---------------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------------

/// IMPL statement.
#[derive(Debug, Clone)]
pub enum ImplStmt {
    /// Let binding: `let x: T = expr;`.
    Let {
        name: String,
        ty: Option<ImplTypeRef>,
        value: ImplExpr,
    },
    /// Mutable let: `let mut x: T = expr;`.
    LetMut {
        name: String,
        ty: Option<ImplTypeRef>,
        value: ImplExpr,
    },
    /// Assignment: `x = expr;`.
    Assign {
        target: String,
        value: ImplExpr,
    },
    /// If statement.
    If {
        cond: ImplExpr,
        then_block: ImplBlock,
        else_block: Option<ImplBlock>,
    },
    /// Match statement.
    Match {
        expr: ImplExpr,
        arms: Vec<ImplMatchArm>,
    },
    /// Return statement.
    Return(Option<ImplExpr>),
    /// Assert: `assert(cond, "message");`.
    Assert {
        cond: ImplExpr,
        message: Option<String>,
    },
    /// While loop.
    While {
        cond: ImplExpr,
        body: ImplBlock,
    },
    /// Loop.
    Loop(ImplBlock),
    /// Break.
    Break,
    /// Continue.
    Continue,
    /// Expression statement.
    Expr(ImplExpr),
}

// ---------------------------------------------------------------------------
// Block
// ---------------------------------------------------------------------------

/// A block of statements with an optional trailing expression.
#[derive(Debug, Clone)]
pub struct ImplBlock {
    pub stmts: Vec<ImplStmt>,
    /// Optional trailing expression (the block's value).
    pub expr: Option<Box<ImplExpr>>,
}

impl ImplBlock {
    pub fn new(stmts: Vec<ImplStmt>, expr: Option<ImplExpr>) -> Self {
        ImplBlock {
            stmts,
            expr: expr.map(Box::new),
        }
    }

    pub fn empty() -> Self {
        ImplBlock {
            stmts: vec![],
            expr: None,
        }
    }
}
