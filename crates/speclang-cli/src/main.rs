//! speclang CLI compiler driver.
//!
//! Subcommands:
//! - `parse`   — Parse an SPL file and print the AST
//! - `check`   — Parse, resolve, and type-check an SPL file
//! - `compile` — Full pipeline: parse → check → lower → verify → codegen
//! - `test`    — Extract and list test cases from a compiled module
//! - `ir`      — Parse a Core IR file and pretty-print it

use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let result = match args[1].as_str() {
        "parse" => cmd_parse(&args[2..]),
        "check" => cmd_check(&args[2..]),
        "compile" => cmd_compile(&args[2..]),
        "test" => cmd_test(&args[2..]),
        "ir" => cmd_ir(&args[2..]),
        "version" | "--version" | "-V" => {
            println!("speclang v{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => {
            eprintln!("error: unknown command '{}'", other);
            print_usage();
            Err(1)
        }
    };

    if let Err(code) = result {
        process::exit(code);
    }
}

fn print_usage() {
    eprintln!(
        "speclang v{} — The speclang compiler

USAGE:
    speclang <COMMAND> [OPTIONS] <FILE>

COMMANDS:
    parse     Parse an SPL file and print the AST
    check     Parse, resolve, and type-check an SPL file
    compile   Full pipeline: parse → lower → verify → generate Rust
    test      Extract and list test cases from a compiled module
    ir        Parse a Core IR file and pretty-print it
    version   Print version information
    help      Print this help message

OPTIONS:
    --mode debug|release|sampled    Contract compilation mode (default: debug)
    -o, --output <FILE>             Output file (default: stdout)",
        env!("CARGO_PKG_VERSION")
    );
}

fn find_flag<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2).find_map(|w| {
        if w[0] == flag {
            Some(w[1].as_str())
        } else {
            None
        }
    })
}

fn find_output(args: &[String]) -> Option<&str> {
    find_flag(args, "-o").or_else(|| find_flag(args, "--output"))
}

fn get_input_path(args: &[String]) -> Option<&str> {
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "-o" || arg == "--output" || arg == "--mode" {
            skip_next = true;
            continue;
        }
        if !arg.starts_with('-') {
            return Some(arg.as_str());
        }
    }
    None
}

fn read_input_smart(args: &[String]) -> Result<(String, String), i32> {
    match get_input_path(args) {
        Some(path) => match fs::read_to_string(path) {
            Ok(source) => Ok((path.to_string(), source)),
            Err(e) => {
                eprintln!("error: cannot read '{}': {}", path, e);
                Err(1)
            }
        },
        None => {
            eprintln!("error: no input file specified");
            Err(1)
        }
    }
}

// -----------------------------------------------------------------------
// parse — Parse an SPL file and print the AST
// -----------------------------------------------------------------------

fn cmd_parse(args: &[String]) -> Result<(), i32> {
    let (path, source) = read_input_smart(args)?;

    match speclang_spl::parser::parse_program(&source) {
        Ok(program) => {
            println!("{:#?}", program);
            Ok(())
        }
        Err(e) => {
            eprintln!("{}:{}", path, e);
            Err(1)
        }
    }
}

// -----------------------------------------------------------------------
// check — Parse, resolve, and type-check
// -----------------------------------------------------------------------

fn cmd_check(args: &[String]) -> Result<(), i32> {
    let (path, source) = read_input_smart(args)?;

    let program = match speclang_spl::parser::parse_program(&source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}:parse error: {}", path, e);
            return Err(1);
        }
    };

    let resolved = match speclang_spl::resolve::resolve(&program) {
        Ok(r) => r,
        Err(errors) => {
            for e in &errors {
                eprintln!("{}:resolve error: {:?}", path, e);
            }
            return Err(1);
        }
    };

    match speclang_spl::typecheck::typecheck(&resolved) {
        Ok(()) => {
            println!("{}: ok", path);
            Ok(())
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}:type error: {:?}", path, e);
            }
            Err(1)
        }
    }
}

// -----------------------------------------------------------------------
// compile — Full pipeline
// -----------------------------------------------------------------------

