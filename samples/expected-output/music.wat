(module $music_scale
  ;; Generated from speclang module `music::scale`

  ;; WASI preview-1 imports
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32))
  )
  (import "wasi_snapshot_preview1" "proc_exit"
    (func $proc_exit (param i32))
  )

  ;; Linear memory (1 page = 64KB)
  (memory (export "memory") 1)

  ;; Stack pointer
  (global $sp (mut i32) (i32.const 1024))

  ;; Type definitions
  ;; type MidiNote = Int
  ;; type Scale = Set<MidiNote>
  ;; enum SnapError { EmptyScale = 0 }

  (func $new_MidiNote (export "new_MidiNote") (param $value i64) (result i64)
    ;; requires contract
    i32.const 1
    local.get $value
    i32.le_s
    local.get $value
    i32.const 12
    i32.le_s
    i32.and
    i32.eqz
    (if (then (unreachable)))
    i32.const 1
    local.get $value
    i32.le_s
    local.get $value
    i32.const 12
    i32.le_s
    i32.and
    i32.eqz
    ;; assert: precondition failed: new_MidiNote
    (if (then (unreachable)))
    local.get $value
  )

  ;; @req REQ-3
  (func $test_snap_to_scale_0 (export "test_snap_to_scale_0") 
    i32.const 12
    i32.const 1
    i32.const 5
    i32.const 8
    call $set_of
    call $snap_to_scale
    i32.const 1
    i32.eq
    i32.eqz
    ;; assert: edge wraps
    (if (then (unreachable)))
  )

  ;; @req REQ-3
  (func $test_snap_to_scale_1 (export "test_snap_to_scale_1") 
    i32.const 1
    i32.const 1
    i32.const 5
    i32.const 8
    call $set_of
    call $snap_to_scale
    i32.const 1
    i32.eq
    i32.eqz
    ;; assert: exact hit
    (if (then (unreachable)))
  )

  ;; @id music.snap.v1
  ;; @compat stable-semantics
  ;; @id oracle:reference
  (func $snap_to_scale (export "snap_to_scale") (param $note i32) (param $scale i32) (result i32)
    ;; requires contract
    local.get $scale
    call $scale_is_nonempty
    i32.eqz
    (if (then (unreachable)))
    local.get $scale
    call $scale_is_nonempty
    i32.eqz
    ;; assert: precondition failed: snap_to_scale [REQ-2]
    (if (then (unreachable)))
    ;; ensures: <contract>
  )

  ;; @req REQ-2
  (func $prop_snap_in_scale (export "prop_snap_in_scale") (param $n i32) (param $s i32)
    local.get $s
    local.get $n
    local.get $s
    call $snap_to_scale
    call $set_contains
    i32.eqz
    ;; assert: property 'snap_in_scale' violated
    (if (then (unreachable)))
  )

)
