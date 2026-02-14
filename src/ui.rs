//! UI rendering for Math Drill.
//!
//! Large problem display, answer input, progress bar, results screen.

extern crate alloc;
use alloc::format;
use alloc::string::String;

use gam::*;
use graphics_server::api::GlyphStyle;
use graphics_server::{DrawStyle, PixelColor, Point, Rectangle, TextBounds};

use crate::app::*;
use crate::problems::Difficulty;

const SCREEN_W: i16 = 336;
const HEADER_H: i16 = 30;
const FOOTER_H: i16 = 46;
const LINE_H: i16 = 22;

fn draw_header(gam: &Gam, canvas: Canvas, text: &str) {
    let header_rect = Rectangle::new(
        Point::new(0, 0),
        Point::new(SCREEN_W - 1, HEADER_H - 1),
    );
    gam.draw_rectangle(canvas, header_rect.style(
        DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 0),
    )).ok();

    let tb = TextBounds::BoundingBox(Rectangle::new(
        Point::new(4, 2),
        Point::new(SCREEN_W - 4, HEADER_H - 2),
    ));
    gam.draw_textview(
        canvas,
        tv::TextView::new(tb, text)
            .style(GlyphStyle::Bold)
            .draw_border(false)
            .invert(true),
    ).ok();
}

fn draw_footer(gam: &Gam, canvas: Canvas, text: &str) {
    let y = 536 - FOOTER_H;
    gam.draw_line(canvas, Point::new(0, y), Point::new(SCREEN_W - 1, y),
        DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 1),
    ).ok();

    let tb = TextBounds::BoundingBox(Rectangle::new(
        Point::new(4, y + 4),
        Point::new(SCREEN_W - 4, 536 - 2),
    ));
    gam.draw_textview(
        canvas,
        tv::TextView::new(tb, text)
            .style(GlyphStyle::Small)
            .draw_border(false),
    ).ok();
}

fn draw_text(gam: &Gam, canvas: Canvas, x: i16, y: i16, text: &str, style: GlyphStyle) {
    let tb = TextBounds::BoundingBox(Rectangle::new(
        Point::new(x, y),
        Point::new(SCREEN_W - 4, y + LINE_H),
    ));
    gam.draw_textview(
        canvas,
        tv::TextView::new(tb, text)
            .style(style)
            .draw_border(false),
    ).ok();
}

fn draw_text_inverted(gam: &Gam, canvas: Canvas, x: i16, y: i16, w: i16, text: &str) {
    let bg = Rectangle::new(Point::new(x, y), Point::new(x + w, y + LINE_H));
    gam.draw_rectangle(canvas, bg.style(
        DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 0),
    )).ok();

    let tb = TextBounds::BoundingBox(Rectangle::new(
        Point::new(x + 2, y),
        Point::new(x + w - 2, y + LINE_H),
    ));
    gam.draw_textview(
        canvas,
        tv::TextView::new(tb, text)
            .style(GlyphStyle::Regular)
            .draw_border(false)
            .invert(true),
    ).ok();
}

fn draw_large_text(gam: &Gam, canvas: Canvas, x: i16, y: i16, text: &str) {
    let tb = TextBounds::BoundingBox(Rectangle::new(
        Point::new(x, y),
        Point::new(SCREEN_W - 4, y + 40),
    ));
    gam.draw_textview(
        canvas,
        tv::TextView::new(tb, text)
            .style(GlyphStyle::Large)
            .draw_border(false),
    ).ok();
}

pub fn draw(app: &MathDrillApp, gam: &Gam, canvas: Canvas) {
    gam.draw_rectangle(
        canvas,
        Rectangle::new(Point::new(0, 0), Point::new(SCREEN_W - 1, 535))
            .style(DrawStyle::new(PixelColor::Light, PixelColor::Light, 0)),
    ).ok();

    match app.state {
        AppState::Menu => draw_menu(app, gam, canvas),
        AppState::Playing => draw_playing(app, gam, canvas),
        AppState::Feedback => draw_feedback(app, gam, canvas),
        AppState::Results => draw_results(app, gam, canvas),
        AppState::BestScores => draw_best_scores(app, gam, canvas),
    }

    gam.redraw().ok();
}

