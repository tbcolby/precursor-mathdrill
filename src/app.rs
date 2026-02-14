//! State machine for Math Drill.
//!
//! States:
//!   Menu        — choose operation, difficulty, start quiz
//!   Playing     — answering problems, timer running
//!   Feedback    — brief correct/wrong display
//!   Results     — session summary with stats
//!   BestScores  — all-time bests per difficulty

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::problems::*;
use crate::rng::Rng;
use crate::storage::{BestStats, Storage};

const KEY_UP: char = '\u{F700}';
const KEY_DOWN: char = '\u{F701}';
const KEY_LEFT: char = '\u{F702}';
const KEY_RIGHT: char = '\u{F703}';
const KEY_ENTER: char = '\u{000D}';
const KEY_BACKSPACE: char = '\u{0008}';
const KEY_MENU: char = '\u{2234}';

const PROBLEMS_PER_SESSION: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppState {
    Menu,
    Playing,
    Feedback,
    Results,
    BestScores,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuField {
    Operation,
    Difficulty,
    Start,
    BestScores,
}

/// Which operation mode the quiz uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpMode {
    Single(Operation),
    Mixed,
}

impl OpMode {
    pub fn label(&self) -> &'static str {
        match self {
            OpMode::Single(op) => op.label(),
            OpMode::Mixed => "Mixed",
        }
    }
}

pub struct MathDrillApp {
    pub state: AppState,
    pub needs_redraw: bool,

    // Menu
    pub menu_field: MenuField,
    pub op_mode: OpMode,
    pub difficulty: Difficulty,

    // Playing
    pub current_problem: Option<Problem>,
    pub answer_buffer: String,
    pub problem_num: usize,
    pub correct_count: u32,
    pub streak: u32,
    pub best_streak: u32,
    pub problem_start_ms: u64,
    pub total_time_ms: u64,

    // Feedback
    pub feedback_correct: bool,
    pub feedback_problem: Option<Problem>,
    pub feedback_user_answer: i32,
    pub feedback_timer: u64,

    // Results
    pub session_problems: Vec<(Problem, i32, bool)>, // (problem, user_answer, correct)

    // Storage
    storage: Option<Storage>,
}

impl MathDrillApp {
    pub fn new() -> Self {
        Self {
            state: AppState::Menu,
            needs_redraw: true,
            menu_field: MenuField::Operation,
            op_mode: OpMode::Single(Operation::Add),
            difficulty: Difficulty::Easy,
            current_problem: None,
            answer_buffer: String::new(),
            problem_num: 0,
            correct_count: 0,
            streak: 0,
            best_streak: 0,
            problem_start_ms: 0,
            total_time_ms: 0,
            feedback_correct: false,
            feedback_problem: None,
            feedback_user_answer: 0,
            feedback_timer: 0,
            session_problems: Vec::new(),
            storage: None,
        }
    }

    pub fn init_storage(&mut self) {
        if let Ok(st) = Storage::new() {
            self.storage = Some(st);
        }
    }

    pub fn save_state(&mut self) {
        // Stats are saved at end of session in handle_results
    }

    fn next_problem(&mut self, rng: &Rng) {
        let problem = match self.op_mode {
            OpMode::Single(op) => generate(rng, op, self.difficulty),
            OpMode::Mixed => generate_mixed(rng, self.difficulty),
        };
        self.current_problem = Some(problem);
        self.answer_buffer.clear();
    }

    fn start_session(&mut self, rng: &Rng, now_ms: u64) {
        self.problem_num = 0;
        self.correct_count = 0;
        self.streak = 0;
        self.best_streak = 0;
        self.total_time_ms = 0;
        self.session_problems.clear();
        self.problem_start_ms = now_ms;
        self.next_problem(rng);
        self.state = AppState::Playing;
    }

    fn submit_answer(&mut self, now_ms: u64) {
        let user_answer = self.answer_buffer.trim().parse::<i32>().unwrap_or(i32::MIN);
        if let Some(ref problem) = self.current_problem {
            let correct = problem.check(user_answer);
            if correct {
                self.correct_count += 1;
                self.streak += 1;
                if self.streak > self.best_streak {
                    self.best_streak = self.streak;
                }
            } else {
                self.streak = 0;
            }

            let elapsed = now_ms.saturating_sub(self.problem_start_ms);
            self.total_time_ms += elapsed;

            self.feedback_correct = correct;
            self.feedback_problem = Some(problem.clone());
            self.feedback_user_answer = user_answer;
            self.session_problems.push((problem.clone(), user_answer, correct));

            self.problem_num += 1;
            self.feedback_timer = now_ms;
            self.state = AppState::Feedback;
        }
    }

    /// Called from main loop to advance feedback → next problem or results.
    pub fn check_feedback_timeout(&mut self, now_ms: u64, rng: &Rng) {
        if self.state == AppState::Feedback {
            let elapsed = now_ms.saturating_sub(self.feedback_timer);
            if elapsed >= 1500 {
                if self.problem_num >= PROBLEMS_PER_SESSION {
                    self.finish_session();
                } else {
                    self.problem_start_ms = now_ms;
                    self.next_problem(rng);
                    self.state = AppState::Playing;
                    self.needs_redraw = true;
                }
            }
        }
    }

