//! Pretty-printer for Core IR textual form.
//!
//! Converts Core IR AST back to canonical textual representation.

use speclang_ir::capability::CapabilityDef;
use speclang_ir::expr::{BinOp, Block, Expr, Literal, Stmt, UnOp};
use speclang_ir::module::{Annotation, ExternFunction, Function, Module, TypeDef};
use speclang_ir::types::{self, Type};

/// Pretty-print a module to a string.
pub fn print_module(module: &Module) -> String {
    let mut printer = Printer::new();
    printer.print_module(module);
    printer.output
}

struct Printer {
    output: String,
    indent: usize,
}

impl Printer {
    fn new() -> Self {
        Printer {
            output: String::new(),
            indent: 0,
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn writeln(&mut self, s: &str) {
        self.write_indent();
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
    }

    fn print_module(&mut self, module: &Module) {
        self.write("module ");
        self.write(&types::qname_to_string(&module.name));
        self.write(" {\n");
        self.indent += 1;

        for td in &module.type_defs {
            self.print_type_def(td);
        }
        for cd in &module.cap_defs {
            self.print_cap_def(cd);
        }
        for ext in &module.externs {
            self.print_extern(ext);
        }
        for func in &module.functions {
            self.print_function(func);
        }

        self.indent -= 1;
        self.writeln("}");
    }

    fn print_type_def(&mut self, td: &TypeDef) {
        self.write_indent();
        self.write("type ");
        self.write(&td.name);
        self.write(" = ");
        self.print_type(&td.ty);
        self.write(";\n");
    }

    fn print_cap_def(&mut self, cd: &CapabilityDef) {
        self.write_indent();
        self.write("cap ");
        self.write(&cd.name);
        if !cd.fields.is_empty() {
            self.write("(");
            for (i, f) in cd.fields.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write(&f.name);
                self.write(": ");
                self.print_type(&f.ty);
            }
            self.write(")");
        }
        self.write(";\n");
    }

    fn print_extern(&mut self, ext: &ExternFunction) {
        self.write_indent();
        self.write("extern fn ");
        self.write(&ext.name);
        self.write("(");
        self.print_params(&ext.params);
        self.write(") -> ");
        self.print_type(&ext.return_type);
        if !ext.effects.is_empty() {
            self.write(" effects(");
            for (i, eff) in ext.effects.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write(&eff.name);
            }
            self.write(")");
        }
        self.write(";\n");
    }

