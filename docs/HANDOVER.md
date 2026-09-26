# Handover

## 1. Preconditions verified

- 2026-09-27, `export CARGO_HOME=$DEVENV_STATE/cargo` (audit 002): `just verify` printed `✓ all gates green`, with 91 tests (79 lib unit, 1 bin, 11 integration).
- `PROPTEST_CASES=3000 cargo test --lib` passed.
- Every planted violation in `docs/specs/001-vernier/plan-M2.md` was seen to fail, then reverted. #1 was an equivalent mutation and got a substitute (audit 005).
- Nested `devenv shell -- …` fails in the agent sandbox. Commits are unsigned (D13) and the pre-commit hook runs `just verify`.

## 2. What changed

- **Spec 001 markers resolved.** All 13 `[NEEDS CLARIFICATION]` markers are answered in `spec.md` (commit c8404d0), from the user's answers and from research. The research is in `docs/specs/001-vernier/research/{licenses,metrics}.md` and `2026-09-26-vernier-open-questions-answered.md`.
  - The user decided:
    - fixed advice per rule;
    - UDPipe's CC BY-NC-SA model is accepted for personal, non-commercial use;
    - no downloader, only `--model-path`;
    - the M3b parser choice is decided before M3b.
  - Clarification PASSED in `gates.md`. Review, Approval and Comprehension are still owed (BACKLOG 2).
- **M2 IMPLEMENTED** (not `DONE`, because the user's review and phase 2 are owed). The plan is `docs/specs/001-vernier/plan-M2.md`.
  - `src/words.rs` `words()`.
  - `src/syllables.rs`: `count_syllables` is a hand port of the regex heuristic, with no regex crate. `is_complex` subtracts a syllabic suffix and exempts capitalized words that do not start a sentence.
  - `src/readability.rs`: the `SurfaceCounts` monoid. `readability()` returns `Option<Readability>` (FRE, FKGL, Fog, ASL).
  - `src/prose.rs` `blocks()`, `src/block.rs` (spans joined into one text, with a byte map back to the source), `src/sentence.rs` (UAX #29, abbreviation merge, trim).
  - `src/analysis.rs`: `analyze`, `Thresholds`, `Flag::LongSentence`.
  - `src/summary.rs` renders two more lines per file. `check` prints `path:line:col: LongSentence: sentence has N words (max M)` and exits 1; an unreadable file still wins with exit 2.
  - `docs/specs/001-vernier/measurements/`: 500 CMUdict words, the generator and CMU's license. Agreement is 469/500 = 93.8 %; the misses are listed in `syllables.md`.
  - Spec fix in place (audit 004): line breaks in block prose read as spaces, and blank lines separate plain-text blocks.
- A changed M1 test: `check … --max-sentence-len 10` on sample.md now exits 1 (sample.md has a 13-word sentence). The old test passes 30, and a new test asserts the flag at 10.

## 3. How to check

```sh
export CARGO_HOME=$DEVENV_STATE/cargo
just verify                                   # ✓ all gates green
cargo run -q -- analyze tests/fixtures/sample.md
# tests/fixtures/sample.md: 7 prose spans, 26 words
#   4 sentences, 52 syllables, 7 complex words
#   Flesch Reading Ease 31.04, Flesch-Kincaid Grade 10.55, Gunning Fog 13.37, average sentence length 6.50
cargo run -q -- check tests/fixtures/long.md; echo $?
# tests/fixtures/long.md:3:20: LongSentence: sentence has 30 words (max 25)
# 1
```

## Open tasks

- Phase 2 for spec 001 is owed (BACKLOG item 2).
- User review of M1 and M2 → `DONE`. By hand:
  - skim the miss list in `measurements/syllables.md`;
  - run `vernier check` on one of your own Markdown files and skim the flagged positions. `check README.md docs/specs/001-vernier/spec.md` flags 28 sentences; they looked right on a spot check.
- M3b parser choice (UDPipe via FFI needs an ADR for `unsafe`, D9), to be decided before M3b.

## Next action

M3a (syntactic metrics on hand-built token graphs, no parser): write `docs/specs/001-vernier/plan-M3a.md` with the tdd-implementation-planner skill, then implement. M4's surface parts depend on M3b's parse, so M3a comes next.

## Known-bad approaches

- Scratch files in `/tmp` inside the agent sandbox; use `.sdd/` (audit 001).
- Cargo with the default `CARGO_HOME` (audit 002).
- Signed commits from the agent sandbox (D13).
- Checking "slice = span text" with decoded pulldown text: character references break it; the span must borrow the slice.
- Reordering the syllable heuristic's alternatives is not a meaningful mutation (audit 005).
- A float witness exactly on a rounding tie (59.635 → "59.63" or "59.64"); pick counts away from ties for rendered output.
