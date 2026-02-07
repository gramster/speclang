//! Generated from speclang module `math::clamp`
#![allow(dead_code, unused_variables)]

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

