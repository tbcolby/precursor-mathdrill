//! Precursor Math Drill
//!
//! Timed arithmetic quiz with TRNG-generated problems.
//! Tracks streaks and best scores per difficulty.

#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]


mod app;
mod problems;
mod rng;
mod storage;
mod ui;

use app::MathDrillApp;
use rng::Rng;
use num_traits::FromPrimitive;
use num_traits::ToPrimitive;

const SERVER_NAME: &str = "_Math Drill_";
const APP_NAME: &str = "Math Drill";

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
enum AppOp {
    Redraw = 0,
    Rawkeys = 1,
    FocusChange = 2,
    Quit = 255,
}

fn main() -> ! {
    log_server::init_wait().unwrap();
    log::set_max_level(log::LevelFilter::Info);
    log::info!("Math Drill starting, PID {}", xous::process::id());

    let xns = xous_names::XousNames::new().unwrap();
    let sid = xns
        .register_name(SERVER_NAME, None)
        .expect("can't register server");
    let gam = gam::Gam::new(&xns).expect("can't connect to GAM");
    let tt = ticktimer_server::Ticktimer::new().unwrap();
    let rng = Rng::new(&xns);

    let token = gam
        .register_ux(gam::UxRegistration {
            app_name: String::from(gam::APP_NAME_MATHDRILL),
            ux_type: gam::UxType::Framebuffer,
            predictor: None,
            listener: sid.to_array(),
            redraw_id: AppOp::Redraw.to_u32().unwrap(),
            gotinput_id: None,
            audioframe_id: None,
            rawkeys_id: Some(AppOp::Rawkeys.to_u32().unwrap()),
            focuschange_id: Some(AppOp::FocusChange.to_u32().unwrap()),
        })
        .expect("couldn't register UX")
        .unwrap();

    let content = gam
        .request_content_canvas(token)
        .expect("couldn't get canvas");
    let screensize = gam
        .get_canvas_bounds(content)
        .expect("couldn't get dimensions");
    log::info!("Canvas size: {:?}", screensize);

    let mut app = MathDrillApp::new();
    app.init_storage();
    let mut allow_redraw = true;
    ui::draw(&app, &gam, content);

    // Cache best scores for display
    let mut cached_easy = app.get_best(&problems::Difficulty::Easy);
    let mut cached_medium = app.get_best(&problems::Difficulty::Medium);
    let mut cached_hard = app.get_best(&problems::Difficulty::Hard);

    loop {
        let msg = xous::receive_message(sid).unwrap();
        let now_ms = tt.elapsed_ms();

        match FromPrimitive::from_usize(msg.body.id()) {
            Some(AppOp::Redraw) => {
                if allow_redraw {
                    app.needs_redraw = true;
                    if app.state == app::AppState::BestScores {
                        ui::draw_best_with_data(
                            &gam, content,
                            cached_easy.as_ref(),
                            cached_medium.as_ref(),
                            cached_hard.as_ref(),
                        );
                    } else {
                        ui::draw(&app, &gam, content);
                    }
                }
            }
            Some(AppOp::Rawkeys) => xous::msg_scalar_unpack!(msg, k1, k2, k3, k4, {
                let keys = [
                    core::char::from_u32(k1 as u32).unwrap_or('\u{0000}'),
                    core::char::from_u32(k2 as u32).unwrap_or('\u{0000}'),
                    core::char::from_u32(k3 as u32).unwrap_or('\u{0000}'),
                    core::char::from_u32(k4 as u32).unwrap_or('\u{0000}'),
                ];
                let mut should_quit = false;
                for &key in keys.iter() {
                    if key != '\u{0000}' {
                        if !app.handle_key(key, now_ms, &rng) {
                            should_quit = true;
                            break;
                        }
                    }
                }
                if should_quit { break; }

                // Check feedback timeout
                app.check_feedback_timeout(now_ms, &rng);

                if app.needs_redraw && allow_redraw {
                    if app.state == app::AppState::BestScores {
                        // Refresh cache
                        cached_easy = app.get_best(&problems::Difficulty::Easy);
                        cached_medium = app.get_best(&problems::Difficulty::Medium);
                        cached_hard = app.get_best(&problems::Difficulty::Hard);
                        ui::draw_best_with_data(
                            &gam, content,
                            cached_easy.as_ref(),
                            cached_medium.as_ref(),
                            cached_hard.as_ref(),
                        );
                    } else {
                        ui::draw(&app, &gam, content);
                    }
                    app.needs_redraw = false;
                }
            }),
            Some(AppOp::FocusChange) => xous::msg_scalar_unpack!(msg, state_code, _, _, _, {
                match gam::FocusState::convert_focus_change(state_code) {
                    gam::FocusState::Background => {
                        allow_redraw = false;
                        app.save_state();
                    }
                    gam::FocusState::Foreground => {
                        allow_redraw = true;
                        // Refresh best scores cache
                        cached_easy = app.get_best(&problems::Difficulty::Easy);
                        cached_medium = app.get_best(&problems::Difficulty::Medium);
                        cached_hard = app.get_best(&problems::Difficulty::Hard);
                        ui::draw(&app, &gam, content);
                    }
                }
            }),
            Some(AppOp::Quit) => break,
            _ => log::warn!("unknown opcode: {:?}", msg.body.id()),
        }
    }

    app.save_state();
    xns.unregister_server(sid).unwrap();
    xous::destroy_server(sid).unwrap();
    xous::terminate_process(0)
}
