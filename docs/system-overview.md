# Two-layer, agent-friendly systems language

	•	SPL = Spec Layer (human-readable, declarative, checkable)
	•	IMPL = Implementation Layer (systems language with ownership/regions/effects; can be compiled or transpiled)



1) Files, modules, identity, and compatibility

File conventions
	•	*.spl — Spec Layer files (authoritative)
	•	*.impl — Implementation Layer files (may be generated/hand-written)
	•	pkg.toml (or similar) — package metadata, dependencies, allowed capabilities

Symbol identity (refactor-stable)

Every public symbol has a stable ID declared in SPL and bound by IMPL.
	•	Stable ID is a string, e.g. "music.snap.v1".
	•	Compiler enforces: a public symbol must keep the same ID across refactors unless intentionally versioned.

Compatibility

In SPL you can declare compatibility intent:
	•	compat stable_call (signature compatible)
	•	compat stable_semantics (behavior compatible; stricter)
	•	compat unstable (allowed to change)

⸻

2) SPL: The Spec Layer (the truth)

SPL constructs (v0)
	•	module
	•	import
	•	type (alias, enum, struct)
	•	refine (range / predicate constraints)
	•	fn (function spec)
	•	error (error domains)
	•	capability (effects)
	•	law / prop (properties; optional)
	•	examples (executable examples)
	•	perf (constraints; some enforceable, some benchmark-checked)
	•	req (requirement declarations with traceability IDs)
	•	decision (explicit ambiguity resolutions; must be resolved to compile)
	•	gen (input generators for property testing & fuzzing)
	•	prop (forall-quantified properties with shrinking)
	•	oracle (reference/optimized implementation linkage)
	•	policy (package-level capability restrictions & determinism)

SPL semantics (important bits)
	•	No loops, no mutation, no I/O. Purely declarative.
	•	All requires/ensures/invariant/examples are compiled into obligations:
	•	runtime checks (debug/sampled/always)
	•	property tests and fuzz harnesses
	•	proof obligations (optional later)
	•	Requirement traceability: REQ tags on contracts/examples produce coverage reports
	•	Decision blocks enforce that ambiguities are explicitly resolved before compilation
	•	Generators and props compile to property-based test and fuzz harnesses
	•	Oracles enable differential testing between reference and optimized implementations
	•	Policy blocks restrict capabilities at the package level (acceptance gate for agents)

Effects (capabilities)

Functions declare effects they are allowed to perform:
	•	effects { Net(host(url)), FileRead(path), Clock }
	•	default is pure (no effects)

The package file (and `policy` blocks) can restrict what modules may ever request.
Policy can also enforce determinism (deny Clock/Rng unless explicitly permitted).

⸻

3) IMPL: Implementation Layer (minimal systems core)

IMPL design goals (v0)
	•	Ownership + borrowing
	•	Regions/arenas
	•	Explicit allocation
	•	Explicit effects “permission passed as parameter”
	•	FFI for existing ecosystems

You can implement IMPL as:
	•	a real compiler (own backend), or
	•	a transpiler to Rust/Zig/C++ (fast adoption)

Key rule

IMPL binds to SPL IDs:
	•	impl fn "music.snap.v1" { ... }

Compiler checks:
	•	signature matches SPL
	•	effects used ⊆ effects declared in SPL
	•	contracts/examples pass (via generated harness)