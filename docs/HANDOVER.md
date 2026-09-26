# Handover

## 1. Preconditions verified

- 2026-09-26, inside the devenv shell with `export CARGO_HOME=$DEVENV_STATE/cargo` (audit 002): `just verify` printed `✓ all gates green` (13 clarify-open warnings expected; 35 unit + 5 integration tests).
- `PROPTEST_CASES=5000 cargo test --lib` passed (all properties).
- Every planted violation in `docs/specs/001-vernier/plan-M1.md` was seen to fail, then reverted.
- Nested `devenv shell -- …` and `devenv print-dev-env` fail in the agent sandbox (`Permission denied`); GPG signing fails too; commits are made unsigned (D13).

## 2. What changed

- The user directed "continue directly with implementation": phase 2 (review, clarification, approval, comprehension) was skipped and recorded in `gates.md` and as BACKLOG item 2. Spec 001 stays `Draft`; M1's two markers run on their fallbacks (headings and alt text dropped; `.md`/`.markdown` are Markdown).
- **M1 IMPLEMENTED** (not `DONE`: needs the user's review and the owed phase 2). Plan: `docs/specs/001-vernier/plan-M1.md` (location now D12).
  - `src/position.rs` — `LineIndex::position(offset) -> Result<Position, PositionError>`, 1-based line, column in scalar values; only `\n` ends a line.
  - `src/prose.rs` — `extract_prose` (role stack over pulldown-cmark events), `SourceFormat::from_path`, `plain_text_prose`. A character reference (`&amp;`) yields no span, so every span is its own source slice.
  - `src/words.rs` — `count_words` (UAX #29, `unicode_words`). `src/summary.rs` — per-file span and word counts.
  - `src/cli.rs` — clap derive, `analyze` / `check`, all M5 flags (`--max-mdd` must be finite and > 0). `src/main.rs` — the shell; exit 2 if any file is unreadable or not UTF-8; `check` flags nothing yet.
  - `tests/cli.rs` + `tests/fixtures/` — end-to-end exit codes and output.
- `devenv.nix` sets `CARGO_HOME` project-local (audit 002). Audit 003: clippy's `allow-unwrap-in-tests` does not cover integration-test helpers.
- Dependencies: `pulldown-cmark` 0.13 (MIT, no default features), `clap` 4 (derive), `unicode-segmentation` 1, `thiserror` 2; dev: `proptest` 1. All MIT/Apache-2.0, pure Rust.

## 3. How to check

```sh
export CARGO_HOME=$DEVENV_STATE/cargo   # only in a shell started before this devenv.nix
just verify                             # ✓ all gates green
cargo run -q -- analyze tests/fixtures/sample.md tests/fixtures/plain.txt
# tests/fixtures/sample.md: 7 prose spans, 26 words
# tests/fixtures/plain.txt: 1 prose spans, 8 words
cargo run -q -- check /nope; echo $?    # vernier: cannot read /nope: … → 2
devenv shell -- sh -c 'echo $CARGO_HOME' # by hand (sandbox cannot): …/vernier/.devenv/state/cargo
```

## Open tasks

- Phase 2 for spec 001 is owed (BACKLOG item 2), including the 13 `[NEEDS CLARIFICATION]` questions.
- User review of M1 → `DONE`.

## Next action

M2 (surface readability engine): write `docs/specs/001-vernier/plan-M2.md` with the tdd-implementation-planner skill, then implement on the M2 fallbacks (CMUdict 500-word sample, ≥ 90 % agreement, measured under `measurements/`). The CMUdict sample needs a Phase 0 license check (CMUdict is BSD-style; verify).

## Known-bad approaches

- Scratch files in `/tmp` inside the agent sandbox; use `.sdd/` (audit 001).
- Cargo with the default `CARGO_HOME` (`~/.cargo` is read-only in the sandbox and global, audit 002).
- Signed commits from the agent sandbox: `~/.gnupg` is on nono's deny list; commit with `git -c commit.gpgsign=false` (D13).
- Checking "slice = span text" with decoded pulldown text: character references break it; the span must borrow the slice.
