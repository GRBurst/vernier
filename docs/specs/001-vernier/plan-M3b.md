# Plan — spec 001 M3b: Parser integration and model handling

Tier: `full-spec`. Spec: [spec.md#M3b](spec.md#M3b), and M4's parse-lemma fallback ([spec.md#M4](spec.md#M4), criterion 1, revised in place in 3d31b3c). Gate record: [gates.md](gates.md) (Clarification passed; Review, Approval, Comprehension still owed, BACKLOG item 2).
Evidence: [research/m3b-parser-spike.md](research/m3b-parser-spike.md), [measurements/m3b-spike-data.md](measurements/m3b-spike-data.md); the spike's working decoder is `.sdd/m3b-spike/onnx/src/main.rs` (scratch, B5), a port of the model's `ud.py` with byte-identical parity on the sample and 0 disagreements over 2000 fuzzed matrices.
Route (user, 2026-09-27): ONNX via `ort`, model `ghotriw/roberta-base-english-ud-goeswith-onnx`, passed as a directory with `--model-path`; no downloader, no bundling; license already recorded (spec criterion 2, README).
Decisions confirmed by main on 2026-09-27 (questions 1–9 of the task handshake) are marked *(main)*.

## Motivation

The syntactic metrics of M3a and the passives of M4 are computed today only from hand-built parses. M3b puts a real Universal Dependencies parser behind the `Parser` trait: with `--model-path DIR`, `analyze` and `check` parse every sentence once and report mean dependency distance, depth, clauses, center-embedding and passives, and raise the syntactic flags. Without a model they stay surface-only, say so once on stderr, and judge the exit code on surface rules. A missing or unusable model is an error (exit 2), never a panic.

## Approach

- **Pure core (new):**
  - `src/mst.rs` — Chu-Liu/Edmonds exactly as `ud.py` (`matrix[head][dep]`, the diagonal is the root score; `h[j] == j` marks a root), split into functions of complexity ≤ 10.
  - `src/decoder.rs` — everything between the tokenizer and `Vec<Token>` that needs no runtime: `Labels` (from `config.json`'s `id2label`/`label2id`), the masked batch, the length rule, the label masks, the goeswith constraint, the single-root fix, the subword merge, and `Token` construction (lemma `_`, `upostag`/`deprel` from the label).
- **Shell (new):** `src/onnx.rs` — `OnnxParser::load(model_dir, runtime_path)` reads the three model files, loads ONNX Runtime with `ort::init_from` (typed error, never ort's panicking implicit load), builds one `Session`, and implements `Parser` by tokenize → batch → `session.run` → `decoder`.
- **Seam changes (M3a code):**
  - `Parser::parse(&mut self, &str) -> Result<Parse, Self::Error>`, `enum Parse { Tokens(Vec<Token>), TooLong { pieces, max } }` — `ort::Session::run` takes `&mut self`, and a sentence beyond the model's positions is an outcome, not a failure.
  - `SentenceAnalysis.syntax: Option<SentenceSyntax>` becomes `enum Syntax { Unparsed, TooLong { pieces, max }, Parsed(SentenceSyntax) }`, so "no model" and "too long for the model" are two variants and render differently *(main)*.
- **Wiring:** `main.rs` loads the parser once per run (not per file), calls `analyze_parsed`, fills `FileSummary` from the parsed `FileAnalysis` (`summary::with_parse`), prints the notices, and maps errors to exit 2.
- **M4 fallback:** `parsed_nominalizations` reads a `_` lemma as `surface_lemma(form)`.
- **Build:** `ort` with `load-dynamic` (no ONNX Runtime at build time; `dlopen` at run time only when `--model-path` is given); `devenv.nix` gains `onnxruntime` and `ORT_DYLIB_PATH`. ADR [0001](../../adr/0001-onnx-runtime-via-ort.md) records the native dependency.

## Decisions made

| Choice | Rationale (evidence) | Rejected |
| :--- | :--- | :--- |
| `ort` `=2.0.0-rc.13` with features `std`, `load-dynamic`, no `api-*` feature | a linked build has `NEEDED libonnxruntime.so.1` (`readelf -d` on `spike-onnx-nixlink`), so a user without the `.so` could not even run `analyze`; `load-dynamic` needs no library at build (`readelf`: `spike-onnx-nixdyn` needs only libc/libm/libgcc) and none at run time until `--model-path` is used. Without an `api-*` feature `ORT_API_VERSION` is 17 (`ort-sys` `version.rs`), so any ONNX Runtime ≥ 1.17 loads; nothing vernier calls needs a newer API | linking nix `onnxruntime` — every run needs the `.so` and a nix `RUNPATH`; `download-binaries` — network at build, breaks B4 (spike); `api-27` — would refuse ONNX Runtime < 1.27 for no used feature |
| vernier resolves the runtime path itself — `ORT_DYLIB_PATH` if set and non-empty, else `libonnxruntime.so` (system loader search) — read in `main`, and calls `ort::init_from(path)?.commit()` | ort's implicit load reads the same variable but **panics** on failure (`expect("Failed to load ONNX Runtime dylib")`, `ort` src/lib.rs:234); `init_from` returns `LoadDynamicError` (`Dlopen`, `MissingApi`, `BadVersion`), which becomes a typed `LoadError::Runtime` → exit 2. The environment is ambient input and belongs in the shell (ENGINEERING §6) | letting ort read `ORT_DYLIB_PATH` — a missing library would panic; a vernier-specific variable — ort's documented name is what users of ort-based tools already know |
| `devenv.nix`: `packages += onnxruntime`, `env.ORT_DYLIB_PATH = "${pkgs.onnxruntime}/lib/libonnxruntime.so"` | B4: the pinned environment provides the runtime for by-hand and model-backed runs. `devenv-nixpkgs` `c2f38fe7` (devenv.lock) evaluates `onnxruntime` to `/nix/store/x46rnziynpalr4phm6yh0x46s42xq5kw-onnxruntime-1.27.1`, the spike's out-link exactly (`nix eval`, 2026-09-27), so the sandbox's by-hand `ORT_DYLIB_PATH` is the path devenv will set | no devenv change — the runtime would be an unpinned host tool |
| Model directory layout: `DIR/config.json`, `DIR/tokenizer.json`, `DIR/onnx/model.onnx` (the Hugging Face repository as cloned); each missing file is named | the user fetches the repository; one layout, one error per missing file | a list of accepted layouts — speculative (A6) |
| Special token ids from the tokenizer (`<s>`, `</s>`, `<mask>`), not constants | a differently exported tokenizer fails with a typed `LoadError::SpecialToken`, not wrong logits | the spike's `CLS = 0, SEP = 2, MASK = 50264` |
| Length rule: a sentence of `n` subword pieces is parsed iff `n + 3 ≤ max_positions`, `max_positions = max_position_embeddings − pad_token_id − 1` from `config.json` (514 − 1 − 1 = 512, so `n ≤ 509`) *(main)*; otherwise `Parse::TooLong { pieces: n, max: max_positions − 3 }` | the batch row is `<s> p₁…pₙ </s> pᵢ` (n + 3 positions); RoBERTa's position ids start at `pad_token_id + 1`. The limit is read from the model, not hard-coded | a practical cap below the model's limit — memory is bounded by chunking instead (next row); time is BACKLOG 6 |
| Inference in row chunks of `CHUNK_ROWS` = 16 rows (the batch is one masked copy per piece; rows are independent) | unchunked, a 400-piece sentence needs ≈ 400·12·403²·4 B ≈ 3.1 GB per attention tensor; chunking bounds it by 16/n of that. Accepted only if the parse of `parser-sample.md` is identical chunked and unchunked (by-hand check B2) | one batch per sentence (the spike) — an OOM kill on a long list item is not a typed outcome |
| A too-long sentence: `Syntax::TooLong`; its metric lines read `absent (too long for the model)`; one stderr notice per such sentence, `vernier: PATH:L:C: sentence too long for the model (N subword pieces, max M); syntactic metrics skipped`; exit code unaffected; JSON unchanged (its diagnostics carry flags, not metric lines; `null` stays `null`) *(main)* | the reader must tell "no model" from "too long" *(main)*; a per-file JSON note would add a schema field for a rare case (main: only if cheap) | skipping silently; failing the file |
| `Parser::parse(&mut self, sentence) -> Result<Parse, Self::Error>` *(main)* | `Session::run(&mut self, …)` (`ort` src/session/mod.rs:236); `&mut` states that a session is used, instead of hiding it behind a `RefCell`/`Mutex` (ENGINEERING §8) | `&self` + `Mutex<Session>` — hidden mutable state |
| `enum Syntax { Unparsed, TooLong { pieces, max }, Parsed(SentenceSyntax) }` replaces `Option<SentenceSyntax>`, in a refactor task first (T1: `Unparsed`/`Parsed` only) | ENGINEERING §7: two ways of coming back empty that render differently are two variants | `Option` + a separate skip list — two fields that depend on each other |
| File-level MDD and passives pool the `Parsed` sentences only; `dependency_distance`/`passives` are `Some` whenever a parser ran | M3a's pooled definition over the sentences that have a tree | counting a too-long sentence as zero distance |
| Decoding = `ud.py`'s algorithm, ported from the spike without change of behaviour (masks, goeswith rule, CLE, single-root fix, merge) | parity with the model author's decoder is the evidence (byte-identical on the sample, 2000-matrix fuzz) | greedy argmax heads — not a tree on 1 of 50 sample sentences (spike) |
| If the property "decode yields a valid tree" finds logits where `ud.py`'s single-root fix leaves several roots (it penalises only the former roots' diagonals, so a new root can appear), a second, strict pass penalises every diagonal except `k`'s and reruns CLE | keeps parity wherever `ud.py` succeeds and makes the tree law total; a residual failure would still be caught by `DependencyTree::new` (typed, exit 2) | changing `ud.py`'s fix for all inputs — breaks parity |
| Form of a token = `sentence.get(start..end)` of its merged piece offsets; an offset off a char boundary → `DecodeError::Offset` | `tokenizers` offsets are byte offsets; `&s[a..b]` would panic on a split multi-byte character | slicing with `[]` |
| Lemma `_`; `upostag` = the label's first `|` field, `deprel` its last | the model gives no lemma (spike); `ud.py`'s CoNLL-U columns | — |
| M4: `_` lemma → `surface_lemma(form)` in `parsed_nominalizations` (spec M4 criterion 1, revised) | ONNX gives `_` for every lemma; without the fallback no parsed noun is ever a nominalization | falling back to the surface count for the whole sentence — the spec says per token |
| Load once per run in `main`, before the first file; a load error → `vernier: cannot load the model DIR: <cause>` (the cause names the file or runtime path), exit 2, no file processed | criterion 6; loading takes 1.3–1.9 s (spike) | per file; lazily at the first sentence (a model error would look like a file error) |
| No `--model-path` → one notice per run on stderr: `vernier: no --model-path given; syntactic metrics skipped` | criterion 5 ("one notice") | one per file |
| A parser failure on a sentence (tokenizer, inference, output shape, decode, not a tree) → `vernier: cannot parse the sentence at PATH:L:C: <cause>`, that file counts as unreadable (exit 2), the other files go on *(main)* | same channel as an unreadable file (M1); the position names the sentence *(main)* | aborting the run |
| `FileSummary` with a parse: `summary::with_parse(summarize(..), &file)` sets `mean_dependency_distance`, `passives` and `nominalizations` from the one parsed `FileAnalysis` *(main)* | one parse per sentence *(main)*; M4: with a parse the candidates are `NOUN` tokens | re-parsing inside `summarize` |
| ORT's default intra-op threads; no tuning *(main)* | performance is not a criterion (BACKLOG 6); the time is recorded below | `with_intra_threads(physical cores)` — std gives logical cores only; a guess |
| Model-backed tests gated by `VERNIER_TEST_MODEL` (a model directory) only: absent → the test prints `skipped: VERNIER_TEST_MODEL is not set` and returns; set but unusable → the test **fails** *(main)*. Two such tests (≈ 0.5 s/sentence) | criterion 4 ("skipped with a printed reason when no model is present"); a set-but-broken setup must not look green | `#[ignore]` — prints no reason and never runs in `just verify` |
| Changed test `tests/cli.rs::check_accepts_every_m5_flag_and_passes_without_rules` loses `--model-path none.udpipe` *(main)* | it encoded M1's "flag accepted but without effect", which M3b criterion 6 supersedes (a missing model now exits 2). Flag parsing stays covered by `cli::accepts_every_m5_flag_on_both_commands`; `a_missing_model_is_named_and_exits_2` covers the new behaviour | keeping it — it would assert exit 0 against criterion 6 |
| ADR 0001 `Status: Proposed` *(main: put to the user)* | the route is the user's; the ADR's wording (native code in `ort`, no `unsafe` in vernier) is new | `Accepted` without review |

## Gate (CONSTITUTION + ENGINEERING)

| Principle | Result |
| :--- | :--- |
| A2 specs are contracts | PASS — no criterion changed; one test changed with its reason (decisions table, T7) |
| A3 ≤ 5 files per task | PASS — see the task table (max 5: T6 with `Cargo.toml`, `Cargo.lock`) |
| A4 properties over constants | PASS — P1–P14; literals only where they are the criterion (notice texts, `proposal`/`caused`) or the model's (512) |
| A6 seams | PASS — `Parse` and `Syntax` are the only new seams, each with ≥ 2 variants in use; no backend switch (BACKLOG 10) |
| B4 hermetic | PASS — crates locked in `Cargo.lock`, fetched into `$DEVENV_STATE/cargo`; the runtime from the pinned nixpkgs; tests without `VERNIER_TEST_MODEL` need neither model nor runtime |
| D9 complexity ≤ 10, no `unsafe` | PASS — CLE split into greedy/cycle/contract/expand; no `unsafe` in vernier (`unsafe_code = "forbid"` stays); the `unsafe` lives in `ort`/`libloading` (ADR 0001) |
| D10 §6 pure core | PASS — `mst`, `decoder` pure; file reads, `dlopen`, the session and `ORT_DYLIB_PATH` only in `onnx.rs`/`main.rs` |
| D10 §7 outcomes | PASS — `Parse`, `Syntax`, `LoadError`, `ParseError`, `DecodeError`, `LabelError` closed enums; exhaustive matches |
| D10 §8 banned constructs | PASS — no `unwrap`/`expect`; no `RefCell`/`Mutex` (`&mut self` instead) |
| Engineering §2 new dependency | PASS — `ort`/`ort-sys` 2.0.0-rc.13 (MIT OR Apache-2.0), `tokenizers` 0.23.2 (Apache-2.0, `default-features = false`, `fancy-regex`): existence, API and license verified in the spike; native code → ADR 0001 |
| Licenses | PASS — Apache-2.0 and MIT OR Apache-2.0 dependencies under vernier's MIT OR Apache-2.0; ONNX Runtime MIT, supplied by the user's system; the model is not redistributed (spec criterion 2) |
| Phase 2 human gates | VIOLATION — Review/Approval/Comprehension still owed (BACKLOG item 2); the user directed implementation to continue |

## Files to read first

`docs/ENGINEERING.md`, `docs/audits/*` (005 for planted violations, 001/002 sandbox and cargo), `research/m3b-parser-spike.md` (sections *ONNX decoding*, *B4*), `.sdd/m3b-spike/onnx/src/main.rs` (`chu_liu_edmonds`, `parse`, `decode`), `.sdd/m3b-spike/models/rbeg-onnx/ud.py`, `src/dependency.rs`, `src/analysis.rs` (`analyze_parsed`, `analyze_with`, `parse_sentence`, the test parsers), `src/diagnostic.rs` (`metric_lines`, `flag_messages`), `src/summary.rs`, `src/nominalization.rs`, `src/main.rs`, `tests/cli.rs`, `src/testing.rs`, this plan.
Before cargo: `export CARGO_HOME=$DEVENV_STATE/cargo` (audit 002). Scratch under `.sdd/` (audit 001). By hand in the sandbox (devenv cannot be reloaded there): `export ORT_DYLIB_PATH=/nix/store/x46rnziynpalr4phm6yh0x46s42xq5kw-onnxruntime-1.27.1/lib/libonnxruntime.so VERNIER_TEST_MODEL=$PWD/.sdd/m3b-spike/models/rbeg-onnx`.

## Type checking strategy

`rustc` + `clippy -D warnings` (`just lint`) after every GREEN. Expected rejections:
- a `match` on `Syntax`, `Parse` or an error enum missing a variant (the refactor T1 and the new variant T2 are driven by these);
- a test parser still implementing `parse(&self)` after T2;
- `Session::run` through `&self`.

## Testing strategy

| Layer | Covers | Needs |
| :--- | :--- | :--- |
| Unit `mst` | P1–P4 | `proptest`; a brute-force maximum over all head assignments (n ≤ 6) |
| Unit `decoder` | P5–P11, the label witnesses | `proptest` over random finite logits and random piece offsets; hand-built `config.json` fragments |
| Unit `analysis`, `diagnostic` | `Syntax` variants (P12), too-long rendering, parse-error position | the existing test parsers plus a `TooLongParser` |
| Unit `nominalization` | P13 | — |
| Unit `summary` | P14 | a `ChainParser`-style parse |
| Unit `onnx` (no model) | load errors: missing dir, missing file, bad `config.json`, bad tokenizer | files under `CARGO_TARGET_TMPDIR`; no runtime (the runtime is loaded after the files are checked) |
| Integration `tests/cli.rs` (no model) | notice without a model, stdout unchanged; missing model dir → exit 2; unloadable runtime (`ORT_DYLIB_PATH` → a missing file, and → a non-ONNX `.so`) → exit 2 | `ORT_DYLIB_PATH` set per test process; a real model directory is not needed because files are checked first — the runtime tests use a directory with the three files present but fake (`{}`-style) contents, so the runtime is reached — see T7 |
| Integration `tests/model.rs` (with `VERNIER_TEST_MODEL`) | criterion 3 and 4: `check` on `sample.md` reports the example's center-embedding proposal/caused; `analyze --format json` fills MDD and passives | the model and the runtime; skipped with a printed reason otherwise |
| By hand | B1 parity with the spike's CoNLL-U on `parser-sample.md`; B2 chunking identical; B3 the timed `check` runs; B4 a too-long sentence | the model, the runtime |

## Properties

Notation: `M ∈ ℝⁿˣⁿ` finite, `M[i][j]` = score of head `i` for dependent `j`, `M[j][j]` = score of `j` as a root. A head vector `h ∈ {0..n−1}ⁿ` is a *forest* iff following `h` from any `j` reaches a fixpoint `h[r] = r` (no cycle through non-roots); `score(h) = Σⱼ M[h[j]][j]`.

- **P1 CLE yields a forest:** `∀ M: chu_liu_edmonds(M)` has length n, entries < n, and is a forest.
- **P2 CLE is optimal:** `∀ M, n ≤ 6: score(chu_liu_edmonds(M)) ≥ max { score(h) | h forest } − 1e-3` (brute force over nⁿ assignments; generated scores are distinct with probability 1).
- **P3 CLE = greedy when greedy is a forest:** if the column-argmax vector is a forest, CLE returns it.
- **P4 cycles are exercised:** the generator is biased so that ≥ 25 % of the cases have a cycle in the greedy argmax (asserted by a counter in the test, audit 005).
- **P5 masked batch:** `masked_batch(<s>, </s>, <mask>, p)` has `n` rows of width `n + 3`; row `i` = `<s> p₁ … p_{i−1} <mask> p_{i+1} … pₙ </s> pᵢ`.
- **P6 length rule:** `fits(n, max_positions) ⇔ n + 3 ≤ max_positions`, and `n = 0` never fits (an empty piece list is a `DecodeError::Empty`, not a parse).
- **P7 decode yields a tree:** `∀` finite logits of shape `n×n×L` (n ≤ 8, L labels incl. label 0, ≥ 1 root label, ≥ 1 relation label, `goeswith`) and piece offsets strictly increasing on char boundaries: `DependencyTree::new(decode(..)?)` is `Ok`.
- **P8 exactly one root:** the tokens of P7 have exactly one `head == 0` (P7 implies it; asserted apart so the single-root plant fails a named check).
- **P9 labels:** every token's `upostag` is the first and `deprel` the last `|` field of some label in the configuration, never label 0's; the root's label ends in `|root` ⇔ it is the root.
- **P10 merge covers the pieces:** the merged tokens' byte ranges are disjoint, in order, and their union covers every piece range; each token's `form == sentence[range]`; a token spans several pieces only if every piece after its first was labelled `goeswith` with head the previous piece chain (`ud.py`'s rule).
- **P11 goeswith only rightwards and contiguous:** no token of the output has `deprel == "goeswith"` with a head to its right.
- **P12 syntax variants:** under `analyze_parsed` with a parser answering `TooLong` for sentences of more than k words and a tree otherwise: each sentence's `syntax` is `TooLong` ⇔ it has more than k words, else `Parsed`; the file's pooled distance equals the sum over `Parsed` only; `metric_lines` of a `TooLong` sentence end its four syntactic lines in `absent (too long for the model)`, of an `Unparsed` one in `absent (no parse)`.
- **P13 lemma fallback:** `parsed_nominalizations(tokens)` = the count over `NOUN` tokens of `is_nominalization(lemma')`, where `lemma' = surface_lemma(form)` if `lemma == "_"` else `lemma`; so for a parse with every lemma `_`, the count equals `surface_nominalizations` over the `NOUN` forms.
- **P14 one parse per sentence:** `with_parse(summarize(s), &analyze_parsed(s, P))` keeps every surface field of `summarize(s)` and takes `mean_dependency_distance = file.dependency_distance.mean()`, `passives = file.passives`, `nominalizations = file.nominalizations`; a counting parser is called exactly once per sentence by a whole `main`-style run (`examine`).

## Witnesses

- **CLE cycle:** `M = [[0, 5, 1], [5, 0, 1], [1, 1, 3]]` (0 and 1 prefer each other): greedy `[1, 0, 2]` is a cycle; CLE returns a forest of score ≥ every alternative (P2 on this matrix, stated as its own test).
- **Labels:** `id2label = {0: "-|_|dep", 1: "NOUN|_|nsubj", 2: "VERB|_|root", 3: "X|_|goeswith"}` → kinds `[label0, relation, root, relation]`, goeswith 3; a configuration without `X|_|goeswith` → `LabelError::NoGoeswith`; an `id2label` with a gap → `LabelError::Gap`.
- **Merge:** pieces `un`(0..2) `believ`(2..8) `able`(8..12) in `unbelievable`, labels goeswith on 2 and 3 with heads 1 and 2 → one token `unbelievable`.
- **Too long:** `fits(509, 512)`, `!fits(510, 512)`.
- **Model (tests/model.rs):** `vernier check --format compact --model-path $M tests/fixtures/sample.md` exits 1 and its line at `:7:1:` contains `CenterEmbedding: subject "proposal" separated from verb "caused" by 8 words` (the spike's ONNX parse equals `EXAMPLE_CONLLU` on every content head; MDD 32/12 ≈ 2.67 raises no `HighMdd`).
- **Model JSON:** `vernier analyze --format json --model-path $M tests/fixtures/sample.md` has `mean_dependency_distance` and `passives` non-null.

## Scenario coverage

| Spec criterion | Check |
| :--- | :--- |
| M3b 2 license recorded, never redistributed | done in 3d31b3c (spec, README); no download/bundle code exists (review); tick at close-out |
| M3b 3 `--model-path` readable → parse each sentence, M3a metrics | P7–P12, P14; `tests/model.rs` JSON witness; by-hand B1/B3 |
| M3b 4 example → center-embedding proposal/caused (skipped with reason without model) | `tests/model.rs::check_reports_the_examples_center_embedding` |
| M3b 5 no `--model-path` → surface metrics, one notice, surface-only exit code | `no_model_prints_one_notice_and_keeps_stdout` (T7), plus every existing `tests/cli.rs` stdout assertion unchanged |
| M3b 6 missing/unusable model → named on stderr, exit 2 | `a_missing_model_is_named_and_exits_2`, `a_model_without_its_tokenizer_is_named_and_exits_2`, `an_unloadable_runtime_is_named_and_exits_2` (T7); `onnx` unit load errors (T6) |
| M5 2 exit 2 on an unusable model, in every format | the T7 tests run with `--format text`, `compact` and `json` |
| M4 1 parse lemma falls back to the surface lemma | P13 (T3) |

## Planted violations (tick when the red was seen)

Each names the input where the mutant differs (audit 005).

| # | Plant | Must fail (input) | Seen |
| :--- | :--- | :--- | :--- |
| 1 | CLE returns the greedy argmax without contraction | P1 on any matrix with a greedy cycle (the cycle witness `[1, 0, 2]` is not a forest) | [ ] |
| 2 | contraction picks the cycle node by `argmin` instead of `argmax` when expanding the entering arc | P2 on the generator's cyclic cases (score below brute force) | [ ] |
| 3 | `masked_batch` puts `<mask>` at position `i` instead of `i + 1` (overwrites `<s>` on row 1) | P5 on any n ≥ 1 | [ ] |
| 4 | `fits` uses `n + 2` | P6 at `n = max − 2` (`fits(510, 512)` true); the "too long" witness | [ ] |
| 5 | root labels allowed off the diagonal (mask `kind` ignored for roots) | P9 (a non-root token with a `…|root` label, logits biased toward root labels) | [ ] |
| 6 | single-root fix skipped | P8 on logits with two strong diagonals (generator biases two diagonals high) | [ ] |
| 7 | goeswith constraint skipped (`r` all zero) | P11 on logits where goeswith is best leftwards | [ ] |
| 8 | merge ignores the "all pieces between are goeswith" condition | P10 (a token spanning a non-goeswith piece) on the generator's goeswith-heavy cases | [ ] |
| 9 | `form` from the unmerged first piece's range | P10 (`form ≠ sentence[range]`); the merge witness (`un` ≠ `unbelievable`) | [ ] |
| 10 | analysis maps `Parse::TooLong` to `Syntax::Unparsed` | P12 (`TooLong` expected); the metric line reads `absent (no parse)` || [x] |
| 11 | `analyze_parsed` sets `dependency_distance: None` when any sentence is `TooLong` | P12 on a document with one long and one short sentence (`Some` expected) || [x] |
| 12 | `parsed_nominalizations` ignores the fallback (`_` lemma stays `_`) | P13 on a `NOUN` `deliberation` with lemma `_` (1 ≠ 0) || [x] |
| 13 | `with_parse` keeps the surface nominalizations | P14 on a parse whose `NOUN` tagging differs from the surface words (the M4 `NOMZ_CONLLU` witness: 3 ≠ 4) | [ ] |
| 14 | `main` prints the no-model notice once per file (the per-file-load mutant is not observable without a model; this is its observable twin) | `no_model_prints_one_notice_and_keeps_stdout` with two files (2 notices ≠ 1) | [ ] |
| 15 | `main` exits 0 when the model fails to load (treats it as "no model") | `a_missing_model_is_named_and_exits_2` | [ ] |
| 16 | `main` lets ort load implicitly (no `init_from`) | `an_unloadable_runtime_is_named_and_exits_2` (panic → exit 101 ≠ 2) | [ ] |
| 17 | `onnx` checks `tokenizer.json` after the runtime | `a_model_without_its_tokenizer_is_named_and_exits_2` run with `ORT_DYLIB_PATH` pointing at a missing library (stderr names the runtime, not `tokenizer.json`) | [ ] |
| 18 | the wiring drops the parse (`examine` calls `analyze` even with a parser) | `tests/model.rs` with `VERNIER_TEST_MODEL` set: no `CenterEmbedding` line at `:7:1:` | [ ] |

## Coverage gap (run by hand)

- Everything model-backed runs only with `VERNIER_TEST_MODEL` and a runtime; `just verify` in a fresh clone skips it with a printed reason. By hand (B1–B4 below) with the model.
- B1 parity: a scratch crate in `.sdd/m3b-impl/` calls `OnnxParser` on `.sdd/m3b-spike/sentences.txt` and compares UPOS/HEAD/DEPREL per token with `.sdd/m3b-spike/out-onnx.conllu` (the spike's output, parity-checked against `ud.py`). Expected: identical.
- B2 chunking: the same with `CHUNK_ROWS` = n (unchunked). Expected: identical to B1.
- B3 timing: `time vernier check --model-path M tests/fixtures/sample.md` and `docs/specs/001-vernier/measurements/parser-sample.md`, recorded in *Measured* below (not a gate, BACKLOG 6).
- B4 a sentence over 509 pieces gives the notice and `absent (too long for the model)`.
- A system ONNX Runtime outside nix (a `cargo install` user): not tested; README (close-out) says `ORT_DYLIB_PATH` or a `libonnxruntime.so` ≥ 1.17 on the loader path.

## Snippets

```rust
// src/dependency.rs
pub enum Parse { Tokens(Vec<Token>), TooLong { pieces: usize, max: usize } }
pub trait Parser {
    type Error: std::error::Error;
    fn parse(&mut self, sentence: &str) -> Result<Parse, Self::Error>;
}

// src/analysis.rs
pub enum Syntax { Unparsed, TooLong { pieces: usize, max: usize }, Parsed(SentenceSyntax) }
impl Syntax { pub fn parsed(&self) -> Option<&SentenceSyntax> }      // Parsed(s) → Some(s)
pub struct SentenceAnalysis { .., pub syntax: Syntax, .. }
pub enum AnalysisError<E> {
    Parse { sentence_start: usize, source: E },                       // gains the position
    Malformed { sentence_start: usize, source: TreeError },
}
pub fn analyze_parsed<P: Parser>(source, format, thresholds, parser: &mut P) -> Result<FileAnalysis, AnalysisError<P::Error>>;

// src/mst.rs   (matrix[head][dep]; h[j] == j ⇔ j is a root)
pub fn chu_liu_edmonds(matrix: &[Vec<f32>]) -> Vec<usize>;
//   greedy = column argmax (first maximum, as numpy)
//   cycle_members(greedy) -> Option<(Vec<usize> cycle, Vec<usize> rest)>   // ud.py's two fixpoint passes, the cycle of the largest representative
//   contract(matrix, &cycle, &rest) -> Vec<Vec<f32>>                        // z = M − colmax; (r+1)×(r+1) block
//   expand(greedy, &cycle, &rest, sub_heads, z) -> Vec<usize>
pub fn argmax(values: impl Iterator<Item = f32>) -> usize;             // first maximum; NaN never wins

// src/decoder.rs
pub struct Labels { names: Vec<String>, kinds: Vec<LabelKind>, goeswith: usize }
pub enum LabelKind { Never /* label 0 */, Root, Relation }
pub enum LabelError { Json(serde_json::Error), NoId2Label, Gap { id: usize }, NoGoeswith, Malformed { label: String } }
impl Labels { pub fn from_config(json: &str) -> Result<Self, LabelError>; pub fn len(&self) -> usize }
pub struct ModelLimits { pub max_positions: usize }                     // max_position_embeddings − pad_token_id − 1
pub fn limits_from_config(json: &str) -> Result<ModelLimits, LabelError>;
pub fn fits(pieces: usize, max_positions: usize) -> bool;              // pieces > 0 && pieces + 3 <= max_positions
pub struct Special { pub cls: i64, pub sep: i64, pub mask: i64 }
pub fn masked_batch(special: Special, pieces: &[i64]) -> Vec<i64>;    // n rows × (n + 3), row-major
pub struct Piece { pub id: i64, pub range: Range<usize> }              // byte range, start < end
pub enum DecodeError { Empty, Shape { expected: usize, got: usize }, Offset { range: Range<usize> } }
pub fn decode(logits: Vec<f32>, n: usize, labels: &Labels, sentence: &str, pieces: &[Piece]) -> Result<Vec<Token>, DecodeError>;
//   logits[(i*n + j)*L + l] = batch output [row i, position j + 1, label l]  (ud.py e.logits[:, 1:-2, :])
//   steps (each a fn ≤ 10): mask_labels, goeswith_constraint, best_scores, single_root, merge_subwords, tokens

// src/onnx.rs  (shell)
pub struct OnnxParser { session: ort::session::Session, tokenizer: tokenizers::Tokenizer, labels: Labels, limits: ModelLimits, special: Special }
pub enum LoadError {
    MissingFile { path: PathBuf },
    Config { path: PathBuf, source: LabelError },
    Tokenizer { path: PathBuf, message: String },
    SpecialToken { name: &'static str },
    Runtime { path: PathBuf, source: ort::LoadDynamicError },
    Session { path: PathBuf, source: ort::Error },
}
pub enum ParseError { Tokenize(String), Inference(ort::Error), Output(String), Decode(DecodeError) }
impl OnnxParser { pub fn load(model_dir: &Path, runtime: &Path) -> Result<Self, LoadError> }
//   order: config.json, tokenizer.json, onnx/model.onnx exist and parse (no runtime needed) → init_from(runtime)?.commit() → Session::builder()?.commit_from_file(model)
impl Parser for OnnxParser { type Error = ParseError; fn parse(&mut self, s: &str) -> Result<Parse, ParseError> }
//   encode(s, true) → pieces with start < end → !fits → TooLong; else run CHUNK_ROWS rows at a time, keep logits[:, 1..=n, :], decode
pub const CHUNK_ROWS: usize = 16;

// src/summary.rs
pub fn with_parse(summary: FileSummary, file: &FileAnalysis) -> FileSummary;

// src/main.rs
//   runtime = env::var_os("ORT_DYLIB_PATH").filter(|v| !v.is_empty()).map_or("libonnxruntime.so".into(), PathBuf::from)
//   parser: Option<OnnxParser> = args.model_path.map(|dir| OnnxParser::load(&dir, &runtime)).transpose() — Err → eprintln + exit 2
//   None → eprintln!("vernier: no --model-path given; syntactic metrics skipped") once
//   examine: Some(p) → analyze_parsed(.., p) + with_parse; None → analyze + summarize (as today)
//   TooLong sentences → one stderr notice each; AnalysisError → "cannot parse the sentence at PATH:L:C: …", file = Unreadable
```

## Tasks

One task = one scenario = one commit (`git -c commit.gpgsign=false commit`, D13; the message states the problem and ends `— Spec 001 M3b Tn (full-spec)`); `just verify` must pass at every commit (the pre-commit hook runs it). Plan ticks go in the task's commit when it stays ≤ 5 files, else in a separate commit.
Split point if the session runs short: M3b.1 = T0–T5 (pure core, lands green with no CLI change), M3b.2 = T6–T10.

| ID | Scenario | Files | RED (must fail first) | GREEN | Status |
| :--- | :--- | :--- | :--- | :--- | :--- |
| T0 | plan, ADR, devenv runtime | this plan, `docs/adr/0001-onnx-runtime-via-ort.md`, `devenv.nix` | — (docs, env) | — | done |
| T1 | refactor: `Option<SentenceSyntax>` → `Syntax { Unparsed, Parsed }` | `src/analysis.rs`, `src/diagnostic.rs` | — (refactor). Evidence: every existing test green before and after with unchanged assertions (`None` → `Syntax::Unparsed`, `Some(x)` → `Syntax::Parsed(x)` in literals and matches) | the enum, `Syntax::parsed()` | done |
| T2 | the seam: `&mut self`, `Parse::TooLong`, `Syntax::TooLong`, parse-error position | `src/dependency.rs`, `src/analysis.rs`, `src/diagnostic.rs` | P12 with a `TooLongParser`, `a_too_long_sentence_reads_absent_too_long_for_the_model`, `a_parse_error_names_the_sentence_start`; plants 10, 11. **Changed test code (not assertions):** the test parsers implement `parse(&mut self)` and wrap tokens in `Parse::Tokens` | trait, enums, `analyze_parsed(&mut P)` | done |
| T3 | M4 lemma fallback | `src/nominalization.rs` | P13 against today's code (a `_`-lemma `NOUN` `deliberation` counts 0); plant 12 | `_` → `surface_lemma(form)` | done |
| T4 | Chu-Liu/Edmonds | `src/mst.rs`, `src/lib.rs` | P1–P4 and the cycle witness against `chu_liu_edmonds → greedy`; plants 1, 2 | port of the spike's `chu_liu_edmonds`, split | todo |
| T5 | decoder | `src/decoder.rs`, `src/lib.rs`, `Cargo.toml` (none if `serde_json` suffices) | P5–P11, label/merge/too-long witnesses against `decode → Err(Empty)`; plants 3–9 | `Labels`, `fits`, `masked_batch`, `decode` | todo |
| T6 | ONNX shell | `Cargo.toml`, `Cargo.lock`, `src/onnx.rs`, `src/lib.rs` | `onnx` unit load errors (missing dir, missing `tokenizer.json`, bad `config.json`) against `load → Err(MissingFile(dir))` for all; with `VERNIER_TEST_MODEL`: `parses_the_example_into_a_tree` | `ort` + `tokenizers`, `OnnxParser` | todo |
| T7 | CLI wiring | `src/main.rs`, `src/summary.rs`, `tests/cli.rs` | P14; `no_model_prints_one_notice_and_keeps_stdout`, `a_missing_model_is_named_and_exits_2` (text/compact/json), `a_model_without_its_tokenizer_is_named_and_exits_2`, `an_unloadable_runtime_is_named_and_exits_2`; plants 13–17. **Changed test:** `check_accepts_every_m5_flag_and_passes_without_rules` drops `--model-path none.udpipe` (reason in the decisions table) | load once, notices, `with_parse`, exit 2 mapping | todo |
| T8 | model-backed integration tests | `tests/model.rs` | `check_reports_the_examples_center_embedding`, `analyze_json_fills_the_parse_metrics`; T7's wiring precedes them, so their RED is plant 18, seen with the model; without it they print `skipped: …` | — | todo |
| T9 | by-hand checks B1–B4, measured times | this plan (*Measured*) | — | — | todo |
| T10 | close-out (main) | `spec.md` (ticks M3b 2–6, Status), `README.md` (model dir layout, `ORT_DYLIB_PATH`, runtime ≥ 1.17, notices), `docs/HANDOVER.md`, this plan (ticks) | `just verify`; every plant seen red; B1–B4 | — | todo |

## Measured

(T9; filled after the by-hand runs.)
