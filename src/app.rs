//! Rendering, input mapping and persistence for Math Drill. All rules live in
//! `mathdrill-core`.

use gam::GlyphStyle;
use mathdrill_core::drill::{Drill, Field, Input, Outcome, Screen as Page};
use mathdrill_core::{Best, Saved, PROBLEMS_PER_SESSION};
use precursor_appkit::keys::Key;
use precursor_appkit::layout::ListState;
use precursor_appkit::rng::Trng;
use precursor_appkit::screen::Screen;
use precursor_appkit::store::Store;

const STATE_KEY: &str = "state";

#[derive(PartialEq, Eq)]
pub enum Action {
    Stay,
    Leave,
}

pub struct App {
    screen: Screen,
    store: Store,
    trng: Trng,
    tt: ticktimer_server::Ticktimer,
    drill: Drill,
    list: ListState,
    visible_rows: usize,
}

fn map_key(key: Key) -> Option<Input> {
    Some(match key {
        Key::Up => Input::Up,
        Key::Down => Input::Down,
        Key::Left => Input::Left,
        Key::Right => Input::Right,
        Key::Enter => Input::Enter,
        Key::Backspace => Input::Delete,
        Key::F(4) => Input::Back,
        Key::Char(c) => Input::Char(c),
        Key::F(_) => return None,
    })
}

fn best_line(b: &Best) -> String {
    let speed = match b.fastest_avg_ms {
        Some(ms) => format!("{}.{}s", ms / 1000, (ms % 1000) / 100),
        None => "-".to_string(),
    };
    format!("{}/{} streak {} pts {} avg {}", b.correct, PROBLEMS_PER_SESSION, b.streak, b.score, speed)
}

impl App {
    pub fn new(screen: Screen, store: Store, trng: Trng) -> Self {
        let mut app = Self {
            screen,
            store,
            trng,
            tt: ticktimer_server::Ticktimer::new().expect("mathdrill: no ticktimer"),
            drill: Drill::new(),
            list: ListState::default(),
            visible_rows: 1,
        };
        app.reload();
        app
    }

    pub fn screen(&self) -> &Screen { &self.screen }

    fn now(&self) -> u64 { self.tt.elapsed_ms() }

    pub fn wants_tick(&self) -> bool { self.drill.wants_tick() }

    /// (Re)load stored data at start-up and whenever the open PDDB bases change.
    pub fn reload(&mut self) {
        match self.store.load::<Saved>(STATE_KEY) {
            Ok(Some(s)) => {
                if self.drill.apply_saved(s).is_err() {
                    self.drill.mark_unreadable("Saved data is from a newer version; not saving");
                }
            }
            Ok(None) => {
                self.drill.apply_saved(Saved::default()).ok();
            }
            Err(e) => {
                log::error!("mathdrill: couldn't load: {:?}", e.kind());
                self.drill.mark_unreadable("Couldn't read saved data; not saving");
            }
        }
        self.draw();
    }

    fn save_if_dirty(&mut self) {
        if !self.drill.is_dirty() {
            return;
        }
        if self.drill.read_only {
            // Never overwrite data we couldn't read.
            self.drill.take_dirty();
            return;
        }
        if let Some(s) = self.drill.take_dirty() {
            if let Err(e) = self.store.save(STATE_KEY, &s) {
                log::error!("mathdrill: save failed: {:?}", e.kind());
                self.drill.set_dirty();
                self.drill.status = "Save failed".to_string();
            }
        }
    }

    pub fn on_key(&mut self, key: Key) -> Action {
        let Some(input) = map_key(key) else { return Action::Stay };
        let now = self.now();
        let was = self.drill.screen;
        if was == Page::Best && matches!(input, Input::Up | Input::Down) {
            let step = if input == Input::Up { -1 } else { 1 };
            let max = self.drill.saved.bests.len().saturating_sub(1);
            self.list.selected = (self.list.selected as isize + step).clamp(0, max as isize) as usize;
            return Action::Stay;
        }
        let out = self.drill.on_input(input, now, &mut self.trng);
        if self.drill.screen != was {
            self.list = ListState::default();
        }
        // Scores are saved when a session ends; selection changes on leaving.
        if out == Outcome::Leave || self.drill.screen == Page::Results {
            self.save_if_dirty();
        }
        if out == Outcome::Leave { Action::Leave } else { Action::Stay }
    }

    pub fn on_tick(&mut self) {
        let now = self.now();
        let before = self.drill.screen;
        self.drill.tick(now, &mut self.trng);
        if self.drill.screen == Page::Results {
            self.save_if_dirty();
        }
        if self.drill.screen != before || self.drill.screen == Page::Playing {
            self.draw();
        }
    }

    pub fn on_foreground(&mut self) {
        self.drill.resume(self.now());
        self.screen.refresh_size();
        self.draw();
    }

    pub fn on_background(&mut self) {
        self.drill.pause(self.now());
        self.save_if_dirty();
    }

