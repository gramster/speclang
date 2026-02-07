//! `std.core` — Option, Result, and primitive equality/ordering.
//!
//! Provides the foundational generic types and their accessor functions.
//! In v0, equality and ordering for primitives are built-in operators;
//! this module provides the Option/Result helpers used across the stdlib.

use speclang_ir::module::{ExternFunction, Module, Param, TypeDef};
use speclang_ir::types::{QName, Type, Variant};

fn qname(s: &str) -> QName {
    s.split('.').map(|p| p.to_string()).collect()
}

fn param(name: &str, ty: Type) -> Param {
    Param {
        name: name.to_string(),
        ty,
    }
}

/// Generic placeholder type `T`.
fn type_t() -> Type {
    Type::Named(vec!["T".to_string()])
}

/// Generic placeholder type `E`.
fn type_e() -> Type {
    Type::Named(vec!["E".to_string()])
}

/// Build the `std.core` module.
pub fn module() -> Module {
    let mut m = Module::new(qname("std.core"));

    // -----------------------------------------------------------------------
    // Option[T] = enum { None, Some(T) }
    // (Option is a built-in Type variant but we also provide a TypeDef
    // so that name resolution can find it.)
    // -----------------------------------------------------------------------
    m.type_defs.push(TypeDef {
        name: "Option".to_string(),
        ty: Type::Enum(vec![
            Variant {
                name: "None".to_string(),
                fields: vec![],
            },
            Variant {
                name: "Some".to_string(),
                fields: vec![type_t()],
            },
        ]),
        annotations: vec![],
    });

    // -----------------------------------------------------------------------
    // Result[T, E] = enum { Ok(T), Err(E) }
    // -----------------------------------------------------------------------
    m.type_defs.push(TypeDef {
        name: "Result".to_string(),
        ty: Type::Enum(vec![
            Variant {
                name: "Ok".to_string(),
                fields: vec![type_t()],
            },
            Variant {
                name: "Err".to_string(),
                fields: vec![type_e()],
            },
        ]),
        annotations: vec![],
    });

    // -----------------------------------------------------------------------
    // Ordering = enum { Less, Equal, Greater }
    // -----------------------------------------------------------------------
    m.type_defs.push(TypeDef {
        name: "Ordering".to_string(),
        ty: Type::Enum(vec![
            Variant {
                name: "Less".to_string(),
                fields: vec![],
            },
            Variant {
                name: "Equal".to_string(),
                fields: vec![],
            },
            Variant {
                name: "Greater".to_string(),
                fields: vec![],
            },
        ]),
        annotations: vec![],
    });

    // -----------------------------------------------------------------------
    // Option helpers
    // -----------------------------------------------------------------------
    m.externs.push(ExternFunction {
        name: "option.is_some".to_string(),
        params: vec![param("o", Type::borrow(Type::option(type_t())))],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    m.externs.push(ExternFunction {
        name: "option.is_none".to_string(),
        params: vec![param("o", Type::borrow(Type::option(type_t())))],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    m.externs.push(ExternFunction {
        name: "option.unwrap".to_string(),
        params: vec![param("o", Type::option(type_t()))],
        return_type: type_t(),
        effects: vec![],
        annotations: vec![],
    });

    m.externs.push(ExternFunction {
        name: "option.unwrap_or".to_string(),
        params: vec![
            param("o", Type::option(type_t())),
            param("default", type_t()),
        ],
        return_type: type_t(),
        effects: vec![],
        annotations: vec![],
    });

    m.externs.push(ExternFunction {
        name: "option.map".to_string(),
        params: vec![
            param("o", Type::option(type_t())),
            // In v0 we don't have first-class closures in the IR;
            // map is modeled as an extern that backends must handle.
        ],
        return_type: Type::option(type_t()),
        effects: vec![],
        annotations: vec![],
    });

    // -----------------------------------------------------------------------
    // Result helpers
    // -----------------------------------------------------------------------
    m.externs.push(ExternFunction {
        name: "result.is_ok".to_string(),
        params: vec![param(
            "r",
            Type::borrow(Type::result(type_t(), type_e())),
        )],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    m.externs.push(ExternFunction {
        name: "result.is_err".to_string(),
        params: vec![param(
            "r",
            Type::borrow(Type::result(type_t(), type_e())),
        )],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    m.externs.push(ExternFunction {
        name: "result.unwrap_ok".to_string(),
        params: vec![param("r", Type::result(type_t(), type_e()))],
        return_type: type_t(),
        effects: vec![],
        annotations: vec![],
    });

    m.externs.push(ExternFunction {
        name: "result.unwrap_err".to_string(),
        params: vec![param("r", Type::result(type_t(), type_e()))],
        return_type: type_e(),
        effects: vec![],
        annotations: vec![],
    });

    // -----------------------------------------------------------------------
    // Primitive equality / ordering helpers
    // (These are used internally; v0 provides built-in == != < etc. for
    //  primitives via BinOp, but these functions are available for use
    //  in contracts and higher-order contexts.)
    // -----------------------------------------------------------------------
    m.externs.push(ExternFunction {
        name: "bool.eq".to_string(),
        params: vec![param("a", Type::bool()), param("b", Type::bool())],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    m.externs.push(ExternFunction {
        name: "int.eq".to_string(),
        params: vec![param("a", Type::int()), param("b", Type::int())],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    m.externs.push(ExternFunction {
        name: "int.cmp".to_string(),
        params: vec![param("a", Type::int()), param("b", Type::int())],
        return_type: Type::Named(qname("std.core.Ordering")),
        effects: vec![],
        annotations: vec![],
    });

    m.externs.push(ExternFunction {
        name: "string.eq".to_string(),
        params: vec![
            param("a", Type::borrow(Type::string())),
            param("b", Type::borrow(Type::string())),
        ],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    m.externs.push(ExternFunction {
        name: "string.cmp".to_string(),
        params: vec![
            param("a", Type::borrow(Type::string())),
            param("b", Type::borrow(Type::string())),
        ],
        return_type: Type::Named(qname("std.core.Ordering")),
        effects: vec![],
        annotations: vec![],
    });

    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_module_has_option_type() {
        let m = module();
        assert!(m.find_type("Option").is_some());
        let opt = m.find_type("Option").unwrap();
        match &opt.ty {
            Type::Enum(variants) => {
                assert_eq!(variants.len(), 2);
                assert_eq!(variants[0].name, "None");
                assert_eq!(variants[1].name, "Some");
            }
            _ => panic!("Option should be an enum"),
        }
    }

    #[test]
    fn core_module_has_result_type() {
        let m = module();
        assert!(m.find_type("Result").is_some());
        let res = m.find_type("Result").unwrap();
        match &res.ty {
            Type::Enum(variants) => {
                assert_eq!(variants.len(), 2);
                assert_eq!(variants[0].name, "Ok");
                assert_eq!(variants[1].name, "Err");
            }
            _ => panic!("Result should be an enum"),
        }
    }

    #[test]
    fn core_module_has_ordering_type() {
        let m = module();
        assert!(m.find_type("Ordering").is_some());
    }

    #[test]
    fn core_module_has_option_helpers() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"option.is_some"));
        assert!(names.contains(&"option.is_none"));
        assert!(names.contains(&"option.unwrap"));
        assert!(names.contains(&"option.unwrap_or"));
    }

    #[test]
    fn core_module_has_result_helpers() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"result.is_ok"));
        assert!(names.contains(&"result.is_err"));
        assert!(names.contains(&"result.unwrap_ok"));
        assert!(names.contains(&"result.unwrap_err"));
    }

    #[test]
    fn core_module_name() {
        let m = module();
        assert_eq!(m.name, vec!["std", "core"]);
    }
}
