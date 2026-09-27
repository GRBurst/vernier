# Plan — spec 001 M5: Diagnostics, check mode and CI output

Tier: `full-spec`. Spec: [spec.md#M5](spec.md#M5), definitions *Position*, *Sentence*. Gate record: [gates.md](gates.md) (Clarification passed; Review, Approval and Comprehension still owed, BACKLOG item 2).
Evidence: [research/m5-rendering-spike.md](research/m5-rendering-spike.md) (renderer and JSON route, measured 2026-09-27 in `.sdd/m5-spike/`).
Before planning, the M5 criteria were clarified in place (spec `Draft`, D4); T0 lists what changed.
M5 is built without a parser: the CLI has none until M3b. Every syntactic line is rendered from `Option`s and tested with pasted parses (as in M3a/M4). The "unusable model → exit 2" half of criterion 2 is met when M3b lands (its `--model-path` criterion); M5 adds no model code.

## Motivation

`check` must be usable as a CI gate and readable by a person:
- one `warning[CognitiveOverload]` diagnostic per flagged sentence, pointing at its first character, underlining it (over all its lines) and listing the rule metrics with the flags they raised;
- the same diagnostics as one JSON document (`schema_version` 1) or as one `path:line:col: code: message` line each;
- exit 0/1/2.

`analyze` prints a table of each file's metrics and always exits 0 on readable files. A GitHub Actions example shows the CI use.

## Approach

- **`src/diagnostic.rs`** (new, pure) holds the diagnostic *model* and its text renderings.
  - `FlagMessage { name, message }`: today's `describe`/`describe_syntactic` texts, moved out of `main.rs`.
  - `Diagnostic { start, end, source_range, flags, metrics }`: one per sentence with at least one surface or syntactic flag.
  - `diagnostics(source, &FileAnalysis, &Thresholds) -> Result<Vec<Diagnostic>, PositionError>`.
  - `render_compact`, and `render_text` through `annotate-snippets` with an explicit `Style`.
- **`src/json.rs`** (new): `serde` output types (the schema lives in this one file) and `document(&[FileReport]) -> Result<String, serde_json::Error>`.
- **`src/summary.rs`**: `render` becomes the M1 line plus a two-column table of the file metrics. `FileSummary` gains `mean_dependency_distance: Option<f64>` and `passives: Option<usize>`; `summarize` (no parser) sets both to `None`, and M3b fills them.
- **`src/main.rs`** (shell):
  - chooses `Style` from `stdout().is_terminal()` and `NO_COLOR`;
  - dispatches on `--format`;
  - for `json`, collects every readable file's report and prints one document after the loop;
  - exit codes as today (2 beats 1 beats 0).
- **`docs/examples/github-actions.yml`**: a workflow for a user's repository, checked by `tests/examples.rs`. It is not a workflow of this repository (BACKLOG 7 is that).

## Spec clarifications (T0, in place, spec `Draft`)

No criterion was wrong, so there is no audit; each was ambiguous in a way the plan had to decide:
1. Criterion 1:
   - the diagnostic is the default `text` format;
   - it underlines "over all its lines";
   - "every metric" means every metric a rule checks (five, named), with value, maximum and flag, and `absent (no parse)` unparsed.
2. Criterion 2 holds "in every format"; the unusable-model case is exercised once M3b loads models.
3. Criterion 3:
   - JSON covers both commands;
   - it is printed once, after all files;
   - it lists path, metrics and diagnostics per *readable* file.
4. Criterion 4: compact belongs to `check`; the code is `CognitiveOverload`, and the message joins the flag messages with `; `.
5. Criterion 5: `analyze` in `text`/`compact` prints M1's line plus the table, one row per metric, `absent` where absent.
6. Criterion 6: positions are also equal across the three formats.
7. Criterion 7: the example lives under `docs/examples/`.
8. Implementation details: `annotate-snippets` (spike), the colour rule, and `serde` + `serde_json` on output-only types.

## Decisions made

| Choice | Rationale | Rejected |
| :--- | :--- | :--- |
| `annotate-snippets` 0.12 renders the text diagnostics | rustc's `warning[Code]: … --> path:l:c` layout (the spec's example). Content never depends on the terminal: the caller picks `plain()`/`styled()` and `strip(styled) == plain` (spike). Column-exact multi-line underline, scalar-value columns equal to *Position*. MIT OR Apache-2.0, 3 crates | `miette` `fancy`: content depends on terminal width and Unicode probing, whole-line multi-line marks, 53 crates, Apache-2.0 only (spike) |
| `Renderer::term_width(100_000)` | Markdown paragraphs are often one long line; the default 140 elides the middle of the sentence the diagnostic is about (spike: a 365-column line is shown whole) | default width — hides the underlined text; probing the terminal — content would depend on it |
| Colour ⇔ stdout is a terminal ∧ (`NO_COLOR` unset ∨ empty), decided in `main.rs` and passed in as `Style` | no-color.org says "present and not an empty string"; environment and TTY are ambient input and belong in the shell (ENGINEERING §6) | reading the environment in the renderer; ignoring an empty `NO_COLOR` rule |
| `serde` + `serde_json` derive on dedicated output types in `json.rs` | correct escaping and floats for free. `syn`/`quote` are already built for `thiserror`; 12 crates, MIT OR Apache-2.0 (spike). The schema lives in one file and does not drift with core refactors | hand-written JSON — escaping and float formatting would need their own tests; `Serialize` on core types — the schema would change with every refactor |
| One diagnostic = one flagged sentence, in every format | criterion 1 defines the diagnostic per sentence; JSON's `diagnostics` array and compact's "one line per diagnostic" then count the same things, so CI totals agree across formats | compact line per flag — the count of lines would differ from the JSON array |
| Compact: `path:line:col: CognitiveOverload: LongSentence: sentence has 30 words (max 25); HighMdd: …` | the code is the diagnostic's code. The message is today's per-flag texts joined by `; `, so today's `check` line gains exactly `CognitiveOverload: ` and stays greppable per flag | the flag name as the code — one line per flag, see above |
| Default `--format` is `text` (already the `cli.rs` default) and means the annotated diagnostic for `check` | the spec's example is the default output; `compact` keeps a one-line form | compact as the default — the spec's example would need a flag |
| Text footer `= metrics:` lists the five rule metrics: words, mean dependency distance, tree depth, subordinate clauses, center-embedding. Each shows its value and `(Flag, max N)` or `(max N)`; without a parse, `absent (no parse)` | spec clarified: "every metric that a rule checks"; the other metrics (scores, nominalization, passives) are file-level in `analyze`/JSON | every metric of the sentence — ten lines per warning, most not about the warning |
| `analyze` (`text` and `compact`): M1's line, then a table `metric`/`value`, one row per file metric, `absent (…)` where absent | criterion 5 asks for a table; M1's line stays, so M1's criterion and its test hold | a separate table without M1's line — M1's test would break |
| `--format json` gives the same document for `analyze` and `check`: `{schema_version: 1, files: [{path, metrics, diagnostics}]}`. Only the exit code differs | one schema to document and test | two schemas |
| JSON positions: `line`, `column` (start, 1-based as *Position*), `end_line`, `end_column` (the position of `source_range.end`, exclusive) | the criterion's line/col, plus the span an editor needs to underline | byte offsets — not a *Position* |
| JSON `path` = `Path::display().to_string()` | the text and compact formats print the same (lossy for non-UTF-8 paths, identically in all formats) | failing on non-UTF-8 paths — a readable file would exit 2 |
| An unreadable file is named on stderr only; the JSON document lists readable files | M1's criterion (stderr, exit 2) unchanged; the exit code tells CI | an `error` entry in JSON — a second error channel to keep in sync |
| JSON serialization error → stderr, exit 2 | a `Result`, never `unwrap` (§8). It cannot occur with these types, but the type says it may | `expect` |
| Example at `docs/examples/github-actions.yml`, not `.github/workflows/` | the criterion asks for an example; a file under `.github/workflows/` would run on this repository's first push, which BACKLOG 7 plans separately | `.github/workflows/vernier.yml` |
| `describe`/`describe_syntactic` move from `main.rs` to `diagnostic.rs` in a refactor task first | the messages are core (pure) and JSON and compact need them | duplicating them |

