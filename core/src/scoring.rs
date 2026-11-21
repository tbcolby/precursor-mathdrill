//! Points, best scores and the stored format.

use crate::problems::{Difficulty, Mode};
use serde::{Deserialize, Serialize};

pub const STATE_VERSION: u32 = 1;

/// Answers faster than this earn a speed bonus that shrinks linearly to 0.
pub const BONUS_WINDOW_MS: u64 = 10_000;
/// Extra points per answer once the streak reaches [`STREAK_AFTER`].
pub const STREAK_BONUS: u32 = 5;
pub const STREAK_AFTER: u32 = 3;

/// Points for one answer. `streak` is the streak *including* this answer when
/// it is correct. Wrong answers score 0. Never overflows: at most
/// `30 + 30 + 5` per answer.
pub fn points(difficulty: Difficulty, correct: bool, elapsed_ms: u64, streak: u32) -> u32 {
    if !correct {
        return 0;
    }
    let base = difficulty.base_points();
    let left = BONUS_WINDOW_MS.saturating_sub(elapsed_ms);
    let speed = (base as u64 * left / BONUS_WINDOW_MS) as u32;
    let streak_bonus = if streak >= STREAK_AFTER { STREAK_BONUS } else { 0 };
    base + speed + streak_bonus
}

/// Best results for one (mode, difficulty). Each metric is tracked
/// independently: a session that improves one never lowers another.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Best {
    pub sessions: u32,
    pub score: u32,
    pub correct: u32,
    pub streak: u32,
    /// Fastest session average (ms per problem) among sessions with at
    /// least one correct answer. `None` until there is one.
    pub fastest_avg_ms: Option<u32>,
}

/// What a finished session achieved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionResult {
    pub score: u32,
    pub correct: u32,
    pub streak: u32,
    pub avg_ms: u32,
}

/// Which metrics a session improved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Improved {
    pub score: bool,
    pub correct: bool,
    pub streak: bool,
    pub speed: bool,
}

impl Improved {
    pub fn any(&self) -> bool { self.score || self.correct || self.streak || self.speed }
}

