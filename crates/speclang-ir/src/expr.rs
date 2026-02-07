//! Core IR expressions, statements, and control flow.
//!
//! Core IR is SSA-friendly:
//! - `let` introduces a new SSA binding
//! - Mutation only through `mutref`/`mutslice` stores
//! - No implicit casts; all conversions explicit
//! - Integer overflow traps for fixed-width types

use crate::types::{Ident, QName, Type};

// ---------------------------------------------------------------------------
// Literals
// ---------------------------------------------------------------------------

/// Literal values.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// Boolean literal.
    Bool(bool),
    /// Fixed-width integer literal (value + type for width).
    Int(i128),
    /// Unbounded integer literal.
    BigInt(String),
    /// 32-bit float.
    F32(f32),
    /// 64-bit float.
    F64(f64),
    /// String literal (UTF-8).
    String(String),
    /// Byte string literal.
    Bytes(Vec<u8>),
    /// Unit value.
    Unit,
}

// ---------------------------------------------------------------------------
// Binary and unary operators
// ---------------------------------------------------------------------------

/// Binary operators.
///
/// Arithmetic ops on fixed-width integers trap on overflow.
/// Division/modulo trap on division by zero.
/// Shift amounts outside `[0, bitwidth)` trap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    // Arithmetic (trap on overflow for fixed-width)
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,

    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    // Logical (bool only)
    And,
    Or,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    /// Arithmetic negation (traps on overflow for fixed-width).
    Neg,
    /// Logical not.
    Not,
    /// Bitwise not.
    BitNot,
}

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

/// Core IR expression.
///
/// All expressions are typed. The type is carried separately in typed IR
/// passes, not embedded in every node.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Literal value.
    Literal(Literal),

    /// Local variable / SSA value reference.
    Var(Ident),

    /// Binary operation.
    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },

    /// Unary operation.
    UnOp {
        op: UnOp,
        operand: Box<Expr>,
    },

    /// Function call: `call f(args...)`.
    Call {
        func: QName,
        args: Vec<Expr>,
    },

    /// Struct literal construction.
    StructLit {
        ty: QName,
        fields: Vec<(Ident, Expr)>,
    },

    /// Field access on a struct value.
    FieldGet {
        expr: Box<Expr>,
        field: Ident,
    },

    /// Enum variant construction.
    EnumLit {
        ty: QName,
        variant: Ident,
        fields: Vec<Expr>,
    },

    /// Tuple construction.
    TupleLit(Vec<Expr>),

    /// If-else expression.
    If {
        cond: Box<Expr>,
        then_block: Block,
        else_block: Block,
    },

    /// Pattern match expression (must be exhaustive).
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
    },

    /// Block expression (evaluates to the last expression).
    Block(Block),

    /// Allocation: `alloc(region, T, value) -> own[R, T]`.
    Alloc {
        region: Box<Expr>,
        ty: Type,
        value: Box<Expr>,
    },

    /// Borrow: `borrow(own) -> ref[T]`.
    Borrow(Box<Expr>),

    /// Mutable borrow: `borrow_mut(own) -> mutref[T]`.
    BorrowMut(Box<Expr>),

    /// Explicit type conversion (may trap if out of range).
    /// e.g., `as_i32(x)`, `int_to_i64(x)`.
    Convert {
        expr: Box<Expr>,
        target: Type,
    },
}

// ---------------------------------------------------------------------------
// Patterns
// ---------------------------------------------------------------------------

/// Pattern for match arms.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Wildcard: `_`.
    Wildcard,

    /// Bind to a variable name.
    Bind(Ident),

    /// Literal pattern.
    Literal(Literal),

    /// Enum variant pattern: `Variant(p1, p2, ...)`.
    Variant {
        ty: QName,
        variant: Ident,
        fields: Vec<Pattern>,
    },

    /// Tuple pattern: `(p1, p2, ...)`.
    Tuple(Vec<Pattern>),

    /// Struct pattern: `{ f1: p1, f2: p2, ... }`.
    Struct {
        ty: QName,
        fields: Vec<(Ident, Pattern)>,
    },
}

/// A match arm: pattern => block.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Block,
}

// ---------------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------------

/// Core IR statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Let binding (SSA): `let x: T = expr;`.
    Let {
        name: Ident,
        ty: Type,
        value: Expr,
    },

    /// Assignment through mutable reference: `*x = expr;`.
    Assign {
        target: Ident,
        value: Expr,
    },

    /// If statement (no value).
    If {
        cond: Expr,
        then_block: Block,
        else_block: Block,
    },

    /// Match statement (no value).
    Match {
        expr: Expr,
        arms: Vec<MatchArm>,
    },

    /// Return statement: `return expr;`.
    Return(Expr),

    /// Assertion: `assert(cond, "message");`.
    ///
    /// If false: trap with message (backend-defined reporting).
    /// Available in pure code (assert has no external effects).
    Assert {
        cond: Expr,
        message: String,
    },

    /// Expression statement (for side effects).
    Expr(Expr),
}

// ---------------------------------------------------------------------------
// Block
// ---------------------------------------------------------------------------

/// A block of statements, optionally evaluating to a trailing expression.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    /// Optional trailing expression (the block's value).
    pub expr: Option<Box<Expr>>,
}

impl Block {
    pub fn new(stmts: Vec<Stmt>, expr: Option<Expr>) -> Self {
        Block {
            stmts,
            expr: expr.map(Box::new),
        }
    }

    pub fn empty() -> Self {
        Block {
            stmts: vec![],
            expr: None,
        }
    }
}
