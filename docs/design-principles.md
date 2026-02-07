# Principles for a readable spec layer

Note: this may differ from other docs as it simply outlines the principles we want to follow, not the 
specific design we come up with.

1) “Design-doc first” syntax: nouns, tables, examples

Humans scan for:
	•	entities
	•	invariants
	•	examples
	•	error cases
	•	non-goals

So the spec layer should look like that, but be checkable.

2) Minimize symbols, maximize structured English

Use a small handful of keywords (type, invariant, requires, ensures, example, error, perf, effects).
Avoid heavy punctuation / lambda soup.

3) Make examples the primary executable unit

Examples are the most readable tests. Treat them as first-class and compile them.

4) Keep the “how” out

No loops, no control flow in spec. Only relations and constraints.

⸻

What it looks like

Entity + invariants (readable, checkable)

```
spec MidiNote:
  base: Int
  invariant: 1 <= self <= 12
  doc: "Pitch class in 1..12 (12 = octave boundary, never 0)."
```

Function contract with examples

```
spec snap_to_scale:
  id: "music.snap.v1"
  input:
    note: MidiNote
    scale: Set[MidiNote]
  requires:
    - scale is not empty
  ensures:
    - result is in scale
    - distance_mod12(result, note) is minimal
  ties:
    - if multiple minima: choose the smaller result
  examples:
    - snap_to_scale(12, {1,5,8}) == 1
    - snap_to_scale(1,  {1,5,8}) == 1
    - snap_to_scale(2,  {1,5,8}) == 1
```

This is basically a literate “function spec” that:
	•	humans can review quickly
	•	agents can’t misinterpret (tie-break is explicit)
	•	compiler can generate tests / obligations from it

Errors are explicit and human-friendly


```
spec parse_config:
  input: bytes: Bytes
  output: Config | ParseError
  errors:
    - ParseError.InvalidHeader: "Bad magic or version"
    - ParseError.Truncated: "Unexpected end of input"
```

Effects/capabilities also read like policy

```
spec fetch_config:
  effects:
    - net: only host(url)
    - cache: optional
  forbids:
    - filesystem
    - clock
```

Performance intent in the spec layer (still readable)

```
spec parse_config:
  perf:
    - linear time in bytes length
    - zero allocations in hot path
    - uses region R for all transient buffers
```

The compiler can enforce some of these (“no heap allocs”) and benchmark-regress others.

⸻

How spec maps to implementation (clean contract boundary)

You’d have a corresponding implementation stub:

```
impl snap_to_scale for spec "music.snap.v1":
  fn(note: MidiNote, scale: Set[MidiNote]) -> MidiNote
```

Rules:
	•	Implementation must satisfy the spec obligations.
	•	Spec changes are semver-visible.
	•	The agent is allowed to rewrite impl freely if spec stays stable.

⸻

Readability tricks that help humans a lot

A) “Glossary types” with domain names

People read:
	•	MidiNote, UserId, Port, EmailAddress
better than:
	•	Int, String

B) A “Decision” section for tie-breakers

Ambiguity lives in tie-breaks. Put it in one place, always.

C) Bullet lists over boolean algebra

Humans prefer:
	•	requires: - scale is not empty
over:
	•	requires scale.len > 0

D) Tables for structured constraints

```
spec HttpStatus:
  base: Int
  allowed:
    | range     | meaning        |
    | 100..199  | informational  |
    | 200..299  | success        |
    | 300..399  | redirect       |
    | 400..499  | client error   |
    | 500..599  | server error   |
```

Still checkable, very readable.

⸻

Enforcement model (so it stays honest)

To keep this from becoming “nice comments”:
	•	Every spec produces:
	•	type obligations (refinements)
	•	contract checks
	•	example tests
	•	optional fuzz/property scaffolds
	•	CI treats spec failures as compile failures.
	•	Release builds can compile out runtime checks except where marked always.

⸻

If you want the crispest “human readable” spec layer…

I’d make it YAML-ish but with strict typing, or Markdown-with-guardrails:

Option 1: Spec is its own .spec format (like above)

Pros: tight syntax, easy parsing, minimal noise
Cons: new syntax to learn

Option 2: “Executable Markdown”

Headings + fenced blocks that must conform:
	•	very readable
	•	lives naturally with docs
	•	still compiler-checked

Example:

```
## spec: snap_to_scale (music.snap.v1)

**Requires**
- scale is not empty

**Ensures**
- result is in scale
- minimal mod-12 distance
- ties choose smaller
```

``` examples
snap_to_scale(12, {1,5,8}) == 1
```


