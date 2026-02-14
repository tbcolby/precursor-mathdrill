//! Problem generation and answer checking for Math Drill.
//!
//! Uses TRNG for cryptographically random operand generation.
//! Division always produces clean integer results.

extern crate alloc;
use alloc::string::String;
use alloc::format;

use crate::rng::Rng;

/// Arithmetic operation type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl Operation {
    pub fn all() -> &'static [Operation] {
        &[Operation::Add, Operation::Subtract, Operation::Multiply, Operation::Divide]
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            Operation::Add => "+",
            Operation::Subtract => "-",
            Operation::Multiply => "x",
            Operation::Divide => "/",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Operation::Add => "Addition",
            Operation::Subtract => "Subtraction",
            Operation::Multiply => "Multiplication",
            Operation::Divide => "Division",
        }
    }
}

/// Difficulty level controlling operand ranges.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    pub fn all() -> &'static [Difficulty] {
        &[Difficulty::Easy, Difficulty::Medium, Difficulty::Hard]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy (1-9)",
            Difficulty::Medium => "Medium (2-19)",
            Difficulty::Hard => "Hard (2-49)",
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Medium => "medium",
            Difficulty::Hard => "hard",
        }
    }

    fn operand_range(&self) -> (u32, u32) {
        match self {
            Difficulty::Easy => (1, 9),
            Difficulty::Medium => (2, 19),
            Difficulty::Hard => (2, 49),
        }
    }
}

/// A math problem with two operands and an operation.
#[derive(Debug, Clone)]
pub struct Problem {
    pub a: i32,
    pub b: i32,
    pub operation: Operation,
    pub answer: i32,
}

impl Problem {
    pub fn display(&self) -> String {
        format!("{} {} {} = ?", self.a, self.operation.symbol(), self.b)
    }

    pub fn display_with_answer(&self) -> String {
        format!("{} {} {} = {}", self.a, self.operation.symbol(), self.b, self.answer)
    }

    pub fn check(&self, user_answer: i32) -> bool {
        user_answer == self.answer
    }
}

/// Generate a random problem using TRNG.
pub fn generate(rng: &Rng, operation: Operation, difficulty: Difficulty) -> Problem {
    let (min, max) = difficulty.operand_range();

    match operation {
        Operation::Add => {
            let a = rng.range_inclusive(min, max) as i32;
            let b = rng.range_inclusive(min, max) as i32;
            Problem { a, b, operation, answer: a + b }
        }
        Operation::Subtract => {
            let mut a = rng.range_inclusive(min, max) as i32;
            let mut b = rng.range_inclusive(min, max) as i32;
            if b > a {
                core::mem::swap(&mut a, &mut b);
            }
            Problem { a, b, operation, answer: a - b }
        }
        Operation::Multiply => {
            let a = rng.range_inclusive(min, max) as i32;
            let b = rng.range_inclusive(min, max) as i32;
            Problem { a, b, operation, answer: a * b }
        }
        Operation::Divide => {
            // Generate answer and divisor, then compute dividend
            // to ensure clean integer division
            let answer = rng.range_inclusive(min, max) as i32;
            let b = rng.range_inclusive(min, max) as i32;
            let a = answer * b;
            Problem { a, b, operation, answer }
        }
    }
}

/// Generate a problem with a randomly selected operation.
pub fn generate_mixed(rng: &Rng, difficulty: Difficulty) -> Problem {
    let ops = Operation::all();
    let idx = rng.range(ops.len() as u32) as usize;
    generate(rng, ops[idx], difficulty)
}
