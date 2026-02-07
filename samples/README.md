# Walkthrough: From Requirements to Verified Code

This walkthrough shows how speclang works end-to-end.  The key idea:
**humans write/review specs, agents write code, the compiler proves they
match.**

---

## The problem speclang solves

In a typical AI-assisted workflow, an agent generates code and you review
it line by line.  For a small function that's fine — but at scale you
can't audit every implementation.  You need a way to state *what* the code
should do and have the toolchain verify that it does.

speclang splits the work into two files:

| File | Who writes it | What it says |
|------|---------------|-------------|
| `.spl` (spec) | Human or agent, **human reviewed** | *What* — contracts, types, examples, properties |
| `.impl` (code) | Agent | *How* — implementation with ownership, control flow |

The compiler checks that the `.impl` conforms to the `.spl`:
signatures match, effects are contained, examples pass.

---

## Step 1: State the requirements

You start with informal requirements — just bullets:

> - `clamp(value, lo, hi)` returns `value` constrained to `[lo, hi]`
> - `lo` must be ≤ `hi`
> - Result is always between `lo` and `hi` inclusive

## Step 2: Write (or generate) the spec

These requirements become an SPL file — [`hello.spl`](hello.spl):

```spl
module math.clamp;

req REQ-1: "Result is within bounds";
req REQ-2: "Idempotent on in-range values";

fn clamp @id("math.clamp.v1") @compat(stable_call)
  (value: Int, lo: Int, hi: Int) -> Int
{
  requires {
    lo <= hi;
  }

  ensures [REQ-1] {
    result >= lo;
    result <= hi;
  }

  examples [REQ-2] {
    "below range":  clamp(0,  1, 10) == 1;
    "above range":  clamp(99, 1, 10) == 10;
    "within range": clamp(5,  1, 10) == 5;
  }
};
```

This is 20 lines, readable by anyone who knows the domain.  No
implementation details. A human can review this in seconds.

**Check it compiles:**

```
$ speclang check samples/hello.spl
samples/hello.spl: ok
```

**See what tests it generates:**

```
$ speclang test samples/hello.spl
samples/hello.spl: 3 test(s) found

  test_clamp_0 (example) [REQ-2]
  test_clamp_1 (example) [REQ-2]
  test_clamp_2 (example) [REQ-2]

Requirement coverage:
  REQ-2 → test_clamp_0, test_clamp_1, test_clamp_2

Fuzz targets (1):
  fuzz_clamp (from clamp)
```

From 3 examples, the compiler generated 3 executable tests with
requirement traceability, plus a fuzz target.

## Step 3: Generate the spec-only scaffold

Before writing any implementation, you can see what the compiler
produces from the spec alone:

```
$ speclang compile samples/hello.spl
```

Output ([expected-output/hello.rs](expected-output/hello.rs)):

```rust
// id: math.clamp.v1
pub fn clamp(value: i64, lo: i64, hi: i64) -> i64 {
    debug_assert!((lo <= hi));
    assert!((lo <= hi), "precondition failed: clamp");
    // ensures: (result >= lo)
    // ensures: (result <= hi)
}
```

The function signature and contracts are there, but **no body** — the
spec says *what*, not *how*.  The `ensures` clauses are comments
because the postconditions need the implementation to check against.

## Step 4: Write (or generate) the implementation

Now an agent (or human) writes the IMPL — [`hello.impl`](hello.impl):

```impl
module math.clamp;

impl fn "math.clamp.v1" clamp(value: int, lo: int, hi: int) -> int {
    if value < lo {
        lo
    } else if value > hi {
        hi
    } else {
        value
    }
}
```

The `impl fn "math.clamp.v1"` links this body to the SPL spec by its
stable ID.  The compiler will verify the binding.

## Step 5: Build — spec + code together

```
$ speclang build samples/hello.spl samples/hello.impl
```

Output ([expected-output/hello-built.rs](expected-output/hello-built.rs)):

```rust
// req: REQ-2
pub fn test_clamp_0() {
    assert!((clamp(0, 1, 10) == 1), "below range");
}
// req: REQ-2
pub fn test_clamp_1() {
    assert!((clamp(99, 1, 10) == 10), "above range");
}
// req: REQ-2
pub fn test_clamp_2() {
    assert!((clamp(5, 1, 10) == 5), "within range");
}

// id: math.clamp.v1
pub fn clamp(value: i64, lo: i64, hi: i64) -> i64 {
    if (value < lo) { lo } else { if (value > hi) { hi } else { value } }
}
```

