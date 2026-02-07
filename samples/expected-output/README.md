# Expected Output

Pre-generated compiler output so you can see the results without
building.

| File | Source | Command |
|------|--------|---------|
| `hello.rs` | Spec only — contracts, no body | `speclang compile hello.spl` |
| `hello-built.rs` | Spec + impl — body + tests | `speclang build hello.spl hello.impl` |
| `hello.wat` | Spec → WebAssembly | `speclang wasm hello.spl` |
| `music.rs` | Full-featured spec → Rust | `speclang compile music.spl` |
| `music.wat` | Full-featured spec → WASM | `speclang wasm music.spl` |

Compare `hello.rs` (stub with contracts) vs `hello-built.rs` (real
implementation with test harness) — that's the core value of the
two-layer approach.