## Gate (CONSTITUTION + ENGINEERING)

| Principle | Result |
| :--- | :--- |
| A2 specs are contracts | PASS — criteria clarified in place while `Draft` (D4), listed in T0; existing tests changed only where M5 changes the output layout (T6, T7), stated in those tasks |
| A3 ≤ 5 files per task | PASS — see the task table (max 5, T4/T8 with `Cargo.lock`) |
| A4 properties over constants | PASS — P1–P11. Literals only where the literal is the criterion: `warning[CognitiveOverload]`, the title, `schema_version` 1, and the one fixture position `3:20` that M2 already pinned |
| A6 seams | PASS — `Style` is the only new seam (TTY/env); no speculative abstraction |
| B4 hermetic | PASS — dependencies fetched into `$DEVENV_STATE/cargo` and locked; no network at test time |
| D9 complexity ≤ 10, no `unsafe` | PASS — one renderer per format; metric lines one `fn` each |
| D10 §5 newtypes | PASS — `Position` reused; `Style` a closed enum |
| D10 §6 pure core | PASS — only `main.rs` reads `NO_COLOR`/TTY and prints |
| D10 §7 outcomes | PASS — `Option` → `absent (…)` in text, `null` in JSON; `PositionError` and `serde_json::Error` propagated; exhaustive `match` on `SyntacticFlag`/`Flag` |
| D10 §8 banned constructs | PASS — no `unwrap`/`expect` outside tests |
| Engineering §2 new dependency | PASS — `annotate-snippets` 0.12, `serde` 1 (`derive`), `serde_json` 1: existence, API and license verified in the spike; no native code |
| Licenses | PASS — all three MIT OR Apache-2.0, the same as vernier |
| Phase 2 human gates | VIOLATION — Review/Approval/Comprehension still owed (BACKLOG item 2); the user directed implementation to continue |

