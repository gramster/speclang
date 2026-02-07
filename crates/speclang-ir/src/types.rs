//! Core IR type system.
//!
//! Covers: primitive scalars, unbounded int, UTF-8 string, bytes,
//! aggregates (struct/enum/tuple), ownership/borrowing types,
//! regions, and refinement types.

use std::fmt;

/// A named identifier.
pub type Ident = String;

/// A qualified name (e.g., `std.core.Option`).
pub type QName = Vec<Ident>;

/// Format a qualified name as a dotted string.
pub fn qname_to_string(qname: &QName) -> String {
    qname.join(".")
}

/// Parse a dotted string into a qualified name.
pub fn qname_from_string(s: &str) -> QName {
    s.split('.').map(|s| s.to_string()).collect()
}

// ---------------------------------------------------------------------------
// Primitive types
// ---------------------------------------------------------------------------

/// Primitive scalar types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    /// Boolean (i1).
    Bool,
    /// Unsigned integers.
    U8,
    U16,
    U32,
    U64,
    U128,
    /// Signed integers.
    I8,
    I16,
    I32,
    I64,
    I128,
    /// Floating point.
    F32,
    F64,
    /// Unit type (zero-sized).
    Unit,
    /// Arbitrary-precision signed integer (mathematical integer).
    /// Never overflows; exact arithmetic.
    Int,
    /// UTF-8 string (invariant: valid UTF-8).
    String,
    /// Raw byte sequence (no encoding invariant).
    Bytes,
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimitiveType::Bool => write!(f, "bool"),
            PrimitiveType::U8 => write!(f, "u8"),
            PrimitiveType::U16 => write!(f, "u16"),
            PrimitiveType::U32 => write!(f, "u32"),
            PrimitiveType::U64 => write!(f, "u64"),
            PrimitiveType::U128 => write!(f, "u128"),
            PrimitiveType::I8 => write!(f, "i8"),
            PrimitiveType::I16 => write!(f, "i16"),
            PrimitiveType::I32 => write!(f, "i32"),
            PrimitiveType::I64 => write!(f, "i64"),
            PrimitiveType::I128 => write!(f, "i128"),
            PrimitiveType::F32 => write!(f, "f32"),
            PrimitiveType::F64 => write!(f, "f64"),
            PrimitiveType::Unit => write!(f, "unit"),
            PrimitiveType::Int => write!(f, "int"),
            PrimitiveType::String => write!(f, "string"),
            PrimitiveType::Bytes => write!(f, "bytes"),
        }
    }
}

// ---------------------------------------------------------------------------
// Regions
// ---------------------------------------------------------------------------

/// A region (arena/bump/pool allocator context).
///
/// Regions are referenced by a token value; they are allocator contexts,
/// not address spaces.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Region {
    /// The default heap region.
    Heap,
    /// A named region (arena/bump/pool).
    Named(Ident),
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Region::Heap => write!(f, "heap"),
            Region::Named(name) => write!(f, "{name}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Struct and enum definitions
// ---------------------------------------------------------------------------

/// A named field in a struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: Ident,
    pub ty: Type,
}

/// A variant in a tagged union (enum).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub name: Ident,
    /// Payload types (may be empty for unit variants).
    pub fields: Vec<Type>,
}

// ---------------------------------------------------------------------------
// Type
// ---------------------------------------------------------------------------

/// Core IR type.
///
/// No implicit casts; all conversions are explicit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// Primitive scalar type.
    Primitive(PrimitiveType),

    /// Struct type: `struct { f1: T1, f2: T2, ... }`.
    Struct(Vec<Field>),

    /// Enum (tagged union): `enum { V1(T...), V2(T...), ... }`.
    Enum(Vec<Variant>),

    /// Tuple: `(T1, T2, ...)`.
    Tuple(Vec<Type>),

    /// Owning pointer to `T` allocated in region `R`: `own[R, T]`.
    Own {
        region: Region,
        inner: Box<Type>,
    },

    /// Immutable borrow (non-owning): `ref[T]`.
    Ref(Box<Type>),

    /// Mutable borrow (non-owning): `mutref[T]`.
    MutRef(Box<Type>),

    /// Immutable view (ptr, len): `slice[T]`.
    Slice(Box<Type>),

    /// Mutable view (ptr, len): `mutslice[T]`.
    MutSlice(Box<Type>),

    /// Named type reference (refers to a type defined elsewhere).
    Named(QName),

    /// Generic type application: `Name[T1, T2, ...]`.
    Generic {
        name: QName,
        args: Vec<Type>,
    },

    /// Option type: `Option[T]`.
    Option(Box<Type>),

    /// Result type: `Result[T, E]`.
    Result {
        ok: Box<Type>,
        err: Box<Type>,
    },

    /// A capability type reference (e.g., `cap.Net`).
    Capability(Ident),

    /// Region token type.
    Region,
}

impl Type {
    // Convenience constructors

    pub fn bool() -> Self {
        Type::Primitive(PrimitiveType::Bool)
    }

    pub fn i32() -> Self {
        Type::Primitive(PrimitiveType::I32)
    }

    pub fn i64() -> Self {
        Type::Primitive(PrimitiveType::I64)
    }

    pub fn u64() -> Self {
        Type::Primitive(PrimitiveType::U64)
    }

    pub fn int() -> Self {
        Type::Primitive(PrimitiveType::Int)
    }

    pub fn string() -> Self {
        Type::Primitive(PrimitiveType::String)
    }

    pub fn unit() -> Self {
        Type::Primitive(PrimitiveType::Unit)
    }

    pub fn own(region: Region, inner: Type) -> Self {
        Type::Own {
            region,
            inner: Box::new(inner),
        }
    }

    pub fn borrow(inner: Type) -> Self {
        Type::Ref(Box::new(inner))
    }

    pub fn borrow_mut(inner: Type) -> Self {
        Type::MutRef(Box::new(inner))
    }

    pub fn slice(inner: Type) -> Self {
        Type::Slice(Box::new(inner))
    }

    pub fn mut_slice(inner: Type) -> Self {
        Type::MutSlice(Box::new(inner))
    }

    pub fn option(inner: Type) -> Self {
        Type::Option(Box::new(inner))
    }

    pub fn result(ok: Type, err: Type) -> Self {
        Type::Result {
            ok: Box::new(ok),
            err: Box::new(err),
        }
    }

    pub fn named(name: &str) -> Self {
        Type::Named(qname_from_string(name))
    }
}
