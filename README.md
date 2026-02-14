# precursor-mathdrill

Timed arithmetic quiz for the [Precursor](https://precursor.dev) hardware platform.

## Features

- **4 Operations** — addition, subtraction, multiplication, division (or mixed)
- **3 Difficulty Levels** — Easy (1-9), Medium (2-19), Hard (2-49)
- **10-Problem Sessions** — timed quiz with progress bar
- **Streak Tracking** — consecutive correct answers tracked
- **Best Scores** — high scores saved per difficulty in PDDB
- **Clean Division** — TRNG-generated problems with guaranteed integer answers
- **Instant Feedback** — correct/wrong shown after each answer

## Controls

| Key | Action |
|-----|--------|
| ↑/↓ | Navigate menu |
| ←/→ | Cycle options |
| 0-9 | Type answer |
| - | Negative sign |
| Backspace | Delete digit |
| Enter | Submit/confirm |
| Menu (∴) | Back/quit |

## Build

```bash
cargo build -p mathdrill --target riscv32imac-unknown-xous-elf
```

## License

Apache 2.0 — see [LICENSE](LICENSE).
