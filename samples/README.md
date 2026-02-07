# Sample SPL Files

This directory contains working SPL specification files that can be
compiled through the full speclang pipeline.

## Files

### `hello.spl` — Minimal starter

A minimal SPL module demonstrating types, a function spec with
contracts and examples. Good for getting started.

```bash
# Parse and print the AST
cargo run -- parse samples/hello.spl

# Type-check
cargo run -- check samples/hello.spl

# Compile to Rust
cargo run -- compile samples/hello.spl

# Compile to WebAssembly (WAT)
cargo run -- wasm samples/hello.spl

# List test cases and requirement coverage
cargo run -- test samples/hello.spl

# Format the source
cargo run -- fmt samples/hello.spl
```

### `music.spl` — Full-featured example

A complete SPL module exercising most language features: refined types,
generators, decisions, function specs with contracts/examples/effects,
universally-quantified properties, oracles, and package-level policy.

```bash
cargo run -- compile samples/music.spl
cargo run -- wasm samples/music.spl
cargo run -- test samples/music.spl
```

## What each command does

| Command   | Pipeline                                           | Output |
|-----------|----------------------------------------------------|--------|
| `parse`   | SPL parse                                          | Debug-printed AST |
| `check`   | Parse → resolve → type-check                      | "ok" or diagnostics |
| `compile` | Parse → check → lower → verify → Rust codegen     | Rust source code |
| `wasm`    | Parse → check → lower → verify → WASM codegen     | WAT (WebAssembly Text) |
| `test`    | Parse → check → lower → list tests & requirements | Test listing |
| `fmt`     | Parse → reformat                                   | Formatted SPL source |

## Contract modes

The `compile` and `wasm` commands accept a `--mode` flag:

```bash
# All contracts checked at runtime (default)
cargo run -- compile --mode debug samples/hello.spl

# Contracts checked probabilistically
cargo run -- compile --mode sampled samples/hello.spl

# No runtime contract checks
cargo run -- compile --mode release samples/hello.spl
```
