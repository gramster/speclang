(module $math_clamp
  ;; Generated from speclang module `math::clamp`

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
  ;; type Bound = Int

  ;; @req REQ-2
  (func $test_clamp_0 (export "test_clamp_0") 
    i32.const 0
    i32.const 1
    i32.const 10
    call $clamp
    i32.const 1
    i32.eq
    i32.eqz
    ;; assert: below range
    (if (then (unreachable)))
  )

  ;; @req REQ-2
  (func $test_clamp_1 (export "test_clamp_1") 
    i32.const 99
    i32.const 1
    i32.const 10
    call $clamp
    i32.const 10
    i32.eq
    i32.eqz
    ;; assert: above range
    (if (then (unreachable)))
  )

  ;; @req REQ-2
  (func $test_clamp_2 (export "test_clamp_2") 
    i32.const 5
    i32.const 1
    i32.const 10
    call $clamp
    i32.const 5
    i32.eq
    i32.eqz
    ;; assert: within range
    (if (then (unreachable)))
  )

  ;; @id math.clamp.v1
  ;; @compat stable-call
  (func $clamp (export "clamp") (param $value i64) (param $lo i32) (param $hi i32) (result i64)
    ;; requires contract
    local.get $lo
    local.get $hi
    i32.le_s
    i32.eqz
    (if (then (unreachable)))
    local.get $lo
    local.get $hi
    i32.le_s
    i32.eqz
    ;; assert: precondition failed: clamp
    (if (then (unreachable)))
    ;; ensures: <contract>
    ;; ensures: <contract>
  )

)