    fn print_function(&mut self, func: &Function) {
        self.write_indent();
        self.write("fn ");
        self.write(&func.name);
        self.write("(");
        self.print_params(&func.params);
        self.write(") -> ");
        self.print_type(&func.return_type);
        if !func.effects.is_empty() {
            self.write(" effects(");
            for (i, eff) in func.effects.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write(&eff.name);
            }
            self.write(")");
        }
        // Print annotations
        for ann in &func.annotations {
            match ann {
                Annotation::Id(id) => {
                    self.write("\n");
                    self.write_indent();
                    self.write(&format!("@id \"{id}\""));
                }
                Annotation::Compat(c) => {
                    self.write("\n");
                    self.write_indent();
                    self.write(&format!("@compat {:?}", c));
                }
                Annotation::ReqTag(tag) => {
                    self.write("\n");
                    self.write_indent();
                    self.write(&format!("@req_tag \"{tag}\""));
                }
            }
        }
        self.write(" ");
        self.print_block(&func.body);
        self.write("\n");
    }

    fn print_params(&mut self, params: &[speclang_ir::module::Param]) {
        for (i, p) in params.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.write(&p.name);
            self.write(": ");
            self.print_type(&p.ty);
        }
    }

    fn print_type(&mut self, ty: &Type) {
        match ty {
            Type::Primitive(p) => self.write(&p.to_string()),
            Type::Struct(fields) => {
                self.write("struct { ");
                for (i, f) in fields.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&f.name);
                    self.write(": ");
                    self.print_type(&f.ty);
                }
                self.write(" }");
            }
            Type::Enum(variants) => {
                self.write("enum { ");
                for (i, v) in variants.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&v.name);
                    if !v.fields.is_empty() {
                        self.write("(");
                        for (j, f) in v.fields.iter().enumerate() {
                            if j > 0 {
                                self.write(", ");
                            }
                            self.print_type(f);
                        }
                        self.write(")");
                    }
                }
                self.write(" }");
            }
            Type::Tuple(types) => {
                self.write("(");
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.print_type(t);
                }
                self.write(")");
            }
            Type::Own { region, inner } => {
                self.write("own[");
                self.write(&region.to_string());
                self.write(", ");
                self.print_type(inner);
                self.write("]");
            }
            Type::Ref(inner) => {
                self.write("ref[");
                self.print_type(inner);
                self.write("]");
            }
            Type::MutRef(inner) => {
                self.write("mutref[");
                self.print_type(inner);
                self.write("]");
            }
            Type::Slice(inner) => {
                self.write("slice[");
                self.print_type(inner);
                self.write("]");
            }
            Type::MutSlice(inner) => {
                self.write("mutslice[");
                self.print_type(inner);
                self.write("]");
            }
            Type::Named(name) => {
                self.write(&types::qname_to_string(name));
            }
            Type::Generic { name, args } => {
                self.write(&types::qname_to_string(name));
                self.write("[");
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.print_type(a);
                }
                self.write("]");
            }
            Type::Option(inner) => {
                self.write("Option[");
                self.print_type(inner);
                self.write("]");
            }
            Type::Result { ok, err } => {
                self.write("Result[");
                self.print_type(ok);
                self.write(", ");
                self.print_type(err);
                self.write("]");
            }
            Type::Capability(name) => {
                self.write("cap.");
                self.write(name);
            }
            Type::Region => {
                self.write("region");
            }
        }
    }

    fn print_block(&mut self, block: &Block) {
        self.write("{\n");
        self.indent += 1;
        for stmt in &block.stmts {
            self.print_stmt(stmt);
        }
        if let Some(expr) = &block.expr {
            self.write_indent();
            self.print_expr(expr);
            self.write("\n");
        }
        self.indent -= 1;
        self.write_indent();
        self.write("}");
    }

    fn print_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, ty, value } => {
                self.write_indent();
                self.write("let ");
                self.write(name);
                self.write(": ");
                self.print_type(ty);
                self.write(" = ");
                self.print_expr(value);
                self.write(";\n");
            }
            Stmt::Assign { target, value } => {
                self.write_indent();
                self.write(target);
                self.write(" = ");
                self.print_expr(value);
                self.write(";\n");
            }
            Stmt::Return(expr) => {
                self.write_indent();
                self.write("return ");
                self.print_expr(expr);
                self.write(";\n");
            }
            Stmt::Assert { cond, message } => {
                self.write_indent();
                self.write("assert(");
                self.print_expr(cond);
                self.write(", \"");
                self.write(message);
                self.write("\");\n");
            }
            Stmt::Expr(expr) => {
                self.write_indent();
                self.print_expr(expr);
                self.write(";\n");
            }
            Stmt::If { cond, then_block, else_block } => {
                self.write_indent();
                self.write("if ");
                self.print_expr(cond);
                self.write(" ");
                self.print_block(then_block);
                self.write(" else ");
                self.print_block(else_block);
                self.write("\n");
            }
            Stmt::Match { expr, arms } => {
                self.write_indent();
                self.write("match ");
                self.print_expr(expr);
                self.write(" {\n");
                self.indent += 1;
                for arm in arms {
                    self.write_indent();
                    self.write("_ => ");
                    self.print_block(&arm.body);
                    self.write("\n");
                }
                self.indent -= 1;
                self.write_indent();
                self.write("}\n");
            }
        }
    }

    fn print_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Bool(b) => self.write(&b.to_string()),
                Literal::Int(i) => self.write(&i.to_string()),
                Literal::BigInt(s) => self.write(s),
                Literal::F32(f) => self.write(&f.to_string()),
                Literal::F64(f) => self.write(&f.to_string()),
                Literal::String(s) => {
                    self.write("\"");
                    self.write(s);
                    self.write("\"");
                }
                Literal::Bytes(_) => self.write("<bytes>"),
                Literal::Unit => self.write("()"),
            },
            Expr::Var(name) => self.write(name),
            Expr::BinOp { op, lhs, rhs } => {
                self.write("(");
                self.print_expr(lhs);
                let op_str = match op {
                    BinOp::Add => " + ",
                    BinOp::Sub => " - ",
                    BinOp::Mul => " * ",
                    BinOp::Div => " / ",
                    BinOp::Mod => " % ",
                    BinOp::BitAnd => " & ",
                    BinOp::BitOr => " | ",
                    BinOp::BitXor => " ^ ",
                    BinOp::Shl => " << ",
                    BinOp::Shr => " >> ",
                    BinOp::Eq => " == ",
                    BinOp::Ne => " != ",
                    BinOp::Lt => " < ",
                    BinOp::Le => " <= ",
                    BinOp::Gt => " > ",
                    BinOp::Ge => " >= ",
                    BinOp::And => " && ",
                    BinOp::Or => " || ",
                };
                self.write(op_str);
                self.print_expr(rhs);
                self.write(")");
            }
            Expr::UnOp { op, operand } => {
                let op_str = match op {
                    UnOp::Neg => "-",
                    UnOp::Not => "!",
                    UnOp::BitNot => "~",
                };
                self.write(op_str);
                self.print_expr(operand);
            }
            Expr::Call { func, args } => {
                self.write("call ");
                self.write(&types::qname_to_string(func));
                self.write("(");
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.print_expr(a);
                }
                self.write(")");
            }
            _ => self.write("<expr>"),
        }
    }
}
