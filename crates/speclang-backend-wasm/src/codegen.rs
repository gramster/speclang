//! Core IR → WebAssembly Text format (WAT) code generation.
//!
//! Produces WAT source from a `Module`, targeting WASM MVP + WASI preview-1.

use speclang_ir::expr::{BinOp, Block, Expr, Literal, MatchArm, Pattern, Stmt, UnOp};
use speclang_ir::contract::{Contract, ContractKind, ContractPolicy};
use speclang_ir::module::{
    Annotation, ExternFunction, Function, Module, TypeDef,
};
use speclang_ir::types::{PrimitiveType, Type};
use speclang_ir::CapabilityDef;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Generate WAT source from a Core IR module.
pub fn generate_wasm(module: &Module) -> String {
    let mut cg = WasmCodeGen::new();
    cg.generate(module)
}

// ---------------------------------------------------------------------------
// Type layout
// ---------------------------------------------------------------------------

/// WASM value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValType {
    I32,
    I64,
    F32,
    F64,
}

impl std::fmt::Display for ValType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValType::I32 => write!(f, "i32"),
            ValType::I64 => write!(f, "i64"),
            ValType::F32 => write!(f, "f32"),
            ValType::F64 => write!(f, "f64"),
        }
    }
}

/// A local variable slot.
#[derive(Clone)]
struct Local {
    name: String,
    ty: ValType,
}

// ---------------------------------------------------------------------------
// CodeGen state
// ---------------------------------------------------------------------------

struct WasmCodeGen {
    buf: String,
    indent: usize,
    /// Next free memory offset for static data.
    data_offset: u32,
    /// String literals accumulated for the data segment.
    data_segments: Vec<(u32, String)>,
    /// Local counter for generating unique labels.
    label_counter: u32,
    /// Locals accumulated for the current function.
    locals: Vec<Local>,
    /// Stack of block labels for break/continue in nested blocks.
    #[allow(dead_code)]
    block_depth: u32,
}

impl WasmCodeGen {
    fn new() -> Self {
        Self {
            buf: String::new(),
            indent: 0,
            data_offset: 1024, // Reserve first 1KB for stack/scratch.
            data_segments: Vec::new(),
            label_counter: 0,
            locals: Vec::new(),
            block_depth: 0,
        }
    }

    fn fresh_label(&mut self) -> String {
        let n = self.label_counter;
        self.label_counter += 1;
        format!("$L{n}")
    }

    fn alloc_string_data(&mut self, s: &str) -> (u32, u32) {
        let offset = self.data_offset;
        let len = s.len() as u32;
        self.data_segments.push((offset, s.to_string()));
        self.data_offset += len;
        // Align to 4 bytes.
        self.data_offset = (self.data_offset + 3) & !3;
        (offset, len)
    }

    fn add_local(&mut self, name: &str, ty: ValType) {
        self.locals.push(Local {
            name: name.to_string(),
            ty,
        });
    }

    // -----------------------------------------------------------------------
    // Output helpers
    // -----------------------------------------------------------------------

