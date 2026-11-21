//! Pure logic for Math Drill: problem generation, scoring and the quiz state
//! machine. No Xous dependencies.

pub mod drill;
pub mod problems;
pub mod scoring;

pub use drill::{Answered, Drill, Field, Input, Outcome, Screen, FEEDBACK_MS, PROBLEMS_PER_SESSION};
pub use problems::{Difficulty, Mode, Operation, Problem};
pub use scoring::{Best, Improved, LoadError, Saved};
