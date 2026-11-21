//! Problem generation and answer checking.
//!
//! All randomness comes from an [`Entropy`] source (the hardware TRNG on the
//! device) through unbiased `below`; a failing source aborts generation with
//! an error instead of producing a predictable problem.
//!
//! Guarantees (all tested):
//! * every answer is an exact integer, and subtraction is never negative;
//! * division never has a zero divisor, and never divides by 1 (`n / 1` is
//!   not a drill) -- divisors and quotients are both at least 2 except on
//!   Easy, where the quotient may be 1;
//! * all arithmetic fits in `i32` with a wide margin (largest operand 49).

use precursor_appkit_core::entropy::{below, Entropy, EntropyError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl Operation {
    pub const ALL: [Operation; 4] = [Operation::Add, Operation::Subtract, Operation::Multiply, Operation::Divide];

    pub fn symbol(self) -> &'static str {
        match self {
            Operation::Add => "+",
            Operation::Subtract => "-",
            Operation::Multiply => "x",
            Operation::Divide => "/",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Operation::Add => "Addition",
            Operation::Subtract => "Subtraction",
            Operation::Multiply => "Multiplication",
            Operation::Divide => "Division",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Operation::Add => "add",
            Operation::Subtract => "sub",
            Operation::Multiply => "mul",
            Operation::Divide => "div",
        }
    }
}

/// Which operation(s) a session drills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    Single(Operation),
    Mixed,
}

