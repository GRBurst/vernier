# M5 rendering spike — `miette` vs `annotate-snippets`, and the JSON route

Date 2026-09-27, inside devenv (Rust 1.98.1, `CARGO_HOME=$DEVENV_STATE/cargo`), scratch crate `.sdd/m5-spike/` (untracked).
Versions: `annotate-snippets` 0.12.16, `miette` 7.6.0 (`fancy`), `serde` 1.0.229 (`derive`), `serde_json` 1.0.151.
Probe diagnostic: a sentence that starts at column 12 of line 3 and ends on line 4 of a Markdown file, with two notes.

## Verdict

**`annotate-snippets`** renders the diagnostics. **`serde` + `serde_json`** (derive, on dedicated output types) writes the JSON.

## Evidence

| Question | `annotate-snippets` 0.12.16 | `miette` 7.6.0 `fancy` |
| :--- | :--- | :--- |
| Header | `warning[CognitiveOverload]: …` then ` --> f.md:3:12`, rustc's layout; the spec's example format | `CognitiveOverload` on its own line, then `⚠ …` and `╭─[f.md:3:12]` |
| Colour versus content | the caller picks `Renderer::plain()` or `Renderer::styled()`. Measured: `strip_ansi(styled) == plain` on the probe; the crate's source never reads the environment or the terminal (no `std::env`, no `IsTerminal`) | the handler probes stderr for colour (`supports-color`, which reads `NO_COLOR`), for Unicode (`supports-unicode`) and for width (`terminal_size`). So the box characters and the line wrapping depend on the terminal. Measured: the `help` text wraps at 80 columns in a pipe and at 100 with a fixed width |
| Survives `NO_COLOR` and non-TTY stdout unchanged in content | yes, by construction: vernier chooses plain or styled in its shell, and both have the same text | no, not with the default handler: content changes with terminal width and Unicode support. A fixed theme and width restore it, but then the auto-detection is unused weight |
| Multi-line sentence | underlines from the start column on the first line to the end column on the last line (`\_____^` / `\|___^`); spans over about 10 lines fold the middle (`...`) | marks whole lines (`╭─▶ … ├─▶`), no columns |
| Long line | elides at a fixed `term_width` (default 140, not probed), so it is deterministic. `term_width(100_000)` shows a 365-column line whole; prose lines are often long, since Markdown paragraphs are commonly one line | wraps at the probed width |
| Column | counts Unicode scalar values: `Café ünï. Next` gives col 11; also checked with CJK, a combining accent, a tab, an emoji, CRLF and a lone CR. This equals vernier's `Position` (M1 definition) in all six cases | not measured (rejected earlier) |
| Footer list | `Level::NOTE.no_name().message("metrics:\n- …")` renders as `= metrics:` plus indented lines, the spec example's shape | `help:` block |
| License | MIT OR Apache-2.0 (the same as vernier) | Apache-2.0 |
| Dependencies (normal, including itself) | 3: `anstyle`, `unicode-width` | 53, including `backtrace`, `gimli`, `object`, `icu_*`, `terminal_size`, `rustix` |
| Release build from clean (scratch crate, this machine) | 3.2 s | 17.4 s |
| Native code | none | none (but `backtrace` links platform unwinding) |

JSON route:

| Question | `serde` + `serde_json` derive | hand-written |
| :--- | :--- | :--- |
| String escaping (paths, `"` in messages, control characters) | correct by the library | must be re-implemented and tested |
| `f64` | shortest round-trip form; non-finite values become `null` (vernier's values are finite, and absent values are `Option`) | must be specified |
| Dependencies | 12 crates for both (`serde_core`, `serde_derive`, `itoa`, `memchr`, `zmij`, …); `syn`/`quote`/`proc-macro2` are already in vernier through `thiserror` | 0 |
| Release build from clean | 7.1 s (both crates) | — |
| License | MIT OR Apache-2.0 | — |

## Consequences for the plan

- The shell decides colour: `Style::Color` only when stdout is a terminal and `NO_COLOR` is unset or empty (no-color.org). Everything else is `Style::Plain`. The core renders a `String` for either and never reads the environment.
- The rendered text is the same whatever the terminal, so the integration tests (pipes) check the exact content users see.
- `Serialize` goes on output types in one module, not on the core types, so the JSON schema is visible in one file and does not drift with refactors.
