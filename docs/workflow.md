# Human-Agent Workflow with SPL

The workflow we want is basically: human steers → agent builds → system produces strong evidence → human validates the evidence, not the code. Here’s how I’d refine it.

The key shift: SPL as “executable requirements”, not “API contracts”

Instead of humans writing SPL by hand, the agent continuously maintains SPL as the canonical spec artifact derived from:
	•	requirements docs
	•	examples
	•	non-goals
	•	constraints (perf, security, compatibility)
	•	external interface definitions (OpenAPI, CLI schema, UI flows)

SPL becomes the thing humans review. Code becomes a derived artifact.

Why SPL is better than “just tests”

Tests are concrete but verbose and often underspecified (especially around edge cases, tie-breakers, invariants, security, effects).
SPL can express:
	•	domain invariants (“never 0, use 12”)
	•	allowed side effects (capabilities)
	•	error taxonomy
	•	algebraic laws / metamorphic properties
	•	performance constraints
…in a compact, reviewable way, and then generate:
	•	tests (examples + properties)
	•	scaffolding
	•	mocks
	•	differential oracles

So SPL is the “high-leverage middle layer” between prose requirements and massive TDD.

⸻

Refined agent-centric workflow

Phase 0: Humans provide intent in natural artifacts

Humans give the agent:
	•	a requirements doc (bullets are fine)
	•	a few examples (inputs/outputs)
	•	constraints: perf, safety, dependencies, deployment target
	•	“definition of done” checklist

That’s it. No code.

Phase 1: Agent produces a Spec Bundle (humans primary review target)

The agent generates:
	1.	SPL modules: types, effects, errors, contracts, examples, properties
	2.	Threat/effects map: which modules can touch disk/net/clock, and why
	3.	Ambiguity ledger: list of decisions the requirements didn’t nail down (tie-breaks, formats, rounding, ordering)
	4.	Acceptance suite plan: what evidence will be produced (tests, fuzzing, perf, differential)

Humans job: approve/adjust this bundle. This is steering.

Phase 2: Agent synthesizes implementation with continuous evidence

Agent generates code (in Rust/Zig/whatever) plus harnesses automatically derived from SPL:
	•	unit tests from examples
	•	property tests from laws/props
	•	fuzzers from input grammars (if present)
	•	contract runtime checks (debug/sampled)
	•	perf microbenchmarks for hot paths
	•	effect/capability checks (static)

Crucially, the agent is not “done” when it compiles — it’s done when it produces the evidence bundle.

Phase 3: Evidence review (mostly automated, human validates)

Humans review:
	•	failing/passing summary
	•	coverage of requirements (traceability: requirement → SPL clause → tests/properties)
	•	perf deltas (benchmarks)
	•	effect diff (“this build now requires Net(host=…)”)
	•	a small curated list of risky changes (FFI, unsafe blocks, parsing)

Not the whole codebase.

Phase 4: Repair loop is automatic and spec-preserving

If tests fail, agent iterates.
If ambiguity is discovered (conflicting examples), agent updates the ambiguity ledger and proposes a spec change.
If spec changes, humans approve it (or reject). Code regenerates.

⸻

What SPL needs to support this workflow well

To move humans to “steering and validation,” SPL should include a few more high-leverage constructs beyond basic requires/ensures.

1) Traceability: requirements IDs and coverage

Allow SPL clauses to reference requirement IDs:

fn normalize_email @id("email.normalize.v1") {
  ensures [REQ-12] { no_whitespace(result); }
  examples [REQ-13] {
    "trims": normalize_email(" a@b.com ") == "a@b.com";
  }
}

Then tooling can output:
	•	which REQs are covered by examples/props
	•	which are only prose (warning)
	•	which code paths map to which REQs

2) Generators (input domains) for property testing & fuzzing

Instead of writing 10k tests, you define input shapes:

gen EmailLocalPart:
  charset: "a-z0-9._+-"
  len: 1..64

gen Email:
  format: "{EmailLocalPart}@{Domain}"

Tooling generates fuzzers and shrinks failing cases. Very agent-friendly.

3) Metamorphic properties (cheap high power)

These catch tons of bugs with little effort:

prop [REQ-22] "idempotent":
  forall s: String => normalize_email(normalize_email(s)) == normalize_email(s)

4) Oracle layering: reference vs optimized

SPL can mark functions as having a reference implementation:

fn snap_to_scale @id("music.snap.v1") { ... }
oracle snap_to_scale: "reference"

Workflow:
	•	agent generates a simple correct reference implementation first
	•	then generates an optimized version
	•	runs differential tests between them

This is way more efficient than pure TDD for agents: the agent gets a correctness anchor.

5) Explicit ambiguity handling

SPL should have a place for decisions:

decision [REQ-7] tie_break:
  when: "multiple minima"
  choose: "smaller numeric note"

Tooling refuses to compile if any “decision required” is unresolved.

6) Capability policy as an acceptance gate

Humans want confidence that the agent didn’t “phone home” or touch files unexpectedly.
SPL + package policy can enforce:
	•	allowed effects per module
	•	allowed hosts/paths
	•	deterministic mode (no clock/rng unless permitted)

And produce an “effect diff” report per change.

7) Performance contracts that are checkable

Some are static (no alloc), some are measured (bench).
SPL can encode both:

perf {
  alloc: none        # statically checked
  latency_p95_us: 50 # benchmarked in CI
}


⸻

The refined workflow in one sentence

Human approves the Spec Bundle; agent generates code + evidence; system enforces effects + contracts + perf; human validates evidence and deltas, not implementation details.

⸻

Why this is more efficient for agents than “requirements → lots of tests”

Because SPL lets the agent:
	•	express broad correctness with properties and generators (fewer artifacts)
	•	create reference oracles automatically (anchors optimization)
	•	keep effects and security boundaries enforceable
	•	maintain requirement traceability without manual test bookkeeping

Tests still exist — but they’re largely generated from higher-level spec primitives.

⸻

Given the above:

Extend the SPL grammar and stdlib with these additions:
	1.	req declarations and [REQ-…] tags
	2.	decision blocks (must be resolved)
	3.	gen blocks (input generators)
	4.	prop blocks with forall and shrinking hints
	5.	oracle blocks (reference/optimized linkage)
	6.	package-level policy (allowed effects/hosts/paths; determinism)

