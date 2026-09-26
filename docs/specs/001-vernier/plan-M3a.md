# Plan — spec 001 M3a: Syntactic metrics on a token graph

Tier: `full-spec`. Spec: [spec.md#M3a](spec.md#M3a), definitions *Content token*, *Clausal relation*, *Sentence*. Gate record: [gates.md](gates.md) (Clarification passed; Review, Approval, Comprehension still owed, BACKLOG item 2).
Evidence: the UD parse of the M3a example sentence below was produced by UDPipe 2 (LINDAT REST service, model `english-ewt-ud-2.17-251125`, fetched 2026-09-27) and matches the hand-built parse except that the second comma attaches to `proposal`; PUNCT is dropped before any metric, so the metrics are the same either way.

## Motivation

M3a computes the syntactic metrics of one sentence as pure functions over a list of UD tokens: mean dependency distance (MDD), tree depth in edges, the number of clausal dependents, and center-embedding (a clausal subtree lying wholly between a nominal subject and its head).
A malformed token list (a cycle, several roots, a head out of range) is an error, never a number.
The engine flags `HighMdd`, `DeepTree`, `ClauseOverload` and `CenterEmbedding` against `--max-mdd`, `--max-tree-depth` and `--max-clauses`.
No parser exists yet: a `Parser` trait is the seam M3b fills; every test uses hand-built parses.

## Approach

- `src/dependency.rs`: `Token`, the `Parser` trait, and `DependencyTree` — a newtype built only by `DependencyTree::new(tokens) -> Result<_, TreeError>`. The metric functions take `&DependencyTree`, so a malformed list is rejected once, at the constructor, and no metric can be computed from it (ENGINEERING §5, parse-don't-validate).
- `src/syntax.rs`: the *content projection* (drop `PUNCT`, renumber 1..N, reattach to the nearest non-`PUNCT` ancestor), then `dependency_distance`, `depth`, `clause_count`, `center_embeddings`, bundled as `SyntacticMetrics`.
  `DependencyDistance { total, dependencies }` is a commutative monoid like `SurfaceCounts`; the file-level MDD is the mean of the sum, never a mean of means.
- `src/analysis.rs`: a second flag enum `SyntacticFlag` whose variants carry the measured value, `syntactic_flags(&SyntacticMetrics, &Thresholds)`, and `analyze_parsed(source, format, thresholds, &impl Parser)`, which parses each `Sentence::text()` and attaches `SentenceSyntax { metrics, flags }`. `analyze` (no parser) is unchanged in behaviour; both share one internal function.
- `src/main.rs`: builds the full `Thresholds` from `Args` and renders syntactic flags (unreached from the CLI until M3b wires a parser). `src/cli.rs`: the `--max-tree-depth` help says *edges*.
- Test support `src/testing.rs` (`#[cfg(test)]` only): a CoNLL-U reader for hand-built parses, the example sentence, and the proptest strategy for well-formed trees.

## Decisions made

| Decision | Rationale | Rejected alternative |
| :--- | :--- | :--- |
| Validation in `DependencyTree::new`; metric functions take `&DependencyTree` | the criterion "metric functions SHALL return an error on a non-tree" is met at the only entry from tokens; downstream code trusts the invariant | every metric returning `Result` (the same check four times) |
| `TreeError` variants, checked in this order: `Empty`; `IdOutOfOrder { position, id }` (ids must be 1..n in order); `HeadOutOfRange { id, head }` (head > n); `SeveralRoots { roots }`; `Cycle { id }` (includes the no-root case, which with n ≥ 1 and heads in range always contains a cycle, and a self-loop) | one variant per distinct failure (§7); the order makes each mutation map to exactly one variant | a single `NotATree` (callers could not tell why) |
| `upostag` and `deprel` stay `String` | the only predicates needed are `== "PUNCT"`, `NOUN` (M4) and the deprel sets; conversion from the parser's format happens at the M3b boundary | a 17-variant `Upos` enum (unused variants, conversion code now) |
| All metrics run on the content projection | the spec renumbers content tokens "before any distance is measured"; `PUNCT` leaves would add depth | depth over all tokens |
| A content token whose ancestors up to the root are all `PUNCT` becomes a root of the projection | the spec's reattach rule has no ancestor to reach when `PUNCT` is the root (invalid UD, but a parser may emit it); making it a root keeps the projection total | an error (a real parse would abort the file) |
| MDD = Σ\|i − head(i)\| over content tokens with a content head ÷ their number; absent when that number is 0 | equals the spec's N − 1 whenever the projection has one root, which is every valid UD tree; stays defined on the degenerate `PUNCT`-root case | dividing by N − 1 regardless (wrong when the projection has two roots) |
| File MDD = Σ total ÷ Σ dependencies (the `DependencyDistance` monoid) | equals the spec's "total ÷ (content tokens − sentences)" when every sentence has one projected root; a 1-token sentence adds 0 to both | mean of sentence MDDs |
| Depth = max over content tokens of edges to its projected root | "edges on a path from the root"; a root-only sentence is 0 | counting nodes |
| Clausal ⇔ `deprel.split(':').next() ∈ {advcl, acl, csubj, ccomp}` | the spec's definition; exact base equality, so `xcomp` and `advmod` do not count | `starts_with` / `contains` |
| Center-embedding: for each content token `s` with `deprel ∈ {nsubj, nsubj:pass}` (exact) and content head `v ≠ 0` with `s < v`: reported iff some clausal token's subtree span `[lo, hi]` satisfies `s < lo ∧ hi < v`; one report per such `(s, v)` | the spec's rule; the head's `upostag` is not checked ("the subject's head verb" — a copular head is an adjective in UD) | requiring `upostag == VERB` |
| `words_between` = original tokens strictly between `s` and `v` whose `form` has an alphanumeric char | the spec counts *words* (the example's two commas are excluded: 8); `SYM` like `%` is not a word either | content-token count (would count `%`), original id difference (10) |
| `SyntacticFlag { HighMdd { mdd }, DeepTree { depth }, ClauseOverload { clauses }, CenterEmbedding(CenterEmbedding) }`, separate from M2's `Flag` | a syntactic flag exists only where a parse exists and carries its value, so the renderer needs no fallback (§7); M2's `Flag` and its tests stay untouched | new `Flag` variants (a rendered `HighMdd` would have to look up an `Option` metric) |
| `SentenceAnalysis.syntax: Option<SentenceSyntax>`, `FileAnalysis.dependency_distance: Option<DependencyDistance>`; `None` ⇔ no parser | absence is the domain state "not parsed" (§7.2); M3b's no-model mode is exactly this | zeros |
| `analyze_parsed` parses `Sentence::text()`, never the source slice | the source slice holds markup and link URLs; the block text is the prose | parsing `&source[range]` |
| `trait Parser { type Error: std::error::Error; fn parse(&self, sentence: &str) -> Result<Vec<Token>, Self::Error>; }` | M3b's two candidate routes have different error types; `&self` because a loaded model is read-only | `&mut self`; a boxed error |
| `AnalysisError<E> { Parse(E), Malformed { sentence_start: usize, source: TreeError } }` | a parser failure and a malformed parse mean different things to the shell (M3b maps both to exit 2 with different messages) | one opaque error |
| Thresholds grows `max_mdd: f64`, `max_tree_depth: usize`, `max_clauses: usize`; `Eq` is dropped from its derive | an `f64` field; nothing compares `Thresholds` for equality except tests, which keep `PartialEq` | a newtype for `max_mdd` (`cli.rs` already validates it positive and finite) |
| Flag laws use `>`: `HighMdd ⇔ mdd > max_mdd`, `DeepTree ⇔ depth > max_tree_depth`, `ClauseOverload ⇔ clauses > max_clauses` | "exceeds" in the criterion | `≥` |

## Gate (CONSTITUTION + ENGINEERING)

| Principle | Result |
| :--- | :--- |
| A2 specs are contracts | PASS — no criterion edited; the degenerate `PUNCT`-root case the spec leaves open is decided above and reported to the user at close-out |
| A3 ≤ 5 files per task | PASS — see the task table (max 4) |
| A4 properties over constants | PASS — P1–P14; literals only for the example sentence's witnesses (proposal, caused, 8), the message texts and the defaults |
| A6 seams | PASS — `Parser` trait is required by the spec; `analyze_parsed` is the M3b entry point; nothing else speculative (the CoNLL-U reader is test-only) |
| B4 hermetic | PASS — no new dependency, no network at test time (the UDPipe 2 parse is pasted as test data) |
| D9 complexity ≤ 10, no `unsafe` | PASS — every metric is one pass or one `max`/`filter`; validation is one function per check |
| D10 §5 newtypes | PASS — `DependencyTree` (private field, one constructor), `ContentTree` private to `syntax` |
| D10 §6 pure core | PASS — the parser comes in as an argument; only `main.rs` does I/O |
| D10 §7 outcomes | PASS — `TreeError` closed enum; `Option` for MDD absence and "not parsed"; flags carry values, no fallback; exhaustive matches |
| D10 §8 banned constructs | PASS — the `Infallible` error of `analyze` is eliminated with `let Ok(x) = …;` (irrefutable since Rust 1.82; devenv has 1.98), no `unwrap` |
| Engineering §2 new dependency | N/A — none added |
| Licenses | PASS — one UDPipe 2 output sentence is used as test data with its provenance; model output of our own sentence carries no model code or weights |
| Phase 2 human gates | VIOLATION — Review/Approval/Comprehension still owed (BACKLOG item 2); the user directed implementation to continue |

## Files to read first

`docs/ENGINEERING.md`, `docs/audits/*` (especially 005 on planted violations), `src/lib.rs`, `src/analysis.rs`, `src/readability.rs` (the monoid pattern), `src/sentence.rs` (`Sentence::text`), `src/main.rs` (`check`, `diagnostics`, `describe`), `src/cli.rs`, this plan.
Before any cargo command: `export CARGO_HOME=$DEVENV_STATE/cargo` (audit 002). Scratch goes to `.sdd/`, never `/tmp` (audit 001). The UDPipe 2 output is also in `.sdd/m3a/udpipe2-example.conllu`.

## Type checking strategy

`rustc` + `clippy -D warnings` (`just lint`) after every GREEN.
Expected rejections: building a `DependencyTree` without `new` (private field); computing a metric from `&[Token]`; a `SyntacticFlag` arm missing in `main.rs`'s renderer; reading syntactic metrics of an unparsed sentence without matching the `Option`.

## Testing strategy

| Layer | Covers | Needs |
| :--- | :--- | :--- |
| Unit (`#[cfg(test)]` per module) | validation laws, projection laws, metric laws P1–P14, the example witnesses | `proptest`; `src/testing.rs` (CoNLL-U reader, example constant, `well_formed_tree()` strategy) |
| Engine (`analysis::tests`) | `analyze_parsed` with a `FixtureParser` (text → tokens table), a `ChainParser` (whitespace tokens, each headed by the next) and a `FailingParser` | test-only `Parser` impls in `analysis::tests` |
| Integration (`tests/cli.rs`) | `--max-tree-depth` help text; existing tests unchanged (no parser reachable from the CLI in M3a) | — |
| End-to-end | N/A in M3a — no real parser until M3b, whose integration test runs the example through a real model | — |

## Properties

`T` ranges over `well_formed_tree()`: n ∈ 1..=20, ids 1..n, a random permutation `π` with `π₀` the root and head(`πₖ`) = `π_j` for a random j < k (so every tree shape and non-projective orders occur), each token `PUNCT` with probability ⅓ (internal and root nodes included, so `PUNCT` chains occur), deprels drawn from `{nsubj, nsubj:pass, obj, det, obl, advmod, xcomp, advcl, acl, acl:relcl, ccomp, csubj, punct}`, forms from `{the, proposal, which, caused, 42, %, ","}`.
N = number of non-`PUNCT` tokens.

- **P1** (accepts trees) ∀ T: `DependencyTree::new(T).is_ok()`.
- **P2** (rejects non-trees) ∀ T, and one mutation each: head of one token set to n + 1 + k → `HeadOutOfRange`; (n ≥ 2) a non-root's head set to 0 → `SeveralRoots`; (n ≥ 2) a non-root `x`'s head set to a node of `x`'s own subtree, `x` itself included → `Cycle`; two ids swapped (n ≥ 2) → `IdOutOfOrder`; `[]` → `Empty`.
- **P3** (projection shape) ∀ T: the projection has N tokens with ids 1..N, preserving order; every head ∈ 0..=N and ≠ own id.
- **P4** (nearest ancestor) ∀ T, ∀ content token t: its projected head is the renumbered id of the first non-`PUNCT` token on t's original head chain, or 0 if there is none (checked against a naive walk written in the test).
- **P5** (punctuation invariance) ∀ T, inserting a `PUNCT` leaf (any position, headed by any token, ids and heads shifted) leaves `SyntacticMetrics` unchanged.
- **P6** (MDD bounds, spec) ∀ T: `mean` is `None` ⇔ `dependencies == 0`; when `Some(m)`, `m ≥ 1`; N < 2 ⇒ `None`; one projected root ⇒ `dependencies == N − 1`.
- **P7** (MDD witnesses by law) chain of n (head(i) = i + 1): MDD = 1; star of n rooted at 1: MDD = n / 2.
- **P8** (monoid) `DependencyDistance`: associative, commutative, `zero` neutral; file MDD of `[a, b]` = `(a.total + b.total) / (a.dependencies + b.dependencies)`.
- **P9** (depth bound, spec) ∀ T: `depth ≤ N − 1` when N ≥ 1; chain of n: depth = n − 1 (tight); root-only: 0.
- **P10** (clausal) ∀ base ∈ {advcl, acl, csubj, ccomp}, ∀ subtype s: `is_clausal(base)` and `is_clausal(base:s)`; ∀ d ∈ {xcomp, advmod, obj, nsubj, obl, aclx}: `!is_clausal(d)`; ∀ T: `clause_count` = number of content tokens with a clausal deprel.
- **P11** (center-embedding soundness) ∀ T, ∀ reported e: its subject precedes its verb, the subject token has deprel `nsubj`/`nsubj:pass` headed by the verb, and some clausal subtree lies strictly inside.
- **P12** (center-embedding completeness on the example) moving the relative clause after the verb ("The proposal caused significant delays, which …") yields no report; a clausal subtree that crosses the verb (non-projective: `1 man nsubj→3, 2 left acl:relcl→1, 3 came root, 4 yesterday obl→2`) yields none.
- **P13** (flag laws) ∀ metrics m, thresholds t: `HighMdd` ∈ flags ⇔ `m.mdd = Some(x) ∧ x > t.max_mdd` (carrying x); `DeepTree` ⇔ `depth > max_tree_depth`; `ClauseOverload` ⇔ `clauses > max_clauses`; one `CenterEmbedding` per reported embedding.
- **P14** (engine) ∀ generated document d (the M2 `document()` strategy) with `ChainParser`: `analyze_parsed(d)` equals `analyze(d)` after erasing `syntax` and `dependency_distance` (same sentences, positions, counts, surface flags); every sentence has `Some(syntax)` with `syntax.flags == syntactic_flags(&syntax.metrics, t)`; the file's `dependency_distance` = Σ of the sentences'.

## Scenario coverage

| M3a criterion | Check |
| :--- | :--- |
| 1 `Token` + `Parser` trait, metrics depend on no parser | `analysis::tests` `FixtureParser`/`ChainParser` impls compile and drive `analyze_parsed` (P14); `syntax` imports nothing from any parser |
| 2 MDD formula; file MDD = total ÷ (tokens − sentences) | `syntax::tests::example_sentence_has_mdd_32_over_12` + P6, P7, P8 + `file_mdd_pools_distances_not_means` (chain of 2 + star of 4 → 7/4, not 1.5) |
| 3 < 2 content tokens → MDD absent | P6 + `single_word_sentence_has_no_mdd` (`1 Go root`, and `1 Go root, 2 ! PUNCT`) |
| 4 depth in edges, root-only 0; help text says edges | P9 + `example_sentence_has_depth_4` + `cli::tests::max_tree_depth_help_says_edges` |
| 5 clause count | P10 + `example_sentence_has_one_clause` |
| 6 center-embedding rule | P11, P12 |
| 7 example → "proposal", "caused", 8 | `syntax::tests::reports_the_subject_verb_gap_of_a_center_embedded_sentence` (UDPipe 2 parse) |
| 8 non-tree → error | P2 |
| 9 depth ≤ N − 1, MDD ≥ 1 (property) | P6, P9 |
| 10 thresholds → `HighMdd`, `DeepTree`, `ClauseOverload`, `CenterEmbedding` | P13 + `analysis::tests::example_is_center_embedded_and_high_mdd_only_below_8_3` (flagged `HighMdd` at `max_mdd` 2.5, not at 3.0) + P14 |
| def: content token (renumber, reattach) | P3, P4, P5 |
| def: clausal relation | P10 |
| def: sentence (same with or without a model) | P14 |

## Planted violations (tick when the red was seen)

Each names the concrete input where the mutant differs (audit 005).

- [x] Skip the cycle walk in `DependencyTree::new` → P2 `Cycle` FAIL (input: `1 a →2, 2 b →1, 3 c root` accepted).
- [x] Reattach only one level (`head = tokens[h].head` once, no loop) → P4 FAIL (input: `1 a →2, 2 , PUNCT →3, 3 ; PUNCT →4, 4 b root`: a's head must be b).
- [ ] Divide the MDD total by N instead of by the dependencies → `example_sentence_has_mdd_32_over_12` FAIL (32/13) and P7 chain FAIL.
- [ ] File MDD as the mean of sentence MDDs → `file_mdd_pools_distances_not_means` FAIL (1.5 ≠ 1.75).
- [ ] Depth counts nodes (root = 1) → P9 root-only FAIL.
- [ ] `is_clausal` as `deprel.ends_with("comp") || …` → P10 `xcomp` FAIL.
- [ ] `words_between` = `v − s − 1` over original ids → example witness FAIL (10 ≠ 8).
- [ ] Drop the `hi < v` check → P12 crossing case FAIL (reported).
- [ ] `>=` for `HighMdd` → P13 FAIL (chain, `mdd = 1.0 = max_mdd`; ensure the generator of P13 hits equality: draw `max_mdd` from the metric itself half the time).
- [ ] `analyze_parsed` parses `&source[range]` → `analysis::tests::parses_the_prose_not_the_markup` FAIL (`**The proposal**, which …` — the fixture parser knows only the prose text and returns an error for anything else).
- [ ] `analyze_parsed` drops `LongSentence` from parsed sentences → P14 FAIL (needs a document with a sentence over the threshold: `document()` with `max` from 0..40 reaches it).
- [ ] Help text without "edges" → `max_tree_depth_help_says_edges` FAIL.

## Coverage gap (run by hand)

- No real parser: a parser's actual output (multiword tokens, empty nodes, `PUNCT` roots) is M3b's boundary conversion and its integration test.
- The four syntactic messages in `main.rs` are unit-tested only; no CLI path produces them until M3b.
- The center-embedding rule on real prose (false positives from appositives, parentheticals) is judged by a human in M3b on their own documents.

## Snippets

```rust
// src/dependency.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token { pub id: usize, pub form: String, pub lemma: String, pub upostag: String, pub head: usize, pub deprel: String }
impl Token { pub fn is_punct(&self) -> bool { self.upostag == "PUNCT" } }

pub trait Parser {
    type Error: std::error::Error;
    /// The tokens of one pre-segmented sentence, ids 1..n in order, head 0 = root.
    fn parse(&self, sentence: &str) -> Result<Vec<Token>, Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TreeError {
    #[error("the sentence has no tokens")] Empty,
    #[error("token {position} has id {id}; ids must run 1..n in order")] IdOutOfOrder { position: usize, id: usize },
    #[error("token {id} has head {head}, outside 0..=n")] HeadOutOfRange { id: usize, head: usize },
    #[error("several roots: {roots:?}")] SeveralRoots { roots: Vec<usize> },
    #[error("token {id} lies on a cycle")] Cycle { id: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyTree { tokens: Vec<Token> }
impl DependencyTree {
    pub fn new(tokens: Vec<Token>) -> Result<Self, TreeError>;
    //   non_empty(&t)?; ids_in_order(&t)?; heads_in_range(&t)?; single_root(&t)?; acyclic(&t)?
    //   single_root: roots = ids with head 0; len 0 → defer to acyclic (it finds the cycle); len > 1 → SeveralRoots
    //   acyclic: ∀ token, walk heads ≤ n steps; not reaching 0 → Cycle { id }
    pub fn tokens(&self) -> &[Token];
}

// src/syntax.rs
struct ContentToken<'t> { token: &'t Token, head: usize }   // head: content id (1-based), 0 = root
struct ContentTree<'t> { tokens: Vec<ContentToken<'t>> }     // index k ↔ content id k + 1
fn content_tree(tree: &DependencyTree) -> ContentTree<'_>;
//   content_id[o] = rank among non-PUNCT tokens (0 for PUNCT)
//   head of t: h = t.head; while h ≠ 0 ∧ tokens[h−1].is_punct() { h = tokens[h−1].head }; content_id[h] (0 if h == 0)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DependencyDistance { pub total: usize, pub dependencies: usize }
impl Add for DependencyDistance; impl Sum for DependencyDistance;
impl DependencyDistance { pub fn mean(self) -> Option<f64> }   // None iff dependencies == 0; total as f64 / dependencies as f64

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CenterEmbedding { pub subject: String, pub verb: String, pub words_between: usize }

#[derive(Debug, Clone, PartialEq)]
pub struct SyntacticMetrics { pub distance: DependencyDistance, pub depth: usize, pub clauses: usize, pub center_embeddings: Vec<CenterEmbedding> }
impl SyntacticMetrics { pub fn mdd(&self) -> Option<f64> { self.distance.mean() } }

pub fn syntactic_metrics(tree: &DependencyTree) -> SyntacticMetrics;
pub fn dependency_distance(tree: &DependencyTree) -> DependencyDistance;  // Σ_{k: head≠0} |k − head|, count
pub fn depth(tree: &DependencyTree) -> usize;                             // max_k steps(k → head … → 0)
pub fn clause_count(tree: &DependencyTree) -> usize;
pub fn is_clausal(deprel: &str) -> bool;   // matches!(deprel.split(':').next(), Some("advcl" | "acl" | "csubj" | "ccomp"))
pub fn center_embeddings(tree: &DependencyTree) -> Vec<CenterEmbedding>;
//   spans: for each content id c, [lo, hi] = min/max id over c and its descendants
//   for s in content ids with deprel ∈ {nsubj, nsubj:pass}, v = head(s), v ≠ 0, s < v:
//     if ∃ c: is_clausal(deprel(c)) ∧ s < lo(c) ∧ hi(c) < v → push { form(s), form(v), words_between(orig(s), orig(v)) }
fn words_between(tree: &DependencyTree, from: usize, to: usize) -> usize;  // original ids strictly between, form has an alphanumeric char

// src/analysis.rs (additions)
pub struct Thresholds { pub max_sentence_len: usize, pub max_mdd: f64, pub max_tree_depth: usize, pub max_clauses: usize }  // derive Debug, Clone, Copy, PartialEq
#[derive(Debug, Clone, PartialEq)]
pub enum SyntacticFlag { HighMdd { mdd: f64 }, DeepTree { depth: usize }, ClauseOverload { clauses: usize }, CenterEmbedding(CenterEmbedding) }
#[derive(Debug, Clone, PartialEq)]
pub struct SentenceSyntax { pub metrics: SyntacticMetrics, pub flags: Vec<SyntacticFlag> }
// SentenceAnalysis gains `pub syntax: Option<SentenceSyntax>`; FileAnalysis gains `pub dependency_distance: Option<DependencyDistance>`
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError<E: std::error::Error> {
    #[error("the parser failed: {0}")] Parse(#[source] E),
    #[error("the parse of the sentence at byte {sentence_start} is not a tree: {source}")] Malformed { sentence_start: usize, source: TreeError },
}
pub fn syntactic_flags(metrics: &SyntacticMetrics, thresholds: &Thresholds) -> Vec<SyntacticFlag>;
pub fn analyze(source: &str, format: SourceFormat, thresholds: &Thresholds) -> FileAnalysis;   // unchanged behaviour
pub fn analyze_parsed<P: Parser>(source: &str, format: SourceFormat, thresholds: &Thresholds, parser: &P)
    -> Result<FileAnalysis, AnalysisError<P::Error>>;
//   both call  fn analyze_with<E>(source, format, thresholds, syntax: impl Fn(&Sentence<'_>) -> Result<Option<SentenceSyntax>, E>) -> Result<FileAnalysis, E>
//   analyze:  let Ok(file) = analyze_with::<Infallible>(…, |_| Ok(None));
//   parsed:   |s| { let tokens = parser.parse(s.text()).map_err(AnalysisError::Parse)?;
//                   let tree = DependencyTree::new(tokens).map_err(|source| Malformed { sentence_start: s.source_range().start, source })?;
//                   let metrics = syntactic_metrics(&tree); Ok(Some(SentenceSyntax { flags: syntactic_flags(&metrics, t), metrics })) }
//   dependency_distance = sentences' syntax: Some(Σ distance) iff every sentence was parsed (i.e. iff a parser was given)

// src/main.rs
//   Thresholds built from all four Args fields
//   diagnostics: surface flags as before, then each sentence's syntax.flags via describe_syntactic
fn describe_syntactic(flag: &SyntacticFlag, thresholds: &Thresholds) -> String;
//   HighMdd  → "HighMdd: mean dependency distance {mdd:.2} (max {max_mdd:.2})"
//   DeepTree → "DeepTree: dependency tree depth {depth} edges (max {max_tree_depth})"
//   ClauseOverload → "ClauseOverload: {clauses} subordinate clauses (max {max_clauses})"
//   CenterEmbedding(e) → "CenterEmbedding: subject \"{subject}\" separated from verb \"{verb}\" by {words_between} words"

// src/cli.rs
/// Flag a sentence whose dependency tree is deeper than this, counting edges from the root (DeepTree).

// src/testing.rs  (#[cfg(test)] mod testing; in lib.rs)
pub(crate) fn tokens_from_conllu(conllu: &str) -> Vec<Token>;   // lines starting with a digit, columns 1,2,3,4,7,8; panics on bad input (test code)
pub(crate) const EXAMPLE_TEXT: &str = "The proposal, which the executive committee rejected after extensive deliberation, caused significant delays.";
pub(crate) const EXAMPLE_CONLLU: &str = /* the UDPipe 2 parse below */;
pub(crate) fn well_formed_tree() -> impl Strategy<Value = Vec<Token>>;
```

Example parse (UDPipe 2, `english-ewt-ud-2.17-251125`; columns ID FORM LEMMA UPOS XPOS FEATS HEAD DEPREL):

```text
1  The           the           DET    2   det
2  proposal      proposal      NOUN   13  nsubj
3  ,             ,             PUNCT  8   punct
4  which         which         PRON   8   obj
5  the           the           DET    7   det
6  executive     executive     ADJ    7   amod
7  committee     committee     NOUN   8   nsubj
8  rejected      reject        VERB   2   acl:relcl
9  after         after         ADP    11  case
10 extensive     extensive     ADJ    11  amod
11 deliberation  deliberation  NOUN   8   obl
12 ,             ,             PUNCT  2   punct
13 caused        cause         VERB   0   root
14 significant   significant   ADJ    15  amod
15 delays        delay         NOUN   13  obj
16 .             .             PUNCT  13  punct
```

Witnesses (hand-computed and re-checked by script, `.sdd/m3a/`): N = 13 content tokens, 12 dependencies, total distance 32, MDD 32/12 ≈ 2.667, depth 4 (`caused → proposal → rejected → committee → the`), 1 clause (`acl:relcl`), one center-embedding (proposal, caused, 8); `committee → rejected` (nsubj, 6 < 7) has no clausal subtree between.

## Tasks

One task = one scenario = one commit (`git -c commit.gpgsign=false commit`, D13); `just verify` must pass at every commit (the pre-commit hook runs it).
Existing tests are preserved: T8 adds fields to `Thresholds`, `SentenceAnalysis` and `FileAnalysis`, so existing constructors in tests gain the new fields (values only, no assertion changes).
This plan's ticks go in a separate commit when a task would exceed 5 files.

| ID | Scenario | Files | RED (must fail first) | GREEN | Status |
| :--- | :--- | :--- | :--- | :--- | :--- |
| T1 | well-formed trees accepted, malformed rejected | `src/dependency.rs`, `src/testing.rs`, `src/lib.rs` | P1, P2 against `new` returning `Err(Empty)` / `Ok` always | `Token`, `Parser`, `TreeError`, `DependencyTree::new` | done |
| T2 | content projection | `src/syntax.rs`, `src/lib.rs` | P3, P4 against a projection that keeps `PUNCT` | `content_tree` | done |
| T3 | MDD + file MDD | `src/syntax.rs` | example 32/12, `single_word_sentence_has_no_mdd`, P6, P7, P8, `file_mdd_pools_distances_not_means` against `total = 0` | `DependencyDistance`, `dependency_distance` | todo |
| T4 | depth | `src/syntax.rs` | example 4, P9 against `depth = 0` | `depth` | todo |
| T5 | clause count | `src/syntax.rs` | P10, example 1 against `is_clausal = false` | `is_clausal`, `clause_count` | todo |
| T6 | center-embedding | `src/syntax.rs` | example witness, P11, P12 against `vec![]` | `center_embeddings`, `words_between`; then `SyntacticMetrics` + P5 | todo |
| T7 | `--max-tree-depth` help says edges | `src/cli.rs` | `max_tree_depth_help_says_edges` | doc comment | todo |
| T8 | syntactic flags + thresholds | `src/analysis.rs`, `src/main.rs` | P13 against `syntactic_flags = vec![]`; `describe_syntactic` message tests in `main.rs` | `Thresholds` fields, `SyntacticFlag`, `syntactic_flags`, `SentenceSyntax`, `syntax: None` everywhere, `main` builds full `Thresholds` and renders syntactic flags | todo |
| T9 | engine with a parser | `src/analysis.rs` | P14, `example_is_center_embedded_and_high_mdd_only_below_8_3`, `parses_the_prose_not_the_markup`, parser/malformed errors against `analyze_parsed` = `Ok(analyze(..))` | `analyze_with`, `analyze_parsed`, `AnalysisError`, `dependency_distance` | todo |
| T10 | close-out | `spec.md` (ticks, Status `IMPLEMENTED`), `docs/HANDOVER.md`, `README.md`, this plan (ticks) | `just verify`; planted violations seen | — | todo |