    pub fn draw(&mut self) {
        let s = &self.screen;
        s.clear();
        let top = s.header("Math Drill");
        match self.drill.screen {
            Page::Menu => self.draw_menu(top),
            Page::Playing => self.draw_playing(top),
            Page::Feedback => self.draw_feedback(top),
            Page::Results => self.draw_results(top),
            Page::Best => self.draw_best(top),
        }
        self.screen.flush();
    }

    fn draw_menu(&mut self, top: isize) {
        let d = &self.drill;
        let bottom = self.screen.footer("↑↓ move  ←→ change  ⏎ select  ⌫ quit");
        let items = vec![
            format!("Operation: < {} >", d.mode.label()),
            format!("Level: < {} >", d.difficulty.label()),
            "Start (10 problems)".to_string(),
            "Best scores".to_string(),
        ];
        self.list.selected = match d.field {
            Field::Mode => 0,
            Field::Difficulty => 1,
            Field::Start => 2,
            Field::Best => 3,
        };
        let status_h = if d.status.is_empty() { 0 } else { self.screen.line_height(GlyphStyle::Small) * 2 };
        self.visible_rows = self.screen.list(top, bottom - status_h, &items, &self.list);
        if !d.status.is_empty() {
            self.screen.para(bottom - status_h, bottom, GlyphStyle::Small, &d.status);
        }
    }

    fn draw_playing(&mut self, top: isize) {
        let d = &self.drill;
        let s = &self.screen;
        s.footer("0-9 - answer  ⏎ submit  ⌫ delete  F4 stop");
        let mut y = top;
        let n = (d.answered.len() + 1).min(PROBLEMS_PER_SESSION);
        y += s.text(y, GlyphStyle::Regular, &format!("Problem {}/{}   Score {}", n, PROBLEMS_PER_SESSION, d.score));
        let secs = d.elapsed_on_problem(self.now()) / 1000;
        y += s.text(y, GlyphStyle::Small, &format!("{}s   streak {}", secs, d.streak));
        y += 16;
        if let Some(p) = &d.problem {
            y += s.text(y, GlyphStyle::ExtraLarge, &p.display()) + 8;
        }
        s.text(y, GlyphStyle::Large, &format!("> {}_", d.answer));
    }

    fn draw_feedback(&mut self, top: isize) {
        let d = &self.drill;
        let s = &self.screen;
        s.footer("⏎ next (continues by itself)  F4 stop");
        if let Some(a) = d.answered.last() {
            let mut y = top + 8;
            let head = if a.correct { format!("Correct!  +{}", a.points) } else { "Wrong".to_string() };
            y += s.text(y, GlyphStyle::Large, &head);
            y += s.text(y, GlyphStyle::Regular, &a.problem.display_with_answer());
            if !a.correct {
                y += s.text(y, GlyphStyle::Regular, &format!("You said {}", a.given));
            }
            s.text(y + 8, GlyphStyle::Small, &format!("Score {}  streak {}", d.score, d.streak));
        }
    }

    fn draw_results(&mut self, top: isize) {
        let d = &self.drill;
        let s = &self.screen;
        s.footer("⏎ menu  ⌫ menu");
        let mut y = top;
        y += s.text(y, GlyphStyle::Large, &format!("{}/{} correct", d.correct_count(), PROBLEMS_PER_SESSION));
        y += s.text(y, GlyphStyle::Regular, &format!("Score {}  best streak {}", d.score, d.best_streak));
        let avg = d.avg_ms();
        y += s.text(y, GlyphStyle::Regular, &format!("Average {}.{}s per problem", avg / 1000, (avg % 1000) / 100));
        let i = d.improved;
        let mut notes = Vec::new();
        if i.score { notes.push("score"); }
        if i.correct { notes.push("correct"); }
        if i.streak { notes.push("streak"); }
        if i.speed { notes.push("speed"); }
        if !notes.is_empty() {
            y += s.text(y, GlyphStyle::Bold, &format!("New best: {}", notes.join(", ")));
        }
        for a in d.answered.iter().filter(|a| !a.correct).take(4) {
            y += s.text(y, GlyphStyle::Small, &format!("Missed {}", a.problem.display_with_answer()));
        }
        if !d.status.is_empty() {
            s.text(y, GlyphStyle::Small, &d.status);
        }
    }

    fn draw_best(&mut self, top: isize) {
        let bottom = self.screen.footer("↑↓ scroll  ⌫ back");
        let entries = &self.drill.saved.bests;
        if entries.is_empty() {
            self.screen.para(top, bottom, GlyphStyle::Regular, "No scores yet. Finish a 10-problem session.");
            return;
        }
        let mut y = top;
        for e in entries.iter().skip(self.list.selected) {
            let need = self.screen.line_height(GlyphStyle::Bold) + self.screen.line_height(GlyphStyle::Small);
            if y + need > bottom {
                break;
            }
            y += self.screen.text(y, GlyphStyle::Bold, &format!("{} {}", e.mode.label(), e.difficulty.key()));
            y += self.screen.text(y, GlyphStyle::Small, &best_line(&e.best));
        }
    }
}
