# speclang

A two-layer systems programming language where **specifications are the source of truth**.

SPL (Spec Layer) is a purely declarative language for expressing types, contracts,
capabilities, and executable examples. IMPL (Implementation Layer) is a systems
language with ownership, regions, and explicit effects that *binds* to SPL
specifications. The compiler verifies that every implementation conforms to its
spec — signatures match, effects are contained, and contracts hold.

## Why?

Traditional development treats tests as the primary correctness artifact.
speclang treats **executable specifications** as the canonical truth:

- **SPL files** declare *what* a module does — types, contracts, error
  taxonomy, required capabilities, and examples.
- **IMPL files** declare *how* — with ownership, borrowing, regions, and
  explicit I/O capabilities.
- The compiler checks that IMPL satisfies SPL, generates test harnesses from
  examples and properties, and transpiles to Rust or WebAssembly.

The result: humans review compact, readable specs; code is a derived artifact
that the toolchain can verify automatically.

## Quick start

```bash
# Build the compiler
cargo build --release

# Type-check a sample spec
cargo run -- check samples/hello.spl

# Compile to Rust
cargo run -- compile samples/hello.spl

# Compile to WebAssembly (WAT)
cargo run -- wasm samples/hello.spl

# List generated tests and requirement coverage
cargo run -- test samples/hello.spl

# Format the source
cargo run -- fmt samples/hello.spl
```

See [`samples/`](samples/) for complete working examples with expected
output — both a minimal starter (`hello.spl`) and a full-featured spec
(`music.spl`) that exercises refined types, generators, properties,
oracles, and policy.

## Language overview

### SPL — the Spec Layer

SPL is declarative: no loops, no mutation, no I/O. It defines checkable
specifications that read like structured requirements.

```text
module music

type MidiNote = refine Int where 1 <= self && self <= 12

fn snap_to_scale(note: MidiNote, scale: Set[MidiNote]) -> MidiNote
  requires not is_empty(scale)
  ensures  contains(scale, result)
  examples
    snap_to_scale(12, {1, 5, 8}) == 1
    snap_to_scale(1,  {1, 5, 8}) == 1
```

SPL constructs include: `type`, `fn`, `refine`, `error`, `capability`,
`law`/`prop`, `examples`, `perf`, `req`, `decision`, `gen`, `oracle`,
and `policy`.

### IMPL — the Implementation Layer

IMPL is a minimal systems language with ownership semantics. It binds to
SPL-declared stable IDs:

```text
impl fn "music.snap.v1"(note: MidiNote, scale: ref[Set[MidiNote]]) -> MidiNote {
    let closest = scale[0];
    let best = distance_mod12(closest, note);
    for candidate in scale {
        let d = distance_mod12(candidate, note);
        if d < best || (d == best && candidate < closest) {
            closest = candidate;
            best = d;
        }
    }
    closest
}
```

The compiler checks:
- Signature matches the SPL spec
- Effects used ⊆ effects declared in SPL
- Contracts and examples pass via generated harnesses

### Core IR

Between SPL/IMPL and the backends sits a small, explicit intermediate
representation. Core IR makes every allocation, copy, and capability token
visible.  See [docs/core-ir-overview.md](docs/core-ir-overview.md) for the
full specification.

### Capabilities and effects

Functions declare what side effects they may perform. The default is pure
(no effects). Capabilities are passed as explicit tokens:

```text
fn fetch_config(net: cap Net) -> Config
  effects { Net(host(url)) }
  forbids { FileRead, Clock }
```

Package-level `policy` blocks restrict what capabilities any module in the
package may request, providing a trust boundary for agent-generated code.

## Architecture

speclang is implemented as a Rust workspace with 13 crates:

