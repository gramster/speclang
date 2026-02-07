//! `std.mem` — Regions, allocation, borrowing, and slices.
//!
//! Provides the low-level memory management primitives. All allocation
//! goes through explicit regions; there is no implicit global allocator.

use speclang_ir::module::{ExternFunction, Module, Param};
use speclang_ir::types::{QName, Region, Type};

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

/// Build the `std.mem` module.
pub fn module() -> Module {
    let mut m = Module::new(qname("std.mem"));

    // -----------------------------------------------------------------------
    // Region management
    // -----------------------------------------------------------------------

    // new_region() -> region
    m.externs.push(ExternFunction {
        name: "new_region".to_string(),
        params: vec![],
        return_type: Type::Region,
        effects: vec![],
        annotations: vec![],
    });

    // drop_region(r: region) -> unit
    m.externs.push(ExternFunction {
        name: "drop_region".to_string(),
        params: vec![param("r", Type::Region)],
        return_type: Type::unit(),
        effects: vec![],
        annotations: vec![],
    });

    // -----------------------------------------------------------------------
    // Allocation
    // -----------------------------------------------------------------------

    // alloc(r: region, value: T) -> own[R, T]
    m.externs.push(ExternFunction {
        name: "alloc".to_string(),
        params: vec![param("r", Type::Region), param("value", type_t())],
        return_type: Type::own(Region::Heap, type_t()),
        effects: vec![],
        annotations: vec![],
    });

    // dealloc(ptr: own[R, T]) -> unit
    m.externs.push(ExternFunction {
        name: "dealloc".to_string(),
        params: vec![param("ptr", Type::own(Region::Heap, type_t()))],
        return_type: Type::unit(),
        effects: vec![],
        annotations: vec![],
    });

    // -----------------------------------------------------------------------
    // Borrowing
    // -----------------------------------------------------------------------

    // borrow(ptr: ref[own[R, T]]) -> ref[T]
    m.externs.push(ExternFunction {
        name: "borrow".to_string(),
        params: vec![param(
            "ptr",
            Type::borrow(Type::own(Region::Heap, type_t())),
        )],
        return_type: Type::borrow(type_t()),
        effects: vec![],
        annotations: vec![],
    });

    // borrow_mut(ptr: mutref[own[R, T]]) -> mutref[T]
    m.externs.push(ExternFunction {
        name: "borrow_mut".to_string(),
        params: vec![param(
            "ptr",
            Type::borrow_mut(Type::own(Region::Heap, type_t())),
        )],
        return_type: Type::borrow_mut(type_t()),
        effects: vec![],
        annotations: vec![],
    });

    // -----------------------------------------------------------------------
    // Slice operations
    // -----------------------------------------------------------------------

    // slice.len(s: slice[T]) -> int
    m.externs.push(ExternFunction {
        name: "slice.len".to_string(),
        params: vec![param("s", Type::slice(type_t()))],
        return_type: Type::int(),
        effects: vec![],
        annotations: vec![],
    });

    // slice.get(s: slice[T], idx: int) -> ref[T]  (traps on out-of-bounds)
    m.externs.push(ExternFunction {
        name: "slice.get".to_string(),
        params: vec![
            param("s", Type::slice(type_t())),
            param("idx", Type::int()),
        ],
        return_type: Type::borrow(type_t()),
        effects: vec![],
        annotations: vec![],
    });

    // slice.get_mut(s: mutslice[T], idx: int) -> mutref[T]
    m.externs.push(ExternFunction {
        name: "slice.get_mut".to_string(),
        params: vec![
            param("s", Type::mut_slice(type_t())),
            param("idx", Type::int()),
        ],
        return_type: Type::borrow_mut(type_t()),
        effects: vec![],
        annotations: vec![],
    });

    // slice.subslice(s: slice[T], start: int, end: int) -> slice[T]
    m.externs.push(ExternFunction {
        name: "slice.subslice".to_string(),
        params: vec![
            param("s", Type::slice(type_t())),
            param("start", Type::int()),
            param("end", Type::int()),
        ],
        return_type: Type::slice(type_t()),
        effects: vec![],
        annotations: vec![],
    });

    // -----------------------------------------------------------------------
    // Copy / move helpers
    // -----------------------------------------------------------------------

    // mem.copy(dst: mutslice[T], src: slice[T]) -> unit
    m.externs.push(ExternFunction {
        name: "mem.copy".to_string(),
        params: vec![
            param("dst", Type::mut_slice(type_t())),
            param("src", Type::slice(type_t())),
        ],
        return_type: Type::unit(),
        effects: vec![],
        annotations: vec![],
    });

    // mem.size_of(T) -> int  (compile-time known)
    m.externs.push(ExternFunction {
        name: "mem.size_of".to_string(),
        params: vec![],
        return_type: Type::int(),
        effects: vec![],
        annotations: vec![],
    });

    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mem_module_name() {
        let m = module();
        assert_eq!(m.name, vec!["std", "mem"]);
    }

    #[test]
    fn mem_has_region_ops() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"new_region"));
        assert!(names.contains(&"drop_region"));
    }

    #[test]
    fn mem_has_alloc_ops() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"alloc"));
        assert!(names.contains(&"dealloc"));
    }

    #[test]
    fn mem_has_borrowing() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"borrow"));
        assert!(names.contains(&"borrow_mut"));
    }

    #[test]
    fn mem_has_slice_ops() {
        let m = module();
        let names: Vec<&str> = m.externs.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"slice.len"));
        assert!(names.contains(&"slice.get"));
        assert!(names.contains(&"slice.get_mut"));
        assert!(names.contains(&"slice.subslice"));
    }
}