fn cmd_compile(args: &[String]) -> Result<(), i32> {
    let (path, source) = read_input_smart(args)?;
    let output_path = find_output(args);
    let mode_str = find_flag(args, "--mode").unwrap_or("debug");

    let mode = match mode_str {
        "debug" => speclang_verify::contract_pass::CompilationMode::Debug,
        "release" => speclang_verify::contract_pass::CompilationMode::Release,
        "sampled" => speclang_verify::contract_pass::CompilationMode::ReleaseSampled,
        other => {
            eprintln!(
                "error: unknown compilation mode '{}' (expected debug/release/sampled)",
                other
            );
            return Err(1);
        }
    };

    // 1. Parse
    let program = match speclang_spl::parser::parse_program(&source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}:parse error: {}", path, e);
            return Err(1);
        }
    };

    // 2. Resolve names
    let resolved = match speclang_spl::resolve::resolve(&program) {
        Ok(r) => r,
        Err(errors) => {
            for e in &errors {
                eprintln!("{}:resolve error: {:?}", path, e);
            }
            return Err(1);
        }
    };

    // 3. Type-check
    if let Err(errors) = speclang_spl::typecheck::typecheck(&resolved) {
        for e in &errors {
            eprintln!("{}:type error: {:?}", path, e);
        }
        return Err(1);
    }

    // 4. Lower to Core IR
    let ir_module = match speclang_lower::lower::lower(&resolved) {
        Ok(m) => m,
        Err(errors) => {
            for e in &errors {
                eprintln!("{}:lower error: {:?}", path, e);
            }
            return Err(1);
        }
    };

    // 5. Verify Core IR
    if let Err(errors) = speclang_verify::typecheck::verify_module(&ir_module) {
        for e in &errors {
            eprintln!("{}:verify error: {:?}", path, e);
        }
        return Err(1);
    }

    // 6. Insert contract assertions
    let ir_with_contracts =
        speclang_verify::contract_pass::insert_contracts(&ir_module, mode);

    // 7. Generate Rust code
    let codegen = speclang_backend_rust::codegen::RustCodeGen::new();
    let rust_source = codegen.generate(&ir_with_contracts);

    // Output
    match output_path {
        Some(out) => {
            if let Err(e) = fs::write(out, &rust_source) {
                eprintln!("error: cannot write '{}': {}", out, e);
                return Err(1);
            }
            eprintln!("{}: compiled to {}", path, out);
        }
        None => {
            print!("{}", rust_source);
        }
    }

    Ok(())
}

// -----------------------------------------------------------------------
// test — Extract and list test cases
// -----------------------------------------------------------------------

fn cmd_test(args: &[String]) -> Result<(), i32> {
    let (path, source) = read_input_smart(args)?;

    // Parse → resolve → lower
    let program = match speclang_spl::parser::parse_program(&source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}:parse error: {}", path, e);
            return Err(1);
        }
    };

    let resolved = match speclang_spl::resolve::resolve(&program) {
        Ok(r) => r,
        Err(errors) => {
            for e in &errors {
                eprintln!("{}:resolve error: {:?}", path, e);
            }
            return Err(1);
        }
    };

    let ir_module = match speclang_lower::lower::lower(&resolved) {
        Ok(m) => m,
        Err(errors) => {
            for e in &errors {
                eprintln!("{}:lower error: {:?}", path, e);
            }
            return Err(1);
        }
    };

    // Extract tests
    let tests = speclang_verify::proptest::extract_tests(&ir_module);
    if tests.is_empty() {
        println!("{}: no tests found", path);
        return Ok(());
    }

    println!("{}: {} test(s) found\n", path, tests.len());
    for test in &tests {
        let kind = match test.kind {
            speclang_verify::proptest::TestKind::Example => "example",
            speclang_verify::proptest::TestKind::Property => "property",
            speclang_verify::proptest::TestKind::Oracle => "oracle",
        };
        let tags = if test.req_tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", test.req_tags.join(", "))
        };
        println!("  {} ({}){}", test.name, kind, tags);
    }

    // Requirement coverage
    let coverage = speclang_verify::proptest::requirement_coverage(&tests);
    if !coverage.is_empty() {
        println!("\nRequirement coverage:");
        for (tag, test_names) in &coverage {
            println!("  {} → {}", tag, test_names.join(", "));
        }
    }

    // Fuzz targets
    let fuzz_targets = speclang_verify::fuzz::generate_fuzz_targets(&ir_module);
    if !fuzz_targets.is_empty() {
        println!("\nFuzz targets ({}):", fuzz_targets.len());
        for target in &fuzz_targets {
            println!("  {} (from {})", target.name, target.source_function);
        }
    }

    Ok(())
}

// -----------------------------------------------------------------------
// ir — Parse and pretty-print Core IR
// -----------------------------------------------------------------------

fn cmd_ir(args: &[String]) -> Result<(), i32> {
    let (path, source) = read_input_smart(args)?;

    match speclang_ir_parser::parse_module(&source) {
        Ok(module) => {
            let output = speclang_ir_parser::print_module(&module);
            print!("{}", output);
            Ok(())
        }
        Err(e) => {
            eprintln!("{}:IR parse error: {}", path, e);
            Err(1)
        }
    }
}