Now you get **both**: the real implementation from the `.impl` file
*and* the test harness generated from the `.spl` contracts.

The `build` command verified that:
- ✓ `clamp` in the IMPL matches the SPL spec signature
- ✓ Parameter types match (`int` ↔ `Int`)  
- ✓ Return type matches
- ✓ No undeclared effects used

If the agent's implementation had the wrong signature, the build would
fail at the bind step — before any code is generated.

## Step 6: What happens when the agent gets it wrong

Say the agent accidentally swaps `lo` and `hi`, or adds an undeclared
network call. The compiler catches it:

**Signature mismatch:**
```
error[bind]: [math.clamp.v1] parameter count mismatch: IMPL has 2 data params, SPL spec has 3
```

**Undeclared effect:**
```
error[effects]: in `clamp`: capability `Net` used but not available
```

The human doesn't need to read the code to find these bugs. The
toolchain reports them structurally.

---

## The workflow in summary

```
 ┌─────────────────────────────────────────────────────┐
 │     Human                        Agent              │
 │                                                     │
 │  1. Write requirements ────────────────────────▶    │
 │                                                     │
 │  2. Review .spl spec  ◀─── Generate .spl spec       │
 │     (20 lines, readable)                            │
 │                                                     │
 │     speclang check spec.spl  ◀── verify spec ok     │
 │     speclang test spec.spl   ◀── see generated tests│
 │                                                     │
 │  3. Approve spec      ────── Generate .impl code ──▶│
 │                                                     │
 │     speclang build spec.spl code.impl               │
 │       ├─ bind: signatures match?     ✓              │
 │       ├─ effects: no undeclared I/O? ✓              │
 │       └─ codegen: Rust with tests    ✓              │
 │                                                     │
 │  4. Run tests         ◀─── Fix if tests fail        │
 │     (generated from spec)                           │
 │                                                     │
 │  5. Ship              ────────────────────────▶     │
 └─────────────────────────────────────────────────────┘
```

The human reviews the *spec* (short, declarative, domain-level).
The compiler verifies the *code* against it.
The generated tests prove the *examples* hold.

---

## Larger example: `music.spl`

[`music.spl`](music.spl) exercises more SPL features:

- **Refined types**: `MidiNote = Int refine (1 <= self and self <= 12)` —
  constructor enforces the constraint at runtime
- **Generators**: `gen MidiNoteGen { range: 1..12; }` — input generators
  for property testing and fuzzing  
- **Decisions**: `decision tie_break: when: "equal distance"; choose: "smaller note";` —
  explicit ambiguity resolution
- **Properties**: `prop snap_in_scale: forall n: MidiNote ...` —
  universally quantified, tested via generated harness
- **Oracles**: `oracle music.scale.snap_to_scale: reference;` —
  differential testing between reference and optimized implementations
- **Policy**: `policy { deny Net; deterministic; };` —
  package-level capability restrictions

```
$ speclang test samples/music.spl
samples/music.spl: 3 test(s) found

  test_snap_to_scale_0 (example) [REQ-3]
  test_snap_to_scale_1 (example) [REQ-3]
  prop_snap_in_scale (property) [REQ-2]

Requirement coverage:
  REQ-2 → prop_snap_in_scale
  REQ-3 → test_snap_to_scale_0, test_snap_to_scale_1

Fuzz targets (2):
  fuzz_new_MidiNote (from new_MidiNote)
  fuzz_snap_to_scale (from snap_to_scale)
```

## CLI commands

| Command | What it does |
|---------|-------------|
| `check <spec.spl>` | Parse, resolve, type-check the spec |
| `test <spec.spl>` | List generated tests and requirement coverage |
| `compile <spec.spl>` | Spec → Rust (contracts only, no body) |
| `build <spec.spl> <code.impl>` | Spec + code → Rust (verified, with body + tests) |
| `wasm <spec.spl>` | Spec → WebAssembly (WAT format) |
| `fmt <file>` | Format SPL or IMPL source |
| `parse <spec.spl>` | Print the parsed AST |

### Options

```bash
# Contract compilation modes
speclang build --mode debug   spec.spl code.impl  # all contracts checked (default)
speclang build --mode sampled spec.spl code.impl  # probablistic checking
speclang build --mode release spec.spl code.impl  # no runtime checks
```
