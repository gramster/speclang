//! Generated from speclang module `calculator`
#![allow(dead_code, unused_variables)]

// id: calc.add.v1
// compat: stable-call
pub fn add(a: i64, b: i64) -> i64 {
    todo!()
}

// id: calc.sub.v1
// compat: stable-call
pub fn subtract(a: i64, b: i64) -> i64 {
    todo!()
}

// id: calc.mul.v1
// compat: stable-call
pub fn multiply(a: i64, b: i64) -> i64 {
    todo!()
}

// id: calc.div.v1
// compat: stable-call
pub fn divide(a: i64, b: i64) -> i64 {
    assert!((b != 0), "precondition failed: divide [REQ-2]");
}

// id: calc.fact.v1
// compat: stable-call
pub fn factorial(n: i64) -> i64 {
    assert!((n >= 0), "precondition failed: factorial [REQ-3]");
    // ensures: (result >= 1)
}

// id: calc.pow.v1
// compat: stable-call
pub fn power(base: i64, exp: i64) -> i64 {
    assert!((exp >= 0), "precondition failed: power [REQ-4]");
    // ensures: ((result >= 0) || (base < 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    // req: REQ-1
    #[test]
    fn test_add_0() {
        assert!((add(2, 3) == 5), "positive");
    }

    // req: REQ-1
    #[test]
    fn test_add_1() {
        assert!((add(0, 0) == 0), "zeros");
    }

    // req: REQ-1
    #[test]
    fn test_add_2() {
        assert!((add(1000, 2000) == 3000), "large");
    }

    // req: REQ-1
    #[test]
    fn test_subtract_0() {
        assert!((subtract(10, 4) == 6), "basic");
    }

    // req: REQ-1
    #[test]
    fn test_subtract_1() {
        assert!((subtract(7, 7) == 0), "same");
    }

    // req: REQ-1
    #[test]
    fn test_multiply_0() {
        assert!((multiply(3, 4) == 12), "basic");
    }

    // req: REQ-1
    #[test]
    fn test_multiply_1() {
        assert!((multiply(5, 0) == 0), "by_zero");
    }

    // req: REQ-1
    #[test]
    fn test_multiply_2() {
        assert!((multiply(7, 1) == 7), "by_one");
    }

    // req: REQ-1
    #[test]
    fn test_divide_0() {
        assert!((divide(10, 2) == 5), "basic");
    }

    // req: REQ-1
    #[test]
    fn test_divide_1() {
        assert!((divide(7, 2) == 3), "truncates");
    }

    #[test]
    fn test_factorial_0() {
        assert!((factorial(0) == 1), "zero");
    }

    #[test]
    fn test_factorial_1() {
        assert!((factorial(1) == 1), "one");
    }

    #[test]
    fn test_factorial_2() {
        assert!((factorial(5) == 120), "five");
    }

    #[test]
    fn test_factorial_3() {
        assert!((factorial(6) == 720), "six");
    }

    #[test]
    fn test_power_0() {
        assert!((power(3, 2) == 9), "square");
    }

    #[test]
    fn test_power_1() {
        assert!((power(2, 3) == 8), "cube");
    }

    #[test]
    fn test_power_2() {
        assert!((power(5, 0) == 1), "zero_exp");
    }

    #[test]
    fn test_power_3() {
        assert!((power(7, 1) == 7), "one_exp");
    }

}
