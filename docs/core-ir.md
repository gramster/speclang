# core-ir.md — Core IR v0 (Safety + Performance, Agent-Friendly)

This document defines the **Core IR**: a small, explicit intermediate representation that:
- SPL (spec layer) lowers into
- backends (LLVM/MLIR, Rust/Zig/C transpilers, WASM/WASI) consume
- tooling uses to enforce **contracts**, **capabilities**, and **refactor-stable IDs**

Core tenets:
- **No implicit effects**: I/O requires explicit capability tokens.
- **No implicit casts**: all conversions are explicit.
- **Overflow traps** for fixed-width integers.
- **UTF-8 strings** as a distinct type with invariant.
- **Unbounded integers** are available as `int` (Pythonic semantics).

---

## 1. Compilation unit model

A compilation unit is a `module` containing:
- nominal type definitions
- capability definitions
- function definitions (pure or effectful)
- optional extern function declarations (FFI)

Every public function/type exposed outside the module must have a **stable symbol ID** (carried from SPL). In Core IR this is metadata, e.g.:

- `@id "music.snap.v1"`

Backends preserve IDs for diagnostics, coverage, and semantic-diff tooling.

---

## 2. Lexical & structural conventions

### 2.1 Identifiers
- `Ident`: `[A-Za-z_][A-Za-z0-9_]*`
- `QName`: `Ident("." Ident)*`

### 2.2 Comments
- `#` to end-of-line (optional in textual form; backends may discard)

### 2.3 Textual IR form
Core IR has a canonical textual form (parseable), but compilers may store a binary form internally.

---

## 3. Types

### 3.1 Primitive scalars
- `bool`  (alias: `i1`)
- `u8 u16 u32 u64 u128`
- `i8 i16 i32 i64 i128`
- `f32 f64`
- `unit`

### 3.2 Unbounded integer
- `int` is an **arbitrary-precision signed integer** (mathematical integer).

Semantics:
- `int` arithmetic is exact; no overflow.
- Implementations may use big-int libraries; performance characteristics are documented in the backend.

### 3.3 UTF-8 string
- `string` is a sequence of bytes with the invariant: **valid UTF-8**.
- `bytes` is raw byte data with no encoding invariant.

### 3.4 Aggregates
- `struct { field1: T1, field2: T2, ... }`
- `enum { Variant1(T...), Variant2(T...), ... }`
- `tuple (T1, T2, ...)`

### 3.5 Pointers, borrows, and views
Core IR distinguishes ownership and borrowing explicitly:

- `own[R, T]`  — owning pointer to `T` allocated in region `R`
- `ref[T]`     — immutable borrow (non-owning)
- `mutref[T]`  — mutable borrow (non-owning)
- `slice[T]`   — (ptr,len) immutable view
- `mutslice[T]`— mutable view

### 3.6 Regions
A region `R` is an allocator context (arena/bump/pool). Regions are referenced by a token value.

- Region identifiers in types are nominal: `own[R, T]`
- Built-in region name: `heap`

---

## 4. Values, control, and SSA

Core IR is **SSA-friendly**:
- `let` introduces a new SSA binding.
- mutation exists only through `mutref`/`mutslice` stores, not rebinding SSA names.

Control structures are expression-compatible in the textual form, but all backends may lower to CFG/SSA blocks.

---

## 5. Operations and semantics

### 5.1 Integer overflow
For fixed-width integer operations:
- `add/sub/mul/neg` **trap on overflow**
- `div/mod` trap on division by zero
- shift amounts outside `[0, bitwidth)` trap

This applies to both signed and unsigned fixed-width ints.

`int` never overflows.

### 5.2 Floating point
- `f32/f64` follow IEEE 754 (NaN, infinities).
- Comparisons: `==` is IEEE (NaN != NaN). Provide `is_nan`, `total_cmp` in stdlib.

### 5.3 Conversions
No implicit conversions.

Required explicit conversions:
- between integer widths: `as_i32(x)` etc (may trap if out of range)
- between `int` and fixed ints: `int_to_i64(x)` etc (trap if out of range)
- between `bytes` and `string`: must validate UTF-8

