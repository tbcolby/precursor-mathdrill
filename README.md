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

---

## Development

This app was developed using the methodology described in [xous-dev-toolkit](https://github.com/tbcolby/xous-dev-toolkit) — an LLM-assisted approach to Precursor app development on macOS ARM64.

---

## Author

Made by Tyler Colby — [Colby's Data Movers, LLC](https://colbysdatamovers.com)

Contact: [tyler@colbysdatamovers.com](mailto:tyler@colbysdatamovers.com) | [GitHub Issues](https://github.com/tbcolby/precursor-mathdrill/issues)

---

## License

Licensed under the Apache License, Version 2.0.

See [LICENSE](LICENSE) for the full text.
