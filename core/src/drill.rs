//! The quiz state machine. Time is passed in (`now_ms`, monotonic), so tests
//! are deterministic and the shell can pause the clock while backgrounded.

use crate::problems::{generate_for_mode, Difficulty, Mode, Problem};
use crate::scoring::{points, Improved, LoadError, Saved, SessionResult};
use precursor_appkit_core::entropy::Entropy;

pub const PROBLEMS_PER_SESSION: usize = 10;
/// Feedback screen auto-advances after this long.
pub const FEEDBACK_MS: u64 = 1500;
/// Longest answer: sign plus six digits, always fits in i32.
pub const MAX_ANSWER_CHARS: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Menu,
    Playing,
    Feedback,
    Results,
    Best,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Mode,
    Difficulty,
    Start,
    Best,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Up,
    Down,
    Left,
    Right,
    Enter,
    /// Backspace: deletes a digit while answering; Back elsewhere.
    Delete,
    /// F4: always Back.
    Back,
    Char(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Stay,
    Leave,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answered {
    pub problem: Problem,
    /// `None` when the answer wasn't a number (never happens via the UI).
    pub given: i32,
    pub correct: bool,
    pub points: u32,
}

pub struct Drill {
    pub screen: Screen,
    pub field: Field,
    pub mode: Mode,
    pub difficulty: Difficulty,
    pub saved: Saved,
    /// Selection or bests changed since the last [`Drill::take_dirty`].
    dirty: bool,
    /// Set when stored data can't be used; the shell refuses to save then.
    pub read_only: bool,

    pub problem: Option<Problem>,
    pub answer: String,
    pub answered: Vec<Answered>,
    pub streak: u32,
    pub best_streak: u32,
    pub score: u32,
    pub total_ms: u64,
    problem_start: u64,
    feedback_start: u64,
    paused_at: Option<u64>,
    pub improved: Improved,
    pub status: String,
}

impl Default for Drill {
    fn default() -> Self { Self::new() }
}

impl Drill {
    pub fn new() -> Self {
        let saved = Saved::default();
        Drill {
            screen: Screen::Menu,
            field: Field::Mode,
            mode: saved.mode,
            difficulty: saved.difficulty,
            saved,
            dirty: false,
            read_only: false,
            problem: None,
            answer: String::new(),
            answered: Vec::new(),
            streak: 0,
            best_streak: 0,
            score: 0,
            total_ms: 0,
            problem_start: 0,
            feedback_start: 0,
            paused_at: None,
            improved: Improved::default(),
            status: String::new(),
        }
    }

    /// Apply stored data (start-up and basis changes). A session in progress
    /// keeps running; only selection and bests are replaced.
    pub fn apply_saved(&mut self, s: Saved) -> Result<(), LoadError> {
        s.check()?;
        self.mode = s.mode;
        self.difficulty = s.difficulty;
        self.saved = s;
        self.read_only = false;
        self.dirty = false;
        Ok(())
    }

    pub fn mark_unreadable(&mut self, msg: &str) {
        self.read_only = true;
        self.status = msg.to_string();
    }

    /// Data to store, if anything changed. Clears the flag.
    pub fn take_dirty(&mut self) -> Option<Saved> {
        if !self.dirty {
            return None;
        }
        self.dirty = false;
        let mut s = self.saved.clone();
        s.mode = self.mode;
        s.difficulty = self.difficulty;
        Some(s)
    }

    pub fn is_dirty(&self) -> bool { self.dirty }

    /// Put the flag back (a save failed).
    pub fn set_dirty(&mut self) { self.dirty = true; }

    pub fn best_now(&self) -> Option<&crate::scoring::Best> { self.saved.best(self.mode, self.difficulty) }

    pub fn avg_ms(&self) -> u32 {
        if self.answered.is_empty() {
            0
        } else {
            (self.total_ms / self.answered.len() as u64).min(u32::MAX as u64) as u32
        }
    }

    pub fn correct_count(&self) -> u32 { self.answered.iter().filter(|a| a.correct).count() as u32 }

    /// Elapsed ms on the current problem (0 when paused or not playing).
    pub fn elapsed_on_problem(&self, now_ms: u64) -> u64 {
        match (self.screen, self.paused_at) {
            (Screen::Playing, None) => now_ms.saturating_sub(self.problem_start),
            (Screen::Playing, Some(p)) => p.saturating_sub(self.problem_start),
            _ => 0,
        }
    }

    /// True when a periodic tick is useful (clock display or feedback timeout).
    pub fn wants_tick(&self) -> bool { matches!(self.screen, Screen::Playing | Screen::Feedback) }

    /// Freeze the clock (app went to the background).
    pub fn pause(&mut self, now_ms: u64) {
        if self.paused_at.is_none() && self.wants_tick() {
            self.paused_at = Some(now_ms);
        }
    }

    /// Unfreeze: time spent paused is not charged to the player.
    pub fn resume(&mut self, now_ms: u64) {
        if let Some(p) = self.paused_at.take() {
            let gap = now_ms.saturating_sub(p);
            self.problem_start = self.problem_start.saturating_add(gap);
            self.feedback_start = self.feedback_start.saturating_add(gap);
        }
    }

    /// Periodic tick: advances feedback after [`FEEDBACK_MS`]. Returns true if
    /// the screen changed.
    pub fn tick<E: Entropy + ?Sized>(&mut self, now_ms: u64, e: &mut E) -> bool {
        if self.screen == Screen::Feedback
            && self.paused_at.is_none()
            && now_ms.saturating_sub(self.feedback_start) >= FEEDBACK_MS
        {
            self.advance(now_ms, e);
            return true;
        }
        false
    }

    fn advance<E: Entropy + ?Sized>(&mut self, now_ms: u64, e: &mut E) {
        if self.answered.len() >= PROBLEMS_PER_SESSION {
            self.finish();
        } else {
            self.next_problem(now_ms, e);
        }
    }

    fn next_problem<E: Entropy + ?Sized>(&mut self, now_ms: u64, e: &mut E) {
        match generate_for_mode(e, self.mode, self.difficulty) {
            Ok(p) => {
                self.problem = Some(p);
                self.answer.clear();
                self.problem_start = now_ms;
                self.screen = Screen::Playing;
            }
            Err(_) => {
                self.status = "Random number source failed; quiz stopped".to_string();
                self.problem = None;
                self.screen = Screen::Menu;
            }
        }
    }

    fn start<E: Entropy + ?Sized>(&mut self, now_ms: u64, e: &mut E) {
        self.answered.clear();
        self.streak = 0;
        self.best_streak = 0;
        self.score = 0;
        self.total_ms = 0;
        self.improved = Improved::default();
        self.status.clear();
        self.paused_at = None;
        self.next_problem(now_ms, e);
    }

    fn submit(&mut self, now_ms: u64) {
        let Some(problem) = self.problem else { return };
        let Ok(given) = self.answer.parse::<i32>() else { return };
        let elapsed = self.elapsed_on_problem(now_ms);
        let correct = problem.check(given);
        if correct {
            self.streak += 1;
            self.best_streak = self.best_streak.max(self.streak);
        } else {
            self.streak = 0;
        }
        let pts = points(self.difficulty, correct, elapsed, self.streak);
        self.score = self.score.saturating_add(pts);
        self.total_ms = self.total_ms.saturating_add(elapsed);
        self.answered.push(Answered { problem, given, correct, points: pts });
        self.feedback_start = now_ms;
        self.screen = Screen::Feedback;
    }

    fn finish(&mut self) {
        let r = SessionResult {
            score: self.score,
            correct: self.correct_count(),
            streak: self.best_streak,
            avg_ms: self.avg_ms(),
        };
        self.improved = self.saved.record(self.mode, self.difficulty, &r);
        self.dirty = true;
        self.problem = None;
        self.screen = Screen::Results;
    }

    pub fn on_input<E: Entropy + ?Sized>(&mut self, input: Input, now_ms: u64, e: &mut E) -> Outcome {
        let back = matches!(input, Input::Back | Input::Delete);
        match self.screen {
            Screen::Menu => return self.menu(input, back, now_ms, e),
            Screen::Playing => match input {
                Input::Back => self.abandon(),
                Input::Delete => {
                    if self.answer.pop().is_none() {
                        self.abandon();
                    }
                }
                Input::Enter => self.submit(now_ms),
                Input::Char('-') if self.answer.is_empty() => self.answer.push('-'),
                Input::Char(c @ '0'..='9') if self.answer.len() < MAX_ANSWER_CHARS => self.answer.push(c),
                _ => {}
            },
            Screen::Feedback => {
                if matches!(input, Input::Enter | Input::Char(' ')) {
                    self.advance(now_ms, e);
                } else if back {
                    self.abandon();
                }
            }
            Screen::Results | Screen::Best => {
                if back || matches!(input, Input::Enter | Input::Char(' ')) {
                    self.screen = Screen::Menu;
                }
            }
        }
        Outcome::Stay
    }

    /// Leave a session early. Nothing is recorded: partial sessions don't
    /// count toward bests.
    fn abandon(&mut self) {
        self.problem = None;
        self.answer.clear();
        self.screen = Screen::Menu;
    }

    fn menu<E: Entropy + ?Sized>(&mut self, input: Input, back: bool, now_ms: u64, e: &mut E) -> Outcome {
        const ORDER: [Field; 4] = [Field::Mode, Field::Difficulty, Field::Start, Field::Best];
        let idx = ORDER.iter().position(|f| *f == self.field).unwrap_or(0);
        match input {
            _ if back => return Outcome::Leave,
            Input::Up => self.field = ORDER[(idx + ORDER.len() - 1) % ORDER.len()],
            Input::Down => self.field = ORDER[(idx + 1) % ORDER.len()],
            Input::Left | Input::Right | Input::Enter if matches!(self.field, Field::Mode | Field::Difficulty) => {
                let d = if input == Input::Left { -1 } else { 1 };
                if self.field == Field::Mode {
                    self.mode = self.mode.cycle(d);
                } else {
                    self.difficulty = self.difficulty.cycle(d);
                }
                self.dirty = true;
            }
            Input::Enter => match self.field {
                Field::Start => self.start(now_ms, e),
                Field::Best => self.screen = Screen::Best,
                _ => {}
            },
            _ => {}
        }
        Outcome::Stay
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problems::Operation;
    use precursor_appkit_core::entropy::testing::{Broken, XorShift};

    fn started() -> (Drill, XorShift) {
        let mut d = Drill::new();
        let mut e = XorShift(42);
        d.field = Field::Start;
        d.on_input(Input::Enter, 1000, &mut e);
        assert_eq!(d.screen, Screen::Playing);
        (d, e)
    }

    fn type_answer(d: &mut Drill, e: &mut XorShift, n: i32, now: u64) {
        for c in n.to_string().chars() {
            d.on_input(Input::Char(c), now, e);
        }
        d.on_input(Input::Enter, now, e);
    }

    /// Play a whole session answering `right` problems correctly, 1 s each.
    fn play(d: &mut Drill, e: &mut XorShift, mut now: u64, pattern: &[bool]) -> u64 {
        for &right in pattern {
            now += 1000;
            let p = d.problem.unwrap();
            let ans = if right { p.answer } else { p.answer + 1 };
            type_answer(d, e, ans, now);
            now += 100;
            d.on_input(Input::Enter, now, e); // skip feedback
        }
        now
    }

    #[test]
    fn menu_navigation_and_selection_persist_flag() {
        let mut d = Drill::new();
        let mut e = XorShift(1);
        assert!(!d.is_dirty());
        d.on_input(Input::Right, 0, &mut e);
        assert_eq!(d.mode, Mode::Single(Operation::Subtract));
        assert!(d.is_dirty());
        d.on_input(Input::Down, 0, &mut e);
        d.on_input(Input::Enter, 0, &mut e);
        assert_eq!(d.difficulty, Difficulty::Medium);
        let s = d.take_dirty().unwrap();
        assert_eq!((s.mode, s.difficulty), (Mode::Single(Operation::Subtract), Difficulty::Medium));
        assert!(d.take_dirty().is_none());
    }

    #[test]
    fn start_is_reachable_with_arrows_regression() {
        // Audit: wrong arrow codes made Start unreachable. Down, Down, Enter.
        let mut d = Drill::new();
        let mut e = XorShift(1);
        d.on_input(Input::Down, 0, &mut e);
        d.on_input(Input::Down, 0, &mut e);
        d.on_input(Input::Enter, 0, &mut e);
        assert_eq!(d.screen, Screen::Playing);
    }

    #[test]
    fn full_session_scores_and_records_best() {
        let (mut d, mut e) = started();
        play(&mut d, &mut e, 1000, &[true; 10]);
        assert_eq!(d.screen, Screen::Results);
        assert_eq!(d.correct_count(), 10);
        assert_eq!(d.best_streak, 10);
        // 1 s per problem on Easy: 10 + 9 + streak bonus from the 3rd on.
        let expect: u32 = (1..=10).map(|s| 19 + if s >= 3 { 5 } else { 0 }).sum();
        assert_eq!(d.score, expect);
        let best = d.best_now().unwrap();
        assert_eq!((best.correct, best.streak, best.score), (10, 10, expect));
        assert!(d.improved.any());
        let saved = d.take_dirty().unwrap();
        assert_eq!(saved.bests.len(), 1);
    }

    #[test]
    fn wrong_answers_break_streak() {
        let (mut d, mut e) = started();
        play(&mut d, &mut e, 1000, &[true, true, false, true, true, true, true, true, true, true]);
        assert_eq!(d.best_streak, 7);
        assert_eq!(d.correct_count(), 9);
    }

    #[test]
    fn worse_session_does_not_lower_best() {
        let (mut d, mut e) = started();
        play(&mut d, &mut e, 1000, &[true; 10]);
        d.on_input(Input::Enter, 0, &mut e); // results -> menu
        d.field = Field::Start;
        d.on_input(Input::Enter, 50_000, &mut e);
        play(&mut d, &mut e, 50_000, &[false; 10]);
        assert_eq!(d.best_now().unwrap().correct, 10);
        assert_eq!(d.best_now().unwrap().sessions, 2);
    }

    #[test]
    fn feedback_auto_advances_without_a_keypress() {
        let (mut d, mut e) = started();
        let p = d.problem.unwrap();
        type_answer(&mut d, &mut e, p.answer, 2000);
        assert_eq!(d.screen, Screen::Feedback);
        assert!(!d.tick(2000 + FEEDBACK_MS - 1, &mut e));
        assert!(d.tick(2000 + FEEDBACK_MS, &mut e));
        assert_eq!(d.screen, Screen::Playing);
        assert!(d.answer.is_empty());
    }

    #[test]
    fn digits_during_feedback_are_not_swallowed_into_next_answer() {
        // Audit: after feedback timed out, the next key was eaten. Now the tick advances,
        // so the first digit lands in the answer.
        let (mut d, mut e) = started();
        let p = d.problem.unwrap();
        type_answer(&mut d, &mut e, p.answer, 2000);
        d.tick(4000, &mut e);
        d.on_input(Input::Char('7'), 4001, &mut e);
        assert_eq!(d.answer, "7");
    }

    #[test]
    fn pause_does_not_charge_background_time() {
        let (mut d, mut e) = started();
        assert_eq!(d.elapsed_on_problem(3000), 2000);
        d.pause(3000);
        assert_eq!(d.elapsed_on_problem(999_999), 2000);
        d.resume(603_000);
        assert_eq!(d.elapsed_on_problem(604_000), 3000);
        let p = d.problem.unwrap();
        type_answer(&mut d, &mut e, p.answer, 604_000);
        assert_eq!(d.total_ms, 3000);
    }

    #[test]
    fn pause_during_feedback_delays_advance() {
        let (mut d, mut e) = started();
        let p = d.problem.unwrap();
        type_answer(&mut d, &mut e, p.answer, 2000);
        d.pause(2500);
        assert!(!d.tick(100_000, &mut e));
        d.resume(100_000);
        assert!(!d.tick(100_000, &mut e));
        assert!(d.tick(100_000 + 1000, &mut e));
    }

    #[test]
    fn answer_entry_rules() {
        let (mut d, mut e) = started();
        d.on_input(Input::Char('-'), 0, &mut e);
        d.on_input(Input::Char('-'), 0, &mut e);
        d.on_input(Input::Char('5'), 0, &mut e);
        assert_eq!(d.answer, "-5");
        for _ in 0..20 {
            d.on_input(Input::Char('9'), 0, &mut e);
        }
        assert_eq!(d.answer.len(), MAX_ANSWER_CHARS);
        assert!(d.answer.parse::<i32>().is_ok());
        d.on_input(Input::Char('x'), 0, &mut e);
        assert_eq!(d.answer.len(), MAX_ANSWER_CHARS);
        // a lone "-" is not submittable
        d.answer = "-".into();
        d.on_input(Input::Enter, 5, &mut e);
        assert_eq!(d.screen, Screen::Playing);
    }

    #[test]
    fn empty_enter_does_not_submit() {
        let (mut d, mut e) = started();
        d.on_input(Input::Enter, 5000, &mut e);
        assert_eq!(d.screen, Screen::Playing);
        assert!(d.answered.is_empty());
    }

    #[test]
    fn back_leaves_every_screen() {
        let mut e = XorShift(3);
        let mut d = Drill::new();
        assert_eq!(d.on_input(Input::Back, 0, &mut e), Outcome::Leave);
        assert_eq!(d.on_input(Input::Delete, 0, &mut e), Outcome::Leave);
        // best screen
        d.field = Field::Best;
        d.on_input(Input::Enter, 0, &mut e);
        assert_eq!(d.screen, Screen::Best);
        d.on_input(Input::Delete, 0, &mut e);
        assert_eq!(d.screen, Screen::Menu);
        // playing: F4 abandons; nothing recorded
        let (mut d, mut e) = started();
        d.on_input(Input::Back, 5, &mut e);
        assert_eq!(d.screen, Screen::Menu);
        assert!(d.saved.bests.is_empty());
        assert!(!d.is_dirty());
        // playing: Backspace deletes digits, then exits when empty
        let (mut d, mut e) = started();
        d.on_input(Input::Char('4'), 5, &mut e);
        d.on_input(Input::Delete, 5, &mut e);
        assert_eq!(d.screen, Screen::Playing);
        d.on_input(Input::Delete, 5, &mut e);
        assert_eq!(d.screen, Screen::Menu);
    }

    #[test]
    fn trng_failure_stops_cleanly() {
        let mut d = Drill::new();
        d.field = Field::Start;
        d.on_input(Input::Enter, 0, &mut Broken);
        assert_eq!(d.screen, Screen::Menu);
        assert!(d.status.contains("Random"));
        assert!(d.problem.is_none());
    }

    #[test]
    fn newer_version_is_refused() {
        let mut d = Drill::new();
        let s = Saved { version: 9, ..Saved::default() };
        assert_eq!(d.apply_saved(s), Err(LoadError::NewerVersion(9)));
    }

    #[test]
    fn apply_saved_restores_selection() {
        let mut d = Drill::new();
        let s = Saved { mode: Mode::Mixed, difficulty: Difficulty::Hard, ..Saved::default() };
        d.apply_saved(s).unwrap();
        assert_eq!((d.mode, d.difficulty), (Mode::Mixed, Difficulty::Hard));
        assert!(!d.is_dirty());
    }

    #[test]
    fn avg_with_no_answers_is_zero() {
        assert_eq!(Drill::new().avg_ms(), 0);
    }
}
