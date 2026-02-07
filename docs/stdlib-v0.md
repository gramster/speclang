# stdlib-v0.md — Standard Library Surface (v0)

This document defines the minimal standard library required to support SPL lowering and practical systems programming with:
- **trap-on-overflow** fixed ints
- **UTF-8 `string`**
- **unbounded `int`**
- ownership + regions
- capability-gated effects

Stdlib is split into:
- `core` (pure)
- `math` (pure)
- `mem` (regions/alloc)
- `bytes` / `text` (encoding, strings)
- `collections` (vec/set/map)
- `contracts` (pure helpers for lowering)
- `effects` + `io` (capability APIs; optional but shape is defined)

Notation:
- Generic types: `T`, `K`, `V`
- References: `ref[T]`, `mutref[T]`
- Regions: `region`, `own[R,T]`

---

## 1. core

### 1.1 Option
```text
type Option[T] = enum { None, Some(T) }

fn option.is_some[T](o: ref[Option[T]]) -> bool
fn option.is_none[T](o: ref[Option[T]]) -> bool
fn option.unwrap[T](o: Option[T]) -> T               # traps if None
fn option.unwrap_or[T](o: Option[T], default: T) -> T

### 1.2 Result

```
type Result[T,E] = enum { Ok(T), Err(E) }

fn result.is_ok[T,E](r: ref[Result[T,E]]) -> bool
fn result.is_err[T,E](r: ref[Result[T,E]]) -> bool
fn result.unwrap_ok[T,E](r: Result[T,E]) -> T        # traps if Err
fn result.unwrap_err[T,E](r: Result[T,E]) -> E       # traps if Ok
```

### 1.3 Equality / Ordering (v0 pragmatic)

v0 provides:
	•	built-in == != < <= > >= only for primitive numeric/bool, int, string, and bytes slices.
	•	For structs/enums, provide explicit helpers or derive at SPL level.

⸻

## 2. math

### 2.1 Integers and conversions

Fixed-width ops are built-in and trap on overflow.

Provide explicit conversion functions (all trap on out-of-range):

```
fn conv.i32_from_int(x: int) -> i32
fn conv.int_from_i32(x: i32) -> int
fn conv.u64_from_int(x: int) -> u64
fn conv.int_from_u64(x: u64) -> int

fn conv.i32_from_u64(x: u64) -> i32      # traps if x > i32::MAX
fn conv.u64_from_i32(x: i32) -> u64      # traps if x < 0
```

### 2.2 Unbounded Int Operations

```
fn int.add(a: int, b: int) -> int
fn int.sub(a: int, b: int) -> int
fn int.mul(a: int, b: int) -> int
fn int.div(a: int, b: int) -> int        # traps if b == 0
fn int.mod(a: int, b: int) -> int        # traps if b == 0
fn int.neg(a: int) -> int
fn int.abs(a: int) -> int
fn int.cmp(a: int, b: int) -> i32        # -1, 0, 1
```

(Backends may inline/optimize small-int cases.)

### 2.3 Float Helpers

```
fn float.is_nan64(x: f64) -> bool
fn float.is_finite64(x: f64) -> bool
fn float.total_cmp64(a: f64, b: f64) -> i32  # total order incl NaN
```

## 3. mem

### 3.1 Regions

```
type region   # opaque token

fn mem.new_region() -> region
fn mem.drop_region(r: region) -> unit
```

### 3.2 Allocation and Borrowing

```
fn mem.alloc[T](r: region, value: T) -> own[R,T]     # R is tied to r (backend-enforced)
fn mem.borrow[T](p: ref[own[R,T]]) -> ref[T]
fn mem.borrow_mut[T](p: ref[own[R,T]]) -> mutref[T]
```

Notes:
	•	Exact own[R,T] spelling in source IR may use own[region_id,T]; textual v0 can elide the region id if region inference is used, but the canonical IR retains it.


### 3.3 Slices

```
fn mem.slice_len[T](s: slice[T]) -> u64
fn mem.slice_get[T](s: slice[T], i: u64) -> Option[ref[T]]
fn mem.mutslice_get[T](s: mutslice[T], i: u64) -> Option[mutref[T]]
```

## 4. bytes

### 4.1 Bytes Types

```
type Bytes = collections.Vec[u8]
type ByteSlice = slice[u8]
```

### 4.2 Common Ops (pure)

```
fn bytes.len(b: ref[Bytes]) -> u64
fn bytes.as_slice(b: ref[Bytes]) -> ByteSlice
fn bytes.eq(a: ByteSlice, b: ByteSlice) -> bool
fn bytes.cmp(a: ByteSlice, b: ByteSlice) -> i32       # lexicographic
```

## 5. text (UTF-8 strings)

### 5.1 String type and invariants

```
type String   # invariant: valid UTF-8
type Str = slice[u8]   # view; must be valid UTF-8 when produced by string APIs
```

### 5.2 Construction

```
type Utf8Error = enum { InvalidUtf8 }

