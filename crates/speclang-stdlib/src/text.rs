//! `std.text` — UTF-8 string operations.
//!
//! The `String` primitive is always valid UTF-8. This module provides
//! construction, access, and ASCII utility functions.

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

/// Build the `std.text` module.
pub fn module() -> Module {
    let mut m = Module::new(qname("std.text"));

    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    // text.from_utf8(b: bytes) -> Result[string, string]
    // Returns Err with a diagnostic message if bytes are not valid UTF-8.
    m.externs.push(ExternFunction {
        name: "text.from_utf8".to_string(),
        params: vec![param("b", bytes())],
        return_type: Type::result(Type::string(), Type::string()),
        effects: vec![],
        annotations: vec![],
    });

    // text.from_utf8_unchecked(b: bytes) -> string
    // Caller must guarantee valid UTF-8. UB otherwise.
    m.externs.push(ExternFunction {
        name: "text.from_utf8_unchecked".to_string(),
        params: vec![param("b", bytes())],
        return_type: Type::string(),
        effects: vec![],
        annotations: vec![],
    });

    // -----------------------------------------------------------------------
    // Length and access
    // -----------------------------------------------------------------------

    // text.len_bytes(s: ref[string]) -> int
    m.externs.push(ExternFunction {
        name: "text.len_bytes".to_string(),
        params: vec![param("s", Type::borrow(Type::string()))],
        return_type: Type::int(),
        effects: vec![],
        annotations: vec![],
    });

    // text.is_empty(s: ref[string]) -> bool
    m.externs.push(ExternFunction {
        name: "text.is_empty".to_string(),
        params: vec![param("s", Type::borrow(Type::string()))],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    // text.as_bytes(s: ref[string]) -> slice[u8]
    m.externs.push(ExternFunction {
        name: "text.as_bytes".to_string(),
        params: vec![param("s", Type::borrow(Type::string()))],
        return_type: Type::slice(u8_ty()),
        effects: vec![],
        annotations: vec![],
    });

    // -----------------------------------------------------------------------
    // Comparison
    // -----------------------------------------------------------------------

    // text.eq(a: ref[string], b: ref[string]) -> bool
    m.externs.push(ExternFunction {
        name: "text.eq".to_string(),
        params: vec![
            param("a", Type::borrow(Type::string())),
            param("b", Type::borrow(Type::string())),
        ],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    // text.cmp(a: ref[string], b: ref[string]) -> Ordering
    m.externs.push(ExternFunction {
        name: "text.cmp".to_string(),
        params: vec![
            param("a", Type::borrow(Type::string())),
            param("b", Type::borrow(Type::string())),
        ],
        return_type: Type::Named(qname("std.core.Ordering")),
        effects: vec![],
        annotations: vec![],
    });

    // -----------------------------------------------------------------------
    // Concatenation and manipulation
    // -----------------------------------------------------------------------

    // text.concat(a: ref[string], b: ref[string]) -> string
    m.externs.push(ExternFunction {
        name: "text.concat".to_string(),
        params: vec![
            param("a", Type::borrow(Type::string())),
            param("b", Type::borrow(Type::string())),
        ],
        return_type: Type::string(),
        effects: vec![],
        annotations: vec![],
    });

    // text.contains(haystack: ref[string], needle: ref[string]) -> bool
    m.externs.push(ExternFunction {
        name: "text.contains".to_string(),
        params: vec![
            param("haystack", Type::borrow(Type::string())),
            param("needle", Type::borrow(Type::string())),
        ],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    // text.starts_with(s: ref[string], prefix: ref[string]) -> bool
    m.externs.push(ExternFunction {
        name: "text.starts_with".to_string(),
        params: vec![
            param("s", Type::borrow(Type::string())),
            param("prefix", Type::borrow(Type::string())),
        ],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    // text.ends_with(s: ref[string], suffix: ref[string]) -> bool
    m.externs.push(ExternFunction {
        name: "text.ends_with".to_string(),
        params: vec![
            param("s", Type::borrow(Type::string())),
            param("suffix", Type::borrow(Type::string())),
        ],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    // -----------------------------------------------------------------------
    // ASCII utilities
    // -----------------------------------------------------------------------

    // text.trim_ascii(s: ref[string]) -> string
    m.externs.push(ExternFunction {
        name: "text.trim_ascii".to_string(),
        params: vec![param("s", Type::borrow(Type::string()))],
        return_type: Type::string(),
        effects: vec![],
        annotations: vec![],
    });

    // text.to_lower_ascii(s: ref[string]) -> string
    m.externs.push(ExternFunction {
        name: "text.to_lower_ascii".to_string(),
        params: vec![param("s", Type::borrow(Type::string()))],
        return_type: Type::string(),
        effects: vec![],
        annotations: vec![],
    });

    // text.to_upper_ascii(s: ref[string]) -> string
    m.externs.push(ExternFunction {
        name: "text.to_upper_ascii".to_string(),
        params: vec![param("s", Type::borrow(Type::string()))],
        return_type: Type::string(),
        effects: vec![],
        annotations: vec![],
    });

    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_module_name() {
        let m = module();
        assert_eq!(m.name, vec!["std", "text"]);
    }

    #[test]
    fn text_has_construction() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"text.from_utf8"));
        assert!(names.contains(&"text.from_utf8_unchecked"));
    }

    #[test]
    fn text_has_access() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"text.len_bytes"));
        assert!(names.contains(&"text.is_empty"));
        assert!(names.contains(&"text.as_bytes"));
    }

    #[test]
    fn text_has_comparison() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"text.eq"));
        assert!(names.contains(&"text.cmp"));
    }

    #[test]
    fn text_has_ascii_utils() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"text.trim_ascii"));
        assert!(names.contains(&"text.to_lower_ascii"));
        assert!(names.contains(&"text.to_upper_ascii"));
    }
}