## Files to read first

`docs/ENGINEERING.md`, `docs/audits/*` (005 for planted violations), `research/m5-rendering-spike.md`, `src/main.rs` (`check`, `diagnostics`, `describe*`, `run`), `src/analysis.rs` (`FileAnalysis`, `SentenceAnalysis`, `Flag`, `SyntacticFlag`), `src/position.rs`, `src/summary.rs`, `src/cli.rs` (`OutputFormat`), `src/testing.rs`, `tests/cli.rs`, this plan.
Before cargo: `export CARGO_HOME=$DEVENV_STATE/cargo` (audit 002). Scratch goes to `.sdd/` (audit 001); the spike crate is `.sdd/m5-spike/` (modes `annotate`, `annotate-compare`, `footer`, `long`, `unicode`).

## Type checking strategy

`rustc` + `clippy -D warnings` (`just lint`) after every GREEN.
Expected rejections:
- a `match` on `Flag`/`SyntacticFlag`/`OutputFormat`/`Style` missing an arm;
- rendering an absent metric without handling its `Option`;
- `json.rs` output types built from a type that is not `Serialize`.

## Testing strategy

| Layer | Covers | Needs |
| :--- | :--- | :--- |
| Unit `diagnostic` | model (P1, P2), compact (P3), text header/colour/underline (P4–P6), metric lines (P7), style law | `proptest`; hand-built `SentenceAnalysis` with syntax from `testing::EXAMPLE_CONLLU` (as in main's current test) |
| Unit `json` | schema (P8) | `serde_json::Value` to read the document back |
| Unit `summary` | the table (P9) | — |
| Integration `tests/cli.rs` | formats wired per command, exit codes, no escape codes off a terminal, one JSON document for several files | fixtures `long.md`, `sample.md`, new `wrapped.md` |
| Integration `tests/positions.rs` | criterion 6 across all three formats over generated documents (P10) | `proptest`, the library API |
| Integration `tests/examples.rs` | the GitHub Actions example (P11) | — |
| By hand | colours on a real terminal; the example in a real GitHub repository | a terminal; a GitHub repository (coverage gap) |

## Properties

`D(f)` = `diagnostics(source, &analyze(source, …), t)`; a sentence s is *flagged* iff `s.flags ≠ [] ∨ s.syntax.is_some_and(|x| x.flags ≠ [])`.

- **P1 one per flagged sentence:** `D(f).map(source_range) == [s.source_range | s ∈ f.sentences, s flagged]`, in source order. Each `d.flags` lists the sentence's surface flag messages, then its syntactic ones.
- **P2 positions:** `d.start == LineIndex::position(d.source_range.start)`, `d.end == LineIndex::position(d.source_range.end)`.
- **P3 compact:** `render_compact(p, d)` splits as `p:L:C: CognitiveOverload: M` with `(L, C) == d.start`, and `M.split("; ") == d.flags.map(to_string)`.
- **P4 text header:** line 0 of `render_text(p, src, d, s)` is `warning[CognitiveOverload]: Sentence exceeds human working-memory capacity`; line 1 trimmed is `--> p:L:C` with `(L, C) == d.start`.
- **P5 colour is only colour:** `strip_ansi(render_text(.., Color)) == render_text(.., Plain)`, and `Plain` contains no `\x1b`.
- **P6 underline covers the sentence:** for an ASCII sentence on one line, the `^` run on the marker line starts under the sentence's first character and its length equals the sentence's length in the source. `annotate-snippets` measures display width, so the generator keeps P6 to ASCII; multi-line is the `wrapped.md` witness.
- **P7 metric lines:**
  - the `words` line contains `LongSentence` ⇔ `words > max_sentence_len`;
  - with `syntax = None`, the four syntactic lines end in `absent (no parse)`;
  - with `Some(x)`, the MDD/depth/clause lines contain `HighMdd`/`DeepTree`/`ClauseOverload` ⇔ that flag ∈ `x.flags`;
  - there is one `center-embedding` entry per `CenterEmbedding` flag.
- **P8 JSON:** `from_str::<Value>(document(r))` has `schema_version == 1`, one entry per report in order, and `metrics.*` equal to the `FileSummary` fields (`null` ⇔ `None`); `diagnostics[i].{line, column, end_line, end_column} == d_i.{start, end}`.
- **P9 table:** `render(p, s)` = M1's line, then one row per metric in the fixed order. Each value is `s`'s field formatted as in today's lines, or `absent (…)` ⇔ `None`.
- **P10 criterion 6 (every format):** ∀ generated documents with `max_sentence_len = 0` (so every sentence is flagged): the `(L, C)` sequences parsed from text (`-->`), compact (`p:L:C:`) and JSON (`line`/`column`) are equal. Each `(L, C)` is `LineIndex::position(s.source_range.start)`, and the source character there is the first character of `s.text()`.
  - The generator includes multi-byte words before sentence starts (`Café`, `漢字`), inline markup at sentence starts (`**The**`), list items, blockquotes and CRLF.
- **P11 example:**
  - `docs/examples/github-actions.yml` has a step whose `run` invokes `vernier check`;
  - no step sets `continue-on-error`;
  - no `run` line holds `|| true`, `|| exit 0` or `set +e`.
- **Style law:** `Style::for_output(tty, no_color) == Color ⇔ tty ∧ no_color.is_none_or(OsStr::is_empty)`.

## Witnesses

- `long.md` (M2 fixture, 30-word sentence at 3:20):
  - compact: `{path}:3:20: CognitiveOverload: LongSentence: sentence has 30 words (max 25)`;
  - text: header as P4, ` --> {path}:3:20`, the whole line shown (no `...`), `= metrics:` with `- words: 30 (LongSentence, max 25)` and `- mean dependency distance: absent (no parse)`.
- `wrapped.md` (new): `Short opener. This sentence is\nhard-wrapped over three lines\nof the file until it ends.\n`, with `--max-sentence-len 5` (the opener's 2 words pass, the second sentence's 14 do not; `hard-wrapped` is two UAX #29 words). The text diagnostic points at 1:15, and the underline starts under column 15 of line 1 and closes on line 3 (`|___…^`).
- Two flags: the hand-built sentence of main's current test (`LongSentence` + `DeepTree { depth: 6 }`) gives one diagnostic with `flags.len() == 2`, and compact joins them with `; `.
- Parsed example (`EXAMPLE_CONLLU`, `max_mdd` 2.5), metric lines:
  - `- mean dependency distance: 2.67 (HighMdd, max 2.50)`
  - `- tree depth: 4 edges (max 5)`
  - `- subordinate clauses: 1 (max 2)`
  - `- center-embedding: subject "proposal" separated from verb "caused" by 8 words (CenterEmbedding)`
- JSON on `long.md` + `sample.md`: one document; `files[0].diagnostics[0]` at line 3, column 20; `files[1].diagnostics == []`; `files[1].metrics.mean_dependency_distance == null`.

## Scenario coverage

| Spec criterion (M5) | Check |
| :--- | :--- |
| 1 one `warning[CognitiveOverload]` per flagged sentence, first char, underline, metric list | P1, P4, P6, P7; `wrapped.md` and `long.md` text witnesses (T6) |
| 2 exit 0 / 1 / 2 | existing `check_on_sample_exits_0`, `check_flags_a_long_sentence_and_exits_1` (compact), `an_unreadable_file_wins_over_a_flag`; T9 JSON exit codes. The model half comes with M3b |
| 3 JSON `schema_version` 1, per-file metrics, positions equal to text | P8, P10, `json_is_one_document_for_all_files` (T9) |
| 4 compact `path:line:col: code: message` | P3; compact witness (T3) |
| 5 `analyze` table, exit 0 | P9; `analyze_prints_a_metrics_table_and_exits_0_on_a_flagged_file` (T7) |
| 6 positions point at the first char (property) | P10 (T10) |
| 7 GitHub Actions example fails on exit 1 | P11 (T11) |
| Implementation detail: survives `NO_COLOR` / non-TTY | P5, style law, `check_writes_no_escape_codes_off_a_terminal` (T6) |

## Planted violations (tick when the red was seen)

Each names the input where the mutant differs (audit 005).

| # | Plant | Must fail (input) | Seen |
| :--- | :--- | :--- | :--- |
| 1 | one diagnostic per flag instead of per sentence | P1; two-flag witness (2 ≠ 1) | [x] (witness red; P1 stays green — its generated documents raise at most one flag, `LongSentence`, per sentence, so only the witness reaches the mutant) |
| 2 | `end = start` | P2 on any sentence of ≥ 2 chars (`One two.`: 1:9 ≠ 1:1) | [x] |
| 3 | compact code = the first flag's name | compact witness on `long.md` (`LongSentence` ≠ `CognitiveOverload`) | [x] (CLI witness, P3 and the two-flag witness red) |
| 4 | compact joins flags with `, ` | two-flag witness | [x] |
| 5 | `Style` ignores `NO_COLOR` | style law at `(true, Some("1"))` | [x] |
| 6 | `Style` treats an empty `NO_COLOR` as set | style law at `(true, Some(""))` | [x] |
| 7 | `render_text` always uses `Renderer::styled()` | P5 (`Plain` has `\x1b`) on any diagnostic | [x] |
| 8 | the annotation span ends at the end of the sentence's first line | `wrapped.md` witness (no closing marker on line 3) | [x] |
| 9 | default `term_width` (140) | `long.md` text witness (the line is shown with `...`) | [x] (unit copy `a_long_line_is_shown_whole`; P6 red too) |
| 10 | `words` line uses `>=` | P7 with `words == max` (the generator draws `max = words` half the time) | [x] (red 2/2) |
| 11 | syntactic lines print `0` when `syntax = None` | P7 on any unparsed sentence | [x] (P7 and the `long.md` footer witness red) |
| 12 | `main` passes `Style::Color` unconditionally | `check_writes_no_escape_codes_off_a_terminal` | [x] |
| 13 | the table drops the `passive voice` row | P9; `sample.md` table witness | [x] (P9 and `analyze_prints_a_metrics_table_and_exits_0_on_a_flagged_file` red) |
| 14 | JSON `line` 0-based | P8; JSON witness (2 ≠ 3) | [x] |
| 15 | `schema_version: 2` | P8 | [x] (the fixture witness is red; P8 compares with the constant itself, so the literal 1 of the witness is what catches it) |
| 16 | JSON printed once per file | `json_is_one_document_for_all_files` (two files: `from_str` fails, "trailing characters") | [x] |
| 17 | `analyze --format json` exits 1 on a flagged file | T9 exit-code test on `long.md` | [x] |
| 18 | compact column counts bytes; *then* P10's generator without multi-byte text | P10 red with multi-byte text before a sentence (`Café ok. word …`), green without it (audit 005: the generator must reach it) | [x] (planted in `LineIndex::position`, the only source of `Diagnostic.start`, since `render_compact` gets no source: P10 red 3/3 with the multi-byte generator, green 3/3 with ASCII-only words) |
| 19 | the example's `run` gains `\|\| true`; separately `continue-on-error: true` | P11 (each) | [x] (both red) |

## Coverage gap (run by hand)

- Colour on a real terminal: automated runs are pipes, so `Style::Color` is covered only by P5 and the style law. By hand: `cargo run -- check tests/fixtures/long.md` in a terminal (coloured), then with `NO_COLOR=1` (plain, same text).
- The GitHub Actions example is checked structurally only. It runs for real only in a GitHub repository with vernier installable. By hand: copy it into a test repository and see a red job on a long sentence.
- Syntactic lines in real output: no CLI path parses until M3b. They are unit-tested from pasted parses; M3b's integration test renders the example through a real model.

## Snippets

```rust
// src/diagnostic.rs
pub const CODE: &str = "CognitiveOverload";
pub const TITLE: &str = "Sentence exceeds human working-memory capacity";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlagMessage { pub name: &'static str, pub message: String }   // Display: "{name}: {message}" == today's describe()

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub start: Position, pub end: Position, pub source_range: Range<usize>,
    pub flags: Vec<FlagMessage>,      // surface first, then syntactic
    pub metrics: Vec<String>,         // "words: 30 (LongSentence, max 25)", … (T5)
}
pub fn flag_messages(sentence: &SentenceAnalysis, t: &Thresholds) -> Vec<FlagMessage>;          // T1/T2
pub fn metric_lines(sentence: &SentenceAnalysis, t: &Thresholds) -> Vec<String>;                // T5
pub fn diagnostics(source: &str, file: &FileAnalysis, t: &Thresholds) -> Result<Vec<Diagnostic>, PositionError>;
pub fn render_compact(path: &str, d: &Diagnostic) -> String;
//   format!("{path}:{}:{}: {CODE}: {}", d.start.line(), d.start.column(), d.flags.iter().map(ToString::to_string).collect::<Vec<_>>().join("; "))

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style { Plain, Color }
impl Style { pub fn for_output(is_terminal: bool, no_color: Option<&OsStr>) -> Self }   // Color iff is_terminal && no_color.is_none_or(OsStr::is_empty)
pub fn render_text(path: &str, source: &str, d: &Diagnostic, style: Style) -> String;
//   Level::WARNING.primary_title(TITLE).id(CODE)
//     .element(Snippet::source(source).path(path).annotation(AnnotationKind::Primary.span(d.source_range.clone())))
//     .element(Level::NOTE.no_name().message(format!("metrics:\n{}", d.metrics.iter().map(|m| format!("- {m}")).join("\n"))))
//   match style { Plain => Renderer::plain(), Color => Renderer::styled() }.term_width(100_000).render(&[group])

// metric lines (T5), absent without a parse:
//   "words: {n} ({LongSentence, }max {m})"
//   "mean dependency distance: {x:.2} ({HighMdd, }max {m:.2})" | "mean dependency distance: absent (fewer than 2 content tokens)" | "…: absent (no parse)"
//   "tree depth: {d} edges ({DeepTree, }max {m})" | "tree depth: absent (no parse)"
//   "subordinate clauses: {c} ({ClauseOverload, }max {m})" | "subordinate clauses: absent (no parse)"
//   "center-embedding: none" | one "center-embedding: subject \"{s}\" separated from verb \"{v}\" by {n} words (CenterEmbedding)" per embedding | "center-embedding: absent (no parse)"

// src/json.rs
pub struct FileReport<'a> { pub path: String, pub summary: &'a FileSummary, pub diagnostics: &'a [Diagnostic] }
pub fn document(files: &[FileReport<'_>]) -> Result<String, serde_json::Error>;   // to_string_pretty
#[derive(Serialize)] struct Document<'a> { schema_version: u32 /* 1 */, files: Vec<FileJson<'a>> }
#[derive(Serialize)] struct FileJson<'a> { path: &'a str, metrics: MetricsJson, diagnostics: Vec<DiagnosticJson<'a>> }
#[derive(Serialize)] struct MetricsJson {
    prose_spans: usize, words: usize, sentences: usize, syllables: usize, complex_words: usize,
    flesch_reading_ease: Option<f64>, flesch_kincaid_grade: Option<f64>, gunning_fog: Option<f64>, average_sentence_length: Option<f64>,
    mean_dependency_distance: Option<f64>, nominalizations: usize, nominalization_ratio: Option<f64>, passives: Option<usize>,
}
#[derive(Serialize)] struct DiagnosticJson<'a> {
    code: &'static str, severity: &'static str /* "warning" */, message: &'static str /* TITLE */,
    line: usize, column: usize, end_line: usize, end_column: usize, flags: Vec<FlagJson<'a>>,
}
#[derive(Serialize)] struct FlagJson<'a> { name: &'static str, message: &'a str }

// src/summary.rs
pub struct FileSummary { .., pub mean_dependency_distance: Option<f64>, pub passives: Option<usize> }   // None from summarize()
pub fn render(path: &Path, s: &FileSummary) -> String;
//   "{path}: {spans} prose spans, {words} words"   (M1, unchanged)
//   "  metric                    value"
//   rows, names padded to the longest: sentences, syllables, complex words, Flesch Reading Ease, Flesch-Kincaid Grade,
//   Gunning Fog, average sentence length ({:.2} or "absent (no sentences)"), mean dependency distance ({:.2} or
//   "absent (no parse)"), nominalization ratio ("{r:.3} ({n} of {w} words)" or "absent (no words)"),
//   passive voice ("{n}" or "absent (no parse)")

// src/main.rs
//   style = Style::for_output(std::io::stdout().is_terminal(), std::env::var_os("NO_COLOR").as_deref())
//   check:   text → render_text per diagnostic; compact → render_compact per diagnostic; json → collect
//   analyze: text | compact → summary::render; json → collect
//   json: after all files, println!(document(&reports)?); on Err → eprintln + exit 2
```

## Tasks

One task = one scenario = one commit (`git -c commit.gpgsign=false commit`, D13; the message states the problem and ends `— Spec 001 M5 Tn (full-spec)`); `just verify` must pass at every commit (the pre-commit hook runs it). Plan ticks go in the task's commit when it stays ≤ 5 files, else in a separate commit.

| ID | Scenario | Files | RED (must fail first) | GREEN | Status |
| :--- | :--- | :--- | :--- | :--- | :--- |
| T0 | spike, spec clarification, plan | `research/m5-rendering-spike.md`, `spec.md`, this plan | — (docs) | criteria clarified in place | done |
| T1 | refactor: messages into the core | `src/diagnostic.rs`, `src/lib.rs`, `src/main.rs` | — (refactor). Evidence: `describes_each_syntactic_flag_with_its_value` and `diagnostics_list_surface_then_syntactic_flags_per_sentence` move with unchanged assertions; all `tests/cli.rs` `check` tests green before and after | `FlagMessage`, `flag_messages`; `main` prints `{path}:{l}:{c}: {flag}` from them | done |
| T2 | diagnostic model | `src/diagnostic.rs`, `src/main.rs` | P1, P2, two-flag witness against `diagnostics → vec![]`; plants 1, 2 | `Diagnostic`, `diagnostics`; `main`'s per-flag lines are built from it (output unchanged) | done |
| T3 | compact format | `src/diagnostic.rs`, `src/main.rs`, `tests/cli.rs` | P3 and `check_compact_prints_one_line_per_flagged_sentence` (long.md) against `render_compact → String::new()`; plants 3, 4 | `render_compact`; `--format compact` for `check` | done |
| T4 | text renderer | `Cargo.toml`, `Cargo.lock`, `src/diagnostic.rs` | P4, P5, P6, style law against `render_text → String::new()` and `for_output → Plain`; plants 5, 6, 7, 9 (on a unit copy of `long.md`) | `annotate-snippets` 0.12, `Style`, `render_text` (no footer yet) | done |
| T5 | metric lines | `src/diagnostic.rs` | P7 and the parsed-example witness against `metric_lines → vec![]`; plants 10, 11 | `metric_lines`, `= metrics:` footer | done |
| T6 | `check` defaults to the annotated text | `src/main.rs`, `tests/cli.rs`, `tests/fixtures/wrapped.md` | `check_prints_a_cognitive_overload_diagnostic` (long.md), `check_underlines_a_hard_wrapped_sentence` (wrapped.md), `check_writes_no_escape_codes_off_a_terminal` (with and without `NO_COLOR`); plants 8, 12. **Changed tests:** `check_flags_a_long_sentence_and_exits_1`, `max_sentence_len_raises_the_bar` and `check_flags_the_samples_13_word_sentence_over_a_limit_of_10` pin today's one-line output. They gain `--format compact`, and `CognitiveOverload: ` is inserted in their expected lines: M5 makes the annotated diagnostic the default, and their laws (position, threshold boundary, exit code) are kept unchanged in compact | text as the default for `check` | done |
| T7 | `analyze` table | `src/summary.rs`, `tests/cli.rs` | P9, `analyze_prints_a_metrics_table_and_exits_0_on_a_flagged_file`; plant 13. **Changed tests (layout only):** `analyze_prints_surface_metrics` (reads rows instead of lines 2–4), `analyze_prints_the_nominalization_ratio` (the row instead of line 4), and summary's `renders_*`/`empty_file_has_no_nominalization_ratio` (rows instead of lines; values unchanged). `FileSummary` literals gain the two new fields | table `render`; `FileSummary.{mean_dependency_distance, passives}` | done |
| T8 | JSON document | `Cargo.toml`, `Cargo.lock`, `src/json.rs`, `src/lib.rs` | P8 and the schema witness against `document → Ok("{}")`; plants 14, 15 | `serde` + `serde_json`, output types, `document` | done (tests read floats back with `serde_json`'s `float_roundtrip`, a dev-only feature: the default parser missed a written `121.22000000000003` by one ulp) |
| T9 | `--format json` for both commands | `src/main.rs`, `tests/cli.rs` | `json_is_one_document_for_all_files` (long.md + sample.md), `json_exit_codes_follow_the_command`; plants 16, 17 | `run` collects reports and prints one document | done (main's `diagnostics_list_surface_then_syntactic_flags_per_sentence` goes with the per-flag lines it tested, which no format prints any more; its law — surface then syntactic flags per sentence, at the sentence's start — is `diagnostic::one_diagnostic_per_flagged_sentence_with_all_its_flags`) |
| T10 | positions in every format (criterion 6) | `tests/positions.rs` | P10 against a planted byte-column compact (plant 18); the property itself adds no production code, so its RED is plant 18 seen red, then reverted | — (property only) | done |
| T11 | GitHub Actions example | `docs/examples/github-actions.yml`, `tests/examples.rs` | P11 against a missing file; plant 19 | the example workflow | done (install step: `cargo install --locked --git https://github.com/OWNER/vernier vernier` with an `OWNER` placeholder and a comment; where vernier is installed from is the user's open question) |
| T12 | close-out (main) | `spec.md` (ticks, Status), `docs/HANDOVER.md`, `README.md` (formats, example), this plan (ticks) | `just verify`; every plant seen red; the by-hand runs | — | done (by-hand colour and GitHub runs left to the user, see HANDOVER) |