    fn line(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.buf.push_str("  ");
        }
        self.buf.push_str(s);
        self.buf.push('\n');
    }

    fn blank(&mut self) {
        self.buf.push('\n');
    }

    fn push(&mut self) {
        self.indent += 1;
    }

    fn pop(&mut self) {
        assert!(self.indent > 0);
        self.indent -= 1;
    }

    // -----------------------------------------------------------------------
    // Type mapping
    // -----------------------------------------------------------------------

    /// Map a Core IR type to a WASM value type.
    /// Returns None for zero-size types (Unit, Capability).
    fn val_type(ty: &Type) -> Option<ValType> {
        match ty {
            Type::Primitive(p) => match p {
                PrimitiveType::Bool => Some(ValType::I32),
                PrimitiveType::U8 | PrimitiveType::I8 => Some(ValType::I32),
                PrimitiveType::U16 | PrimitiveType::I16 => Some(ValType::I32),
                PrimitiveType::U32 | PrimitiveType::I32 => Some(ValType::I32),
                PrimitiveType::U64 | PrimitiveType::I64 => Some(ValType::I64),
                PrimitiveType::U128 | PrimitiveType::I128 => Some(ValType::I64), // lossy
                PrimitiveType::Int => Some(ValType::I64),
                PrimitiveType::F32 => Some(ValType::F32),
                PrimitiveType::F64 => Some(ValType::F64),
                PrimitiveType::Unit => None,
                PrimitiveType::String | PrimitiveType::Bytes => Some(ValType::I32), // ptr
            },
            Type::Named(_) => Some(ValType::I32), // pointer into linear memory
            Type::Struct(_) => Some(ValType::I32),
            Type::Enum(_) => Some(ValType::I32),
            Type::Tuple(elems) => {
                if elems.is_empty() {
                    None
                } else {
                    Some(ValType::I32) // pointer
                }
            }
            Type::Own { .. } => Some(ValType::I32), // pointer
            Type::Ref(_) | Type::MutRef(_) => Some(ValType::I32),
            Type::Slice(_) | Type::MutSlice(_) => Some(ValType::I32), // ptr
            Type::Option(_) => Some(ValType::I32), // tagged ptr
            Type::Result { .. } => Some(ValType::I32),
            Type::Generic { .. } => Some(ValType::I32),
            Type::Capability(_) => None,  // zero-size
            Type::Region => None,         // elided
        }
    }

    /// Size of a type in bytes in linear memory.
    fn type_size(ty: &Type) -> u32 {
        match ty {
            Type::Primitive(p) => match p {
                PrimitiveType::Bool | PrimitiveType::U8 | PrimitiveType::I8 => 1,
                PrimitiveType::U16 | PrimitiveType::I16 => 2,
                PrimitiveType::U32 | PrimitiveType::I32 | PrimitiveType::F32 => 4,
                PrimitiveType::U64 | PrimitiveType::I64 | PrimitiveType::F64 | PrimitiveType::Int => 8,
                PrimitiveType::U128 | PrimitiveType::I128 => 16,
                PrimitiveType::Unit => 0,
                PrimitiveType::String | PrimitiveType::Bytes => 8, // ptr + len
            },
            Type::Named(_) | Type::Struct(_) | Type::Enum(_) => 4, // pointer
            Type::Tuple(elems) => {
                elems.iter().map(|e| Self::type_size(e)).sum()
            }
            Type::Own { .. } | Type::Ref(_) | Type::MutRef(_) |
            Type::Slice(_) | Type::MutSlice(_) | Type::Option(_) |
            Type::Result { .. } | Type::Generic { .. } => 4,
            Type::Capability(_) | Type::Region => 0,
        }
    }

    // -----------------------------------------------------------------------
    // Module generation
    // -----------------------------------------------------------------------

    fn generate(&mut self, module: &Module) -> String {
        let mod_name = module.name.join("_");
        self.line(&format!("(module ${mod_name}"));
        self.push();

        // Comment header.
        self.line(&format!(";; Generated from speclang module `{}`", module.name.join("::")));
        self.blank();

        // WASI imports.
        self.emit_wasi_imports();

        // Memory.
        self.line(";; Linear memory (1 page = 64KB)");
        self.line("(memory (export \"memory\") 1)");
        self.blank();

        // Global stack pointer.
        self.line(";; Stack pointer");
        self.line("(global $sp (mut i32) (i32.const 1024))");
        self.blank();

        // Type definitions (as comments for documentation).
        self.emit_type_defs(&module.type_defs);

        // Capability definitions (as comments).
        self.emit_cap_defs(&module.cap_defs);

        // Extern function imports.
        self.emit_extern_imports(&module.externs);

        // Functions.
        for f in &module.functions {
            self.emit_function(f);
            self.blank();
        }

        // Data segments (string literals, etc.).
        self.emit_data_segments();

        self.pop();
        self.line(")");

        self.buf.clone()
    }

    // -----------------------------------------------------------------------
    // WASI imports
    // -----------------------------------------------------------------------

    fn emit_wasi_imports(&mut self) {
        self.line(";; WASI preview-1 imports");
        self.line("(import \"wasi_snapshot_preview1\" \"fd_write\"");
        self.push();
        self.line("(func $fd_write (param i32 i32 i32 i32) (result i32))");
        self.pop();
        self.line(")");
        self.line("(import \"wasi_snapshot_preview1\" \"proc_exit\"");
        self.push();
        self.line("(func $proc_exit (param i32))");
        self.pop();
        self.line(")");
        self.blank();
    }

    // -----------------------------------------------------------------------
    // Type definitions
    // -----------------------------------------------------------------------

    fn emit_type_defs(&mut self, defs: &[TypeDef]) {
        if defs.is_empty() {
            return;
        }
        self.line(";; Type definitions");
        for td in defs {
            match &td.ty {
                Type::Struct(fields) => {
                    let field_strs: Vec<String> = fields
                        .iter()
                        .map(|f| format!("{}: {} ({}B)", f.name, self.render_type_comment(&f.ty), Self::type_size(&f.ty)))
                        .collect();
                    self.line(&format!(
                        ";; struct {} {{ {} }} (total {}B)",
                        td.name,
                        field_strs.join(", "),
                        fields.iter().map(|f| Self::type_size(&f.ty)).sum::<u32>()
                    ));
                }
                Type::Enum(variants) => {
                    let var_strs: Vec<String> = variants
                        .iter()
                        .enumerate()
                        .map(|(i, v)| format!("{} = {}", v.name, i))
                        .collect();
                    self.line(&format!(
                        ";; enum {} {{ {} }}",
                        td.name,
                        var_strs.join(", ")
                    ));
                }
                _ => {
                    self.line(&format!(
                        ";; type {} = {}",
                        td.name,
                        self.render_type_comment(&td.ty)
                    ));
                }
            }
        }
        self.blank();
    }

    fn emit_cap_defs(&mut self, caps: &[CapabilityDef]) {
        if caps.is_empty() {
            return;
        }
        self.line(";; Capabilities (zero-size tokens, elided from signatures)");
        for cap in caps {
            if cap.fields.is_empty() {
                self.line(&format!(";; capability {} (unit)", cap.name));
            } else {
                let fields: Vec<String> = cap
                    .fields
                    .iter()
                    .map(|f| format!("{}: {}", f.name, self.render_type_comment(&f.ty)))
                    .collect();
                self.line(&format!(";; capability {}({})", cap.name, fields.join(", ")));
            }
        }
        self.blank();
    }

    fn render_type_comment(&self, ty: &Type) -> String {
        match ty {
            Type::Primitive(p) => match p {
                PrimitiveType::Bool => "bool".into(),
                PrimitiveType::U8 => "u8".into(),
                PrimitiveType::U16 => "u16".into(),
                PrimitiveType::U32 => "u32".into(),
                PrimitiveType::U64 => "u64".into(),
                PrimitiveType::U128 => "u128".into(),
                PrimitiveType::I8 => "i8".into(),
                PrimitiveType::I16 => "i16".into(),
                PrimitiveType::I32 => "i32".into(),
                PrimitiveType::I64 => "i64".into(),
                PrimitiveType::I128 => "i128".into(),
                PrimitiveType::Int => "Int".into(),
                PrimitiveType::F32 => "f32".into(),
                PrimitiveType::F64 => "f64".into(),
                PrimitiveType::Unit => "()".into(),
                PrimitiveType::String => "String".into(),
                PrimitiveType::Bytes => "Bytes".into(),
            },
            Type::Named(n) => n.join("::"),
            Type::Struct(_) => "struct{...}".into(),
            Type::Enum(_) => "enum{...}".into(),
            Type::Tuple(elems) => {
                let inner: Vec<String> = elems.iter().map(|t| self.render_type_comment(t)).collect();
                format!("({})", inner.join(", "))
            }
            Type::Own { inner, .. } => format!("Own<{}>", self.render_type_comment(inner)),
            Type::Ref(inner) => format!("&{}", self.render_type_comment(inner)),
            Type::MutRef(inner) => format!("&mut {}", self.render_type_comment(inner)),
            Type::Slice(inner) => format!("[{}]", self.render_type_comment(inner)),
            Type::MutSlice(inner) => format!("&mut [{}]", self.render_type_comment(inner)),
            Type::Option(inner) => format!("Option<{}>", self.render_type_comment(inner)),
            Type::Result { ok, err } => {
                format!("Result<{}, {}>", self.render_type_comment(ok), self.render_type_comment(err))
            }
            Type::Generic { name, args } => {
                let arg_strs: Vec<String> = args.iter().map(|a| self.render_type_comment(a)).collect();
                format!("{}<{}>", name.join("::"), arg_strs.join(", "))
            }
            Type::Capability(name) => format!("cap:{name}"),
            Type::Region => "Region".into(),
        }
    }

    // -----------------------------------------------------------------------
    // Extern imports
    // -----------------------------------------------------------------------

    fn emit_extern_imports(&mut self, externs: &[ExternFunction]) {
        if externs.is_empty() {
            return;
        }
        self.line(";; Extern function imports");
        for e in externs {
            let params: Vec<String> = e
                .params
                .iter()
                .filter_map(|p| Self::val_type(&p.ty).map(|vt| format!("(param ${} {})", p.name, vt)))
                .collect();
            let result = Self::val_type(&e.return_type)
                .map(|vt| format!(" (result {})", vt))
                .unwrap_or_default();
            self.line(&format!(
                "(import \"env\" \"{}\" (func ${} {}{}))",
                e.name,
                e.name,
                params.join(" "),
                result
            ));
        }
        self.blank();
    }

    // -----------------------------------------------------------------------
    // Functions
    // -----------------------------------------------------------------------

    fn emit_function(&mut self, f: &Function) {
        self.locals.clear();
        self.label_counter = 0;

        // Annotations as comments.
        for ann in &f.annotations {
            match ann {
                Annotation::Id(id) => self.line(&format!(";; @id {id}")),
                Annotation::Compat(c) => {
                    let s = match c {
                        speclang_ir::module::Compat::StableCall => "stable-call",
                        speclang_ir::module::Compat::StableSemantics => "stable-semantics",
                        speclang_ir::module::Compat::Unstable => "unstable",
                    };
                    self.line(&format!(";; @compat {s}"));
                }
                Annotation::ReqTag(tag) => self.line(&format!(";; @req {tag}")),
            }
        }

        // Effects comment.
        if !f.effects.is_empty() {
            let eff_names: Vec<String> = f.effects.iter().map(|e| e.name.clone()).collect();
            self.line(&format!(";; effects: {}", eff_names.join(", ")));
        }

        // Build param list, filtering out zero-size types (capabilities, etc.).
        let params: Vec<String> = f
            .params
            .iter()
            .filter_map(|p| Self::val_type(&p.ty).map(|vt| format!("(param ${} {})", p.name, vt)))
            .collect();

        let result = Self::val_type(&f.return_type)
            .map(|vt| format!(" (result {})", vt))
            .unwrap_or_default();

        self.line(&format!(
            "(func ${name} (export \"{name}\") {params}{result}",
            name = f.name,
            params = params.join(" "),
            result = result,
        ));
        self.push();

        // Pre-scan body for local variables needed.
        self.collect_locals_from_block(&f.body);
        // Emit locals.
        for local in &self.locals.clone() {
            self.line(&format!("(local ${} {})", local.name, local.ty));
        }

        // Requires contracts.
        for c in &f.contracts {
            if c.kind == ContractKind::Requires {
                self.emit_contract(c);
            }
        }

        // Body.
        self.emit_block(&f.body);

        // Ensures as comments.
        for c in &f.contracts {
            if c.kind == ContractKind::Ensures {
                self.line(&format!(";; ensures: <contract>"));
            }
        }

        self.pop();
        self.line(")");
    }

    // -----------------------------------------------------------------------
    // Local variable collection
    // -----------------------------------------------------------------------

    fn collect_locals_from_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.collect_locals_from_stmt(stmt);
        }
    }

    fn collect_locals_from_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, ty, value: _ } => {
                if let Some(vt) = Self::val_type(ty) {
                    // Only add if not already declared.
                    if !self.locals.iter().any(|l| l.name == *name) {
                        self.add_local(name, vt);
                    }
                }
            }
            Stmt::If { then_block, else_block, .. } => {
                self.collect_locals_from_block(then_block);
                self.collect_locals_from_block(else_block);
            }
            Stmt::Match { arms, .. } => {
                for arm in arms {
                    self.collect_locals_from_arm(arm);
                }
            }
            _ => {}
        }
    }

    fn collect_locals_from_arm(&mut self, arm: &MatchArm) {
        self.collect_pattern_bindings(&arm.pattern);
        self.collect_locals_from_block(&arm.body);
    }

    fn collect_pattern_bindings(&mut self, pat: &Pattern) {
        match pat {
            Pattern::Bind(name) => {
                if !self.locals.iter().any(|l| l.name == *name) {
                    self.add_local(name, ValType::I32); // default to i32
                }
            }
            Pattern::Variant { fields, .. } => {
                for f in fields {
                    self.collect_pattern_bindings(f);
                }
            }
            Pattern::Tuple(pats) => {
                for p in pats {
                    self.collect_pattern_bindings(p);
                }
            }
            Pattern::Struct { fields, .. } => {
                for (_, p) in fields {
                    self.collect_pattern_bindings(p);
                }
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // Contracts
    // -----------------------------------------------------------------------

    fn emit_contract(&mut self, c: &Contract) {
        match c.policy {
            ContractPolicy::Always | ContractPolicy::Debug => {
                self.line(";; requires contract");
                self.emit_expr(&c.predicate);
                self.line("i32.eqz");
                self.line("(if (then (unreachable)))");
            }
            ContractPolicy::Sampled(_) => {
                self.line(";; sampled requires contract (skipped)");
            }
        }
    }

    // -----------------------------------------------------------------------
    // Block / Statements
    // -----------------------------------------------------------------------

    fn emit_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.emit_stmt(stmt);
        }
        if let Some(tail) = &block.expr {
            self.emit_expr(tail);
        }
    }

    fn emit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, value, .. } => {
                self.emit_expr(value);
                self.line(&format!("local.set ${name}"));
            }
            Stmt::Assign { target, value } => {
                self.emit_expr(value);
                self.line(&format!("local.set ${target}"));
            }
            Stmt::If { cond, then_block, else_block } => {
                self.emit_expr(cond);
                if Self::block_has_value(then_block) {
                    let vt = "i32"; // default
                    self.line(&format!("(if (result {vt})"));
                } else {
                    self.line("(if");
                }
                self.push();
                self.line("(then");
                self.push();
                self.emit_block(then_block);
                self.pop();
                self.line(")");
                self.line("(else");
                self.push();
                self.emit_block(else_block);
                self.pop();
                self.line(")");
                self.pop();
                self.line(")");
            }
            Stmt::Match { expr, arms } => {
                self.emit_match(expr, arms);
            }
            Stmt::Return(e) => {
                self.emit_expr(e);
                self.line("return");
            }
            Stmt::Assert { cond, message } => {
                self.emit_expr(cond);
                self.line("i32.eqz");
                if message.is_empty() {
                    self.line("(if (then (unreachable)))");
                } else {
                    self.line(&format!(";; assert: {message}"));
                    self.line("(if (then (unreachable)))");
                }
            }
            Stmt::Expr(e) => {
                self.emit_expr(e);
                // If the expression leaves a value on the stack, drop it.
                self.line("drop");
            }
        }
    }

    fn block_has_value(block: &Block) -> bool {
        block.expr.is_some()
    }

    fn emit_match(&mut self, expr: &Expr, arms: &[MatchArm]) {
        // Simple match lowering: emit as nested if-else on the discriminant.
        // For enum matching, we extract the tag and compare.
        self.line(";; match expression");
        self.emit_expr(expr);

        if arms.is_empty() {
            self.line("drop");
            return;
        }

        // Store scrutinee in a temp local.
        let scrutinee = self.fresh_label();
        let local_name = scrutinee.trim_start_matches('$');
        if !self.locals.iter().any(|l| l.name == local_name) {
            // Locals already collected; use the label as-is.
        }
        self.line(&format!(";; match scrutinee"));

        // Simple: just emit each arm as block.
        // For now, support simple patterns by emitting block/br_table.
        self.line("(block $match_end");
        self.push();
        for (i, arm) in arms.iter().enumerate() {
            self.line(&format!(";; arm {i}: {:?}", arm.pattern));
            self.emit_block(&arm.body);
            if i < arms.len() - 1 {
                self.line("br $match_end");
            }
        }
        self.pop();
        self.line(")");
    }

    // -----------------------------------------------------------------------
    // Expressions
    // -----------------------------------------------------------------------

    fn emit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal(lit) => self.emit_literal(lit),
            Expr::Var(name) => {
                self.line(&format!("local.get ${name}"));
            }
            Expr::BinOp { op, lhs, rhs } => {
                self.emit_expr(lhs);
                self.emit_expr(rhs);
                self.emit_binop(op, lhs);
            }
            Expr::UnOp { op, operand } => {
                self.emit_expr(operand);
                self.emit_unop(op, operand);
            }
            Expr::Call { func, args } => {
                for arg in args {
                    self.emit_expr(arg);
                }
                let func_name = func.join("_");
                self.line(&format!("call ${func_name}"));
            }
            Expr::StructLit { fields, .. } => {
                // Allocate struct in linear memory using the bump allocator.
                let total_size = fields.len() as u32 * 4; // rough: 4B per field
                self.line(&format!(";; struct literal ({}B)", total_size));
                // Push current sp as the struct pointer.
                self.line("global.get $sp");
                // Write each field.
                for (i, (name, val)) in fields.iter().enumerate() {
                    let offset = i as u32 * 4;
                    self.line(&format!(";; field {name} at offset {offset}"));
                    self.line("global.get $sp");
                    self.emit_expr(val);
                    self.line(&format!("i32.store offset={offset}"));
                }
                // Bump sp.
                self.line("global.get $sp");
                self.line(&format!("i32.const {total_size}"));
                self.line("i32.add");
                self.line("global.set $sp");
            }
            Expr::FieldGet { expr, field } => {
                self.line(&format!(";; field access .{field}"));
                self.emit_expr(expr);
                // For simplicity, field offset would be computed from type layout.
                // For now, treat first field as offset 0, etc.
                self.line("i32.load");
            }
            Expr::EnumLit { variant, .. } => {
                // Emit discriminant tag. For simplicity, use the variant name hash.
                // In a real backend, we'd assign sequential tags.
                self.line(&format!(";; enum variant {variant}"));
                self.line("i32.const 0"); // placeholder tag
            }
            Expr::TupleLit(elems) => {
                if elems.is_empty() {
                    // Unit — no value
                } else {
                    // Allocate tuple in memory.
                    self.line(";; tuple literal");
                    self.line("global.get $sp");
                    for (i, elem) in elems.iter().enumerate() {
                        let offset = i as u32 * 4;
                        self.line("global.get $sp");
                        self.emit_expr(elem);
                        self.line(&format!("i32.store offset={offset}"));
                    }
                    let total = elems.len() as u32 * 4;
                    self.line("global.get $sp");
                    self.line(&format!("i32.const {total}"));
                    self.line("i32.add");
                    self.line("global.set $sp");
                }
            }
            Expr::If { cond, then_block, else_block } => {
                self.emit_expr(cond);
                self.line("(if (result i32)");
                self.push();
                self.line("(then");
                self.push();
                self.emit_block(then_block);
                self.pop();
                self.line(")");
                self.line("(else");
                self.push();
                self.emit_block(else_block);
                self.pop();
                self.line(")");
                self.pop();
                self.line(")");
            }
            Expr::Match { expr: scrut, arms } => {
                self.emit_match(scrut, arms);
            }
            Expr::Block(block) => {
                self.emit_block(block);
            }
            Expr::Alloc { value, .. } => {
                // Allocate in linear memory (bump allocation).
                self.line(";; alloc");
                self.line("global.get $sp");
                self.emit_expr(value);
                self.line("i32.store");
                self.line("global.get $sp");
                // Bump by 4 bytes.
                self.line("global.get $sp");
                self.line("i32.const 4");
                self.line("i32.add");
                self.line("global.set $sp");
            }
            Expr::Borrow(inner) | Expr::BorrowMut(inner) => {
                // Borrows are just the same pointer in WASM land.
                self.emit_expr(inner);
            }
            Expr::Convert { expr, target } => {
                self.emit_expr(expr);
                self.emit_conversion(target);
            }
        }
    }

    fn emit_literal(&mut self, lit: &Literal) {
        match lit {
            Literal::Bool(b) => {
                self.line(&format!("i32.const {}", if *b { 1 } else { 0 }));
            }
            Literal::Int(n) => {
                // Decide i32 vs i64 based on magnitude.
                if *n >= i32::MIN as i128 && *n <= i32::MAX as i128 {
                    self.line(&format!("i32.const {n}"));
                } else {
                    self.line(&format!("i64.const {n}"));
                }
            }
            Literal::BigInt(s) => {
                self.line(&format!("i64.const {s}"));
            }
            Literal::F32(f) => {
                self.line(&format!("f32.const {f}"));
            }
            Literal::F64(f) => {
                self.line(&format!("f64.const {f}"));
            }
            Literal::String(s) => {
                let (offset, len) = self.alloc_string_data(s);
                self.line(&format!(";; string \"{s}\""));
                self.line(&format!("i32.const {offset}"));
                // Length is available but we just push the pointer for now.
                let _ = len;
            }
            Literal::Bytes(b) => {
                self.line(&format!(";; bytes ({} bytes)", b.len()));
                self.line("i32.const 0"); // placeholder
            }
            Literal::Unit => {
                // Unit has no representation; nothing to push.
            }
        }
    }

    fn emit_binop(&mut self, op: &BinOp, lhs: &Expr) {
        // Infer the type from the LHS to choose i32 vs i64 instructions.
        // For simplicity, default to i32 unless we see i64/f32/f64 literals.
        let prefix = self.infer_numeric_prefix(lhs);
        let instr = match op {
            BinOp::Add => format!("{prefix}.add"),
            BinOp::Sub => format!("{prefix}.sub"),
            BinOp::Mul => format!("{prefix}.mul"),
            BinOp::Div => {
                if prefix == "f32" || prefix == "f64" {
                    format!("{prefix}.div")
                } else {
                    format!("{prefix}.div_s")
                }
            }
            BinOp::Mod => format!("{prefix}.rem_s"),
            BinOp::BitAnd => format!("{prefix}.and"),
            BinOp::BitOr => format!("{prefix}.or"),
            BinOp::BitXor => format!("{prefix}.xor"),
            BinOp::Shl => format!("{prefix}.shl"),
            BinOp::Shr => format!("{prefix}.shr_s"),
            BinOp::Eq => format!("{prefix}.eq"),
            BinOp::Ne => format!("{prefix}.ne"),
            BinOp::Lt => {
                if prefix == "f32" || prefix == "f64" {
                    format!("{prefix}.lt")
                } else {
                    format!("{prefix}.lt_s")
                }
            }
            BinOp::Le => {
                if prefix == "f32" || prefix == "f64" {
                    format!("{prefix}.le")
                } else {
                    format!("{prefix}.le_s")
                }
            }
            BinOp::Gt => {
                if prefix == "f32" || prefix == "f64" {
                    format!("{prefix}.gt")
                } else {
                    format!("{prefix}.gt_s")
                }
            }
            BinOp::Ge => {
                if prefix == "f32" || prefix == "f64" {
                    format!("{prefix}.ge")
                } else {
                    format!("{prefix}.ge_s")
                }
            }
            BinOp::And => {
                // Logical AND: both sides are i32 booleans.
                "i32.and".to_string()
            }
            BinOp::Or => {
                "i32.or".to_string()
            }
        };
        self.line(&instr);
    }

    fn emit_unop(&mut self, op: &UnOp, operand: &Expr) {
        let prefix = self.infer_numeric_prefix(operand);
        match op {
            UnOp::Neg => {
                if prefix == "f32" || prefix == "f64" {
                    self.line(&format!("{prefix}.neg"));
                } else {
                    // WASM doesn't have i32.neg. Use: 0 - x.
                    // But value is already on stack, so we need to restructure.
                    // Emit: i32.const 0, swap, i32.sub — but WASM has no swap.
                    // Instead, let's use: i32.const -1, i32.mul.
                    self.line(&format!("{prefix}.const -1"));
                    self.line(&format!("{prefix}.mul"));
                }
            }
            UnOp::Not => {
                self.line("i32.eqz");
            }
            UnOp::BitNot => {
                self.line(&format!("{prefix}.const -1"));
                self.line(&format!("{prefix}.xor"));
            }
        }
    }

    fn infer_numeric_prefix(&self, expr: &Expr) -> &'static str {
        match expr {
            Expr::Literal(Literal::F32(_)) => "f32",
            Expr::Literal(Literal::F64(_)) => "f64",
            Expr::Literal(Literal::Int(n)) if *n > i32::MAX as i128 || *n < i32::MIN as i128 => "i64",
            Expr::Literal(Literal::BigInt(_)) => "i64",
            _ => "i32",
        }
    }

    fn emit_conversion(&mut self, target: &Type) {
        match Self::val_type(target) {
            Some(ValType::I32) => self.line("i32.wrap_i64"),
            Some(ValType::I64) => self.line("i64.extend_i32_s"),
            Some(ValType::F32) => self.line("f32.convert_i32_s"),
            Some(ValType::F64) => self.line("f64.convert_i32_s"),
            None => {} // Converting to unit/void — drop.
        }
    }

    // -----------------------------------------------------------------------
    // Data segments
    // -----------------------------------------------------------------------

    fn emit_data_segments(&mut self) {
        if self.data_segments.is_empty() {
            return;
        }
        self.blank();
        let segments = std::mem::take(&mut self.data_segments);
        self.line(";; Data segments");
        for (offset, data) in &segments {
            // Escape the string for WAT.
            let escaped = data
                .bytes()
                .map(|b| {
                    if b.is_ascii_graphic() || b == b' ' {
                        (b as char).to_string()
                    } else {
                        format!("\\{b:02x}")
                    }
                })
                .collect::<String>();
            self.line(&format!("(data (i32.const {offset}) \"{escaped}\")"));
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use speclang_ir::expr::{BinOp, Block, Expr, Literal, Stmt};
    use speclang_ir::contract::{Contract, ContractKind, ContractPolicy};
    use speclang_ir::module::{Function, Module, Param, TypeDef, ExternFunction};
    use speclang_ir::types::{Field, PrimitiveType, Region, Type, Variant};

    fn make_module(name: &str) -> Module {
        Module::new(vec![name.to_string()])
    }

    #[test]
    fn generates_module_wrapper() {
        let m = make_module("hello");
        let wat = generate_wasm(&m);
        assert!(wat.contains("(module $hello"), "got:\n{wat}");
        assert!(wat.contains(";; Generated from speclang module"), "got:\n{wat}");
        assert!(wat.ends_with(")\n"), "got:\n{wat}");
    }

    #[test]
    fn includes_wasi_imports() {
        let m = make_module("test");
        let wat = generate_wasm(&m);
        assert!(wat.contains("wasi_snapshot_preview1"), "got:\n{wat}");
        assert!(wat.contains("$fd_write"), "got:\n{wat}");
        assert!(wat.contains("$proc_exit"), "got:\n{wat}");
    }

    #[test]
    fn includes_memory_export() {
        let m = make_module("test");
        let wat = generate_wasm(&m);
        assert!(wat.contains("(memory (export \"memory\") 1)"), "got:\n{wat}");
    }

    #[test]
    fn generates_struct_type_comment() {
        let mut m = make_module("test");
        m.type_defs.push(TypeDef {
            name: "Point".into(),
            ty: Type::Struct(vec![
                Field { name: "x".into(), ty: Type::i32() },
                Field { name: "y".into(), ty: Type::i32() },
            ]),
            annotations: vec![],
        });
        let wat = generate_wasm(&m);
        assert!(wat.contains(";; struct Point"), "got:\n{wat}");
        assert!(wat.contains("x: i32"), "got:\n{wat}");
    }

    #[test]
    fn generates_enum_type_comment() {
        let mut m = make_module("test");
        m.type_defs.push(TypeDef {
            name: "Color".into(),
            ty: Type::Enum(vec![
                Variant { name: "Red".into(), fields: vec![] },
                Variant { name: "Green".into(), fields: vec![] },
                Variant { name: "Blue".into(), fields: vec![] },
            ]),
            annotations: vec![],
        });
        let wat = generate_wasm(&m);
        assert!(wat.contains(";; enum Color"), "got:\n{wat}");
        assert!(wat.contains("Red = 0"), "got:\n{wat}");
    }

    #[test]
    fn generates_simple_function() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "add".into(),
            params: vec![
                Param { name: "a".into(), ty: Type::i32() },
                Param { name: "b".into(), ty: Type::i32() },
            ],
            return_type: Type::i32(),
            effects: vec![],
            contracts: vec![],
            body: Block::new(
                vec![],
                Some(Expr::BinOp {
                    op: BinOp::Add,
                    lhs: Box::new(Expr::Var("a".into())),
                    rhs: Box::new(Expr::Var("b".into())),
                }),
            ),
            annotations: vec![],
        });
        let wat = generate_wasm(&m);
        assert!(wat.contains("(func $add (export \"add\")"), "got:\n{wat}");
        assert!(wat.contains("(param $a i32)"), "got:\n{wat}");
        assert!(wat.contains("(param $b i32)"), "got:\n{wat}");
        assert!(wat.contains("(result i32)"), "got:\n{wat}");
        assert!(wat.contains("local.get $a"), "got:\n{wat}");
        assert!(wat.contains("local.get $b"), "got:\n{wat}");
        assert!(wat.contains("i32.add"), "got:\n{wat}");
    }

    #[test]
    fn generates_function_with_locals() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "square".into(),
            params: vec![Param { name: "x".into(), ty: Type::i32() }],
            return_type: Type::i32(),
            effects: vec![],
            contracts: vec![],
            body: Block::new(
                vec![Stmt::Let {
                    name: "result".into(),
                    ty: Type::i32(),
                    value: Expr::BinOp {
                        op: BinOp::Mul,
                        lhs: Box::new(Expr::Var("x".into())),
                        rhs: Box::new(Expr::Var("x".into())),
                    },
                }],
                Some(Expr::Var("result".into())),
            ),
            annotations: vec![],
        });
        let wat = generate_wasm(&m);
        assert!(wat.contains("(local $result i32)"), "got:\n{wat}");
        assert!(wat.contains("local.set $result"), "got:\n{wat}");
        assert!(wat.contains("i32.mul"), "got:\n{wat}");
        assert!(wat.contains("local.get $result"), "got:\n{wat}");
    }

    #[test]
    fn generates_contract_trap() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "checked".into(),
            params: vec![Param { name: "x".into(), ty: Type::i32() }],
            return_type: Type::i32(),
            effects: vec![],
            contracts: vec![Contract {
                kind: ContractKind::Requires,
                predicate: Expr::BinOp {
                    op: BinOp::Gt,
                    lhs: Box::new(Expr::Var("x".into())),
                    rhs: Box::new(Expr::Literal(Literal::Int(0))),
                },
                policy: ContractPolicy::Debug,
                req_tags: vec![],
            }],
            body: Block::new(vec![], Some(Expr::Var("x".into()))),
            annotations: vec![],
        });
        let wat = generate_wasm(&m);
        assert!(wat.contains("unreachable"), "got:\n{wat}");
        assert!(wat.contains("i32.eqz"), "got:\n{wat}");
    }

    #[test]
    fn generates_if_statement() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "abs".into(),
            params: vec![Param { name: "x".into(), ty: Type::i32() }],
            return_type: Type::i32(),
            effects: vec![],
            contracts: vec![],
            body: Block::new(
                vec![Stmt::If {
                    cond: Expr::BinOp {
                        op: BinOp::Lt,
                        lhs: Box::new(Expr::Var("x".into())),
                        rhs: Box::new(Expr::Literal(Literal::Int(0))),
                    },
                    then_block: Block::new(
                        vec![],
                        Some(Expr::UnOp {
                            op: UnOp::Neg,
                            operand: Box::new(Expr::Var("x".into())),
                        }),
                    ),
                    else_block: Block::new(vec![], Some(Expr::Var("x".into()))),
                }],
                None,
            ),
            annotations: vec![],
        });
        let wat = generate_wasm(&m);
        assert!(wat.contains("(if"), "got:\n{wat}");
        assert!(wat.contains("(then"), "got:\n{wat}");
        assert!(wat.contains("(else"), "got:\n{wat}");
        assert!(wat.contains("i32.lt_s"), "got:\n{wat}");
    }

    #[test]
    fn generates_return_statement() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "early".into(),
            params: vec![],
            return_type: Type::i32(),
            effects: vec![],
            contracts: vec![],
            body: Block::new(
                vec![Stmt::Return(Expr::Literal(Literal::Int(42)))],
                None,
            ),
            annotations: vec![],
        });
        let wat = generate_wasm(&m);
        assert!(wat.contains("i32.const 42"), "got:\n{wat}");
        assert!(wat.contains("return"), "got:\n{wat}");
    }

    #[test]
    fn generates_assert() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "check".into(),
            params: vec![Param { name: "x".into(), ty: Type::i32() }],
            return_type: Type::Primitive(PrimitiveType::Unit),
            effects: vec![],
            contracts: vec![],
            body: Block::new(
                vec![Stmt::Assert {
                    cond: Expr::BinOp {
                        op: BinOp::Gt,
                        lhs: Box::new(Expr::Var("x".into())),
                        rhs: Box::new(Expr::Literal(Literal::Int(0))),
                    },
                    message: "must be positive".into(),
                }],
                None,
            ),
            annotations: vec![],
        });
        let wat = generate_wasm(&m);
        assert!(wat.contains("unreachable"), "got:\n{wat}");
        assert!(wat.contains(";; assert: must be positive"), "got:\n{wat}");
    }

    #[test]
    fn generates_string_literal() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "greet".into(),
            params: vec![],
            return_type: Type::Primitive(PrimitiveType::String),
            effects: vec![],
            contracts: vec![],
            body: Block::new(
                vec![],
                Some(Expr::Literal(Literal::String("hello".into()))),
            ),
            annotations: vec![],
        });
        let wat = generate_wasm(&m);
        assert!(wat.contains("(data (i32.const"), "got:\n{wat}");
        assert!(wat.contains("hello"), "got:\n{wat}");
    }

    #[test]
    fn generates_call() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "double".into(),
            params: vec![Param { name: "x".into(), ty: Type::i32() }],
            return_type: Type::i32(),
            effects: vec![],
            contracts: vec![],
            body: Block::new(
                vec![],
                Some(Expr::Call {
                    func: vec!["add".into()],
                    args: vec![
                        Expr::Var("x".into()),
                        Expr::Var("x".into()),
                    ],
                }),
            ),
            annotations: vec![],
        });
        let wat = generate_wasm(&m);
        assert!(wat.contains("call $add"), "got:\n{wat}");
    }

    #[test]
    fn generates_owned_alloc() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "make_box".into(),
            params: vec![],
            return_type: Type::own(Region::Heap, Type::i32()),
            effects: vec![],
            contracts: vec![],
            body: Block::new(
                vec![],
                Some(Expr::Alloc {
                    region: Box::new(Expr::Literal(Literal::Unit)),
                    ty: Type::i32(),
                    value: Box::new(Expr::Literal(Literal::Int(42))),
                }),
            ),
            annotations: vec![],
        });
        let wat = generate_wasm(&m);
        assert!(wat.contains(";; alloc"), "got:\n{wat}");
        assert!(wat.contains("global.get $sp"), "got:\n{wat}");
        assert!(wat.contains("i32.const 42"), "got:\n{wat}");
        assert!(wat.contains("i32.store"), "got:\n{wat}");
    }

    #[test]
    fn capability_params_elided() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "net_call".into(),
            params: vec![
                Param { name: "cap".into(), ty: Type::Capability("Net".into()) },
                Param { name: "x".into(), ty: Type::i32() },
            ],
            return_type: Type::i32(),
            effects: vec![],
            contracts: vec![],
            body: Block::new(vec![], Some(Expr::Var("x".into()))),
            annotations: vec![],
        });
        let wat = generate_wasm(&m);
        // Capability params should NOT appear in the WASM signature.
        assert!(!wat.contains("$cap"), "capability param should be elided, got:\n{wat}");
        assert!(wat.contains("(param $x i32)"), "got:\n{wat}");
    }

    #[test]
    fn extern_imports() {
        let mut m = make_module("test");
        m.externs.push(ExternFunction {
            name: "get_time".into(),
            params: vec![],
            return_type: Type::i64(),
            effects: vec![],
            annotations: vec![],
        });
        let wat = generate_wasm(&m);
        assert!(wat.contains("(import \"env\" \"get_time\""), "got:\n{wat}");
        assert!(wat.contains("(result i64)"), "got:\n{wat}");
    }

    #[test]
    fn val_type_mapping() {
        assert_eq!(WasmCodeGen::val_type(&Type::i32()), Some(ValType::I32));
        assert_eq!(WasmCodeGen::val_type(&Type::i64()), Some(ValType::I64));
        assert_eq!(WasmCodeGen::val_type(&Type::Primitive(PrimitiveType::F32)), Some(ValType::F32));
        assert_eq!(WasmCodeGen::val_type(&Type::Primitive(PrimitiveType::F64)), Some(ValType::F64));
        assert_eq!(WasmCodeGen::val_type(&Type::Primitive(PrimitiveType::Bool)), Some(ValType::I32));
        assert_eq!(WasmCodeGen::val_type(&Type::Primitive(PrimitiveType::Unit)), None);
        assert_eq!(WasmCodeGen::val_type(&Type::Capability("X".into())), None);
    }

    #[test]
    fn type_size_calculation() {
        assert_eq!(WasmCodeGen::type_size(&Type::Primitive(PrimitiveType::Bool)), 1);
        assert_eq!(WasmCodeGen::type_size(&Type::i32()), 4);
        assert_eq!(WasmCodeGen::type_size(&Type::i64()), 8);
        assert_eq!(WasmCodeGen::type_size(&Type::Primitive(PrimitiveType::F64)), 8);
        assert_eq!(WasmCodeGen::type_size(&Type::Primitive(PrimitiveType::Unit)), 0);
    }

    #[test]
    fn generates_float_operations() {
        let mut m = make_module("test");
        m.functions.push(Function {
            name: "fadd".into(),
            params: vec![
                Param { name: "a".into(), ty: Type::Primitive(PrimitiveType::F64) },
                Param { name: "b".into(), ty: Type::Primitive(PrimitiveType::F64) },
            ],
            return_type: Type::Primitive(PrimitiveType::F64),
            effects: vec![],
            contracts: vec![],
            body: Block::new(
                vec![],
                Some(Expr::BinOp {
                    op: BinOp::Add,
                    lhs: Box::new(Expr::Literal(Literal::F64(1.0))),
                    rhs: Box::new(Expr::Literal(Literal::F64(2.0))),
                }),
            ),
            annotations: vec![],
        });
        let wat = generate_wasm(&m);
        assert!(wat.contains("f64.const"), "got:\n{wat}");
        assert!(wat.contains("f64.add"), "got:\n{wat}");
    }
}
