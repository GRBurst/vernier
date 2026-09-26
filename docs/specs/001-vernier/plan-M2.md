# Plan — spec 001 M2: Surface readability engine

Tier: `full-spec`. Spec: [spec.md#M2](spec.md#M2), definitions *Word*, *Block prose*, *Sentence*. Gate record: [gates.md](gates.md) (Clarification passed; Review, Approval, Comprehension still owed, BACKLOG item 2).
Evidence: [research/metrics.md](research/metrics.md) (syllable counter, UAX #29 tailoring), [audit 004](../../audits/004-block-prose-kept-line-breaks.md) (line breaks in block prose, fixed in the spec before this plan).

## Motivation

M2 turns prose into *sentences* and scores them: words, syllables and complex words per sentence, then Flesch Reading Ease (FRE), Flesch-Kincaid Grade (FKGL), Gunning Fog and average sentence length (ASL), per sentence and per file, with metrics *absent* when a text has no word or no sentence.
A sentence longer than `--max-sentence-len` words (default 25) is flagged `LongSentence`.
Sentences are the unit every later milestone parses (M3), counts (M4) and reports (M5), so their boundaries and source positions must be right here, with or without a parser.

## Approach

- Pipeline (pure core, one module per step): `prose` groups spans into blocks → `block` joins a block's spans into one string with an offset map back to the source → `sentence` splits a block with UAX #29 and the abbreviation merge list → `syllables` counts syllables and decides *complex* → `readability` turns counts into scores → `analysis` builds per-sentence and per-file results and flags → `summary` renders `analyze`; `main.rs` adds the `check` output and exit 1.
- `extract_prose` keeps its signature and behaviour: it becomes `extract_blocks(..).into_iter().flatten()`. The block boundary is any `Start`/`End` of a block-level tag, so tight list items, nested lists and blockquotes split correctly.
- The syllable counter is a hand-written port of the regex heuristic the research measured (SE answer 89312): a scan with leftmost-first alternation, no regex crate.
- `SurfaceCounts` is a commutative monoid (`+`, `Sum`, zero); a file's counts are the sum of its sentences' counts, so per-file metrics are formulas over sums, never averages of scores.

## Decisions made

| Decision | Rationale | Rejected alternative |
| :--- | :--- | :--- |
| Block boundary = every `Start`/`End` of a non-inline tag (inline: `Emphasis`, `Strong`, `Strikethrough`, `Link`, `Image`, `Superscript`, `Subscript`) | a tight list item has no `Paragraph`; a nested list must not glue its text to the parent item's | boundary only at `Paragraph` (merges tight items) |
| Block text maps `\n` and `\r` to `' '` in place | 1 byte → 1 byte, so the offset map stays a set of shifts; required by the spec after audit 004 | deleting them (breaks the map) |
| A separator space maps to the end of the preceding span | sentences are trimmed, so no sentence starts or ends on a separator; the end offset of a span-final sentence lands on the span's end | mapping to the next span's start |
| Sentence = UAX #29 segment, merged forward while it ends in an abbreviation, then trimmed of whitespace; a segment with no word is dropped | the spec's definition; trimming makes "first character" a visible character | keep leading whitespace |
| Abbreviation match: last whitespace-separated token of the segment, leading non-alphanumerics stripped (`(e.g.` → `e.g.`), compared **case-sensitively** | lowercase matching would merge after "…said no. Then …" | case-insensitive (the spike's choice) |
| Syllable counter ported by hand (9 alternatives, leftmost-first scan) instead of the `regex` crate | the pattern needs a lookahead (`ia(?!n$)`) that `regex` lacks; no new dependency (ENGINEERING §2), no `expect` on a lazy static | `regex` + post-filter + `#[allow(expect_used)]`; `fancy-regex` (backtracking, new dep) |
| Counter works on the lowercased word; vowels are ASCII `aeiouy` | exactly what was measured (93.8 %); non-ASCII letters count as consonants | Unicode-aware vowel classes (unmeasured) |
| Syllabic suffix test on the lowercased word: ends in `ing`; or `ed` preceded by `t`/`d`; or `es` preceded by `s`, `x`, `z`, `ch`, `sh`, `c`, `g` | the spec's "-es after `ce`/`ge`" is the stem `place`+`s` = `plac`+`es`: the letter before `es` is `c`/`g` | literal "`ce`+`es`" (never occurs) |
| *Capitalized* = the word's first character is uppercase; the first word of a sentence is never exempt | Gunning's proper-noun rule as the spec states it | also exempting all-caps acronyms (not in the spec) |
| `LongSentence` ⇔ `words > max_sentence_len` | "more than" in the criterion | `≥` |
| Scores are `f64`; rendering rounds to 2 decimals | the witnesses are checked on the unrounded value (1e-9) | rounding in the core |
| `analyze` keeps M1's first line per file and adds two indented lines (counts; scores or `metrics absent`) | M1's integration test derives its expectation from `render(summarize(..))` and stays unchanged | a table now (M5's job) |
| `check` in M2 prints `path:line:col: LongSentence: sentence has N words (max M)` per flagged sentence on stdout, exits 1 if any, 2 if any file unreadable (2 wins) | lands M2 runnable and already in M5's compact shape; M5 replaces the text rendering | keep `check` silent until M5 |
| The 500-word reference: `CMUdict ∩ top 20k` of hermitdave FrequencyWords `en_50k` (2018), `random.Random(2026)`, drawn exactly as `.sdd/research/syllable_spike.py` did | reproduces the measured 93.8 % sample; a match is any CMUdict pronunciation | uniform CMUdict sample (88.8 %, fails a 90 % bar) |
| `Thresholds { max_sentence_len }` struct, built in `main` from `Args` | M3a adds fields without changing signatures (A6 seam, one field now) | passing a bare `usize` |

## Gate (CONSTITUTION + ENGINEERING)

| Principle | Result |
| :--- | :--- |
| A2 specs are contracts | PASS — no criterion edited; the *Block prose* definition was corrected in place while `Draft` (D4), with audit 004 |
| A3 ≤ 5 files per task | PASS — see the task table (T3 touches exactly 5) |
| A4 properties over constants | PASS — P1–P12; literals only where they are the criterion (59.635, 9.91, the flag name, exit codes, the 90 % bar) |
| A6 seams | PASS — `Thresholds`, `Flag` enum, `Block` offset map reused by M3/M5; nothing speculative |
| B4 hermetic | PASS — no new dependency; Python only for the one-off, committed sample generator, run inside devenv |
| D9 complexity ≤ 10, no `unsafe` | PASS — each suffix/alternative is its own function; the scan loop is one `find_map` |
| D10 §5 newtypes | PASS — `Block` (text and map private, built only from spans), `Sentence` (built only by `sentences`) |
| D10 §6 pure core | PASS — only `main.rs` does I/O and exit codes |
| D10 §7 outcomes | PASS — `readability(..) -> Option<Readability>`: absence is the spec's domain state, not an error; `Position` errors in `main` map to exit 2 |
| D10 §8 banned constructs | PASS — no `unwrap`/`expect`/`panic!` outside tests; no regex lazy static |
| Engineering §2 new dependency | N/A — none added |
| Licenses | PASS — CMUdict BSD-2-style: notice committed next to the list; word selection credited to hermitdave FrequencyWords (CC BY-SA 4.0, only used as a filter, no list shipped) |
| Phase 2 human gates | VIOLATION — Review/Approval/Comprehension still owed (BACKLOG item 2); the user directed implementation to continue |

## Files to read first

`docs/ENGINEERING.md`, `docs/audits/*`, `Cargo.toml`, `src/lib.rs`, `src/prose.rs`, `src/words.rs`, `src/summary.rs`, `src/main.rs`, `tests/cli.rs`, `.sdd/research/syllable_spike.py`, `.sdd/uax29-spike/src/main.rs`, this plan.
Before any cargo command: `export CARGO_HOME=$DEVENV_STATE/cargo` (audit 002). Scratch goes to `.sdd/`, never `/tmp` (audit 001).

## Type checking strategy

`rustc` + `clippy -D warnings` (`just lint`) after every GREEN.
Expected rejections: building a `Block` or `Sentence` outside its module (private fields); reading `Readability` fields when the scores are absent (it is an `Option`); a missing `Flag` arm in a renderer.

## Testing strategy

| Layer | Covers | Needs |
| :--- | :--- | :--- |
| Unit (`#[cfg(test)]` per module) | witnesses per rule; laws P1–P12 via `proptest`; the 500-word agreement test | `proptest`; `include_str!` of the committed TSV |
| Integration (`tests/cli.rs`) | `analyze` metrics lines, `check` exit 1 / 0 / 2, `--max-sentence-len` effect | committed fixtures `tests/fixtures/long.md` (one 30-word sentence), existing `sample.md`, `plain.txt` |
| Measurement | agreement rate recorded in `measurements/syllables.md` | `cargo test syllables -- --nocapture` prints `agreement: k/500` |
| End-to-end by hand | `cargo run -- analyze README.md`, `check` on a real doc | — |

## Properties

- **P1** (words) `count_words(t) == words(t).count()` and every item of `words(t)` is a substring of `t` in order (refactor evidence for T1).
- **P2** (syllables) ∀ word `w` with a letter: `count_syllables(w) ≥ 1`; ∀ `w ∈ [0-9]+`: `count_syllables(w) == 1`; `count_syllables(w) == count_syllables(w.to_uppercase())` (case-invariant).
- **P3** (port fidelity) ∀ word in a committed table of 40 words, the Rust count equals the Python spike's `regex_counter` (table generated once by the script, pasted as a test constant).
- **P4** (complex) `is_complex(w, initial) ⇒ count_syllables(w) ≥ 3`; ∀ `w` with a capitalized first letter: `is_complex(w, false) == false`; ∀ lowercase `w`: `is_complex(w, true) == is_complex(w, false)`.
- **P5** (readability monotone) ∀ `words ≥ 1, sentences ≥ 1, syllables ≥ words`, `δ ≥ 1`: FRE(s + δ) < FRE(s) ∧ FKGL(s + δ) > FKGL(s); also FRE decreasing and FKGL increasing in `words/sentences` at fixed syllables/words.
- **P6** (absence) `readability(c).is_none() ⇔ c.words == 0 ∨ c.sentences == 0`.
- **P7** (monoid) `(a + b) + c == a + (b + c)`, `a + zero == a`, `a + b == b + a` for `SurfaceCounts`.
- **P8** (blocks) ∀ generated Markdown `m`: `extract_prose(m) == extract_blocks(m).flatten()` (M1 behaviour preserved) and every block is non-empty.
- **P9** (offset map) ∀ block `b`, ∀ offset `o` inside one of its spans: `source[b.source_offset(o)..].chars().next() == b.text()[o..].chars().next()` (after the `\r\n → ' '` mapping, compare with the mapped source char).
- **P10** (word conservation) ∀ block: `Σ words(sentences(b)) == count_words(b.text()) == Σ count_words(span)`.
- **P11** (sentence order) sentences of a block are non-empty, trimmed, strictly ordered and disjoint in both block and source offsets; each holds ≥ 1 word.
- **P12** (flag law) ∀ `n`, `max`: a sentence of `n` words carries `LongSentence` ⇔ `n > max`; a file's counts equal the sum of its sentences' counts.

## Scenario coverage

| M2 criterion | Check |
| :--- | :--- |
| 1 FRE, FKGL, Fog formulas | `readability::tests::fog_matches_the_formula` + the two witnesses below + P5 |
| 2 100 w / 5 s / 150 syl → FRE 59.635, FKGL 9.91 (1e-9) | `readability::tests::witness_100_words_5_sentences_150_syllables` |
| 3 more syllables → lower FRE, higher FKGL (property) | P5 (`readability::tests::more_syllables_lower_fre_higher_fkgl`) |
| 4 zero words or sentences → absent | P6 (`readability::tests::metrics_absent_iff_no_word_or_no_sentence`) + `analysis::tests::empty_file_has_absent_metrics` |
| 5 ≥ 1 for letters, exactly 1 for digits | P2 (`syllables::tests::at_least_one_for_letters`, `…digits_only_is_one`) |
| 6 ≥ 90 % on 500 CMUdict words, committed under `measurements/` | `syllables::tests::agrees_with_cmudict_on_the_reference_list` + `measurements/syllables.md` |
| 7 complex word rule (suffix, proper noun) | `syllables::tests::syllabic_suffix_is_subtracted` (`-ing`, `-ted`, `-ded`, each `-es` stem), `…non_syllabic_suffix_is_not` (`jumped`, `cakes`), `…capitalized_non_initial_is_not_complex` + P4 |
| 8 > `--max-sentence-len` words → `LongSentence` | P12 (`analysis::tests::long_sentence_iff_more_words_than_max`) + `tests/cli.rs::check_flags_a_long_sentence_and_exits_1`, `…max_sentence_len_raises_the_bar` |
| impl: UAX #29 boundaries | `sentence::tests::*` (spike cases: `Dr. Smith …` 2, `e.g. Serde` 1, `3.14` 1, hard-wrapped plain text 1, `Wait... Really?` 2) |
| def: block prose / plain-text blocks | `prose::tests::tight_list_items_are_separate_blocks`, `…nested_list_is_its_own_block`, `…plain_text_blank_lines_separate_blocks`, P8, P9 |
| def: abbreviation merge list | `sentence::tests::each_listed_abbreviation_merges` (∀ a ∈ list: `"We saw {a} Smith there. He left."` → 2 sentences), `…lowercase_no_does_not_merge` |

## Planted violations (tick when the red was seen)

- [ ] Drop the leftmost-first order (try `ia` before `.y[aeiou]`) → P3 FAIL. **Not observable** (planted, P3 stayed green): no two of the nine alternatives can match at the same position, so their order is an equivalent mutation (also none of 117,493 CMUdict words changes).
  - [x] Substitute: advance the scan by 1 instead of the match length (overlapping matches) → P3 FAIL on the witness table (`cronyism`, `dandyism`, `mccarthyism`, `shiyuan`).
- [x] Remove `max(1, …)` in `count_syllables` → P2 FAIL (`nth`, `gps` and digit strings have no vowel run).
- [x] Subtract the suffix for `jumped` (`-ed` after any letter) → `non_syllabic_suffix_is_not` FAIL.
- [x] `84.6` → `84.0` in FRE → witness FAIL.
- [x] Return `Some` with `NaN` for zero sentences → P6 FAIL (after P6's generator was biased to zero counts; before, only `zero_words_or_sentences_are_absent` caught it).
- [x] Block boundary only at `Paragraph` → `tight_list_items_are_separate_blocks` FAIL.
- [x] Skip the `\n → ' '` mapping → `sentence::tests::hard_wrapped_plain_text_is_one_sentence` FAIL.
- [x] Map a separator space to the next span's start and stop trimming → P9 or P11 FAIL.
- [x] Case-insensitive abbreviation match → `lowercase_no_does_not_merge` FAIL.
- [x] `>=` instead of `>` for `LongSentence` → P12 FAIL.
- [x] Corrupt 60 rows of the TSV (set count 9) → agreement test FAIL (rate < 90 %); revert.
- [x] `check` exits 0 on a flagged sentence → `check_flags_a_long_sentence_and_exits_1` FAIL.

## Coverage gap (run by hand)

- The 90 % bar is measured on frequent words only; technical vocabulary (e.g. `initiative`, `timeline`) is known to miss (research/metrics.md). A human reads `measurements/syllables.md`'s miss list once.
- UAX #29 on real documents (quotes, ellipses, `U.S.`): a human runs `vernier check` on one of their own Markdown files and skims the flagged sentence positions.
- Terminal rendering: plain text, no styling yet.

## Snippets

```rust
// src/words.rs  (T1: refactor)
pub fn words(text: &str) -> impl Iterator<Item = &str>;     // UAX #29 words with ≥ 1 alphanumeric char
pub fn count_words(text: &str) -> usize { words(text).count() }

// src/syllables.rs
pub fn count_syllables(word: &str) -> usize;               // max(1, vowel_runs − exceptions + additions), on lowercase chars
pub fn is_complex(word: &str, is_sentence_initial: bool) -> bool;
//   !(capitalized(word) && !is_sentence_initial) && count_syllables(word) − has_syllabic_suffix(word) as usize ≥ 3
fn vowel_runs(w: &[char]) -> usize;                        // maximal runs of [aeiouy]
fn exceptions(w: &[char]) -> usize;                        // 1 if w matches  [^aeiou]e[sd]?$  or  [^e]ely$, else 0
fn additions(w: &[char]) -> usize {                        // non-overlapping, leftmost-first, like Python re.findall
    let (mut i, mut n) = (0, 0);
    while i < w.len() {
        match ADDITIONS.iter().find_map(|alt| alt(w, i)) { Some(len) => { n += 1; i += len } None => i += 1 }
    }
    n
}
type Alternative = fn(&[char], usize) -> Option<usize>;   // match length at i, if the alternative matches there
const ADDITIONS: [Alternative; 9] = [
    lr_e_end,      // [^aeioulr][lr]e[sd]?$   (len 3 or 4, must reach the end)
    csgz_es_end,   // [csgz]es$
    td_ed_end,     // [td]ed$
    any_y_vowel,   // .y[aeiou]                (vowel here: aeiou, no y)
    ia_not_ian_end,// ia(?!n$)                 i.e. "ia" unless w[i+2]=='n' && i+3==len
    eo,            // eo
    ism_end,       // ism$
    consonant_ire_end, // [^aeiou]ire$
    ua_not_after_gq,   // [^gq]ua
];
fn has_syllabic_suffix(lower: &str) -> bool;              // ing | [td]ed | (s|x|z|ch|sh|c|g)es

// src/readability.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SurfaceCounts { pub words: usize, pub sentences: usize, pub syllables: usize, pub complex_words: usize }
impl Add for SurfaceCounts; impl Sum for SurfaceCounts;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Readability { pub flesch_reading_ease: f64, pub flesch_kincaid_grade: f64, pub gunning_fog: f64, pub average_sentence_length: f64 }
pub fn readability(counts: SurfaceCounts) -> Option<Readability>;
//   None if words == 0 || sentences == 0; asl = w/s, spw = syl/w, cpw = complex/w
//   FRE = 206.835 − 1.015·asl − 84.6·spw; FKGL = 0.39·asl + 11.8·spw − 15.59; Fog = 0.4·(asl + 100·cpw)

// src/prose.rs  (additions; extract_prose / plain_text_prose unchanged in behaviour)
pub fn extract_blocks(markdown: &str) -> Vec<Vec<ProseSpan<'_>>>;   // block id += 1 on every Start/End of a non-inline tag
pub fn plain_text_blocks(text: &str) -> Vec<Vec<ProseSpan<'_>>>;    // maximal runs of non-blank lines, one span each (no trailing '\n')
pub fn blocks(source: &str, format: SourceFormat) -> Vec<Vec<ProseSpan<'_>>>;
fn is_inline(tag: &Tag) -> bool;

// src/block.rs
pub struct Block { text: String, pieces: Vec<Piece> }      // text: spans joined by ' ', '\n' '\r' → ' '
struct Piece { block_start: usize, source_start: usize }   // sorted by block_start
impl Block {
    pub fn from_spans(spans: &[ProseSpan<'_>]) -> Block;
    pub fn text(&self) -> &str;
    pub fn source_offset(&self, block_offset: usize) -> usize;
    //   k = pieces.partition_point(|p| p.block_start <= o) − 1;  pieces[k].source_start + (o − pieces[k].block_start)
    //   (a separator at o == end of piece k maps to span k's end, by the same formula)
}

// src/sentence.rs
pub const ABBREVIATIONS: [&str; 13] = ["Mr.", "Mrs.", "Ms.", "Dr.", "Prof.", "St.", "e.g.", "i.e.", "etc.", "vs.", "Fig.", "No.", "cf."];
pub struct Sentence<'b> { text: &'b str, source_start: usize, source_end: usize }
impl<'b> Sentence<'b> { pub fn text(&self) -> &'b str; pub fn source_range(&self) -> Range<usize>; }
pub fn sentences(block: &Block) -> Vec<Sentence<'_>>;
//   carry = None; for (i, seg) in block.text().split_sentence_bound_indices():
//     start = carry.take().unwrap_or(i); end = i + seg.len();
//     if ends_with_abbreviation(&text[start..end]) { carry = Some(start) } else { emit(start, end) }
//   if let Some(start) = carry { emit(start, text.len()) }
//   emit: trim whitespace → [s, e); keep iff count_words > 0; source_start = map(s), source_end = map(e)
fn ends_with_abbreviation(segment: &str) -> bool;

// src/analysis.rs
pub struct Thresholds { pub max_sentence_len: usize }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag { LongSentence }
pub struct SentenceAnalysis { pub source_range: Range<usize>, pub counts: SurfaceCounts, pub readability: Option<Readability>, pub flags: Vec<Flag> }
pub struct FileAnalysis { pub spans: usize, pub sentences: Vec<SentenceAnalysis>, pub totals: SurfaceCounts, pub readability: Option<Readability> }
pub fn analyze(source: &str, format: SourceFormat, thresholds: &Thresholds) -> FileAnalysis;
fn sentence_counts(sentence: &Sentence) -> SurfaceCounts;   // words; syllables Σ; complex: is_complex(w, index == 0); sentences = 1

// src/summary.rs
pub struct FileSummary { pub spans: usize, pub words: usize, pub counts: SurfaceCounts, pub readability: Option<Readability> }
pub fn summarize(source: &str, format: SourceFormat) -> FileSummary;   // spans/words as in M1; counts = Σ sentence counts (reuse analysis' per-sentence counting, no thresholds)
pub fn render(path: &Path, summary: &FileSummary) -> String;
//   "{path}: {spans} prose spans, {words} words\n  {sentences} sentences, {syllables} syllables, {complex} complex words\n  "
//   + "Flesch Reading Ease {:.2}, Flesch-Kincaid Grade {:.2}, Gunning Fog {:.2}, average sentence length {:.2}"
//   | "metrics absent (no sentences)"

// src/main.rs (check)
//   for s in analysis.sentences where !s.flags.is_empty():
//     println!("{path}:{line}:{col}: LongSentence: sentence has {w} words (max {max})")   // position of s.source_range.start
//   exit: 2 if any file unreadable; else 1 if any flag; else 0
```

Reference-list generator (`docs/specs/001-vernier/measurements/sample_cmudict.py`, stdlib only, run once from `.sdd/research/`): reuse the spike's loading code verbatim; `rng = random.Random(2026); rng.sample(allwords, 500); sample = rng.sample(top20k, 500)` (the first call is kept so the draw matches the measured sample); write `cmudict-500.tsv` as `word<TAB>counts` (counts comma-joined, ascending), sorted by word, preceded by `#` comment lines naming the source and the license file.
It also prints the Python `regex_counter` value for the first 40 words, which become P3's table.

## Tasks

One task = one scenario = one commit (`git -c commit.gpgsign=false commit`, D13); `just verify` must pass at every commit (the pre-commit hook runs it).
All tasks T0–T12 done 2026-09-27 (91 tests); `just verify` green. Planted violations #1 and #5 did not fail as predicted: [audit 005](../../audits/005-planted-violations-predicted-unchecked.md).

| ID | Scenario | Files | RED (must fail first) | GREEN | Status |
| :--- | :--- | :--- | :--- | :--- | :--- |
| T0 | spec fix for plain-text line breaks | `spec.md`, `docs/audits/004-…`, this plan | — (docs) | definition corrected | done |
| T1 | `words()` iterator (refactor) | `src/words.rs` | P1 against a `words` stub yielding nothing | `count_words = words().count()`; existing P6 still green (before/after evidence) | done |
| T2 | syllable counter | `src/syllables.rs`, `src/lib.rs` | P2, P3 against `fn count_syllables(_) -> usize { 0 }` | port | done |
| T3 | 500-word reference + measurement | `measurements/{sample_cmudict.py, cmudict-500.tsv, CMUDICT-LICENSE, syllables.md}`, `src/syllables.rs` (test) | agreement test against a TSV with 60 rows corrupted (planted), then real TSV | run generator, record rate and misses | done |
| T4 | complex word | `src/syllables.rs` | suffix/proper-noun witnesses + P4 against `is_complex = count ≥ 3` | `has_syllabic_suffix`, capitalization | done |
| T5 | readability formulas + absence | `src/readability.rs`, `src/lib.rs` | witness, P5, P6, P7 against a stub | formulas, `Add`, `Sum` | done |
| T6 | blocks from prose | `src/prose.rs` | `tight_list_items_are_separate_blocks`, `plain_text_blank_lines_separate_blocks`, P8 | `extract_blocks`, `plain_text_blocks`, `blocks`; `extract_prose` = flatten | done |
| T7 | block join + offset map | `src/block.rs`, `src/lib.rs` | P9 against identity map | `Block` | done |
| T8 | sentence splitting | `src/sentence.rs`, `src/lib.rs` | spike cases, abbreviation laws, P10, P11 against one-sentence-per-block | UAX #29 + merge + trim | done |
| T9 | analysis + `LongSentence` | `src/analysis.rs`, `src/lib.rs` | P12, `empty_file_has_absent_metrics` | `analyze` | done |
| T10 | `analyze` renders metrics | `src/summary.rs`, `src/main.rs`, `tests/cli.rs` | `analyze_prints_surface_metrics` (expectation from the library); M1's test unchanged | `render` lines 2–3 | done |
| T11 | `check` flags and exits 1 | `src/main.rs`, `tests/cli.rs`, `tests/fixtures/long.md` | `check_flags_a_long_sentence_and_exits_1`, `…max_sentence_len_raises_the_bar`, `check_on_sample_exits_0` | `check` loop | done |
| T12 | close-out | `spec.md` (ticks, Status `IMPLEMENTED`), `docs/HANDOVER.md`, `README.md`, this plan (ticks) | `just verify`; planted violations seen | — | done |

## Follow-up F1 — block prose joins across inline markup

### Motivation
The spec's *Block prose* definition (corrected in place, [audit 006](../../audits/006-block-prose-spaces-inside-markup.md)) joins two consecutive spans directly when only inline emphasis, strong, strikethrough, superscript, subscript or link delimiters lie between them, and with one space otherwise.
`Block::from_spans` always inserted a space, so `**The proposal**, which` became `The proposal , which` and `un*believ*able` became three words; M3b would hand that text to the parser.

### Approach
`extract_blocks` already walks the event stream; it records, per kept span, whether the previous kept span of the same block is reachable through transparent events only.
`ProseSpan` carries that bit (private field, set only in `prose.rs`); `Block::from_spans` writes no separator before a span whose bit is set.
Plain-text spans never join (their bit is `false`), so plain text is unchanged.

### Decisions made
| Choice | Rationale | Rejected |
| :--- | :--- | :--- |
| Decide the join from the event stream | the event stream is the only place that knows what separated two `Text` events | source-gap test "gap consists of `*`, `_`, `[`, `](url)`" — re-implements CommonMark and misreads `a\*b`, link URLs, `~` |
| Bit on `ProseSpan`, not a separate `Vec<bool>` | the join belongs to the span; one vector keeps `blocks()`' signature | parallel vector — can drift out of step with the spans |
| Transparent set = Start/End of `Emphasis`, `Strong`, `Strikethrough`, `Superscript`, `Subscript`, `Link` | exactly the inline tags whose text is kept and whose delimiters render as nothing | including `Image` — its alt text is dropped, so the reader sees a picture, not a joined word |
| Any other event between spans (dropped `Text`, `Code`, `SoftBreak`, `HardBreak`, `InlineHtml`, `InlineMath`, `Start/End(Image)`, …) forces a space | conservative: a word boundary where something non-textual was | joining across a character reference (`AT&amp;T` → `ATT`) — the decoded `&` cannot be kept (the span must borrow the slice), and `ATT` is a word the reader never sees |

### Gate
| Principle | Result |
| :--- | :--- |
| A2 specs are contracts | PASS — definition corrected in place while `Draft` (D4), audit 006; one existing test changes (F1.2, justified there) |
| A3 ≤ 5 files per task | PASS — F1.1: 2 files, F1.2: 3 files |
| A4 properties over constants | PASS — P13 markup invariance; literals only for the boundary-kind witnesses |
| D9 complexity ≤ 10, no `unsafe` | PASS — the transparency test is its own `fn`; the loop gains one branch |
| D10 §5 newtypes | PASS — the bit is private, set only by `extract_blocks` |
| D10 §6/§7/§8 | PASS — pure core, no new outcome, no banned construct |
| Engineering §2 new dependency | N/A — none added |
| Phase 2 human gates | VIOLATION — still owed (BACKLOG item 2), as for M2 |

### Files to read first
`docs/ENGINEERING.md`, `docs/audits/005-…` and `006-…`, `src/prose.rs`, `src/block.rs`, `src/analysis.rs` (test `parses_the_prose_not_the_markup`), `src/testing.rs`, this section.
`export CARGO_HOME=$DEVENV_STATE/cargo` before cargo (audit 002); scratch in `.sdd/` (audit 001). `.sdd/join/` has an `ev` bin that prints pulldown events for an argument string (options may differ from `prose::options()`).

### Type checking strategy
`just lint` after every GREEN. Expected rejections: constructing `ProseSpan` with the bit outside `prose.rs` (private field); a `match` on `Tag` in the transparency test missing no arm is not needed — use `matches!` with an explicit list so a new tag defaults to "not transparent".

### Testing strategy
| Layer | Covers | Needs |
| :--- | :--- | :--- |
| Unit `prose` | the join bit per boundary kind (witness table W1–W9) | — |
| Unit `block` | P13 markup invariance; P9's separator law restricted to real separators; intraword witness | `proptest` |
| Unit `analysis` | `parses_the_prose_not_the_markup` now sees `EXAMPLE_TEXT` | `testing::EXAMPLE_CONLLU` |
| Integration | none new: `tests/cli.rs` derives its expectations from the library, so it stays green unchanged | — |
| By hand | `cargo run -- analyze` on a file with bold words vs. the same file with the markup removed: equal counts | — |

### Properties and witnesses
- **P13 markup invariance:** ∀ words w₁…wₙ ∈ `[a-z]{1,8}`, ∀ wrapping mᵢ ∈ {none, `*w*`, `**w**`, `[w](u)`, `~~w~~` iff strikethrough is enabled in `options()`}, with punctuation p ∈ {none, `,`, `.`} after any word: `Block::from_spans(blocks(wrapped)[0]).text() == Block::from_spans(blocks(plain)[0]).text()`, and `analyze` gives the same `totals` for both. The expectation is the plain paragraph's own block, not a literal.
- **P9′:** every block character that is not an inserted separator maps to its source character (unchanged); inserted separators exist only before spans whose bit is `false`.
- Witnesses (`prose`, bit of each span after the first in the block):

| # | Markdown | Bits | Block text |
| :--- | :--- | :--- | :--- |
| W1 | `**The proposal**, which` | `[true]` | `The proposal, which` |
| W2 | `un*believ*able` | `[true, true]` | `unbelievable` |
| W3 | `a [b](u) c` | `[true, true]` | `a b c` |
| W4 | `a\*b` | `[true]` | `a*b` |
| W5 | `AT&amp;T` | `[false]` | `AT T` |
| W6 | `a  ⏎b` (hard break) | `[false]` | `a b` |
| W7 | `a⏎b` (soft break) | `[false]` | `a b` |
| W8 | ``a `x` b`` / `a <i>b</i> c` / `x ![alt](i) y` / `x ![](i) y` | all `false` | one space per dropped construct |
| W9 | `see <http://x.y> now` (autolink, text dropped) | `[false]` | `see   now` — only checked for the bit |

(W5–W9 texts: assert the bit, and for the texts derive "words equal" rather than exact spacing where more than one space is involved.)

W8's empty-alt image was added during F1.1: in `x ![alt](i) y` the dropped alt `Text` already clears the bit, so plant 3 is equivalent there (audit 005). Observed events for `x ![](i) y` with `prose::options()`: `Text "x "` 0..2, `Start(Image)` 2..8, `End(Image)` 2..8, `Text " y"` 8..10; only the image's own boundary can clear the bit.

### Scenario coverage
| Spec item | Check |
| :--- | :--- |
| *Block prose*: joined directly across inline delimiters | W1–W4, P13 |
| *Block prose*: one space otherwise | W5–W9 |
| Positions kept | P9′ (existing P9 generator) |
| M2/M3a criteria | unchanged checks stay green; `parses_the_prose_not_the_markup` updated (F1.2) |

### Planted violations (tick when the red was seen)
| # | Plant | Must fail | Seen |
| :--- | :--- | :--- | :--- |
| 1 | bit always `false` | W1–W4 | [x] |
| 2 | bit always `true` | W5–W8 | [x] (planted as "true for every span after the first"; all of W5–W9 red) |
| 3 | `Image` in the transparent set | W8 (empty-alt image `x ![](i) y`) | [x] (only that witness red) |
| 4 | a dropped `Text` does not clear the bit | W9, W5 | [x] (role-dropped `Text` → W9 red; kept `Text` without a span → W5 red) |
| 5 | `Block::from_spans` ignores the bit | P13 | [x] |
| 6 | P13 generator without wrappings (mᵢ = none only) | plant #5 no longer fails P13 — shows the generator reaches the wrapped case (audit 005) | [x] (P13 green 3/3 with #5 and #6 planted together) |
| 7 | `summarize` counts words per prose span (the pre-F1.2b code) | F1.2b `inline_markup_changes_no_word_count` (e.g. `a*a*`: 2 ≠ 1) and `markup_inside_a_word_keeps_one_word` (21 ≠ 19) | [x] (red 3/3 without regression seeds) |

### Coverage gap (run by hand)
Inline constructs behind options that `options()` does not enable (footnotes, wikilinks, smart punctuation) are not generated; if an option is enabled later, its events fall into "not transparent" by the `matches!` default — a space, never a wrong join.
By hand: analyze a real document with bold/italic/link words and the same document stripped of that markup; the counts must be equal.

### Snippets
```rust
// src/prose.rs
pub struct ProseSpan<'a> { text: &'a str, range: Range<usize>, joins_previous: bool }
impl<'a> ProseSpan<'a> {
    /// True iff only inline emphasis/strong/strikethrough/super-/subscript/link
    /// delimiters lie between this span and the previous span of its block.
    pub fn joins_previous(&self) -> bool;
}
fn is_transparent(tag: TagEnd | &Tag) -> bool  // matches!(…, Emphasis | Strong | Strikethrough | Superscript | Subscript | Link{..})

// extract_blocks loop state: `joinable: bool`
//   kept Text  → span.joins_previous = joinable && !current.is_empty(); joinable = true
//   Start(t)/End(t) with is_transparent(t) → unchanged
//   block boundary (existing close_unless) → current closed; joinable = false
//   any other event, incl. a dropped Text or span_at == None → joinable = false
// plain_text_blocks: joins_previous = false

// src/block.rs, Block::from_spans
//   for (i, span): if i > 0 && !span.joins_previous() { push separator ' ' }
```

### Tasks
| ID | Scenario | Files | RED (must fail first) | GREEN | Status |
| :--- | :--- | :--- | :--- | :--- | :--- |
| F1.0 | spec + audit | `spec.md`, `docs/audits/006-…`, this plan | — (docs) | definition corrected | done |
| F1.1 | join bit per boundary kind | `src/prose.rs`, this plan (ticks) | W1–W9 bits against `joins_previous() → false` (W1–W4 red) | `joinable` state in `extract_blocks`; plants 1–4 | done |
| F1.2 | block joins on the bit | `src/block.rs`, `src/analysis.rs`, this plan | P13 + W texts red against the always-space join | `from_spans` uses the bit. `parses_the_prose_not_the_markup` changes because its fixture was keyed on the buggy `The proposal , which`; it now expects `EXAMPLE_TEXT` (the corrected text is the parser input the test always meant). Plants 5–6 | done |
| F1.2b | M1's word count on block prose | `src/summary.rs`, this plan | markup invariance of `summarize(..).words` (marked vs. stripped, intraword markup included) and `words == counts.words`, red against the per-span sum; found by the F1 by-hand check (`un*believ*able` still counted 3 words on the M1 line) | `words` = Σ `count_words(Block::from_spans(b).text())` over `blocks()`; plant 7 | done |
| F1.3 | close-out | `docs/HANDOVER.md`, this plan | `just verify` | — | todo |
