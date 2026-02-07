//! Generated from speclang module `music::scale`
#![allow(dead_code, unused_variables)]

pub type MidiNote = i64;

pub type Scale = Set<MidiNote>;

#[derive(Debug, Clone)]
pub enum SnapError {
    EmptyScale(String),
}

pub fn new_MidiNote(value: i64) -> i64 {
    assert!(((1 <= value) && (value <= 12)));
    assert!(((1 <= value) && (value <= 12)), "precondition failed: new_MidiNote");
    value
}

// req: REQ-3
pub fn test_snap_to_scale_0() {
    assert!((snap_to_scale(12, set_of(1, 5, 8)) == 1), "edge wraps");
}

// req: REQ-3
pub fn test_snap_to_scale_1() {
    assert!((snap_to_scale(1, set_of(1, 5, 8)) == 1), "exact hit");
}

// id: music.snap.v1
// compat: stable-semantics
// id: oracle:reference
pub fn snap_to_scale(note: MidiNote, scale: Scale) -> MidiNote {
    debug_assert!(scale_is_nonempty(scale));
    assert!(scale_is_nonempty(scale), "precondition failed: snap_to_scale [REQ-2]");
    // ensures: set_contains(scale, result)
}

// req: REQ-2
pub fn prop_snap_in_scale(n: MidiNote, s: Scale) {
    assert!(set_contains(s, snap_to_scale(n, s)), "property 'snap_in_scale' violated");
}

