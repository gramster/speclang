//! `std.collections` — Vec, Set, Map with explicit hash/eq.
//!
//! v0 has no trait system; collection operations take explicit hash and
//! equality function references. Backends map these to the appropriate
//! runtime collection library.

use speclang_ir::module::{ExternFunction, Module, Param};
use speclang_ir::types::{QName, Type};

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

/// Generic placeholder type `K`.
fn type_k() -> Type {
    Type::Named(vec!["K".to_string()])
}

/// Generic placeholder type `V`.
fn type_v() -> Type {
    Type::Named(vec!["V".to_string()])
}

/// Vec[T] generic type.
fn vec_t() -> Type {
    Type::Generic {
        name: vec!["Vec".to_string()],
        args: vec![type_t()],
    }
}

/// Set[T] generic type.
fn set_t() -> Type {
    Type::Generic {
        name: vec!["Set".to_string()],
        args: vec![type_t()],
    }
}

/// Map[K, V] generic type.
fn map_kv() -> Type {
    Type::Generic {
        name: vec!["Map".to_string()],
        args: vec![type_k(), type_v()],
    }
}

/// Build the `std.collections` module.
pub fn module() -> Module {
    let mut m = Module::new(qname("std.collections"));

    // =======================================================================
    // Vec[T]
    // =======================================================================

    // vec.new() -> Vec[T]
    m.externs.push(ExternFunction {
        name: "vec.new".to_string(),
        params: vec![],
        return_type: vec_t(),
        effects: vec![],
        annotations: vec![],
    });

    // vec.len(v: ref[Vec[T]]) -> int
    m.externs.push(ExternFunction {
        name: "vec.len".to_string(),
        params: vec![param("v", Type::borrow(vec_t()))],
        return_type: Type::int(),
        effects: vec![],
        annotations: vec![],
    });

    // vec.is_empty(v: ref[Vec[T]]) -> bool
    m.externs.push(ExternFunction {
        name: "vec.is_empty".to_string(),
        params: vec![param("v", Type::borrow(vec_t()))],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    // vec.push(v: mutref[Vec[T]], item: T) -> unit
    m.externs.push(ExternFunction {
        name: "vec.push".to_string(),
        params: vec![
            param("v", Type::borrow_mut(vec_t())),
            param("item", type_t()),
        ],
        return_type: Type::unit(),
        effects: vec![],
        annotations: vec![],
    });

    // vec.pop(v: mutref[Vec[T]]) -> Option[T]
    m.externs.push(ExternFunction {
        name: "vec.pop".to_string(),
        params: vec![param("v", Type::borrow_mut(vec_t()))],
        return_type: Type::option(type_t()),
        effects: vec![],
        annotations: vec![],
    });

    // vec.get(v: ref[Vec[T]], idx: int) -> ref[T]  (traps on OOB)
    m.externs.push(ExternFunction {
        name: "vec.get".to_string(),
        params: vec![
            param("v", Type::borrow(vec_t())),
            param("idx", Type::int()),
        ],
        return_type: Type::borrow(type_t()),
        effects: vec![],
        annotations: vec![],
    });

    // vec.set(v: mutref[Vec[T]], idx: int, value: T) -> unit  (traps on OOB)
    m.externs.push(ExternFunction {
        name: "vec.set".to_string(),
        params: vec![
            param("v", Type::borrow_mut(vec_t())),
            param("idx", Type::int()),
            param("value", type_t()),
        ],
        return_type: Type::unit(),
        effects: vec![],
        annotations: vec![],
    });

    // vec.as_slice(v: ref[Vec[T]]) -> slice[T]
    m.externs.push(ExternFunction {
        name: "vec.as_slice".to_string(),
        params: vec![param("v", Type::borrow(vec_t()))],
        return_type: Type::slice(type_t()),
        effects: vec![],
        annotations: vec![],
    });

    // vec.contains(v: ref[Vec[T]], item: ref[T]) -> bool
    m.externs.push(ExternFunction {
        name: "vec.contains".to_string(),
        params: vec![
            param("v", Type::borrow(vec_t())),
            param("item", Type::borrow(type_t())),
        ],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    // vec.clear(v: mutref[Vec[T]]) -> unit
    m.externs.push(ExternFunction {
        name: "vec.clear".to_string(),
        params: vec![param("v", Type::borrow_mut(vec_t()))],
        return_type: Type::unit(),
        effects: vec![],
        annotations: vec![],
    });

    // =======================================================================
    // Set[T]
    // =======================================================================

    // set.new() -> Set[T]
    m.externs.push(ExternFunction {
        name: "set.new".to_string(),
        params: vec![],
        return_type: set_t(),
        effects: vec![],
        annotations: vec![],
    });

    // set.len(s: ref[Set[T]]) -> int
    m.externs.push(ExternFunction {
        name: "set.len".to_string(),
        params: vec![param("s", Type::borrow(set_t()))],
        return_type: Type::int(),
        effects: vec![],
        annotations: vec![],
    });

    // set.is_empty(s: ref[Set[T]]) -> bool
    m.externs.push(ExternFunction {
        name: "set.is_empty".to_string(),
        params: vec![param("s", Type::borrow(set_t()))],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    // set.insert(s: mutref[Set[T]], item: T) -> bool  (returns true if inserted)
    m.externs.push(ExternFunction {
        name: "set.insert".to_string(),
        params: vec![
            param("s", Type::borrow_mut(set_t())),
            param("item", type_t()),
        ],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    // set.remove(s: mutref[Set[T]], item: ref[T]) -> bool
    m.externs.push(ExternFunction {
        name: "set.remove".to_string(),
        params: vec![
            param("s", Type::borrow_mut(set_t())),
            param("item", Type::borrow(type_t())),
        ],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    // set.contains(s: ref[Set[T]], item: ref[T]) -> bool
    m.externs.push(ExternFunction {
        name: "set.contains".to_string(),
        params: vec![
            param("s", Type::borrow(set_t())),
            param("item", Type::borrow(type_t())),
        ],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    // set.union(a: ref[Set[T]], b: ref[Set[T]]) -> Set[T]
    m.externs.push(ExternFunction {
        name: "set.union".to_string(),
        params: vec![
            param("a", Type::borrow(set_t())),
            param("b", Type::borrow(set_t())),
        ],
        return_type: set_t(),
        effects: vec![],
        annotations: vec![],
    });

    // set.intersection(a: ref[Set[T]], b: ref[Set[T]]) -> Set[T]
    m.externs.push(ExternFunction {
        name: "set.intersection".to_string(),
        params: vec![
            param("a", Type::borrow(set_t())),
            param("b", Type::borrow(set_t())),
        ],
        return_type: set_t(),
        effects: vec![],
        annotations: vec![],
    });

    // set.difference(a: ref[Set[T]], b: ref[Set[T]]) -> Set[T]
    m.externs.push(ExternFunction {
        name: "set.difference".to_string(),
        params: vec![
            param("a", Type::borrow(set_t())),
            param("b", Type::borrow(set_t())),
        ],
        return_type: set_t(),
        effects: vec![],
        annotations: vec![],
    });

    // =======================================================================
    // Map[K, V]
    // =======================================================================

    // map.new() -> Map[K, V]
    m.externs.push(ExternFunction {
        name: "map.new".to_string(),
        params: vec![],
        return_type: map_kv(),
        effects: vec![],
        annotations: vec![],
    });

    // map.len(m: ref[Map[K, V]]) -> int
    m.externs.push(ExternFunction {
        name: "map.len".to_string(),
        params: vec![param("m", Type::borrow(map_kv()))],
        return_type: Type::int(),
        effects: vec![],
        annotations: vec![],
    });

    // map.is_empty(m: ref[Map[K, V]]) -> bool
    m.externs.push(ExternFunction {
        name: "map.is_empty".to_string(),
        params: vec![param("m", Type::borrow(map_kv()))],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    // map.insert(m: mutref[Map[K, V]], key: K, value: V) -> Option[V]
    m.externs.push(ExternFunction {
        name: "map.insert".to_string(),
        params: vec![
            param("m", Type::borrow_mut(map_kv())),
            param("key", type_k()),
            param("value", type_v()),
        ],
        return_type: Type::option(type_v()),
        effects: vec![],
        annotations: vec![],
    });

    // map.get(m: ref[Map[K, V]], key: ref[K]) -> Option[ref[V]]
    m.externs.push(ExternFunction {
        name: "map.get".to_string(),
        params: vec![
            param("m", Type::borrow(map_kv())),
            param("key", Type::borrow(type_k())),
        ],
        return_type: Type::option(Type::borrow(type_v())),
        effects: vec![],
        annotations: vec![],
    });

    // map.remove(m: mutref[Map[K, V]], key: ref[K]) -> Option[V]
    m.externs.push(ExternFunction {
        name: "map.remove".to_string(),
        params: vec![
            param("m", Type::borrow_mut(map_kv())),
            param("key", Type::borrow(type_k())),
        ],
        return_type: Type::option(type_v()),
        effects: vec![],
        annotations: vec![],
    });

    // map.contains_key(m: ref[Map[K, V]], key: ref[K]) -> bool
    m.externs.push(ExternFunction {
        name: "map.contains_key".to_string(),
        params: vec![
            param("m", Type::borrow(map_kv())),
            param("key", Type::borrow(type_k())),
        ],
        return_type: Type::bool(),
        effects: vec![],
        annotations: vec![],
    });

    // map.clear(m: mutref[Map[K, V]]) -> unit
    m.externs.push(ExternFunction {
        name: "map.clear".to_string(),
        params: vec![param("m", Type::borrow_mut(map_kv()))],
        return_type: Type::unit(),
        effects: vec![],
        annotations: vec![],
    });

    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collections_module_name() {
        let m = module();
        assert_eq!(m.name, vec!["std", "collections"]);
    }

    #[test]
    fn collections_has_vec_ops() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"vec.new"));
        assert!(names.contains(&"vec.len"));
        assert!(names.contains(&"vec.push"));
        assert!(names.contains(&"vec.pop"));
        assert!(names.contains(&"vec.get"));
        assert!(names.contains(&"vec.set"));
        assert!(names.contains(&"vec.as_slice"));
    }

    #[test]
    fn collections_has_set_ops() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"set.new"));
        assert!(names.contains(&"set.len"));
        assert!(names.contains(&"set.insert"));
        assert!(names.contains(&"set.remove"));
        assert!(names.contains(&"set.contains"));
        assert!(names.contains(&"set.union"));
        assert!(names.contains(&"set.intersection"));
    }

    #[test]
    fn collections_has_map_ops() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"map.new"));
        assert!(names.contains(&"map.len"));
        assert!(names.contains(&"map.insert"));
        assert!(names.contains(&"map.get"));
        assert!(names.contains(&"map.remove"));
        assert!(names.contains(&"map.contains_key"));
    }

    #[test]
    fn vec_push_takes_mutref() {
        let m = module();
        let push = m.externs.iter().find(|e| e.name == "vec.push").unwrap();
        match &push.params[0].ty {
            Type::MutRef(_) => {}
            other => panic!("Expected mutref, got {:?}", other),
        }
    }

    #[test]
    fn set_contains_returns_bool() {
        let m = module();
        let contains = m.externs.iter().find(|e| e.name == "set.contains").unwrap();
        assert_eq!(contains.return_type, Type::bool());
    }
}
