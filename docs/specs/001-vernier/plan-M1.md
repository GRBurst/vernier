# Plan — spec 001 M1: CLI skeleton and prose extractor

Tier: `full-spec`. Spec: [spec.md#M1](spec.md#M1). Gate record: [gates.md](gates.md) (phase 2 skipped by user direction; fallbacks in force).
No plan template or location exists in `D1`; this file sets one (`docs/specs/<NNN>-<slug>/plan-<milestone>.md`) and the handover records it.

## Motivation

M1 turns Markdown into *prose spans* — text from paragraphs, list items and blockquotes, each with its byte range — and maps any byte offset to a 1-based line and Unicode-scalar column.
Every later metric (M2–M4) counts words in these spans, and every diagnostic (M5) points through the converter, so both must be provably faithful to the source.
The CLI already accepts all M5 flags so later milestones add behaviour, not interface.

## Approach

- Pure core in a library crate (`src/lib.rs`): `position`, `words`, `prose`, `summary`, `cli` (types only). `src/main.rs` is the shell: read files, call the core, print, map to an exit code.
- Extractor: one pass over `pulldown_cmark::Parser::new_ext(src, opts).into_offset_iter()`, with a stack of `Role`s (push on `Start`, pop on `End`). A `Text(t)` at range `r` is kept iff the stack holds a `Prose` and no `Dropped` role **and** `t == src[r]`.
- `ProseSpan<'a>` borrows `&src[r]`, so *slice = text* holds by construction; the property test guards the construction.
- Converter: `LineIndex` of line starts (after every `\n`); `line = partition_point(start ≤ off)`, `column = chars(src[line_start..off]) + 1`.

## Decisions made

| Decision | Rationale | Rejected alternative |
| :--- | :--- | :--- |
| Drop a `Text` event whose decoded text ≠ its source slice | Only character references (`&amp;`, `&#169;`) do this (spike, `.sdd/spike`); keeping the slice would count `amp` as a word, keeping the decoded text breaks the slice criterion | decoded text with range (breaks the criterion); raw slice (fake words) |
| Headings and image alt text dropped | spec fallback | kept |
| `.md` / `.markdown` compared ASCII-case-insensitively (`README.MD` is Markdown) | the extension names a format, not a spelling | case-sensitive |
| Only `\n` ends a line; `\r` is an ordinary column character | "as rustc does"; CRLF then puts `\r` at the last column of its line | treating lone `\r` as a line end |
| File word count = Σ words per span | M1 prints a count only; sentence assembly across spans is M2 | joining spans (M2's job) |
| `check` in M1 reads files, flags nothing, exits 0 (2 on unreadable) | consistent with M5's exit-code criterion while no rule exists | refusing `check` until M5 |
| Several files: every file is processed; exit 2 if any is unreadable | one bad path does not hide the others' results | stop at first error |
| Non-UTF-8 content counts as "cannot be read" | the core takes `&str` | lossy decoding (silent) |
| `--max-mdd` must be finite and > 0, rejected by clap (exit 2) | a NaN threshold would silently never fire | accept any `f64` |
| No `anyhow` yet | `main` has no error chain in M1 (guide §7: add when needed) | add now |

## Gate (CONSTITUTION + ENGINEERING)

| Principle | Result |
| :--- | :--- |
| A2 specs are contracts | PASS — no criterion edited; fallbacks applied as written |
| A3 ≤ 5 files per task | PASS — see task table |
| A4 properties over constants | PASS — P1–P6 below; literals only for flag defaults and exit codes, which *are* the criteria |
| A6 seams | PASS — `SourceFormat`, `OutputFormat`, threshold flags exist; no speculative logic |
| B4 hermetic | PASS — deps fetched into project-local `CARGO_HOME` (audit 002) |
| D9 complexity ≤ 10, no `unsafe` | PASS — extractor split into `role`, `keeps`, `extract_prose` |
| D10 §5 newtypes | PASS — `Position` (private fields, ≥ 1 by construction), `ProseSpan` (slice by construction) |
| D10 §6 pure core | PASS — only `main.rs` touches files, stdout, stderr, exit code |
| D10 §7 outcomes | PASS — `PositionError { OutOfBounds, NotCharBoundary }`; `model_path: Option<PathBuf>` (absence is normal) |
| D10 §8 banned constructs | PASS — no `unwrap`/`expect` outside tests |
| Phase 2 human gates | VIOLATION — skipped by user direction; recorded in gates.md and BACKLOG item 2 |

## Files to read first

`docs/ENGINEERING.md`, `Cargo.toml`, `clippy.toml`, `src/main.rs`, `docs/audits/*`, this plan.

## Type checking strategy

`rustc` + `clippy -D warnings` (`just lint`) after every GREEN. Expected rejections: using `Position` fields directly outside `position`; constructing `ProseSpan` with foreign text; a missing `SourceFormat`/`OutputFormat` arm.

## Testing strategy

| Layer | Covers | Needs |
| :--- | :--- | :--- |
| Unit (in-module `#[cfg(test)]`) | construct-by-construct witnesses; laws P1–P6 via `proptest` | `proptest` dev-dep |
| Integration (`tests/cli.rs`) | exit codes, stderr naming the file, stdout summary, flag acceptance | `env!("CARGO_BIN_EXE_vernier")`, committed `tests/fixtures/`, `CARGO_TARGET_TMPDIR` for a missing path |
| End-to-end by hand | `cargo run -- analyze README.md` | — |

## Properties

- **P1** ∀ generated Markdown `m`, ∀ `s ∈ extract_prose(m)`: `&m[s.range()] == s.text()`.
- **P2** spans are strictly ordered and disjoint: `sᵢ.range().end ≤ sᵢ₊₁.range().start`.
- **P3** a sentinel word placed only in dropped constructs (code, heading, HTML, math, table, frontmatter, image alt, autolink, URL) never appears in any span; every word placed in kept constructs does.
- **P4** ∀ `src`, ∀ char-boundary `off ≤ len`: `LineIndex::new(src).position(off) == scan(src, off)` (oracle: walk chars, `\n` → line+1, col=1, else col+1). Alphabet includes `é 漢 🦀 \r\n \n \r`.
- **P5** non-boundary offset → `Err(NotCharBoundary)`; `off > len` → `Err(OutOfBounds)`.
- **P6** `count_words(a + " " + b) == count_words(a) + count_words(b)` for word lists; punctuation/whitespace-only strings count 0; word count invariant under wrapping prose in `*…*`/`**…**`.

## Scenario coverage

| M1 criterion | Check |
| :--- | :--- |
| 1 only paragraph/list/quote text, with byte range | `prose::tests::keeps_paragraph_list_item_and_blockquote_text` + P3 |
| 2 drops code, HTML, frontmatter, math, tables, headings, alt | `prose::tests::drops_<construct>` (one per construct) + P3 |
| 3 link text kept, URL dropped, autolink contributes nothing | `prose::tests::keeps_link_text_but_not_its_url`, `…autolink_contributes_no_text` |
| 4 slice = span text (property) | P1 (`prose::tests::every_span_is_its_source_slice`) + P2 |
| 5 offset → position (property, multibyte, CRLF) | P4, P5 (`position::tests`) |
| 6 `analyze FILE` prints spans and words, exit 0 | `tests/cli.rs::analyze_prints_span_and_word_counts_per_file` |
| 7 unreadable file → stderr names it, exit 2 | `tests/cli.rs::unreadable_file_is_named_and_exits_2` (+ mixed files, + `check`) |
| 8 clap derive, all M5 flags accepted | `cli::tests::accepts_every_m5_flag_on_both_commands`, `…defaults_match_the_spec`, `tests/cli.rs::accepts_m5_flags` |
| impl: plain text = one span | `prose::tests::plain_text_is_one_span`, `…format_from_extension` |

## Planted violations (tick when the red was seen)

- [x] Remove `Heading` from `Dropped` → `drops_headings` FAIL. (P3 does not: its generator nests no heading inside a list item.)
- [x] Remove the `t == src[r]` filter → P3 and `a_character_reference_contributes_no_span` FAIL. (P1 holds by construction, since the span borrows the slice; P3 catches the stray word `amp`.)
- [x] `column = chars + 0` → P4 FAIL.
- [x] Treat `\r` as a line end in `LineIndex` → P4 FAIL on CRLF input.
- [x] Accept any `f64` for `--max-mdd` → `rejects_a_non_positive_or_non_finite_max_mdd` FAIL.
- [x] Exit 1 instead of 2 on an unreadable file → `unreadable_file_is_named_and_exits_2` FAIL.

## Coverage gap (run by hand)

- `CARGO_HOME` from `devenv.nix` cannot be evaluated inside the agent sandbox (nested devenv is denied): the user runs `devenv shell -- sh -c 'echo $CARGO_HOME'` once and expects `…/vernier/.devenv/state/cargo`.
- Rendering on a real terminal (colour, width) — no output styling yet, so nothing else.

## Snippets

```rust
// src/position.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position { line: usize, column: usize }          // both ≥ 1
impl Position { pub fn line(self) -> usize; pub fn column(self) -> usize; }
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PositionError {
    #[error("byte offset {offset} is past the end ({len} bytes)")] OutOfBounds { offset: usize, len: usize },
    #[error("byte offset {offset} is inside a character")]      NotCharBoundary { offset: usize },
}
pub struct LineIndex<'a> { source: &'a str, line_starts: Vec<usize> }
impl<'a> LineIndex<'a> {
    pub fn new(source: &'a str) -> Self;
    pub fn position(&self, offset: usize) -> Result<Position, PositionError>;
}

// src/prose.rs
pub enum SourceFormat { Markdown, PlainText }
impl SourceFormat { pub fn from_path(path: &Path) -> Self; }
pub struct ProseSpan<'a> { text: &'a str, range: Range<usize> }
impl<'a> ProseSpan<'a> { pub fn text(&self) -> &'a str; pub fn range(&self) -> Range<usize>; }
pub fn extract_prose(markdown: &str) -> Vec<ProseSpan<'_>>;
pub fn plain_text_prose(text: &str) -> Vec<ProseSpan<'_>>;   // exactly one span, 0..len
pub fn prose(source: &str, format: SourceFormat) -> Vec<ProseSpan<'_>>;
enum Role { Prose, Dropped, Neutral }
fn role(tag: &Tag) -> Role;   // Paragraph|Item → Prose; Heading|CodeBlock|HtmlBlock|Table|Image|MetadataBlock|Link{Autolink|Email} → Dropped
fn keeps(stack: &[Role]) -> bool { stack.iter().any(Prose) && !stack.iter().any(Dropped) }

// src/words.rs
pub fn count_words(text: &str) -> usize;   // UAX #29 segments with ≥ 1 alphanumeric char

// src/summary.rs
pub struct FileSummary { pub spans: usize, pub words: usize }
pub fn summarize(source: &str, format: SourceFormat) -> FileSummary;
pub fn render(path: &Path, s: &FileSummary) -> String;      // "{path}: {spans} prose spans, {words} words"

// src/cli.rs  (clap derive)
Cli { command: Command }   Command::{Analyze(Args), Check(Args)}
Args { files: Vec<PathBuf> (required), format: OutputFormat = text, max_sentence_len: usize = 25,
       max_mdd: f64 = 3.0 (parse_max_mdd: finite ∧ > 0), max_tree_depth: usize = 5, max_clauses: usize = 2,
       model_path: Option<PathBuf> }
OutputFormat::{Text, Json, Compact}

// src/main.rs
fn main() -> ExitCode { run(Cli::parse()) }   // 0 clean, 2 if any file unreadable
```

## Tasks

One task = one scenario = one commit (the user commits: GPG signing is denied in the sandbox). All tasks T1–T10 done 2026-09-26; `just verify` green.

| ID | Scenario | Files | RED (must fail first) | GREEN |
| :--- | :--- | :--- | :--- | :--- |
| T1 | offset → position | `Cargo.toml`, `src/lib.rs`, `src/position.rs` | P4/P5 against a stub returning line 1 col 1 | `LineIndex` |
| T2 | word count | `Cargo.toml`, `src/words.rs`, `src/lib.rs` | P6 against `0` | `unicode_words` filter |
| T3 | keep paragraph/item/quote text | `Cargo.toml`, `src/prose.rs`, `src/lib.rs` | witness test against empty `Vec` | role stack |
| T4 | drop each non-prose construct | `src/prose.rs` | `drops_*` one at a time | `role` arms |
| T5 | links, autolinks, entities; P1–P3 | `src/prose.rs` | autolink + P1 on entities | autolink arm, slice filter |
| T6 | plain text + format detection | `src/prose.rs` | `plain_text_is_one_span` | `SourceFormat` |
| T7 | summary + render | `src/summary.rs`, `src/lib.rs` | law: code block adds no words | `summarize` |
| T8 | CLI definition | `Cargo.toml`, `src/cli.rs`, `src/lib.rs` | `accepts_every_m5_flag…`, `rejects…max_mdd` | clap derive |
| T9 | shell + exit codes | `src/main.rs`, `tests/cli.rs`, `tests/fixtures/*.md` | integration tests against the old `main` | `run` |
| T10 | close-out | `docs/specs/001-vernier/spec.md` (ticks, Status), `docs/HANDOVER.md`, `README.md` | `just verify` | — |
