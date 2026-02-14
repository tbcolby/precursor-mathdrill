# Math Drill — Build Notes

## Architecture
- 5-state machine: Menu, Playing, Feedback, Results, BestScores
- TRNG-generated operands with rejection sampling
- Division uses reverse-generation (answer * b = a) for clean integers
- Ticktimer for per-problem timing and feedback auto-advance

## Key Patterns
**OpMode enum** — Single(Operation) or Mixed for random operation selection
**Feedback state** — 1.5s auto-advance or key skip; shows correct answer on wrong
**Progress bar** — filled rectangle proportional to problems completed
**Best score caching** — loaded into main loop vars to avoid borrow issues with storage

## Build
```bash
cargo build -p mathdrill --target riscv32imac-unknown-xous-elf
```
