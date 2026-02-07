Core IR goals
	1.	Unambiguous semantics (agents can’t “interpret” things)
	2.	Tiny (easy to implement & audit)
	3.	Explicit cost model (allocations, copies, effects are visible)
	4.	Good lowering target for:
	•	native (LLVM/MLIR)
	•	transpile-to-Rust/Zig/C
	•	WASM

⸻

Core IR at a glance

Files / Units
	•	One compilation unit = module with:
	•	type defs
	•	function defs
	•	external imports (FFI)
	•	capability/effect declarations

Two “kinds” of code
	•	Pure (no effects)
	•	Effectful (explicit capability tokens passed in; IR enforces “no hidden I/O”)

⸻

Core IR: Type system

Primitive types
	•	i1 (bool)
	•	i8 i16 i32 i64 i128
	•	u8 u16 u32 u64 u128
	•	f32 f64
	•	unit

Aggregates
	•	struct { f1:T1, f2:T2, ... }
	•	enum { V1(T...), V2(T...), ... } (tagged union)
	•	tuple (T1, T2, ...)

References and ownership
	•	own[T] — owning pointer to heap/region allocated T
	•	ref[T] — non-owning immutable borrow
	•	mutref[T] — non-owning mutable borrow
	•	slice[T] — (ptr,len) view (immutable)
	•	mutslice[T] — mutable view

Regions (arenas)
	•	region R is a first-class value only as a token, not an address space.
	•	Types can be annotated to mean “allocated in region R”:
	•	own[R, T] (owning pointer whose allocation lives in region R)
	•	You can also allow heap as an implicit region in early versions:
	•	own[heap, T]

Refinements

Core IR does not need full dependent typing. It needs checked wrappers:
	•	refined<T, PRED> becomes:
	•	runtime-checked constructor + internal representation T
	•	optional static discharge by optimizer later

In IR, represent it as:
	•	newtype MidiNote = i32 plus a verified constructor MidiNote.try_new(i32) -> Option[MidiNote]
	•	and/or attached contract metadata.

⸻

Core IR: Effects & capabilities

Capability type

Capabilities are opaque tokens:
	•	cap Net(host: Host)
	•	cap FileRead(path: Path)
	•	cap Clock

In IR, these are just distinct nominal types:
	•	cap.Net
	•	cap.FileRead
	•	etc.

Effectful functions must take capability arguments explicitly.

Example signature:
	•	Pure: fn f(x:i32)->i32
	•	Effectful: fn fetch(net: cap.Net, url: Url)->Result<Bytes, FetchError>

This is the key trick that makes backends easy: even if you transpile to Rust, you just thread these tokens.

⸻

Core IR: Expressions and statements

Core expression forms (SSA-friendly)
	•	literals: ints, floats, bool, unit
	•	local variables (SSA values)
	•	let x = expr
	•	if cond then a else b
	•	match e { ... } (exhaustive)
	•	function call: call f(args...)
	•	struct/enum construction + field access
	•	arithmetic, comparisons (no implicit casts)
	•	addr_of, load, store only in tightly controlled subset (optional v0; you can rely on references instead)

Memory + allocation (explicit)
	•	alloc(region_token, Type) -> own[R,T]
	•	free(own[heap,T]) (only for heap mode; regions typically free wholesale)
	•	borrow(own[T]) -> ref[T] (with lifetime rules enforced by verifier)
	•	borrow_mut(own[T]) -> mutref[T]

If you transpile to Rust, alloc(R,T) maps to bumpalo::Bump or similar; to LLVM, it maps to arena allocation.

Errors (no exceptions)
	•	Use Result[T,E] and Option[T] (stdlib types)

Concurrency (v0 optional)

If included, keep it structured:
	•	spawn(fn, args...) -> task_handle[T]
	•	join(handle) -> T
But you can also defer concurrency to v1.

⸻

Core IR: Contracts as first-class metadata (enforced by tooling)

Contracts compile to:
	1.	generated check blocks (debug / always / sampled)
	2.	generated property tests
	3.	generated differential tests (oracle)
	4.	requirement coverage metadata (req tags)
	5.	optional proof obligations (later)

In Core IR, represent contracts as attached annotations plus optional explicit checks.

Example:
	•	Function has metadata: requires, ensures
	•	Contracts may carry `@req_tag "REQ-001"` for traceability
	•	Compiler may insert checks at entry/exit.

So IR has:
	•	@requires pred_expr
	•	@ensures pred_expr
	•	@req_tag "REQ-xxx" (traceability to SPL req declarations)
…and/or:
	•	assert(pred_expr, "msg") statements inserted by a lowerer.

### Additional lowering targets (from new SPL constructs)
	•	`gen` blocks → generator functions in test harness module
	•	`prop` blocks → property-test functions with forall-driven generation
	•	`oracle` blocks → differential-test functions (reference vs optimized)
	•	`decision` blocks → compile-time resolution; no runtime IR emitted
	•	`policy` blocks → static capability verification constraints (compile-time)
