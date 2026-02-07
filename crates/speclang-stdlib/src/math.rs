//! `std.math` — Integer conversions, unbounded int ops, float helpers.
//!
//! Fixed-width arithmetic is built-in (traps on overflow). This module
//! provides explicit conversion functions between numeric types and
//! operations on the unbounded `int` type.

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

/// Build the `std.math` module.
pub fn module() -> Module {
    let mut m = Module::new(qname("std.math"));

    // -----------------------------------------------------------------------
    // Integer conversions (all trap on out-of-range)
    // -----------------------------------------------------------------------
    let conversions: Vec<(&str, PrimitiveType, PrimitiveType)> = vec![
        ("conv.i32_from_int", PrimitiveType::Int, PrimitiveType::I32),
        ("conv.int_from_i32", PrimitiveType::I32, PrimitiveType::Int),
        ("conv.u64_from_int", PrimitiveType::Int, PrimitiveType::U64),
        ("conv.int_from_u64", PrimitiveType::U64, PrimitiveType::Int),
        ("conv.i32_from_u64", PrimitiveType::U64, PrimitiveType::I32),
        ("conv.u64_from_i32", PrimitiveType::I32, PrimitiveType::U64),
        ("conv.i64_from_int", PrimitiveType::Int, PrimitiveType::I64),
        ("conv.int_from_i64", PrimitiveType::I64, PrimitiveType::Int),
        ("conv.u32_from_int", PrimitiveType::Int, PrimitiveType::U32),
        ("conv.int_from_u32", PrimitiveType::U32, PrimitiveType::Int),
        ("conv.i32_from_i64", PrimitiveType::I64, PrimitiveType::I32),
        ("conv.i64_from_i32", PrimitiveType::I32, PrimitiveType::I64),
        ("conv.u8_from_int", PrimitiveType::Int, PrimitiveType::U8),
        ("conv.int_from_u8", PrimitiveType::U8, PrimitiveType::Int),
    ];

    for (name, from, to) in conversions {
        m.externs.push(ExternFunction {
            name: name.to_string(),
            params: vec![param("x", Type::Primitive(from))],
            return_type: Type::Primitive(to),
            effects: vec![],
            annotations: vec![],
        });
    }

    // -----------------------------------------------------------------------
    // Unbounded int operations
    // -----------------------------------------------------------------------
    let int_binops = ["int.add", "int.sub", "int.mul", "int.div", "int.mod"];
    for name in &int_binops {
        m.externs.push(ExternFunction {
            name: name.to_string(),
            params: vec![param("a", Type::int()), param("b", Type::int())],
            return_type: Type::int(),
            effects: vec![],
            annotations: vec![],
        });
    }

    // Unary int ops
    m.externs.push(ExternFunction {
        name: "int.neg".to_string(),
        params: vec![param("x", Type::int())],
        return_type: Type::int(),
        effects: vec![],
        annotations: vec![],
    });

    m.externs.push(ExternFunction {
        name: "int.abs".to_string(),
        params: vec![param("x", Type::int())],
        return_type: Type::int(),
        effects: vec![],
        annotations: vec![],
    });

    // int.min / int.max
    m.externs.push(ExternFunction {
        name: "int.min".to_string(),
        params: vec![param("a", Type::int()), param("b", Type::int())],
        return_type: Type::int(),
        effects: vec![],
        annotations: vec![],
    });

    m.externs.push(ExternFunction {
        name: "int.max".to_string(),
        params: vec![param("a", Type::int()), param("b", Type::int())],
        return_type: Type::int(),
        effects: vec![],
        annotations: vec![],
    });

    // int.pow
    m.externs.push(ExternFunction {
        name: "int.pow".to_string(),
        params: vec![param("base", Type::int()), param("exp", Type::int())],
        return_type: Type::int(),
        effects: vec![],
        annotations: vec![],
    });

    // -----------------------------------------------------------------------
    // Float helpers
    // -----------------------------------------------------------------------
    m.externs.push(ExternFunction {
        name: "float.is_nan64".to_string(),
        params: vec![param("x", Type::Primitive(PrimitiveType::F64))],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    m.externs.push(ExternFunction {
        name: "float.is_finite64".to_string(),
        params: vec![param("x", Type::Primitive(PrimitiveType::F64))],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    m.externs.push(ExternFunction {
        name: "float.total_cmp64".to_string(),
        params: vec![
            param("a", Type::Primitive(PrimitiveType::F64)),
            param("b", Type::Primitive(PrimitiveType::F64)),
        ],
        return_type: Type::Named(qname("std.core.Ordering")),
        effects: vec![],
        annotations: vec![],
    });

    m.externs.push(ExternFunction {
        name: "float.is_nan32".to_string(),
        params: vec![param("x", Type::Primitive(PrimitiveType::F32))],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    m.externs.push(ExternFunction {
        name: "float.is_finite32".to_string(),
        params: vec![param("x", Type::Primitive(PrimitiveType::F32))],
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
    fn math_module_name() {
        let m = module();
        assert_eq!(m.name, vec!["std", "math"]);
    }

    #[test]
    fn math_has_conversion_functions() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"conv.i32_from_int"));
        assert!(names.contains(&"conv.int_from_i32"));
        assert!(names.contains(&"conv.u64_from_int"));
        assert!(names.contains(&"conv.int_from_u64"));
    }

    #[test]
    fn math_has_int_ops() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"int.add"));
        assert!(names.contains(&"int.sub"));
        assert!(names.contains(&"int.mul"));
        assert!(names.contains(&"int.div"));
        assert!(names.contains(&"int.mod"));
        assert!(names.contains(&"int.neg"));
        assert!(names.contains(&"int.abs"));
    }

    #[test]
    fn math_has_float_helpers() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"float.is_nan64"));
        assert!(names.contains(&"float.is_finite64"));
        assert!(names.contains(&"float.total_cmp64"));
    }

    #[test]
    fn conversion_functions_have_correct_types() {
        let m = module();
        let conv = m.externs.iter().find(|e| e.name == "conv.i32_from_int").unwrap();
        assert_eq!(conv.params[0].ty, Type::int());
        assert_eq!(conv.return_type, Type::i32());
    }
}