```
┌─────────────────────────────────────────────────────────┐
│                     speclang-cli                        │
│              (parse, check, compile, wasm, fmt, ir)     │
└──────┬──────────┬──────────┬───────────┬────────────────┘
       │          │          │           │
 ┌─────▼────┐ ┌──▼───┐ ┌───▼────┐ ┌────▼──────┐
 │  spl     │ │ impl │ │ stdlib │ │ ir-parser │
 │ (parse,  │ │      │ │        │ │           │
 │  check)  │ │      │ │        │ │           │
 └─────┬────┘ └──┬───┘ └───┬────┘ └───────────┘
       │         │         │
  ┌────▼─────────▼─────────▼────┐
  │          lower              │
  └──────────┬──────────────────┘
             │
  ┌──────────▼──────────────────┐
  │          verify             │
  │  (typecheck, contracts,     │
  │   ownership, capabilities,  │
  │   proptest, fuzz)           │
  └──────────┬──────────────────┘
             │
  ┌──────────▼────┬─────────────┐
  │ backend-rust  │ backend-wasm│
  └───────────────┴─────────────┘
```

Supporting crates:

| Crate | Purpose |
|-------|---------|
| `speclang-ir` | Core IR types, expressions, modules, contracts |
| `speclang-ir-parser` | Textual Core IR parser and pretty-printer |
| `speclang-spl` | SPL lexer, parser, resolver, and type checker |
| `speclang-impl` | IMPL lexer, parser, spec binding, effects checker |
| `speclang-lower` | SPL → Core IR lowering |
| `speclang-verify` | IR type checking, contracts, ownership, exhaustiveness, proptest, fuzz |
| `speclang-stdlib` | Standard library modules (core, math, mem, text, bytes, collections, contracts) |
| `speclang-backend-rust` | Core IR → idiomatic Rust source transpiler |
| `speclang-backend-wasm` | Core IR → WebAssembly Text (WAT) with WASI preview-1 |
| `speclang-diagnostic` | Structured diagnostics with source-annotated rendering |
| `speclang-fmt` | Canonical SPL and IMPL source formatter |
| `speclang-pkg` | `pkg.toml` manifest parsing and dependency resolution |
| `speclang-cli` | CLI compiler driver |

## Compilation pipeline

```
  .spl file
     │
     ▼
  ┌──────────┐    ┌──────────┐
  │  parse   │───▶│ resolve  │───▶ type-check
  └──────────┘    └──────────┘        │
                                      ▼
                                   lower (SPL → Core IR)
                                      │
                                      ▼
                                   verify (type-check IR,
                                           contracts,
                                           capabilities)
                                      │
                            ┌─────────┴──────────┐
                            ▼                    ▼
                       Rust codegen         WASM codegen
                            │                    │
                            ▼                    ▼
                        .rs file             .wat file
```

## Testing

```bash
# Run all 327 tests across all crates
cargo test

# Run tests for a specific crate
cargo test -p speclang-spl
cargo test -p speclang-verify
```

The test suite covers:
- **SPL parsing and type checking** — 54 tests
- **IMPL parsing, binding, and effects** — 61 tests
- **Core IR verification** — 59 tests (including property tests and fuzzing)
- **Standard library** — 36 tests
- **IR parser round-tripping** — 21 tests
- **Rust backend codegen** — 15 tests
- **WASM backend codegen** — 19 tests
- **Diagnostics** — 15 tests
- **Formatter** — 19 tests
- **Package manifest** — 16 tests
- **SPL → IR lowering** — 12 tests

## Documentation

Design documents live in [`docs/`](docs/):

- [System overview](docs/system-overview.md) — two-layer architecture, SPL vs IMPL, effects
- [Design principles](docs/design-principles.md) — readability, examples-first, no-how-in-spec
- [Core IR overview](docs/core-ir-overview.md) — type system, expressions, contracts, lowering targets
- [Core IR grammar](docs/ir-grammar.md) — textual syntax for Core IR
- [Standard library](docs/stdlib-v0.md) — v0 stdlib surface (Option, Result, math, mem, text, collections)
- [Workflow](docs/workflow.md) — human-agent workflow with SPL as executable requirements

## License

MIT
