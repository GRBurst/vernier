# Plan — spec 001 M4: Nominalization and passive voice

Tier: `full-spec`. Spec: [spec.md#M4](spec.md#M4), definitions *Word*, *Sentence*. Gate record: [gates.md](gates.md) (Clarification passed; Review, Approval, Comprehension still owed, BACKLOG item 2).
Evidence: the two parses below were produced by UDPipe 2 (LINDAT REST service, `tokenizer=presegmented&tagger&parser`, model `english-ewt-ud-2.17-251125`, fetched 2026-09-27). Before planning, the criteria were corrected in place ([audit 007](../../audits/007-nominalization-lemma-dropped-ies.md)).
M4 is built, like M3a, against the `Parser` trait: the CLI has no parser until M3b, so the parsed half is library-tested only.

## Motivation

M4 counts nominalizations (nouns derived from verbs or adjectives, by suffix, length, `NOUN` tag and a committed stoplist), per sentence and per file as a ratio over words. From a parse it also reports every passive construction (the head of an `aux:pass` token) and where it is. Without a parse, passive voice is *absent*, not zero.

## Approach

- Two new pure modules. `nominalization` holds the stoplist (compiled in with `include_str!`), the surface lemma, the predicate and the `NominalizationCount` monoid. `passive` holds the `aux:pass` heads and the in-order alignment of token forms to the sentence text.
- `analysis` wiring:
  - every `SentenceAnalysis` gets a `NominalizationCount`: surface words without a parse, `NOUN` tokens with one;
  - `SentenceSyntax` gets the sentence's `Passive`s;
  - `FileAnalysis` gets the summed count and `passives: Option<usize>` (`None` from `analyze`).
- `summary::render` gains a fourth line. No new flag and no `check` change (the spec sets no threshold).

## Decisions made

| Choice | Rationale | Rejected |
| :--- | :--- | :--- |
| Stoplist compiled in (`include_str!`) and parsed per `analyze` call into a sorted `Vec<&'static str>` | ~50 lines, µs to parse; no global state, no lazy static (ENGINEERING §8) | a `LazyLock` static — a global for a µs saving; a runtime file path — the spec makes the list committed data |
| Stoplist content: pybiber's 30 singulars + 22 research false positives (see file header) | the spec says "seeded from pybiber"; research §4 measured the next most frequent false positives; matching is on the lemma, so plurals are redundant | pybiber's plural entries verbatim — dead lines under lemma matching |
| With a parse, the ratio's numerator comes from `NOUN` tokens, the denominator stays M2's words | the spec: words counted as in M2; the tokenizer splits `committee's` differently, and one denominator keeps the ratio comparable with and without a model | tokens as the denominator — the ratio would jump when a model is added |
| Alignment: skip whitespace, then `starts_with(form)` at the cursor; else the next later word start (position preceded by whitespace) where `form` begins; a miss gives `None` and leaves the cursor (revised in T4b, audit 009; known limit: a next form that is a prefix of the substituted word lands one word early) | never matches inside a word; an inserted **or substituted** form costs one position, not the rest | cursor-only matching — a substituted form (`was`→`WAS`, `’`→`'`) leaves the text's word at the cursor and every later token misses (audit 009); `find` from the cursor — `a` matches inside `banana`; failing the file on a miss — one normalized quote would lose all metrics |
| A missed verb form is reported at the sentence's first character | the spec's fallback; the diagnostic still points into the right sentence | dropping the passive — it would under-count |
| `passive_heads` dedups (`has been being written` → one) | the spec: once per head | one per `aux:pass` token |
| `deprel == "aux:pass"` exactly | the spec names this relation; UD has no other passive-auxiliary subtype | prefix match `aux:pass*` — nothing to match |
| Lower-casing with `str::to_lowercase` | Unicode-correct, and the suffixes are ASCII | ASCII-only lower-casing — `Émotion` would miss the stoplist |

## Gate (CONSTITUTION + ENGINEERING)

| Principle | Result |
| :--- | :--- |
| A2 specs are contracts | PASS — criteria corrected in place while `Draft` (D4) with audit 007, before any code; no test weakened |
| A3 ≤ 5 files per task | PASS — see the task table (max 4) |
| A4 properties over constants | PASS — P1–P12; literals only for the parse witnesses (4/14, 3/14, `written`/`rejected`), the stoplist file and the rendered line |
| A6 seams | PASS — reuses `Parser`/`analyze_parsed`; `NominalizationCount` is the only new public type the file level needs |
| B4 hermetic | PASS — no new dependency; no network at test time (parses pasted as test data) |
| D9 complexity ≤ 10, no `unsafe` | PASS — each predicate part is its own `fn`; alignment is one fold |
| D10 §5 newtypes | PASS — `Stoplist` (private field, one constructor `committed()`) |
| D10 §6 pure core | PASS — no I/O; the data file is compiled in |
| D10 §7 outcomes | PASS — `Option` for "no words" and "not parsed" (domain absence, the spec's words); alignment miss is `Option<usize>`; no new error |
| D10 §8 banned constructs | PASS — no `unwrap`/`expect`/`panic!` outside tests, no lazy static |
| Engineering §2 new dependency | N/A — none added |
| Licenses | PASS — pybiber: MIT (PyPI classifier; its LICENSE file also carries Apache-2.0 text, permissive as well), copied verbatim to `data/nominalization-stoplist.PYBIBER-LICENSE`, derivation stated in the list header; UDPipe 2 output of our own sentences as test data, as in M3a |
| Phase 2 human gates | VIOLATION — Review/Approval/Comprehension still owed (BACKLOG item 2); the user directed implementation to continue |

## Files to read first

`docs/ENGINEERING.md`, `docs/audits/*`, `data/nominalization-stoplist.txt`, `src/lib.rs`, `src/words.rs`, `src/readability.rs` (the monoid pattern), `src/dependency.rs` (`Token`), `src/sentence.rs`, `src/block.rs`, `src/analysis.rs`, `src/testing.rs`, `src/summary.rs`, `tests/cli.rs`, this plan.
Before cargo: `export CARGO_HOME=$DEVENV_STATE/cargo` (audit 002). Scratch goes to `.sdd/` (audit 001); the raw LINDAT responses are in `.sdd/m4/{passive,nomz}.json`.

## Type checking strategy

`rustc` + `clippy -D warnings` (`just lint`) after every GREEN.
Expected rejections:
- building a `Stoplist` other than through `committed()`;
- using `ratio()` or `passives` as a number without handling `None`;
- a `SentenceAnalysis`/`SentenceSyntax`/`FileAnalysis` literal in an existing test that lacks the new field. Such tests get the field added and nothing else; the task says so.

## Testing strategy

| Layer | Covers | Needs |
| :--- | :--- | :--- |
| Unit `nominalization` | stoplist parse, surface lemma, predicate, monoid: P1–P7, witnesses | `proptest` |
| Unit `passive` | heads and alignment: P8–P9, witnesses | `testing::PASSIVE_CONLLU` |
| Unit `analysis` | wiring with and without a parse: P10–P12, parse witnesses | `testing::{NOMZ_*, PASSIVE_*}`, a test `Parser` returning pasted tokens (as in M3a) |
| Unit `summary` | fourth line from the library | — |
| Integration `tests/cli.rs` | `analyze` prints the nominalization line; expectation from `render(summarize(..))` | new fixture `tests/fixtures/nominal.md` (`NOMZ_TEXT` as one paragraph) |
| By hand | stoplist quality on technical prose: run `analyze` on `README.md` and `docs/specs/001-vernier/spec.md` and read the ratio | — |

## Properties

- **P1 stoplist shape:** every committed entry is non-empty, lower-case (`e == e.to_lowercase()`), without whitespace, and unique; comment and blank lines yield no entry.
- **P2 stoplist wins:** ∀ entry e: ¬`is_nominalization(e)` ∧ ¬`is_nominalization(surface_lemma(plural(e)))`, where `plural` appends `s`, or turns a final `y` into `ies`.
- **P3 plural invariance:** ∀ L ∈ `[a-z]{3,10}` + suffix: `surface_lemma(L + "s") == L`. If L ends in `ity`: `surface_lemma(L[..len-1] + "ies") == L`.
- **P3′ possessive invariance (audit 008):** ∀ w ∈ `[a-zA-Z]{1,10}` + {none, `tion`, `ity`, `ies`, `s`}, ∀ p ∈ {`'s`, `’s`, `'`, `’`}: `surface_lemma(w + p) == surface_lemma(w)`.
- **P4 case invariance:** ∀ word w: `is_nominalization(surface_lemma(w)) == is_nominalization(surface_lemma(w.to_uppercase()))`.
- **P5 length:** ∀ L with a suffix and fewer than 7 letters: ¬`is_nominalization(L)`. ∀ L with a suffix, ≥ 7 letters and not stoplisted: `is_nominalization(L)`.
- **P6 monoid:** `NominalizationCount` addition is associative, `Default` is its identity, and `Sum` equals folding with `+`.
- **P7 ratio:** `ratio()` is `None` ⇔ `words == 0`; otherwise it is `nominalizations / words` and lies in [0, 1] whenever `nominalizations ≤ words`.
- **P8 heads:** `passive_heads(t)` is strictly ascending, and it equals the set { `head(x)` | x ∈ t, `deprel(x) == "aux:pass"`, `head(x) ≠ 0` }.
- **P9 alignment:** for any text built from forms f₁…fₙ (`[a-z']{1,6}`) joined by 1–3 spaces, `align` gives `Some(oᵢ)`. The offsets strictly increase, and `text[oᵢ..].starts_with(fᵢ)`. Inserting one form absent from the text gives `None` for it alone; every other offset is unchanged.
- **P9″ substitution (T4b, audit 009):** substituting one token's form by a form absent from the text gives `None` for it alone; every other offset is unchanged. Precondition, built into the generator: the next form is not a prefix of the substituted token's original form. The unconditioned case is the witness `the_known_limit_of_a_substitution` (`'` `'` over `' ' `, first substituted → `[None, Some(0)]`).
- **P9‴ no match inside a word (T4b):** a form that occurs in the text only inside words, never at a word start, gets `None`, and every other offset is unchanged.
- **P10 file = Σ sentences:** `file.nominalizations == Σ sentence.nominalizations`, and `file.nominalizations.words == file.totals.words`, with or without a parse.
- **P11 absent, not zero:** for every generated document, `analyze(..).passives == None` and `analyze_parsed(.., fake).passives == Some(Σ sentence passives)`.
- **P12 passive position:** under `analyze_parsed`, every `Passive` has `source_offset ∈ sentence.source_range`. Either `source[offset..].starts_with(verb)`, or `offset == sentence.source_range.start`.

## Witnesses

- **`NOMZ_TEXT`** = `We commission a review of the committee's decisions because their implementation needs careful consideration.`
  - Surface: 4 nominalizations of 14 words (`commission`, `decisions`→`decision`, `implementation`, `consideration`).
  - Parsed: 3 of 14 (`commission` is `VERB`; lemma `decision`).
  - `NOMZ_CONLLU` (the 7-8 multiword line is kept, as UDPipe emits it; the test reader skips range lines — check `tokens_from_conllu` does):

```
1	We	we	PRON	PRP	Case=Nom|Number=Plur|Person=1|PronType=Prs	2	nsubj	_	_
2	commission	commission	VERB	VBP	Mood=Ind|Number=Plur|Person=1|Tense=Pres|VerbForm=Fin	0	root	_	_
3	a	a	DET	DT	Definite=Ind|PronType=Art	4	det	_	_
4	review	review	NOUN	NN	Number=Sing	2	obj	_	_
5	of	of	ADP	IN	_	9	case	_	_
6	the	the	DET	DT	Definite=Def|PronType=Art	7	det	_	_
7-8	committee's	_	_	_	_	_	_	_	_
7	committee	committee	NOUN	NN	Number=Sing	9	nmod:poss	_	_
8	's	's	PART	POS	_	7	case	_	_
9	decisions	decision	NOUN	NNS	Number=Plur	4	nmod	_	_
10	because	because	SCONJ	IN	_	13	mark	_	_
11	their	their	PRON	PRP$	Case=Gen|Number=Plur|Person=3|Poss=Yes|PronType=Prs	12	nmod:poss	_	_
12	implementation	implementation	NOUN	NN	Number=Sing	13	nsubj	_	_
13	needs	need	VERB	VBZ	Mood=Ind|Number=Sing|Person=3|Tense=Pres|VerbForm=Fin	2	advcl	_	_
14	careful	careful	ADJ	JJ	Degree=Pos	15	amod	_	_
15	consideration	consideration	NOUN	NN	Number=Sing	13	obj	_	SpaceAfter=No
16	.	.	PUNCT	.	_	2	punct	_	SpaceAfter=No
```

- **`PASSIVE_TEXT`** = `The report was written by the committee after the proposal had been rejected.`
  - Passives: `written` (id 4) and `rejected` (id 13), in that order. Each offset is derived as `source.find("written")` or `source.find("rejected")`, not pinned.

```
1	The	the	DET	DT	Definite=Def|PronType=Art	2	det	_	_
2	report	report	NOUN	NN	Number=Sing	4	nsubj:pass	_	_
3	was	be	AUX	VBD	Mood=Ind|Number=Sing|Person=3|Tense=Past|VerbForm=Fin	4	aux:pass	_	_
4	written	write	VERB	VBN	Tense=Past|VerbForm=Part|Voice=Pass	0	root	_	_
5	by	by	ADP	IN	_	7	case	_	_
6	the	the	DET	DT	Definite=Def|PronType=Art	7	det	_	_
7	committee	committee	NOUN	NN	Number=Sing	4	obl:agent	_	_
8	after	after	SCONJ	IN	_	13	mark	_	_
9	the	the	DET	DT	Definite=Def|PronType=Art	10	det	_	_
10	proposal	proposal	NOUN	NN	Number=Sing	13	nsubj:pass	_	_
11	had	have	AUX	VBD	Mood=Ind|Number=Sing|Person=3|Tense=Past|VerbForm=Fin	13	aux	_	_
12	been	be	AUX	VBN	Tense=Past|VerbForm=Part	13	aux:pass	_	_
13	rejected	reject	VERB	VBN	Tense=Past|VerbForm=Part|Voice=Pass	4	advcl	_	SpaceAfter=No
14	.	.	PUNCT	.	_	4	punct	_	SpaceAfter=No
```

- Surface-lemma table:

  | Input | Lemma |
  | :--- | :--- |
  | `activities` | `activity` |
  | `Decisions` | `decision` |
  | `business` | `business` |
  | `cities` | `city` |
  | `class` | `class` |
  | `implementation` | `implementation` |
  | `rations` | `ration` (6 letters, so not a nominalization) |
  | `decision's` | `decision` (possessive, audit 008) |
  | `decisions'` | `decision` |
  | `activity’s` | `activity` |
  | `decisions’` | `decision` |

- Dedup: synthetic tokens `it(1) has(2,aux) been(3,aux:pass→5) being(4,aux:pass→5) written(5,root)` give `[5]`.

## Scenario coverage

| Spec criterion | Check |
| :--- | :--- |
| nominalization predicate (suffix, ≥ 7 letters, stoplist, surface lemma / `NOUN` + lemma) | P1–P5, surface-lemma table, `NOMZ` surface 4/14 and parsed 3/14 |
| ratio per sentence and per file | P6, P7, P10, `NOMZ` witnesses, `empty_file_has_no_nominalization_ratio`, integration `analyze_prints_the_nominalization_ratio` |
| `aux:pass` head reported once, with position | P8, P9, P12, `PASSIVE` witness, dedup witness |
| no parse → passive absent | P11, render line `passive voice absent (no parse)` |

## Planted violations (tick when the red was seen)

| # | Plant | Must fail | Seen |
| :--- | :--- | :--- | :--- |
| 1 | `surface_lemma` strips `s` only, with no `ies` → `y` | table row `activities`; P3 (`ity` branch) | [x] |
| 2 | `is_nominalization` ignores the stoplist | P2 | [x] |
| 3 | length measured on the word before lemmatizing | `rations` witness (surface count 1 ≠ 0) | [x] |
| 4 | parsed count ignores `upostag` | `NOMZ` parsed 3 (becomes 4) | [x] |
| 5 | parsed count uses `form` instead of `lemma` | `NOMZ` parsed 3 (becomes 2) | [x] |
| 6 | `passive_heads` without dedup | dedup witness | [x] (P8 red too) |
| 7 | `align` uses `text.find(form)` from 0 | P9 (repeated forms); `PASSIVE` (two `the`) offsets not increasing | [x] |
| 8 | the passive offset is always the sentence start | `PASSIVE` witness offsets | [x] |
| 9 | `analyze` sets `passives: Some(0)` | P11 | [x] (also M3a P14, whose parse-only eraser now resets `passives`) |
| 10 | `ratio()` returns `Some(0.0)` for zero words | P7; `empty_file_has_no_nominalization_ratio` | [x] (P7 red in T3; re-planted in T7: `empty_file_has_no_nominalization_ratio` and `renders_absent_metrics` red) |
| 12 | the possessive strip ignores `’` (U+2019) | P3′ and the `activity’s` / `decisions’` rows (added with audit 008) | [x] (planted twice: bare `’` only → P3′ + `decisions’`; `’s` and `’` → P3′ + both rows) |
| 11 | P9 generator never inserts an absent form | plant 7′: drop the "cursor unchanged on a miss" rule. It must fail P9 only while the generator inserts; seen green without insertion, red with it (audit 005) | [x] (7′ = a miss advances the cursor by the form's length: P9 red 3/3 with insertion, green 3/3 with the generator fixed to no insertion) |
| 7″ | `align` matches at the cursor only (the pre-T4b rule, audit 009) | P9″ (substitute branch), the `WAS` witness `a_substituted_form_costs_one_position` | [x] (red 3/3; `never_matches_inside_a_word` red too, by the cascade) |
| 13 | `align` searches with `find` from the cursor (matches inside a word) | `never_matches_inside_a_word` | [x] (red 3/3; P9 red in 2 of 3) |
| 11 (T4b re-run) | P9 generator with no edits (neither insert nor substitute) | plant 7′ green without edits, red with them | [x] (red 3/3 with edits, green 3/3 without) |

## Coverage gap (run by hand)

- Parse quality: `NOUN` tags and lemmas come from a real model only in M3b; here they are pasted. After M3b, re-run the `NOMZ`/`PASSIVE` witnesses through the real model (M3b's integration test).
- Stoplist quality on technical Markdown (research §4 caveat: `function`, `element`, `instance`, `version`, `application`). By hand: `analyze README.md docs/specs/001-vernier/spec.md`, read the ratios, and note any obvious false positive for a later stoplist pass. A committed measurement is deferred to the first user report (BACKLOG).

## Snippets

```rust
// src/nominalization.rs
const STOPLIST: &str = include_str!("../data/nominalization-stoplist.txt");
const SUFFIXES: [&str; 6] = ["tion", "sion", "ment", "ance", "ence", "ity"];
const MIN_LETTERS: usize = 7;

#[derive(Debug, Clone)]
pub struct Stoplist { lemmas: Vec<&'static str> }          // sorted, deduped
impl Stoplist {
    pub fn committed() -> Self;                             // lines: trim; skip "" and "#…"
    pub fn contains(&self, lemma: &str) -> bool;            // binary_search
}

pub fn surface_lemma(word: &str) -> String;
//   w = word.to_lowercase();
//   if let Some(stem) = w.strip_suffix("ies").filter(|s| !s.is_empty()) { stem + "y" }
//   else if w.ends_with('s') && !w.ends_with("ss") { w[..len-1] } else { w }

pub fn is_nominalization(lemma: &str, stoplist: &Stoplist) -> bool;
//   l = lemma.to_lowercase(); has_suffix(&l) && letters(&l) >= MIN_LETTERS && !stoplist.contains(&l)

pub fn surface_nominalizations<'a>(words: impl IntoIterator<Item = &'a str>, stoplist: &Stoplist) -> usize;
pub fn parsed_nominalizations(tokens: &[Token], stoplist: &Stoplist) -> usize;   // upostag == "NOUN" && is_nominalization(&t.lemma, ..)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NominalizationCount { pub nominalizations: usize, pub words: usize }
impl Add for NominalizationCount { .. }   impl Sum for NominalizationCount { .. }
impl NominalizationCount { pub fn ratio(&self) -> Option<f64> }   // None iff words == 0

// src/passive.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Passive { pub verb: String, pub source_offset: usize }
pub fn passive_heads(tokens: &[Token]) -> Vec<usize>;               // token ids, ascending, deduped
pub fn align(tokens: &[Token], text: &str) -> Vec<Option<usize>>;   // byte offsets into `text`
//   fold with cursor: c' = c + leading whitespace; if text[c'..].starts_with(form) { Some(c'), cursor = c' + form.len() } else { None, cursor = c }
//   T4b (audit 009): else the first word start w > c' (text[w-1] is whitespace) with text[w..].starts_with(form) → Some(w), cursor = w + form.len()

// src/sentence.rs (if missing): pub fn source_offset(&self, offset_in_sentence: usize) -> usize   // via the block's map

// src/analysis.rs (adapt names to the file)
pub struct SentenceSyntax { pub metrics, pub flags, pub passives: Vec<Passive>, pub nominalizations: usize }
pub struct SentenceAnalysis { .., pub nominalizations: NominalizationCount }  // words = counts.words
pub struct FileAnalysis { .., pub nominalizations: NominalizationCount, pub passives: Option<usize> }
//   analyze: passives None;  analyze_parsed: Some(Σ sentence syntax passives.len())
//   the Stoplist is built once per call and passed down

// src/summary.rs — FileSummary gains `nominalizations: NominalizationCount`; render line 4:
//   "  nominalization ratio {r:.3} ({n} of {w} words), passive voice absent (no parse)"
//   | "  nominalization ratio absent (no words), passive voice absent (no parse)"
```

## Tasks

One task = one scenario = one commit (`git -c commit.gpgsign=false commit`, D13; message states the problem and ends `— Spec 001 M4 Tn (full-spec)`); `just verify` must pass at every commit (the pre-commit hook runs it).

| ID | Scenario | Files | RED (must fail first) | GREEN | Status |
| :--- | :--- | :--- | :--- | :--- | :--- |
| T0 | spec correction, data, plan | `spec.md`, `docs/audits/007-…`, `data/nominalization-stoplist.{txt,PYBIBER-LICENSE}`, this plan | — (docs/data) | criteria corrected; list committed | done |
| T1 | committed stoplist | `src/nominalization.rs`, `src/lib.rs` | P1 + `contains("city")` against `committed()` returning an empty list | parse `STOPLIST` | done |
| T2 | surface lemma | `src/nominalization.rs` | table + P3 against identity lower-casing; plant 1 | `surface_lemma` | done |
| T3 | predicate + counts + monoid | `src/nominalization.rs` | P2, P4, P5, P6, P7, `rations` against `is_nominalization → false` and a zero `ratio`; plants 2, 3, 10 | `is_nominalization`, `surface_nominalizations`, `parsed_nominalizations`, `NominalizationCount` | done |
| T4 | passive heads + alignment | `src/passive.rs`, `src/lib.rs`, `src/testing.rs` (`NOMZ_*`, `PASSIVE_*`) | P8, P9, dedup and `PASSIVE` heads against empty outputs; plants 6, 7, 11 | `passive_heads`, `align` | done |
| T5 | surface wiring | `src/analysis.rs` | P10 (no parse) and the `NOMZ` surface 4/14 against a zero count; P11's `None` half | `SentenceAnalysis`/`FileAnalysis.nominalizations`, `passives: None`. Existing struct-literal tests get the new fields only | done |
| T6 | parsed wiring | `src/analysis.rs`, `src/sentence.rs` (if `source_offset` is missing) | `NOMZ` parsed 3/14, `PASSIVE` offsets, P11's `Some` half, P12 against the surface count and no passives; plants 4, 5, 8, 9 | `SentenceSyntax.{passives, nominalizations}`, file `passives` | done |
| T7 | `analyze` prints line 4 | `src/summary.rs`, `tests/cli.rs`, `tests/fixtures/nominal.md` | `analyze_prints_the_nominalization_ratio` (expectation from `render(summarize(..))` plus the `4 of 14` literal the user reads) and `empty_file_has_no_nominalization_ratio` in `summary` | `FileSummary.nominalizations`, render line 4 | done |
| T8 | close-out | `spec.md` (ticks, Status `IMPLEMENTED`), `docs/HANDOVER.md`, `README.md` (pybiber notice, status), this plan (ticks) | `just verify`; every plant seen red; the by-hand run | — | done (by hand: `nominal.md` 4 of 14; `The decision's implementation requires activities.` 3 of 5) |
