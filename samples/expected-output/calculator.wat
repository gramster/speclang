 40 | impl Parser {
...
109 |     fn check(&self, kind: &TokenKind) -> bool {
46 | impl LowerCtx {
...
54 |     fn err(&mut self, msg: impl Into<String>) {
(module $calculator
  ;; Generated from speclang module `calculator`
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
  ;; @req REQ-1
  (func $test_add_0 (export "test_add_0") 
    i32.const 2
    i32.const 3
    call $add
    i32.const 5
    i32.eq
    i32.eqz
    ;; assert: positive
    (if (then (unreachable)))
  )
  ;; @req REQ-1
  (func $test_add_1 (export "test_add_1") 
    i32.const 0
    i32.const 0
    call $add
    i32.const 0
    i32.eq
    i32.eqz
    ;; assert: zeros
    (if (then (unreachable)))
  )
  ;; @req REQ-1
  (func $test_add_2 (export "test_add_2") 
    i32.const 1000
    i32.const 2000
    call $add
    i32.const 3000
    i32.eq
    i32.eqz
    ;; assert: large
    (if (then (unreachable)))
  )
  ;; @id calc.add.v1
  ;; @compat stable-call
  (func $add (export "add") (param $a i64) (param $b i64) (result i64)
  )
  ;; @req REQ-1
  (func $test_subtract_0 (export "test_subtract_0") 
    i32.const 10
    i32.const 4
    call $subtract
    i32.const 6
    i32.eq
    i32.eqz
    ;; assert: basic
    (if (then (unreachable)))
  )
  ;; @req REQ-1
  (func $test_subtract_1 (export "test_subtract_1") 
    i32.const 7
    i32.const 7
    call $subtract
    i32.const 0
    i32.eq
    i32.eqz
    ;; assert: same
    (if (then (unreachable)))
  )
  ;; @id calc.sub.v1
  ;; @compat stable-call
  (func $subtract (export "subtract") (param $a i64) (param $b i64) (result i64)
  )
  ;; @req REQ-1
  (func $test_multiply_0 (export "test_multiply_0") 
    i32.const 3
    i32.const 4
    call $multiply
    i32.const 12
    i32.eq
    i32.eqz
    ;; assert: basic
    (if (then (unreachable)))
  )
  ;; @req REQ-1
  (func $test_multiply_1 (export "test_multiply_1") 
    i32.const 5
    i32.const 0
    call $multiply
    i32.const 0
    i32.eq
    i32.eqz
    ;; assert: by_zero
    (if (then (unreachable)))
  )
  ;; @req REQ-1
  (func $test_multiply_2 (export "test_multiply_2") 
    i32.const 7
    i32.const 1
    call $multiply
    i32.const 7
    i32.eq
    i32.eqz
    ;; assert: by_one
    (if (then (unreachable)))
  )
  ;; @id calc.mul.v1
  ;; @compat stable-call
  (func $multiply (export "multiply") (param $a i64) (param $b i64) (result i64)
  )
  ;; @req REQ-1
  (func $test_divide_0 (export "test_divide_0") 
    i32.const 10
    i32.const 2
    call $divide
    i32.const 5
    i32.eq
    i32.eqz
    ;; assert: basic
    (if (then (unreachable)))
  )
  ;; @req REQ-1
  (func $test_divide_1 (export "test_divide_1") 
    i32.const 7
    i32.const 2
    call $divide
    i32.const 3
    i32.eq
    i32.eqz
    ;; assert: truncates
    (if (then (unreachable)))
  )
  ;; @id calc.div.v1
  ;; @compat stable-call
  (func $divide (export "divide") (param $a i64) (param $b i64) (result i64)
    ;; requires contract
    local.get $b
    i32.const 0
    i32.ne
    i32.eqz
    (if (then (unreachable)))
    local.get $b
    i32.const 0
    i32.ne
    i32.eqz
    ;; assert: precondition failed: divide [REQ-2]
    (if (then (unreachable)))
  )
  (func $test_factorial_0 (export "test_factorial_0") 
    i32.const 0
    call $factorial
    i32.const 1
    i32.eq
    i32.eqz
    ;; assert: zero
    (if (then (unreachable)))
  )
  (func $test_factorial_1 (export "test_factorial_1") 
    i32.const 1
    call $factorial
    i32.const 1
    i32.eq
    i32.eqz
    ;; assert: one
    (if (then (unreachable)))
  )
  (func $test_factorial_2 (export "test_factorial_2") 
    i32.const 5
    call $factorial
    i32.const 120
    i32.eq
    i32.eqz
    ;; assert: five
    (if (then (unreachable)))
  )
  (func $test_factorial_3 (export "test_factorial_3") 
    i32.const 6
    call $factorial
    i32.const 720
    i32.eq
    i32.eqz
    ;; assert: six
    (if (then (unreachable)))
  )
  ;; @id calc.fact.v1
  ;; @compat stable-call
  (func $factorial (export "factorial") (param $n i64) (result i64)
    ;; requires contract
    local.get $n
    i32.const 0
    i32.ge_s
    i32.eqz
    (if (then (unreachable)))
    local.get $n
    i32.const 0
    i32.ge_s
    i32.eqz
    ;; assert: precondition failed: factorial [REQ-3]
    (if (then (unreachable)))
    ;; ensures: <contract>
  )
  (func $test_power_0 (export "test_power_0") 
    i32.const 3
    i32.const 2
    call $power
    i32.const 9
    i32.eq
    i32.eqz
    ;; assert: square
    (if (then (unreachable)))
  )
  (func $test_power_1 (export "test_power_1") 
    i32.const 2
    i32.const 3
    call $power
    i32.const 8
    i32.eq
    i32.eqz
    ;; assert: cube
    (if (then (unreachable)))
  )
  (func $test_power_2 (export "test_power_2") 
    i32.const 5
    i32.const 0
    call $power
    i32.const 1
    i32.eq
    i32.eqz
    ;; assert: zero_exp
    (if (then (unreachable)))
  )
  (func $test_power_3 (export "test_power_3") 
    i32.const 7
    i32.const 1
    call $power
    i32.const 7
    i32.eq
    i32.eqz
    ;; assert: one_exp
    (if (then (unreachable)))
  )
  ;; @id calc.pow.v1
  ;; @compat stable-call
  (func $power (export "power") (param $base i64) (param $exp i64) (result i64)
    ;; requires contract
    local.get $exp
    i32.const 0
    i32.ge_s
    i32.eqz
    (if (then (unreachable)))
    local.get $exp
    i32.const 0
    i32.ge_s
    i32.eqz
    ;; assert: precondition failed: power [REQ-4]
    (if (then (unreachable)))
    ;; ensures: <contract>
  )
)
