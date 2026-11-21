# precursor-mathdrill

A timed arithmetic quiz for [Precursor](https://precursor.dev): ten problems per session, a score that rewards speed and streaks, and saved best results per operation and level.

## Features

- Operations: addition, subtraction, multiplication, division, or mixed. Levels: Easy (operands 1-9), Medium (2-19), Hard (2-49).
- Problems are drawn with the hardware TRNG through an unbiased range function (no modulo bias). If the TRNG fails, the quiz stops with a message; it never falls back to a predictable problem.
- Every answer is an exact integer: subtraction is never negative, division always has a divisor of at least 2 and an exact quotient (no `n / 1`, never by zero).
- Scoring per correct answer: base points by level (10/20/30), a speed bonus that shrinks to zero over 10 seconds, and +5 once the streak reaches 3. Wrong answers score 0 and reset the streak.
- Best results are kept per (operation, level) and each metric is tracked on its own: best score, most correct, longest streak, fastest average. A weaker session never lowers a stored best. Quitting a session early records nothing.
- The feedback screen advances by itself after 1.5 s (or on Enter).
- The clock is paused while the app is in the background, and the app's timer stops there too.

## Keys

| Screen | Keys |
|---|---|
| Menu | Up/Down move, Left/Right (or Enter) change operation or level, Enter on Start / Best scores, Backspace or F4 leave the app |
| Answering | 0-9 and `-` type, Backspace deletes (on an empty answer it stops the session), Enter submits, F4 stops the session |
| Feedback | Enter or Space skips ahead, F4 stops the session |
| Results, Best scores | Enter, Space, Backspace or F4 back to the menu; Up/Down scroll the best list |

The menu key (centre of the D-pad) is consumed by Xous and never reaches the app.

## Storage

PDDB dictionary `mathdrill`, key `state`: one JSON blob with a `version` field holding the last operation/level selection and the best results. It is written only when something changed, replaced in the basis it already lives in, and reloaded when a basis is locked or unlocked. If stored data can't be read, or was written by a newer version, the app runs but does not save over it.

## Limitations

- Bests are stored in plaintext JSON in the PDDB (the PDDB itself provides encryption at rest; the app adds none).
- Fixed 10-problem sessions; there is no session timer or custom length.
- Layout has not been checked on a device. Renode capture not yet run: `capture.py` is written but not executed; verification so far is the hosted build, clippy, lint and host tests.
- Earlier versions stored bests under `mathdrill.stats` keyed by level only; those records are ignored.

## Building

Builds inside a [xous-core](https://github.com/betrusted-io/xous-core) tree. With [xous-dev-toolkit](https://github.com/tbcolby/xous-dev-toolkit):

```bash
bin/integrate --xous-root ~/xous-core --grade ~/Dev/precursor/precursor-mathdrill
```

Host tests alone: `cd core && cargo test`.

## Layout

```
core/src/problems.rs  operations, levels, TRNG-driven generation
core/src/scoring.rs   points, per-metric bests, stored format
core/src/drill.rs     quiz state machine
src/main.rs, app.rs   GAM registration, key mapping, drawing, saving
capture.py            Renode capture sequence (not yet run)
```

## Author

Tyler Colby, [Colby's Data Movers, LLC](https://colbysdatamovers.com). Licensed under the Apache License, Version 2.0 (see [LICENSE](LICENSE)).
