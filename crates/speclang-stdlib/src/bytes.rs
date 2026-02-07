//! `std.bytes` — Byte sequences and byte slice operations.
//!
//! `Bytes` is the primitive byte sequence type (raw, no encoding invariant).
//! This module provides length, comparison, and slice conversion helpers.

use speclang_ir::module::{ExternFunction, Module, Param};
use speclang_ir::types::{PrimitiveType, QName, Type};

fn qname(s: &str) -> QName {
    s.split('.').map(|p| p.to_string()).collect()
}

fn param(name: &str, ty: Type) -> Param {
    Param {
        name: name.to_string(),
        ty,
    }
}

fn bytes() -> Type {
    Type::Primitive(PrimitiveType::Bytes)
}

fn u8_ty() -> Type {
    Type::Primitive(PrimitiveType::U8)
}

/// Build the `std.bytes` module.
pub fn module() -> Module {
    let mut m = Module::new(qname("std.bytes"));

    // bytes.len(b: ref[bytes]) -> int
    m.externs.push(ExternFunction {
        name: "bytes.len".to_string(),
        params: vec![param("b", Type::borrow(bytes()))],
        return_type: Type::int(),
        effects: vec![],
        annotations: vec![],
    });

    // bytes.as_slice(b: ref[bytes]) -> slice[u8]
    m.externs.push(ExternFunction {
        name: "bytes.as_slice".to_string(),
        params: vec![param("b", Type::borrow(bytes()))],
        return_type: Type::slice(u8_ty()),
        effects: vec![],
        annotations: vec![],
    });

    // bytes.from_slice(s: slice[u8]) -> bytes
    m.externs.push(ExternFunction {
        name: "bytes.from_slice".to_string(),
        params: vec![param("s", Type::slice(u8_ty()))],
        return_type: bytes(),
        effects: vec![],
        annotations: vec![],
    });

    // bytes.eq(a: ref[bytes], b: ref[bytes]) -> bool
    m.externs.push(ExternFunction {
        name: "bytes.eq".to_string(),
        params: vec![
            param("a", Type::borrow(bytes())),
            param("b", Type::borrow(bytes())),
        ],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    // bytes.cmp(a: ref[bytes], b: ref[bytes]) -> Ordering
    m.externs.push(ExternFunction {
        name: "bytes.cmp".to_string(),
        params: vec![
            param("a", Type::borrow(bytes())),
            param("b", Type::borrow(bytes())),
        ],
        return_type: Type::Named(qname("std.core.Ordering")),
        effects: vec![],
        annotations: vec![],
    });

    // bytes.get(b: ref[bytes], idx: int) -> u8  (traps on OOB)
    m.externs.push(ExternFunction {
        name: "bytes.get".to_string(),
        params: vec![
            param("b", Type::borrow(bytes())),
            param("idx", Type::int()),
        ],
        return_type: u8_ty(),
        effects: vec![],
        annotations: vec![],
    });

    // bytes.concat(a: ref[bytes], b: ref[bytes]) -> bytes
    m.externs.push(ExternFunction {
        name: "bytes.concat".to_string(),
        params: vec![
            param("a", Type::borrow(bytes())),
            param("b", Type::borrow(bytes())),
        ],
        return_type: bytes(),
        effects: vec![],
        annotations: vec![],
    });

    // bytes.is_empty(b: ref[bytes]) -> bool
    m.externs.push(ExternFunction {
        name: "bytes.is_empty".to_string(),
        params: vec![param("b", Type::borrow(bytes()))],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_module_name() {
        let m = module();
        assert_eq!(m.name, vec!["std", "bytes"]);
    }

    #[test]
    fn bytes_has_len_and_slice() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"bytes.len"));
        assert!(names.contains(&"bytes.as_slice"));
        assert!(names.contains(&"bytes.from_slice"));
    }

    #[test]
    fn bytes_has_comparison() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"bytes.eq"));
        assert!(names.contains(&"bytes.cmp"));
    }

    #[test]
    fn bytes_has_access() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"bytes.get"));
        assert!(names.contains(&"bytes.concat"));
        assert!(names.contains(&"bytes.is_empty"));
    }
}