impl Best {
    pub fn record(&mut self, r: &SessionResult) -> Improved {
        let mut imp = Improved::default();
        self.sessions = self.sessions.saturating_add(1);
        if r.score > self.score {
            self.score = r.score;
            imp.score = true;
        }
        if r.correct > self.correct {
            self.correct = r.correct;
            imp.correct = true;
        }
        if r.streak > self.streak {
            self.streak = r.streak;
            imp.streak = true;
        }
        if r.correct > 0 && self.fastest_avg_ms.is_none_or(|f| r.avg_ms < f) {
            self.fastest_avg_ms = Some(r.avg_ms);
            imp.speed = true;
        }
        imp
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BestEntry {
    pub mode: Mode,
    pub difficulty: Difficulty,
    pub best: Best,
}

/// The one stored blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Saved {
    pub version: u32,
    pub mode: Mode,
    pub difficulty: Difficulty,
    pub bests: Vec<BestEntry>,
}

impl Default for Saved {
    fn default() -> Self {
        Saved {
            version: STATE_VERSION,
            mode: Mode::Single(crate::problems::Operation::Add),
            difficulty: Difficulty::Easy,
            bests: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// Written by a newer app version; don't overwrite it.
    NewerVersion(u32),
}

impl Saved {
    pub fn check(&self) -> Result<(), LoadError> {
        if self.version > STATE_VERSION {
            Err(LoadError::NewerVersion(self.version))
        } else {
            Ok(())
        }
    }

    pub fn best(&self, mode: Mode, difficulty: Difficulty) -> Option<&Best> {
        self.bests.iter().find(|e| e.mode == mode && e.difficulty == difficulty).map(|e| &e.best)
    }

    pub fn record(&mut self, mode: Mode, difficulty: Difficulty, r: &SessionResult) -> Improved {
        if let Some(e) = self.bests.iter_mut().find(|e| e.mode == mode && e.difficulty == difficulty) {
            return e.best.record(r);
        }
        let mut best = Best::default();
        let imp = best.record(r);
        self.bests.push(BestEntry { mode, difficulty, best });
        imp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problems::Operation;

    fn res(score: u32, correct: u32, streak: u32, avg_ms: u32) -> SessionResult {
        SessionResult { score, correct, streak, avg_ms }
    }

    #[test]
    fn wrong_answers_score_nothing() {
        assert_eq!(points(Difficulty::Hard, false, 0, 9), 0);
    }

    #[test]
    fn speed_bonus_shrinks_and_ends() {
        let fast = points(Difficulty::Medium, true, 0, 1);
        let mid = points(Difficulty::Medium, true, 5_000, 1);
        let slow = points(Difficulty::Medium, true, 10_000, 1);
        let way_slow = points(Difficulty::Medium, true, u64::MAX, 1);
        assert_eq!(fast, 40);
        assert_eq!(mid, 30);
        assert_eq!(slow, 20);
        assert_eq!(way_slow, 20);
    }

    #[test]
    fn streak_bonus_starts_at_three() {
        let p2 = points(Difficulty::Easy, true, u64::MAX, 2);
        let p3 = points(Difficulty::Easy, true, u64::MAX, 3);
        assert_eq!(p3 - p2, STREAK_BONUS);
    }

    #[test]
    fn harder_levels_pay_more() {
        let e = points(Difficulty::Easy, true, 2_000, 1);
        let m = points(Difficulty::Medium, true, 2_000, 1);
        let h = points(Difficulty::Hard, true, 2_000, 1);
        assert!(e < m && m < h);
    }

    #[test]
    fn extreme_inputs_do_not_overflow() {
        assert!(points(Difficulty::Hard, true, 0, u32::MAX) <= 65);
    }

    #[test]
    fn best_never_goes_down_regression() {
        // Audit: 9/10 streak 5, then 6/10 streak 6 replaced the whole record.
        let mut b = Best::default();
        b.record(&res(300, 9, 5, 3000));
        let imp = b.record(&res(200, 6, 6, 4000));
        assert_eq!(b.correct, 9);
        assert_eq!(b.score, 300);
        assert_eq!(b.streak, 6);
        assert_eq!(b.fastest_avg_ms, Some(3000));
        assert!(imp.streak && !imp.correct && !imp.score && !imp.speed);
        assert_eq!(b.sessions, 2);
    }

    #[test]
    fn zero_correct_sessions_do_not_set_speed() {
        let mut b = Best::default();
        let imp = b.record(&res(0, 0, 0, 0));
        assert_eq!(b.fastest_avg_ms, None);
        assert!(!imp.any());
    }

    #[test]
    fn bests_are_per_mode_and_difficulty() {
        let mut s = Saved::default();
        let add = Mode::Single(Operation::Add);
        s.record(add, Difficulty::Easy, &res(100, 10, 10, 1000));
        s.record(Mode::Mixed, Difficulty::Easy, &res(50, 5, 5, 2000));
        assert_eq!(s.best(add, Difficulty::Easy).unwrap().correct, 10);
        assert_eq!(s.best(Mode::Mixed, Difficulty::Easy).unwrap().correct, 5);
        assert!(s.best(add, Difficulty::Hard).is_none());
    }

    #[test]
    fn json_round_trip_and_version_gate() {
        let mut s = Saved::default();
        s.record(Mode::Mixed, Difficulty::Hard, &res(10, 1, 1, 9000));
        let json = serde_json::to_string(&s).unwrap();
        let back: Saved = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
        assert!(back.check().is_ok());
        let newer = Saved { version: 99, ..Saved::default() };
        assert_eq!(newer.check(), Err(LoadError::NewerVersion(99)));
    }
}