### 5.4 Pattern matching
`match` is **exhaustive**:
- matching over enums must cover all variants or include `_`.
- matching over integers must include `_` unless compiler can prove full coverage.

### 5.5 Equality and ordering
- Primitive integers and bool: total equality and ordering.
- `int`: total equality and ordering.
- `string`: bytewise lexicographic ordering (UTF-8 preserves code-point ordering for ASCII; for general Unicode collation use library APIs).
- Struct/enum equality is **not automatic** in Core IR; derived implementations are produced at a higher layer or via explicit stdlib functions.

---

## 6. Memory, lifetimes, and borrowing rules

### 6.1 Allocation
Allocation is explicit:
- `alloc(r: region, T, value: T) -> own[R,T]` (R is the region tied to `r`)

### 6.2 Region lifetime
- A `region` token must outlive all `own[R,*]` allocated within it.
- Regions are freed as a whole:
  - `drop_region(r)` frees all allocations (except those moved out; moving out is disallowed unless explicitly supported via `transfer_to_heap`-like primitives).

`heap` allocations may be individually freed, but v0 may omit explicit `free` and rely on ownership-based drops in backends that support it.

### 6.3 Borrowing
`ref[T]` and `mutref[T]` follow Rust-like rules:

- You may create any number of `ref[T]` borrows from an owned value, as long as no `mutref[T]` is active.
- You may create at most one `mutref[T]` borrow, and no `ref[T]` may overlap it.
- Borrows cannot outlive the owned value or region.

These are enforced by a verifier over Core IR (not by convention).

### 6.4 Aliasing and mutation
Mutation occurs only through `mutref`/`mutslice`. Direct stores to an `own` require taking a mutable borrow first.

---

## 7. Effects and capabilities

### 7.1 Capability types
Capabilities are opaque nominal types declared in a module:

- `cap Net(host: host)`
- `cap FileRead(path: path)`

The internal payload is backend-defined, but type identity is stable.

### 7.2 Effectful functions
A function is effectful if and only if it takes capability parameters (or lists them in metadata for tooling). The call graph is checked:

- If `fn g()` calls `fn f(net: cap.Net, ...)`, then `g` must also accept and pass `cap.Net` (or otherwise obtain it via explicit injection APIs, which are themselves capability-gated).

No ambient I/O. No hidden global capability.

---

## 8. Error handling
No exceptions. Use:
- `Option[T]`
- `Result[T,E]`

Backends may represent traps (overflow, invalid UTF-8 constructor, failed assert) as:
- process abort
- language panic
- structured trap (WASM trap)
Policy is per backend; traps are never silently ignored.

---

## 9. Contracts and assertions

Contracts arrive from SPL as metadata and/or explicit assertions inserted by lowering.

### 9.1 `assert`
- `assert(cond: bool, msg: string)`:
  - if false: trap with message (backend-defined reporting)
  - available in pure code (assert has no external effects)

### 9.2 Contract policies
Contracts can be compiled under modes:
- `always` (inserted and never removed)
- `debug` (inserted in debug builds only)
- `sampled(p)` (inserted with probability p; requires `cap.Rng` if truly random, or deterministic sampling based on inputs; policy defined by build system)

Core IR carries these as metadata tags; insertion/elision is performed by a contract pass.

---

## 10. Concurrency (v0 status)
Concurrency primitives are **not required** in v0 Core IR. If included, they must be:
- structured (`spawn` within a scope, `join` mandatory)
- capability-gated for threading and timing
- determinism option for tests

Recommended: defer to v1 unless needed immediately.

---

## 11. Extern/FFI
Extern functions are declared with:
- explicit parameter/return types
- explicit capability requirements
- explicit ownership conventions (documented in stdlib wrappers)

Core IR itself does not assume ABI; backend defines ABI mapping.

---

## 12. Lowering notes from SPL
- SPL `refine` types become:
  - `newtype` wrappers in IR
  - checked constructors that return `Option`/`Result`
  - optional `assume_refined` for internal trusted code (capability-gated)

- SPL `examples` become generated test IR functions.
- SPL `ensures/requires` become `assert` blocks (per contract policy) plus metadata.

---