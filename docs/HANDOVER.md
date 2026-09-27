# Handover

## 1. Preconditions verified

- 2026-09-27, `export CARGO_HOME=$DEVENV_STATE/cargo` (audit 002): `just verify` printed `✓ all gates green`, with 194 tests (171 lib unit, 1 bin, 20 cli, 1 positions, 1 examples) after M5.
- `PROPTEST_CASES=3000 cargo test --all-targets` passed.
- Every M5 plant in `plan-M5.md` (1–19) was seen red, then reverted.
- Every planted violation in `docs/specs/001-vernier/plan-M3a.md` was seen to fail (12/12, with the proptest seeds moved aside), then reverted; so was every F1 plant in `plan-M2.md` (7/7) and every M4 plant in `plan-M4.md` (1–13, plus 7″).
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
- **M3a IMPLEMENTED** (plan `docs/specs/001-vernier/plan-M3a.md`, commits a57a0a1..271ef96):
  - `src/dependency.rs`: `Token`, the `Parser` trait, `DependencyTree::new` (rejects empty, ids out of order, head out of range, several roots, cycle — one `TreeError` each).
  - `src/syntax.rs`: content projection (drop `PUNCT`, renumber, reattach; a root when only `PUNCT` ancestors), `DependencyDistance` monoid (file MDD pools, never averages), `depth` in edges, `clause_count`, `center_embeddings`, `SyntacticMetrics`.
  - `src/analysis.rs`: `SyntacticFlag` (carries its value), `syntactic_flags`, `analyze_parsed(.., &impl Parser)`; `analyze` unchanged. `main.rs` renders the four messages (unit-tested only; no CLI path has a parser until M3b). `--max-tree-depth` help says edges.
  - The example parse is UDPipe 2's real output (LINDAT, `english-ewt-ud-2.17`): MDD 32/12 ≈ 2.67 (so `HighMdd` is not raised at 3.0), depth 4, 1 clause, center-embedding proposal/caused/8.
- **Follow-up F1, block prose across inline markup** (audit 006, plan-M2 section F1, commits 33d9158..4f05aaa): spans are joined directly when only emphasis/strong/strikethrough/super-/subscript/link delimiters lie between them, otherwise with one space (`ProseSpan::joins_previous`, set by `extract_blocks`). `**The proposal**, which` now reaches sentences as `The proposal, which`; `un*believ*able` is one word, also on M1's word line (`summarize` counts words on block text). Changed test: `analysis::parses_the_prose_not_the_markup` now keys on `EXAMPLE_TEXT` (it was keyed on the buggy text).
- **M4 IMPLEMENTED** (plan `docs/specs/001-vernier/plan-M4.md`, commits c04738a..f6659be):
  - `data/nominalization-stoplist.txt` (pybiber seed + research false positives) and its MIT notice.
  - `src/nominalization.rs`: `Stoplist::committed()`, `surface_lemma` (possessive, then `-ies`→`-y` or one final `-s`), `is_nominalization`, surface/parsed counts, the `NominalizationCount` monoid (`ratio()` is `None` without words).
  - `src/passive.rs`: `passive_heads`, `align` (forms in order; at the cursor, else at the next word start; a miss costs one position).
  - `analysis.rs`: `SentenceAnalysis.nominalizations` is the effective count (surface words without a parse, `NOUN` lemmas with one); `SentenceSyntax.passives`; `FileAnalysis.passives` is `None` without a parse. `summary` prints a fourth line: `nominalization ratio 0.286 (4 of 14 words), passive voice absent (no parse)`.
  - Spec fixed in place: the `-ies` lemma (audit 007) and possessives (audit 008). Plan fixed: alignment after a substituted form (audit 009, with a documented one-offset limit).
  - Changed tests (layout only): `tests/cli.rs analyze_prints_surface_metrics` expects 8 lines with a stride of 4; summary's two full-render witnesses gain line 4; M3a P14's parse-only eraser also resets `passives`.
  - A by-hand run gave README 5/117, spec 46/2107. Candidate stoplist additions for later: `segment`, maybe `distance`, `evidence`, `density`, `reference`.
- **M3b spike report committed** (7f60be9): `docs/specs/001-vernier/research/m3b-parser-spike.md`, sample `measurements/parser-sample.md` (50 sentences), condensed logs `measurements/m3b-spike-data.md`.
  - udpipe (UDPipe 1 via `udpipe-rs` 0.2.0): builds offline in devenv, 1.3 MB binary, model 16 MB, per sentence median 13–17 ms / p95 32–49 ms, RSS ~104 MiB, lemmas yes, UAS vs UDPipe 2 83.0 %.
  - ONNX via `ort`: `download-binaries` breaks B4 (nix onnxruntime works), 27 MB binary, model 507 MB, median ~450 ms / p95 2.3 s (quadratic in sentence length), RSS ~1 GiB, no lemmas/XPOS, UAS 94.8 %, MST decoding.
  - The UDPipe 1 EWT model parses the M3a example wrong (root "proposal"); GUM and ParTUT models get proposal/caused/8. ONNX matches `EXAMPLE_CONLLU`.
  - `udpipe-rs` hard-wires its tokenizer (1/50 sentences re-split into an invalid tree); one-line fix in its Rust wrapper. Its `ureq`/`rustls`/`ring` dependency is mandatory but unused in the binary.
  - Recommendation: udpipe with a presegmented patch. Budget proposal (BACKLOG 6): load ≤ 2.5 s, p95 ≤ 50 ms per sentence, ≤ 1.5 s per 1000 words, RSS ≤ 200 MiB, as a measurement, not a gate.