impl Mode {
    pub const ALL: [Mode; 5] = [
        Mode::Single(Operation::Add),
        Mode::Single(Operation::Subtract),
        Mode::Single(Operation::Multiply),
        Mode::Single(Operation::Divide),
        Mode::Mixed,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Mode::Single(op) => op.label(),
            Mode::Mixed => "Mixed",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Mode::Single(op) => op.key(),
            Mode::Mixed => "mixed",
        }
    }

    /// Next mode in [`Mode::ALL`], wrapping.
    pub fn cycle(self, delta: isize) -> Mode {
        let i = Mode::ALL.iter().position(|m| *m == self).unwrap_or(0) as isize;
        Mode::ALL[(i + delta).rem_euclid(Mode::ALL.len() as isize) as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];

    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy (1-9)",
            Difficulty::Medium => "Medium (2-19)",
            Difficulty::Hard => "Hard (2-49)",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Medium => "medium",
            Difficulty::Hard => "hard",
        }
    }

    pub fn cycle(self, delta: isize) -> Difficulty {
        let i = Difficulty::ALL.iter().position(|m| *m == self).unwrap_or(0) as isize;
        Difficulty::ALL[(i + delta).rem_euclid(Difficulty::ALL.len() as isize) as usize]
    }

    /// Inclusive operand range.
    pub fn range(self) -> (u32, u32) {
        match self {
            Difficulty::Easy => (1, 9),
            Difficulty::Medium => (2, 19),
            Difficulty::Hard => (2, 49),
        }
    }

    /// Base points for a correct answer.
    pub fn base_points(self) -> u32 {
        match self {
            Difficulty::Easy => 10,
            Difficulty::Medium => 20,
            Difficulty::Hard => 30,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Problem {
    pub a: i32,
    pub b: i32,
    pub operation: Operation,
    pub answer: i32,
}

impl Problem {
    pub fn display(&self) -> String { format!("{} {} {} = ?", self.a, self.operation.symbol(), self.b) }

    pub fn display_with_answer(&self) -> String {
        format!("{} {} {} = {}", self.a, self.operation.symbol(), self.b, self.answer)
    }

    pub fn check(&self, user_answer: i32) -> bool { user_answer == self.answer }
}

/// Uniform in `lo..=hi`.
fn operand<E: Entropy + ?Sized>(e: &mut E, lo: u32, hi: u32) -> Result<i32, EntropyError> {
    Ok((lo + below(e, hi - lo + 1)?) as i32)
}

pub fn generate<E: Entropy + ?Sized>(
    e: &mut E,
    operation: Operation,
    difficulty: Difficulty,
) -> Result<Problem, EntropyError> {
    let (lo, hi) = difficulty.range();
    Ok(match operation {
        Operation::Add => {
            let (a, b) = (operand(e, lo, hi)?, operand(e, lo, hi)?);
            Problem { a, b, operation, answer: a + b }
        }
        Operation::Subtract => {
            // Larger operand first, so the answer is never negative. Ordering
            // two independent uniform draws does not bias the pair choice
            // beyond making (a, b) and (b, a) the same problem.
            let (x, y) = (operand(e, lo, hi)?, operand(e, lo, hi)?);
            let (a, b) = if x >= y { (x, y) } else { (y, x) };
            Problem { a, b, operation, answer: a - b }
        }
        Operation::Multiply => {
            let (a, b) = (operand(e, lo, hi)?, operand(e, lo, hi)?);
            Problem { a, b, operation, answer: a * b }
        }
        Operation::Divide => {
            // Pick quotient and divisor, then build the dividend, so the
            // division is always exact. The divisor is at least 2.
            let quotient = operand(e, lo, hi)?;
            let divisor = operand(e, lo.max(2), hi)?;
            Problem { a: quotient * divisor, b: divisor, operation, answer: quotient }
        }
    })
}

pub fn generate_for_mode<E: Entropy + ?Sized>(
    e: &mut E,
    mode: Mode,
    difficulty: Difficulty,
) -> Result<Problem, EntropyError> {
    let op = match mode {
        Mode::Single(op) => op,
        Mode::Mixed => Operation::ALL[below(e, Operation::ALL.len() as u32)? as usize],
    };
    generate(e, op, difficulty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use precursor_appkit_core::entropy::testing::{Broken, Seq, XorShift};

    fn all_cases() -> impl Iterator<Item = (Mode, Difficulty)> {
        Mode::ALL.into_iter().flat_map(|m| Difficulty::ALL.into_iter().map(move |d| (m, d)))
    }

    #[test]
    fn answers_are_exact_and_in_range() {
        let mut e = XorShift(0x1234_5678);
        for (m, d) in all_cases() {
            let (lo, hi) = d.range();
            for _ in 0..2000 {
                let p = generate_for_mode(&mut e, m, d).unwrap();
                match p.operation {
                    Operation::Add => assert_eq!(p.answer, p.a + p.b),
                    Operation::Subtract => {
                        assert!(p.answer >= 0, "{:?}", p);
                        assert_eq!(p.answer, p.a - p.b);
                    }
                    Operation::Multiply => assert_eq!(p.answer, p.a * p.b),
                    Operation::Divide => {
                        assert!(p.b >= 2, "divisor {:?}", p);
                        assert_eq!(p.a % p.b, 0, "inexact {:?}", p);
                        assert_eq!(p.a / p.b, p.answer);
                        assert!(p.answer >= lo as i32 && p.answer <= hi as i32);
                    }
                }
                assert!(p.b >= lo as i32 && p.b <= hi as i32);
            }
        }
    }

    #[test]
    fn division_never_by_zero_or_one_even_at_extremes() {
        // Smallest and largest possible draws.
        for first in [5u32, u32::MAX] {
            let mut e = Seq(vec![first; 8], 0);
            let p = generate(&mut e, Operation::Divide, Difficulty::Easy).unwrap();
            assert!(p.b >= 2);
        }
    }

    #[test]
    fn every_operand_value_is_reachable() {
        let mut e = XorShift(99);
        let mut seen = [false; 50];
        for _ in 0..5000 {
            let p = generate(&mut e, Operation::Add, Difficulty::Hard).unwrap();
            seen[p.a as usize] = true;
        }
        assert!(!seen[0] && !seen[1], "below the range");
        assert!(seen[2..50].iter().all(|s| *s), "gap in the range");
    }

    #[test]
    fn operands_are_roughly_uniform() {
        let mut e = XorShift(7);
        let mut counts = [0u32; 10];
        let n = 90_000;
        for _ in 0..n {
            let p = generate(&mut e, Operation::Multiply, Difficulty::Easy).unwrap();
            counts[p.a as usize] += 1;
        }
        let expect = n / 9;
        for c in &counts[1..] {
            assert!((*c as i64 - expect as i64).abs() < (expect as i64) / 10, "{:?}", counts);
        }
    }

    #[test]
    fn mixed_uses_every_operation() {
        let mut e = XorShift(5);
        let mut seen = [false; 4];
        for _ in 0..200 {
            let p = generate_for_mode(&mut e, Mode::Mixed, Difficulty::Easy).unwrap();
            seen[Operation::ALL.iter().position(|o| *o == p.operation).unwrap()] = true;
        }
        assert!(seen.iter().all(|s| *s));
    }

    #[test]
    fn broken_entropy_is_an_error() {
        for (m, d) in all_cases() {
            assert!(generate_for_mode(&mut Broken, m, d).is_err());
        }
    }

    #[test]
    fn mode_and_difficulty_cycle_wrap() {
        assert_eq!(Mode::Mixed.cycle(1), Mode::Single(Operation::Add));
        assert_eq!(Mode::Single(Operation::Add).cycle(-1), Mode::Mixed);
        assert_eq!(Difficulty::Hard.cycle(1), Difficulty::Easy);
        assert_eq!(Difficulty::Easy.cycle(-1), Difficulty::Hard);
    }

    #[test]
    fn display_formats() {
        let p = Problem { a: 12, b: 4, operation: Operation::Divide, answer: 3 };
        assert_eq!(p.display(), "12 / 4 = ?");
        assert_eq!(p.display_with_answer(), "12 / 4 = 3");
        assert!(p.check(3) && !p.check(4));
    }
}
