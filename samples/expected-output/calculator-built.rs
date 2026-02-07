 40 | impl Parser {
...
109 |     fn check(&self, kind: &TokenKind) -> bool {
46 | impl LowerCtx {
...
54 |     fn err(&mut self, msg: impl Into<String>) {
//! Generated from speclang module `calculator`
#![allow(dead_code, unused_variables)]
// req: REQ-1
pub fn test_add_0() {
    assert!((add(2, 3) == 5), "positive");
}
// req: REQ-1
pub fn test_add_1() {
    assert!((add(0, 0) == 0), "zeros");
}
// req: REQ-1
pub fn test_add_2() {
    assert!((add(1000, 2000) == 3000), "large");
}
// req: REQ-1
pub fn test_subtract_0() {
    assert!((subtract(10, 4) == 6), "basic");
}
// req: REQ-1
pub fn test_subtract_1() {
    assert!((subtract(7, 7) == 0), "same");
}
// req: REQ-1
pub fn test_multiply_0() {
    assert!((multiply(3, 4) == 12), "basic");
}
// req: REQ-1
pub fn test_multiply_1() {
    assert!((multiply(5, 0) == 0), "by_zero");
}
// req: REQ-1
pub fn test_multiply_2() {
    assert!((multiply(7, 1) == 7), "by_one");
}
// req: REQ-1
pub fn test_divide_0() {
    assert!((divide(10, 2) == 5), "basic");
}
// req: REQ-1
pub fn test_divide_1() {
    assert!((divide(7, 2) == 3), "truncates");
}
pub fn test_factorial_0() {
    assert!((factorial(0) == 1), "zero");
}
pub fn test_factorial_1() {
    assert!((factorial(1) == 1), "one");
}
pub fn test_factorial_2() {
    assert!((factorial(5) == 120), "five");
}
pub fn test_factorial_3() {
    assert!((factorial(6) == 720), "six");
}
pub fn test_power_0() {
    assert!((power(3, 2) == 9), "square");
}
pub fn test_power_1() {
    assert!((power(2, 3) == 8), "cube");
}
pub fn test_power_2() {
    assert!((power(5, 0) == 1), "zero_exp");
}
pub fn test_power_3() {
    assert!((power(7, 1) == 7), "one_exp");
}
// id: calc.add.v1
pub fn add(a: i64, b: i64) -> i64 {
    (a + b)
}
// id: calc.sub.v1
pub fn subtract(a: i64, b: i64) -> i64 {
    (a - b)
}
// id: calc.mul.v1
pub fn multiply(a: i64, b: i64) -> i64 {
    (a * b)
}
// id: calc.div.v1
pub fn divide(a: i64, b: i64) -> i64 {
    (a / b)
}
// id: calc.fact.v1
pub fn factorial(n: i64) -> i64 {
    if (n <= 1) { 1 } else { (n * factorial((n - 1))) }
}
// id: calc.pow.v1
pub fn power(base: i64, exp: i64) -> i64 {
    if (exp == 0) { 1 } else { (base * power(base, (exp - 1))) }
}