fn draw_menu(app: &MathDrillApp, gam: &Gam, canvas: Canvas) {
    draw_header(gam, canvas, "Math Drill");

    let mut y = HEADER_H + 20;

    // ASCII art title
    draw_text(gam, canvas, 40, y, "Quick arithmetic training", GlyphStyle::Regular);
    y += LINE_H + 20;

    // Operation selector
    let op_label = format!("Operation: < {} >", app.op_mode.label());
    if app.menu_field == MenuField::Operation {
        draw_text_inverted(gam, canvas, 20, y, SCREEN_W - 40, &op_label);
    } else {
        draw_text(gam, canvas, 24, y, &op_label, GlyphStyle::Regular);
    }
    y += LINE_H + 10;

    // Difficulty selector
    let diff_label = format!("Difficulty: < {} >", app.difficulty.label());
    if app.menu_field == MenuField::Difficulty {
        draw_text_inverted(gam, canvas, 20, y, SCREEN_W - 40, &diff_label);
    } else {
        draw_text(gam, canvas, 24, y, &diff_label, GlyphStyle::Regular);
    }
    y += LINE_H + 20;

    // Start button
    let start_label = ">>> START QUIZ <<<";
    if app.menu_field == MenuField::Start {
        draw_text_inverted(gam, canvas, 60, y, SCREEN_W - 120, start_label);
    } else {
        draw_text(gam, canvas, 80, y, start_label, GlyphStyle::Regular);
    }
    y += LINE_H + 10;

    // Best scores link
    let best_label = "View Best Scores";
    if app.menu_field == MenuField::BestScores {
        draw_text_inverted(gam, canvas, 60, y, SCREEN_W - 120, best_label);
    } else {
        draw_text(gam, canvas, 80, y, best_label, GlyphStyle::Regular);
    }

    draw_footer(gam, canvas, "Up/Down=Select  </>=Cycle  Enter=Go  Menu=Quit");
}

fn draw_playing(app: &MathDrillApp, gam: &Gam, canvas: Canvas) {
    let header = format!(
        "Problem {}/10  Streak: {}  Score: {}/{}",
        app.problem_num + 1,
        app.streak,
        app.correct_count,
        app.problem_num
    );
    draw_header(gam, canvas, &header);

    // Progress bar
    let bar_y = HEADER_H + 4;
    let bar_h = 8;
    let filled_w = ((app.problem_num as i16) * (SCREEN_W - 20)) / 10;
    let bar_bg = Rectangle::new(
        Point::new(10, bar_y),
        Point::new(SCREEN_W - 10, bar_y + bar_h),
    );
    gam.draw_rectangle(canvas, bar_bg.style(
        DrawStyle::new(PixelColor::Light, PixelColor::Dark, 1),
    )).ok();
    if filled_w > 0 {
        let bar_fill = Rectangle::new(
            Point::new(10, bar_y),
            Point::new(10 + filled_w, bar_y + bar_h),
        );
        gam.draw_rectangle(canvas, bar_fill.style(
            DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 0),
        )).ok();
    }

    // Problem display — large and centered
    if let Some(ref problem) = app.current_problem {
        let problem_text = problem.display();
        let y_problem = HEADER_H + 80;
        draw_large_text(gam, canvas, 30, y_problem, &problem_text);

        // Answer input
        let y_answer = y_problem + 80;
        let answer_display = if app.answer_buffer.is_empty() {
            String::from("Type your answer: _")
        } else {
            format!("Your answer: {}_", app.answer_buffer)
        };
        draw_text(gam, canvas, 30, y_answer, &answer_display, GlyphStyle::Regular);

        // Hint for negative
        let y_hint = y_answer + LINE_H + 10;
        draw_text(gam, canvas, 30, y_hint, "0-9, -, Backspace, Enter", GlyphStyle::Small);
    }

    draw_footer(gam, canvas, "Type answer + Enter  Menu=Quit session");
}

fn draw_feedback(app: &MathDrillApp, gam: &Gam, canvas: Canvas) {
    if app.feedback_correct {
        draw_header(gam, canvas, "CORRECT!");
    } else {
        draw_header(gam, canvas, "WRONG");
    }

    let y = HEADER_H + 60;

    if let Some(ref problem) = app.feedback_problem {
        if app.feedback_correct {
            draw_large_text(gam, canvas, 30, y, &problem.display_with_answer());
            let y2 = y + 50;
            let streak_msg = format!("Streak: {}", app.streak);
            draw_text(gam, canvas, 30, y2, &streak_msg, GlyphStyle::Regular);
        } else {
            let wrong = format!("You said: {}", app.feedback_user_answer);
            draw_text(gam, canvas, 30, y, &wrong, GlyphStyle::Regular);
            let y2 = y + LINE_H + 10;
            let correct = format!("Correct: {}", problem.display_with_answer());
            draw_large_text(gam, canvas, 30, y2, &correct);
        }
    }

    draw_footer(gam, canvas, "Enter=Next  (auto-advances in 1.5s)");
}