- **M5 IMPLEMENTED** (plan `docs/specs/001-vernier/plan-M5.md`, commits b6eb32e..9153c4e; spike `research/m5-rendering-spike.md`):
  - `src/diagnostic.rs`: `FlagMessage`, `Diagnostic` (one per flagged sentence, start/end positions), compact render, annotate-snippets render (`Style` Plain/Color; colour only on a TTY without `NO_COLOR`; `term_width` huge so long lines are never elided), `= metrics:` lines (words, MDD, depth, clauses, center-embedding; `absent (no parse)`).
  - `check` default text = annotated warning; `--format compact` = `path:l:c: CognitiveOverload: <flag msgs joined by '; '>`; `--format json` = one document after all files (`src/json.rs`, serde, `schema_version` 1), for `analyze` and `check`.
  - `analyze` prints M1's line + a metric/value table. `FileSummary.mean_dependency_distance`/`passives` are `None` until M3b.
  - `tests/positions.rs`: text, compact and JSON give the same line/column, at the sentence's first character. `docs/examples/github-actions.yml` + `tests/examples.rs`.
  - Audit 010: serde_json's default parser does not round-trip floats; tests read with the dev-only `float_roundtrip` feature.
  - Changed tests (layout only, stated in the plan): three exact-line `check` tests run with `--format compact`; analyze/summary tests read table rows; main's per-flag-lines test removed (law kept in `diagnostic::one_diagnostic_per_flagged_sentence_with_all_its_flags`).
- The git history was rebased on 2026-09-27 09:14, outside the agent, and rewritten again during M5. The content is identical (`git range-diff` shows `=`), and the hashes changed. The hashes above are the current ones.
- A changed M1 test: `check … --max-sentence-len 10` on sample.md now exits 1 (sample.md has a 13-word sentence). The old test passes 30, and a new test asserts the flag at 10.

## 3. How to check

```sh
export CARGO_HOME=$DEVENV_STATE/cargo
just verify                                   # ✓ all gates green
cargo run -q -- analyze tests/fixtures/sample.md
# tests/fixtures/sample.md: 7 prose spans, 26 words
#   metric                    value
#   sentences                 4
#   ...
#   Flesch Reading Ease       31.04
#   nominalization ratio      0.038 (1 of 26 words)
#   passive voice             absent (no parse)
cargo run -q -- check tests/fixtures/long.md; echo $?
# warning[CognitiveOverload]: Sentence exceeds human working-memory capacity
#  --> tests/fixtures/long.md:3:20
#   ... (sentence underlined)
#   = metrics:
#     - words: 30 (LongSentence, max 25)
#     - mean dependency distance: absent (no parse)  ...
# 1
cargo run -q -- check --format compact tests/fixtures/long.md
# tests/fixtures/long.md:3:20: CognitiveOverload: LongSentence: sentence has 30 words (max 25)
cargo run -q -- check --format json tests/fixtures/long.md   # {"schema_version": 1, "files": [...]}
```

## Open tasks

- Phase 2 for spec 001 is owed (BACKLOG item 2).
- User review of M1, M2, M3a, M4 and M5 → `DONE`. By hand:
  - M5 colour on a real terminal (`cargo run -- check tests/fixtures/long.md`, then with `NO_COLOR=1`: same text; the sandbox has no PTY);
  - the GitHub Actions example in a real repository (install source has an `OWNER` placeholder);
  - skim the miss list in `measurements/syllables.md`;
  - run `vernier check` on one of your own Markdown files and skim the flagged positions. `check README.md docs/specs/001-vernier/spec.md` flags 28 sentences; they looked right on a spot check.
- M3a decisions to confirm (plan-M3a "Decisions made"): a content token with only `PUNCT` ancestors becomes a projected root; `words_between` counts tokens whose form has a letter or digit; the subject's head is not required to be a `VERB`.
- M4 candidate stoplist additions (see above); decide on real prose, not by guess.
- M3b: user decisions pending (asked 2026-09-27): route (udpipe recommended), which model the M3a-example integration test names (EWT parses it wrong; GUM/ParTUT right) or reword the criterion, the performance budget, a LICENSE file, the example's install source.
- M3b if udpipe: vendor `udpipe-rs` with the presegmented one-line patch and without `ureq`/`rustls`/`ring` (plus an upstream PR later); write an ADR for the in-process C++ dependency (no `unsafe` in vernier itself).

## Next action

1. With the user's M3b answers: record the route and model license in the spec (M3b criteria 1–2) and README, then plan M3b (`plan-M3b.md`) test-first and implement.

## Known-bad approaches

- Scratch files in `/tmp` inside the agent sandbox; use `.sdd/` (audit 001).
- Cargo with the default `CARGO_HOME` (audit 002).
- Signed commits from the agent sandbox (D13).
- Checking "slice = span text" with decoded pulldown text: character references break it; the span must borrow the slice.
- Reordering the syllable heuristic's alternatives is not a meaningful mutation (audit 005).
- A float witness exactly on a rounding tie (59.635 → "59.63" or "59.64"); pick counts away from ties for rendered output.