    fn finish_session(&mut self) {
        // Check if this is a new best
        if let Some(ref mut st) = self.storage {
            let total = self.session_problems.len() as u32;
            let avg_ms = if total > 0 {
                (self.total_time_ms / total as u64) as u32
            } else {
                0
            };

            let is_new_best = match st.load_best(&self.difficulty) {
                Some(prev) => {
                    self.correct_count > prev.correct
                        || (self.correct_count == prev.correct && avg_ms < prev.avg_ms)
                        || self.best_streak > prev.streak
                }
                None => true,
            };

            if is_new_best {
                let new_best = BestStats {
                    streak: self.best_streak,
                    correct: self.correct_count,
                    total,
                    avg_ms,
                };
                st.save_best(&self.difficulty, &new_best);
            }
        }

        self.state = AppState::Results;
    }

    pub fn handle_key(&mut self, key: char, now_ms: u64, rng: &Rng) -> bool {
        self.needs_redraw = true;
        match self.state {
            AppState::Menu => self.handle_menu(key, now_ms, rng),
            AppState::Playing => self.handle_playing(key, now_ms),
            AppState::Feedback => self.handle_feedback(key, now_ms, rng),
            AppState::Results => self.handle_results(key),
            AppState::BestScores => self.handle_best_scores(key),
        }
    }

    fn handle_menu(&mut self, key: char, now_ms: u64, rng: &Rng) -> bool {
        match key {
            KEY_MENU => return false,
            KEY_UP => {
                self.menu_field = match self.menu_field {
                    MenuField::Operation => MenuField::BestScores,
                    MenuField::Difficulty => MenuField::Operation,
                    MenuField::Start => MenuField::Difficulty,
                    MenuField::BestScores => MenuField::Start,
                };
            }
            KEY_DOWN => {
                self.menu_field = match self.menu_field {
                    MenuField::Operation => MenuField::Difficulty,
                    MenuField::Difficulty => MenuField::Start,
                    MenuField::Start => MenuField::BestScores,
                    MenuField::BestScores => MenuField::Operation,
                };
            }
            KEY_LEFT | KEY_RIGHT => match self.menu_field {
                MenuField::Operation => {
                    self.op_mode = match self.op_mode {
                        OpMode::Mixed => OpMode::Single(Operation::Add),
                        OpMode::Single(Operation::Add) => OpMode::Single(Operation::Subtract),
                        OpMode::Single(Operation::Subtract) => OpMode::Single(Operation::Multiply),
                        OpMode::Single(Operation::Multiply) => OpMode::Single(Operation::Divide),
                        OpMode::Single(Operation::Divide) => OpMode::Mixed,
                    };
                }
                MenuField::Difficulty => {
                    self.difficulty = match self.difficulty {
                        Difficulty::Easy => Difficulty::Medium,
                        Difficulty::Medium => Difficulty::Hard,
                        Difficulty::Hard => Difficulty::Easy,
                    };
                }
                _ => {}
            },
            KEY_ENTER => match self.menu_field {
                MenuField::Start => {
                    self.start_session(rng, now_ms);
                }
                MenuField::BestScores => {
                    self.state = AppState::BestScores;
                }
                MenuField::Operation => {
                    // Also cycle on Enter
                    self.op_mode = match self.op_mode {
                        OpMode::Mixed => OpMode::Single(Operation::Add),
                        OpMode::Single(Operation::Add) => OpMode::Single(Operation::Subtract),
                        OpMode::Single(Operation::Subtract) => OpMode::Single(Operation::Multiply),
                        OpMode::Single(Operation::Multiply) => OpMode::Single(Operation::Divide),
                        OpMode::Single(Operation::Divide) => OpMode::Mixed,
                    };
                }
                MenuField::Difficulty => {
                    self.difficulty = match self.difficulty {
                        Difficulty::Easy => Difficulty::Medium,
                        Difficulty::Medium => Difficulty::Hard,
                        Difficulty::Hard => Difficulty::Easy,
                    };
                }
            },
            _ => {}
        }
        true
    }

    fn handle_playing(&mut self, key: char, now_ms: u64) -> bool {
        match key {
            KEY_MENU => {
                self.state = AppState::Menu;
            }
            KEY_BACKSPACE => {
                self.answer_buffer.pop();
            }
            KEY_ENTER => {
                if !self.answer_buffer.is_empty() {
                    self.submit_answer(now_ms);
                }
            }
            '-' => {
                if self.answer_buffer.is_empty() {
                    self.answer_buffer.push('-');
                }
            }
            c @ '0'..='9' => {
                if self.answer_buffer.len() < 8 {
                    self.answer_buffer.push(c);
                }
            }
            _ => {}
        }
        true
    }

    fn handle_feedback(&mut self, key: char, now_ms: u64, rng: &Rng) -> bool {
        // Any key skips the feedback timer
        if key == KEY_ENTER || key == ' ' {
            if self.problem_num >= PROBLEMS_PER_SESSION {
                self.finish_session();
            } else {
                self.problem_start_ms = now_ms;
                self.next_problem(rng);
                self.state = AppState::Playing;
            }
        }
        true
    }

    fn handle_results(&mut self, key: char) -> bool {
        match key {
            KEY_ENTER | KEY_MENU | ' ' => {
                self.state = AppState::Menu;
            }
            _ => {}
        }
        true
    }

    fn handle_best_scores(&mut self, key: char) -> bool {
        match key {
            KEY_ENTER | KEY_MENU | KEY_LEFT => {
                self.state = AppState::Menu;
            }
            _ => {}
        }
        true
    }

    /// Load best stats for display.
    pub fn get_best(&mut self, diff: &Difficulty) -> Option<BestStats> {
        if let Some(ref mut st) = self.storage {
            st.load_best(diff)
        } else {
            None
        }
    }

    pub fn avg_time_ms(&self) -> u32 {
        let total = self.session_problems.len() as u64;
        if total > 0 {
            (self.total_time_ms / total) as u32
        } else {
            0
        }
    }
}
