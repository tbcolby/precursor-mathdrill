"""Renode capture sequence for Math Drill (not yet run).

Run from xous-dev-toolkit:
    python3 scripts/renode_capture.py --app mathdrill --script ../precursor-mathdrill/capture.py

Contract: run(ctl, outdir) -> number of failed steps. The app is already launched.
"""
import os


def run(ctl, outdir) -> int:
    failures = 0

    def shot(name):
        nonlocal failures
        if not ctl.try_screenshot(os.path.join(outdir, name)):
            failures += 1

    shot("01_menu.png")
    ctl.timed_key("Down", after=0.5)
    ctl.timed_key("Down", after=0.5)       # Start row
    ctl.inject_line("")                    # Enter: begin a session
    ctl.sleep(1.0)
    shot("02_problem.png")
    ctl.inject_string("12")
    ctl.inject_line("")                    # submit (probably wrong; either way shows feedback)
    ctl.sleep(0.5)
    shot("03_feedback.png")
    ctl.timed_key("F4", after=1.0)         # stop the session, back to the menu
    ctl.timed_key("Up", after=0.5)         # Difficulty
    ctl.timed_key("Up", after=0.5)         # Operation
    ctl.timed_key("Right", after=0.5)
    shot("04_menu_changed.png")
    ctl.timed_key("F4", after=1.0)         # leave the app
    return failures