fn string.from_utf8(bytes: Bytes) -> Result[String, Utf8Error]
fn string.from_utf8_lossy(bytes: ByteSlice, r: mem.region) -> String   # replacement chars
```

### 5.3 Access

```
fn string.len_bytes(s: ref[String]) -> u64
fn string.as_bytes(s: ref[String]) -> ByteSlice
fn string.eq(a: ref[String], b: ref[String]) -> bool
fn string.cmp(a: ref[String], b: ref[String]) -> i32        # bytewise lexicographic
```

### 5.4 ASCII utilities (explicit, allocation-visible)

```
fn string.trim_ascii(s: ref[String], r: mem.region) -> String
fn string.to_lower_ascii(s: ref[String], r: mem.region) -> String
```

Unicode-aware operations are intentionally v0-minimal; add unicode module later.

## 6. collections

### 6.1 Vec

```
type Vec[T]

fn vec.new[T](r: mem.region) -> Vec[T]
fn vec.with_capacity[T](r: mem.region, cap: u64) -> Vec[T]
fn vec.len[T](v: ref[Vec[T]]) -> u64
fn vec.push[T](v: mutref[Vec[T]], x: T) -> unit
fn vec.get[T](v: ref[Vec[T]], i: u64) -> Option[ref[T]]
fn vec.as_slice[T](v: ref[Vec[T]]) -> slice[T]
```

### 6.2 Hashing support (explicit)

v0 avoids a full trait system. Collections that need hashing take explicit hash/eq functions.

```
type HashFn[T] = fn(ref[T]) -> u64
type EqFn[T]   = fn(ref[T], ref[T]) -> bool
```

Provide builtin hash/eq for primitives, bytes, and string:

```
fn hash.u64(x: ref[u64]) -> u64
fn hash.int(x: ref[int]) -> u64
fn hash.string(x: ref[String]) -> u64
fn hash.bytes(x: ByteSlice) -> u64

fn eq.u64(a: ref[u64], b: ref[u64]) -> bool
fn eq.int(a: ref[int], b: ref[int]) -> bool
fn eq.string(a: ref[String], b: ref[String]) -> bool
fn eq.bytes(a: ByteSlice, b: ByteSlice) -> bool
```

### 6.3 set

```
type Set[T]

fn set.new[T](r: mem.region, hash: HashFn[T], eq: EqFn[T]) -> Set[T]
fn set.len[T](s: ref[Set[T]]) -> u64
fn set.contains[T](s: ref[Set[T]], x: ref[T]) -> bool
fn set.insert[T](s: mutref[Set[T]], x: T) -> bool        # true if inserted
```

Like Python, sets preserve insertion order.

### 6.4 map

```
type Map[K,V]

fn map.new[K,V](r: mem.region, hash: HashFn[K], eq: EqFn[K]) -> Map[K,V]
fn map.len[K,V](m: ref[Map[K,V]]) -> u64
fn map.get[K,V](m: ref[Map[K,V]], k: ref[K]) -> Option[ref[V]]
fn map.insert[K,V](m: mutref[Map[K,V]], k: K, v: V) -> Option[V]
```

Rationale:
	•	Explicit hash/eq keeps Core IR tiny and makes transpilation straightforward.
	•	SPL can sugar this away by selecting default hash/eq for common types.


## 7. contracts (pure helper layer)

These functions exist mostly to simplify SPL-to-IR lowering.

```
fn contracts.implies(a: bool, b: bool) -> bool          # returns (!a) or b
fn contracts.and(a: bool, b: bool) -> bool
fn contracts.or(a: bool, b: bool) -> bool
fn contracts.not(a: bool) -> bool
```

Quantifiers (forall, exists) are not runtime stdlib in v0.

	•	SPL forall lowers to generated tests/fuzzing, not loops in Core IR.


## 8. effects and io (capability-gated; optional modules)

### 8.1 Capability types (examples)

```
cap Net(host: Host)
cap FileRead(path: Path)
cap FileWrite(path: Path)
cap Clock
```

### 8.2 IO APIs (shape)

```
type Url
type Host
type Path

type NetError = enum { Unreachable, Timeout, Protocol, Other }
type FsError  = enum { NotFound, Permission, Io, Other }

fn net.get(net: cap.Net, url: ref[Url], r: mem.region) -> Result[bytes.Bytes, NetError]
fn fs.read_all(fr: cap.FileRead, path: ref[Path], r: mem.region) -> Result[bytes.Bytes, FsError]
fn fs.write_all(fw: cap.FileWrite, path: ref[Path], data: bytes.ByteSlice) -> Result[unit, FsError]
```

Notes:
	•	Allocation region is explicit (r) to keep performance predictable.
	•	Backends can map these to native OS APIs or WASI.


## 9. SPL lowering expectations (stdlib obligations)

To support SPL v0, stdlib must also provide (or the compiler must synthesize) predicates used in contracts, e.g.:
	•	set_contains → collections.set.contains
	•	len(x) → appropriate len function for the type
	•	is_ok / unwrap_ok → core.result.*

The SPL compiler owns the mapping from surface predicates to concrete stdlib functions.