fn draw_results(app: &MathDrillApp, gam: &Gam, canvas: Canvas) {
    let pct = if !app.session_problems.is_empty() {
        (app.correct_count as u32 * 100) / app.session_problems.len() as u32
    } else {
        0
    };
    let header = format!("Results — {}%", pct);
    draw_header(gam, canvas, &header);

    let mut y = HEADER_H + 10;

    let score_line = format!(
        "Score: {}/{}  Best Streak: {}",
        app.correct_count,
        app.session_problems.len(),
        app.best_streak
    );
    draw_text(gam, canvas, 8, y, &score_line, GlyphStyle::Regular);
    y += LINE_H + 4;

    let avg_line = format!("Avg time: {}ms per problem", app.avg_time_ms());
    draw_text(gam, canvas, 8, y, &avg_line, GlyphStyle::Regular);
    y += LINE_H + 10;

    // Show each problem result
    draw_text(gam, canvas, 8, y, "Review:", GlyphStyle::Small);
    y += 16;

    for (i, (problem, user_ans, correct)) in app.session_problems.iter().enumerate() {
        let mark = if *correct { "+" } else { "X" };
        let line = if *correct {
            format!("{} {} {}", mark, problem.display_with_answer(), "")
        } else {
            format!("{} {} (you: {})", mark, problem.display_with_answer(), user_ans)
        };
        draw_text(gam, canvas, 8, y, &line, GlyphStyle::Small);
        y += 16;
        if y > 536 - FOOTER_H - 16 {
            break;
        }
    }

    draw_footer(gam, canvas, "Enter=Menu");
}

fn draw_best_scores(app: &MathDrillApp, gam: &Gam, canvas: Canvas) {
    draw_header(gam, canvas, "Best Scores");

    let mut y = HEADER_H + 10;

    // Clone app to avoid borrow issues — we need mutable access for storage
    // but only have immutable. Work around by just showing what we can.
    for diff in Difficulty::all() {
        draw_text(gam, canvas, 8, y, diff.label(), GlyphStyle::Bold);
        y += LINE_H + 2;

        // We can't call get_best from here since we only have &app
        // The scores will be loaded in a dedicated data struct
        draw_text(gam, canvas, 16, y, "(Select to see stats)", GlyphStyle::Small);
        y += 20;
    }

    // Note: In practice, best scores are cached in the app struct
    // before entering this state. This is a simplified display.

    draw_footer(gam, canvas, "Enter=Back  Menu=Back");
}

/// Draw best scores with pre-loaded data.
pub fn draw_best_with_data(
    gam: &Gam,
    canvas: Canvas,
    easy: Option<&crate::storage::BestStats>,
    medium: Option<&crate::storage::BestStats>,
    hard: Option<&crate::storage::BestStats>,
) {
    gam.draw_rectangle(
        canvas,
        Rectangle::new(Point::new(0, 0), Point::new(SCREEN_W - 1, 535))
            .style(DrawStyle::new(PixelColor::Light, PixelColor::Light, 0)),
    ).ok();

    draw_header(gam, canvas, "Best Scores");

    let mut y = HEADER_H + 10;

    let entries: [(&str, Option<&crate::storage::BestStats>); 3] = [
        ("Easy", easy),
        ("Medium", medium),
        ("Hard", hard),
    ];

    for (label, stats) in entries.iter() {
        draw_text(gam, canvas, 8, y, label, GlyphStyle::Bold);
        y += LINE_H;

        match stats {
            Some(s) => {
                let line1 = format!("  Score: {}/{}  Streak: {}", s.correct, s.total, s.streak);
                draw_text(gam, canvas, 8, y, &line1, GlyphStyle::Regular);
                y += LINE_H;
                let line2 = format!("  Avg: {}ms/problem", s.avg_ms);
                draw_text(gam, canvas, 8, y, &line2, GlyphStyle::Small);
                y += 18;
            }
            None => {
                draw_text(gam, canvas, 16, y, "  No scores yet", GlyphStyle::Small);
                y += 18;
            }
        }
        y += 8;
    }

    draw_footer(gam, canvas, "Enter=Back  Menu=Back");
    gam.redraw().ok();
}
