# Math Drill build notes

- Problem generation, scoring and the quiz state machine (`Drill`) are in `core/` (host-tested: `cd core && cargo test`). All randomness goes through `precursor_appkit_core::entropy` (`below`), never modulo. Time is passed in as `now_ms`; the shell pauses it on Background (`pause`/`resume`).
- Shell: `src/main.rs` routes GAM messages and runs the appkit pump only while a quiz screen is showing; `src/app.rs` maps `Key` to core `Input`, saves and draws.
- Storage: one versioned JSON blob (`mathdrill`/`state`: selection plus bests per mode and level), saved only when dirty (menu leave, session end, background). Unreadable or newer data is never overwritten.
- Every screen exits on Backspace/F4. `∴` is never delivered. Leave with `kit::leave`, never `terminate_process`.
