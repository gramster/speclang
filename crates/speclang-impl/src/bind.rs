//! SPL-to-IMPL binding verification.
//!
//! Verifies that IMPL function implementations correctly bind to their
//! SPL specs:
//! - Stable ID exists in the SPL program
//! - Parameter count and types match
//! - Return type matches
//! - Capability parameters correspond to declared effects

use crate::ast::{ImplFunction, ImplParam, ImplProgram, ImplItem, ImplTypeRef};
use speclang_spl::ast::{FnSpecDecl, Program as SplProgram, ModuleItem, FnBlock, TypeRef};
use std::fmt;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// A binding verification error.
#[derive(Debug, Clone)]
pub struct BindError {
    pub message: String,
    pub stable_id: String,
}

impl fmt::Display for BindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bind error [{}]: {}", self.stable_id, self.message)
    }
}

impl std::error::Error for BindError {}

// ---------------------------------------------------------------------------
// Verification
// ---------------------------------------------------------------------------

/// Verify that all IMPL functions bind correctly to their SPL specs.
pub fn verify_bindings(
    impl_prog: &ImplProgram,
    spl_prog: &SplProgram,
) -> Result<(), Vec<BindError>> {
    let mut errors = Vec::new();

    // Collect all SPL function specs by stable ID
    let spl_fns: Vec<&FnSpecDecl> = spl_prog
        .items
        .iter()
        .filter_map(|item| match item {
            ModuleItem::FnSpec(f) => Some(f),
            _ => None,
        })
        .collect();

    // Check each IMPL function
    for item in &impl_prog.items {
        if let ImplItem::Function(impl_fn) = item {
            match find_spec_by_id(&spl_fns, &impl_fn.stable_id) {
                None => {
                    errors.push(BindError {
                        message: format!(
                            "no SPL spec found for stable ID \"{}\"",
                            impl_fn.stable_id
                        ),
                        stable_id: impl_fn.stable_id.clone(),
                    });
                }
                Some(spec) => {
                    check_param_count(impl_fn, spec, &mut errors);
                    check_param_types(impl_fn, spec, &mut errors);
                    check_return_type(impl_fn, spec, &mut errors);
                    check_cap_params(impl_fn, spec, &mut errors);
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Find an SPL spec by stable ID.
fn find_spec_by_id<'a>(
    specs: &[&'a FnSpecDecl],
    stable_id: &str,
) -> Option<&'a FnSpecDecl> {
    specs.iter().find(|s| s.stable_id == stable_id).copied()
}

/// Check that parameter counts match (excluding capability params in IMPL).
fn check_param_count(
    impl_fn: &ImplFunction,
    spec: &FnSpecDecl,
    errors: &mut Vec<BindError>,
) {
    let impl_data_params: Vec<&ImplParam> =
        impl_fn.params.iter().filter(|p| !p.is_cap).collect();
    let spl_params = &spec.params;

    if impl_data_params.len() != spl_params.len() {
        errors.push(BindError {
            message: format!(
                "parameter count mismatch: IMPL has {} data params, SPL spec has {}",
                impl_data_params.len(),
                spl_params.len(),
            ),
            stable_id: impl_fn.stable_id.clone(),
        });
    }
}

/// Check that parameter types are compatible.
fn check_param_types(
    impl_fn: &ImplFunction,
    spec: &FnSpecDecl,
    errors: &mut Vec<BindError>,
) {
    let impl_data_params: Vec<&ImplParam> =
        impl_fn.params.iter().filter(|p| !p.is_cap).collect();
    let spl_params = &spec.params;

    for (i, (impl_p, spl_p)) in impl_data_params
        .iter()
        .zip(spl_params.iter())
        .enumerate()
    {
        if !types_compatible(&impl_p.ty, &spl_p.ty) {
            errors.push(BindError {
                message: format!(
                    "parameter {} ({}) type mismatch: IMPL type does not match SPL type",
                    i, impl_p.name
                ),
                stable_id: impl_fn.stable_id.clone(),
            });
        }
    }
}

/// Check that the return type is compatible.
fn check_return_type(
    impl_fn: &ImplFunction,
    spec: &FnSpecDecl,
    errors: &mut Vec<BindError>,
) {
    if !types_compatible(&impl_fn.return_type, &spec.return_type) {
        errors.push(BindError {
            message: "return type mismatch: IMPL type does not match SPL type".to_string(),
            stable_id: impl_fn.stable_id.clone(),
        });
    }
}

/// Check that capability parameters in IMPL correspond to declared effects in SPL.
fn check_cap_params(
    impl_fn: &ImplFunction,
    spec: &FnSpecDecl,
    errors: &mut Vec<BindError>,
) {
    let impl_caps: Vec<&ImplParam> =
        impl_fn.params.iter().filter(|p| p.is_cap).collect();

    // Collect declared effects from the SPL spec
    let mut spl_effects = Vec::new();
    for block in &spec.blocks {
        if let FnBlock::Effects(effects) = block {
            for eff in effects {
                spl_effects.push(eff.name.clone());
            }
        }
    }

    // Each IMPL cap param must correspond to a declared SPL effect
    for cap_param in &impl_caps {
        if let ImplTypeRef::Capability(cap_name) = &cap_param.ty {
            if !spl_effects.iter().any(|e| e == cap_name) {
                errors.push(BindError {
                    message: format!(
                        "capability parameter `{}` has no corresponding effect `{}` in SPL spec",
                        cap_param.name, cap_name
                    ),
                    stable_id: impl_fn.stable_id.clone(),
                });
            }
        }
    }

    // Each SPL declared effect should have a capability parameter in IMPL
    for effect in &spl_effects {
        let has_cap = impl_caps.iter().any(|p| {
            if let ImplTypeRef::Capability(cap_name) = &p.ty {
                cap_name == effect
            } else {
                false
            }
        });
        if !has_cap {
            errors.push(BindError {
                message: format!(
                    "SPL declares effect `{}` but IMPL has no corresponding capability parameter",
                    effect
                ),
                stable_id: impl_fn.stable_id.clone(),
            });
        }
    }
}

/// Check type compatibility between IMPL and SPL type references.
///
/// This is a structural comparison — SPL types are high-level (e.g., `Int`, `Set[MidiNote]`)
/// while IMPL types include ownership annotations. The comparison ignores ownership wrappers
/// and checks that the base types match.
fn types_compatible(impl_ty: &ImplTypeRef, spl_ty: &TypeRef) -> bool {
    // Unwrap ownership wrappers from IMPL
    let impl_base = unwrap_ownership(impl_ty);
    let spl_name = spl_ty.name.join(".");

    match impl_base {
        ImplTypeRef::Named(name) => {
            let normalized = normalize_type_name(name);
            let spl_normalized = normalize_type_name(&spl_name);
            normalized == spl_normalized
        }
        ImplTypeRef::Qualified(parts) => {
            parts == &spl_ty.name
        }
        ImplTypeRef::Generic { name, args } => {
            if name != &spl_ty.name {
                return false;
            }
            if args.len() != spl_ty.args.len() {
                return false;
            }
            args.iter()
                .zip(spl_ty.args.iter())
                .all(|(ia, sa)| types_compatible(ia, sa))
        }
        ImplTypeRef::Tuple(items) => {
            // SPL doesn't have tuples directly, but could be a named type
            if spl_name == "Unit" && items.is_empty() {
                return true;
            }
            false
        }
        ImplTypeRef::Option(inner) => {
            if spl_ty.nullable {
                // T? in SPL matches Option[T] in IMPL
                return types_compatible(inner, &TypeRef {
                    name: spl_ty.name.clone(),
                    args: spl_ty.args.clone(),
                    nullable: false,
                });
            }
            spl_name == "Option" && spl_ty.args.len() == 1
                && types_compatible(inner, &spl_ty.args[0])
        }
        ImplTypeRef::Result { ok, err } => {
            spl_name == "Result" && spl_ty.args.len() == 2
                && types_compatible(ok, &spl_ty.args[0])
                && types_compatible(err, &spl_ty.args[1])
        }
        _ => false,
    }
}

/// Unwrap ownership wrappers (own, ref, mutref, slice, mutslice) to get the base type.
fn unwrap_ownership(ty: &ImplTypeRef) -> &ImplTypeRef {
    match ty {
        ImplTypeRef::Own { inner, .. } => unwrap_ownership(inner),
        ImplTypeRef::Ref(inner) => unwrap_ownership(inner),
        ImplTypeRef::MutRef(inner) => unwrap_ownership(inner),
        ImplTypeRef::Slice(inner) => unwrap_ownership(inner),
        ImplTypeRef::MutSlice(inner) => unwrap_ownership(inner),
        other => other,
    }
}

/// Normalize type names between SPL conventions and IMPL conventions.
fn normalize_type_name(name: &str) -> String {
    match name {
        // SPL uses capitalized; IMPL uses lowercase
        "Int" | "int" => "int".to_string(),
        "Bool" | "bool" => "bool".to_string(),
        "String" | "string" => "string".to_string(),
        "Bytes" | "bytes" => "bytes".to_string(),
        "Unit" | "unit" | "()" => "unit".to_string(),
        "I8" | "i8" => "i8".to_string(),
        "I16" | "i16" => "i16".to_string(),
        "I32" | "i32" => "i32".to_string(),
        "I64" | "i64" => "i64".to_string(),
        "I128" | "i128" => "i128".to_string(),
        "U8" | "u8" => "u8".to_string(),
        "U16" | "u16" => "u16".to_string(),
        "U32" | "u32" => "u32".to_string(),
        "U64" | "u64" => "u64".to_string(),
        "U128" | "u128" => "u128".to_string(),
        "F32" | "f32" => "f32".to_string(),
        "F64" | "f64" => "f64".to_string(),
        other => other.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_impl;
    use speclang_spl::parser::parse_program;

    fn verify(spl_src: &str, impl_src: &str) -> Result<(), Vec<BindError>> {
        let spl = parse_program(spl_src).unwrap();
        let imp = parse_impl(impl_src).unwrap();
        verify_bindings(&imp, &spl)
    }

    #[test]
    fn test_basic_binding() {
        let spl = r#"
            fn add @id("test.add.v1") (a: Int, b: Int) -> Int {
                ensures { is_sum(result, a, b); }
            };
        "#;
        let imp = r#"
            impl fn "test.add.v1" add(a: int, b: int) -> int {
                a + b
            }
        "#;
        assert!(verify(spl, imp).is_ok());
    }

    #[test]
    fn test_missing_spec() {
        let spl = r#"
            fn add @id("test.add.v1") (a: Int, b: Int) -> Int {
                ensures { is_sum(result, a, b); }
            };
        "#;
        let imp = r#"
            impl fn "test.other.v1" other(a: int) -> int {
                a
            }
        "#;
        let errs = verify(spl, imp).unwrap_err();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].message.contains("no SPL spec found"));
    }

    #[test]
    fn test_param_count_mismatch() {
        let spl = r#"
            fn add @id("test.add.v1") (a: Int, b: Int) -> Int {
                ensures { is_sum(result, a, b); }
            };
        "#;
        let imp = r#"
            impl fn "test.add.v1" add(a: int) -> int {
                a
            }
        "#;
        let errs = verify(spl, imp).unwrap_err();
        assert!(errs.iter().any(|e| e.message.contains("parameter count")));
    }

    #[test]
    fn test_return_type_mismatch() {
        let spl = r#"
            fn add @id("test.add.v1") (a: Int, b: Int) -> Int {
                ensures { is_sum(result, a, b); }
            };
        "#;
        let imp = r#"
            impl fn "test.add.v1" add(a: int, b: int) -> string {
                "hello"
            }
        "#;
        let errs = verify(spl, imp).unwrap_err();
        assert!(errs.iter().any(|e| e.message.contains("return type")));
    }

    #[test]
    fn test_cap_param_binding() {
        let spl = r#"
            fn fetch @id("test.fetch.v1") (url: String) -> String {
                effects { Net }
            };
        "#;
        let imp = r#"
            impl fn "test.fetch.v1" fetch(url: string, net: cap Net) -> string {
                url
            }
        "#;
        assert!(verify(spl, imp).is_ok());
    }

    #[test]
    fn test_missing_cap_param() {
        let spl = r#"
            fn fetch @id("test.fetch.v1") (url: String) -> String {
                effects { Net }
            };
        "#;
        let imp = r#"
            impl fn "test.fetch.v1" fetch(url: string) -> string {
                url
            }
        "#;
        let errs = verify(spl, imp).unwrap_err();
        assert!(errs.iter().any(|e| e.message.contains("no corresponding capability")));
    }

    #[test]
    fn test_extra_cap_param() {
        let spl = r#"
            fn add @id("test.add.v1") (a: Int, b: Int) -> Int {
                ensures { is_sum(result, a, b); }
            };
        "#;
        let imp = r#"
            impl fn "test.add.v1" add(a: int, b: int, fs: cap FileRead) -> int {
                a + b
            }
        "#;
        let errs = verify(spl, imp).unwrap_err();
        assert!(errs.iter().any(|e| e.message.contains("no corresponding effect")));
    }

    #[test]
    fn test_ownership_type_compatibility() {
        let spl = r#"
            fn get @id("test.get.v1") (data: String) -> Int {
                ensures { result >= 0; }
            };
        "#;
        let imp = r#"
            impl fn "test.get.v1" get(data: ref[string]) -> int {
                0
            }
        "#;
        // ref[string] matches String (ownership wrapper is stripped)
        assert!(verify(spl, imp).is_ok());
    }

    #[test]
    fn test_param_type_mismatch() {
        let spl = r#"
            fn add @id("test.add.v1") (a: Int, b: Int) -> Int {
                ensures { is_sum(result, a, b); }
            };
        "#;
        let imp = r#"
            impl fn "test.add.v1" add(a: string, b: int) -> int {
                0
            }
        "#;
        let errs = verify(spl, imp).unwrap_err();
        assert!(errs.iter().any(|e| e.message.contains("type mismatch")));
    }
}
