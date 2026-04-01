# speclang

A two-layer systems programming language where **specifications are the
source of truth**.

Humans write (or more likely agents write, humans check) short, readable `.spl` specs — types, contracts,
examples.  Agents write `.impl` code.  The compiler verifies that every
implementation conforms to its spec, generates test harnesses from the
spec, and transpiles everything to Rust or WebAssembly.

**The key insight**: you review the spec (20 lines), not the code (200 lines).
The toolchain proves they match.

This is an early experiment. It's usable for specifying algothmic style code
but needs more generality.

## Quick start

```bash
cargo build --release

# Check a spec
cargo run -- check samples/hello.spl

# Build spec + implementation → verified Rust (with generated tests)
cargo run -- build samples/hello.spl samples/hello.impl

# See what tests the spec generates
cargo run -- test samples/hello.spl
```

See [`samples/README.md`](samples/README.md) for a full walkthrough
from requirements → spec → implementation → verified output.

## How it works

### 1. Write a spec (`.spl`)

SPL is declarative: no loops, no mutation, no I/O. It defines checkable
specifications that read like structured requirements.

```text
module math.clamp;

req REQ-1: "Result is within bounds";

fn clamp @id("math.clamp.v1")
  (value: Int, lo: Int, hi: Int) -> Int
{
  requires { lo <= hi; }
  ensures [REQ-1] { result >= lo; result <= hi; }
  examples {
    "below":  clamp(0,  1, 10) == 1;
    "above":  clamp(99, 1, 10) == 10;
    "within": clamp(5,  1, 10) == 5;
  }
};
```

### 2. Write the code (`.impl`)

IMPL is a systems language with ownership. It binds to the spec by stable ID:

```text
module math.clamp;

impl fn "math.clamp.v1" clamp(value: int, lo: int, hi: int) -> int {
    if value < lo { lo }
    else if value > hi { hi }
    else { value }
}
```

### 3. Build — the compiler verifies the binding

```
$ speclang build hello.spl hello.impl
```

The `build` command:
1. Parses and type-checks the SPL spec
2. Parses the IMPL code
3. **Binds**: verifies signatures match (types, parameter counts, return type)
4. **Effects**: verifies no undeclared capabilities are used
5. **Merges**: combines spec-generated tests with implementation bodies
6. **Codegen**: emits Rust with both the real `clamp()` body and the generated test functions

If the agent's code doesn't match the spec, the build fails *before*
any code is generated.

### Additional SPL constructs

SPL can express much more than pre/postconditions:

| Construct | Purpose |
|-----------|---------|
| `refine` | Constrained types: `MidiNote = Int refine (1 <= self and self <= 12)` |
| `error` | Error taxonomy: `error ParseError { BadInput: "bad input"; }` |
| `capability` | Effect declarations: `capability Net(host: Host)` |
| `gen` | Input generators for property testing / fuzzing |
| `prop` | Universally-quantified properties: `forall x: T ...` |
| `decision` | Explicit ambiguity resolution (tie-break rules) |
| `oracle` | Differential testing: reference vs optimized implementation |
| `policy` | Package-level capability restrictions: `deny Net; deterministic;` |
| `req` | Requirement IDs for traceability |
| `perf` | Performance constraints |

See [`samples/music.spl`](samples/music.spl) for a spec using all of these.

## Architecture

speclang is a Rust workspace with 13 crates:

| Crate | Purpose |
|-------|---------|
| `speclang-cli` | CLI driver — `compile`, `build`, `wasm`, `check`, `fmt`, `ir` |
| `speclang-spl` | SPL lexer, parser, resolver, type checker |
| `speclang-impl` | IMPL lexer, parser, spec binding, effects checker |
| `speclang-ir` | Core IR types, expressions, modules, contracts |
| `speclang-ir-parser` | Textual Core IR parser and pretty-printer |
| `speclang-lower` | SPL/IMPL → Core IR lowering |
| `speclang-verify` | IR type checking, contracts, ownership, proptest, fuzz |
| `speclang-stdlib` | Standard library (core, math, mem, text, collections, contracts) |
| `speclang-backend-rust` | Core IR → idiomatic Rust transpiler |
| `speclang-backend-wasm` | Core IR → WebAssembly Text (WAT) with WASI preview-1 |
| `speclang-diagnostic` | Structured diagnostics with source annotations |
| `speclang-fmt` | Canonical SPL and IMPL source formatter |
| `speclang-pkg` | `pkg.toml` manifest parsing and dependency resolution |

## Compilation pipelines

**Spec only** (`compile` / `wasm`) — generate stubs and test harnesses from
a spec alone, useful for early validation before any code is written:

```
  .spl ──▶ parse ──▶ resolve ──▶ typecheck ──▶ lower ──▶ verify ──▶ codegen
```

**Full build** (`build`) — combine spec and implementation into verified,
working code with generated tests:

```
  .spl ──▶ parse/resolve/typecheck ──▶ lower ──┐
                                                ├──▶ merge ──▶ verify ──▶ codegen
  .impl ──▶ parse ──▶ bind ──▶ effects ──▶ lower ┘
```

The `bind` step structurally verifies every IMPL function matches its SPL
declaration (parameter types, return type, capabilities). The `merge`
step combines SPL-generated contracts and test harnesses with IMPL function
bodies, so the output is both working code and a test suite.

## Testing

```bash
cargo test          # all tests across all crates
cargo test -p speclang-spl   # just one crate
```

327 tests cover parsing, type checking, binding, effects, IR verification
(including property tests and fuzzing), codegen for both backends,
diagnostics, formatting, and package manifests.

## Documentation

Design docs in [`docs/`](docs/):

- [System overview](docs/system-overview.md) — two-layer architecture, SPL vs IMPL, effects
- [Design principles](docs/design-principles.md) — readability, examples-first, no-how-in-spec
- [Core IR overview](docs/core-ir-overview.md) — type system, expressions, contracts, lowering targets
- [Core IR grammar](docs/ir-grammar.md) — textual syntax for Core IR
- [Standard library](docs/stdlib-v0.md) — v0 stdlib surface (Option, Result, math, mem, text, collections)
- [Workflow](docs/workflow.md) — human-agent workflow with SPL as executable requirements

See [`samples/`](samples/) for a complete walkthrough from requirements to
verified code.

## License

MIT
