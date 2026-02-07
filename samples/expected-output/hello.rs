//! Generated from speclang module `math::clamp`
#![allow(dead_code, unused_variables)]

pub type Bound = i64;

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
// compat: stable-call
pub fn clamp(value: i64, lo: Bound, hi: Bound) -> i64 {
    debug_assert!((lo <= hi));
    assert!((lo <= hi), "precondition failed: clamp");
    // ensures: (result >= lo)
    // ensures: (result <= hi)
}

