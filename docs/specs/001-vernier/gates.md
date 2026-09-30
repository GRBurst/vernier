# Gate record — spec 001

Append-only (A2). One dated block per gate verdict; a re-decided gate adds a new block.
Verdict is `PASSED`, `FAILED` or `NOT_RUN` (with reason).

## Review — pass 1

- Verdict: NOT_RUN
- Date:
- Reviewer: fresh context (name the model)
- Blocking findings:
- Residue (left unrepaired, with where it went):

## Clarification

- Verdict: NOT_RUN
- Date:
- Decided by:
- Open questions: <count>, one line each: question → outcome (decided / answered / deferred to BACKLOG item / withdrawn)

## Spec-Approval

- Verdict: NOT_RUN
- Date:
- Decided by:

## Comprehension

- Verdict: NOT_RUN
- Date:
- Decided by:
- Questions (written by a fresh-context reviewer):
- The user's own words:

## Phase 2 — skipped by user direction (2026-09-26)

- Verdict: NOT_RUN — the user said "continue directly with implementation" at the start of the session.
- Date: 2026-09-26
- Decided by: the user
- Consequence: the spec stays `Draft`; each `[NEEDS CLARIFICATION]` marker is implemented with its stated fallback and stays open for a batched answer.
  Review, Clarification, Spec-Approval and Comprehension remain owed before any milestone is marked `DONE`.
- Friction recorded (A7): `docs/BACKLOG.md`, item "Phase 2 owed for spec 001".

## Clarification — 2026-09-26

- Verdict: PASSED
- Date: 2026-09-26
- Decided by: the user (markers 1, 9, 11 directly; the other ten by accepting the research answers in `2026-09-26-vernier-open-questions-answered.md` and asking for them to be written into the spec)
- Open questions: 13
  - 1 suggestions: generated rewrite or fixed advice → decided: fixed advice per rule
  - 2 authoritative sentence split → answered: UAX #29 per block, abbreviation merge list
  - 3 renumber content tokens → answered: renumber; PUNCT heads re-attached
  - 4 headings and alt text → answered: dropped (as implemented)
  - 5 plain text by extension → answered: `.md`/`.markdown`, any case (as implemented)
  - 6 syllable reference → answered: 500 frequent CMUdict words, ≥ 90 %
  - 7 proper nouns in complex words → answered: capitalization rule; suffix subtracted only when it is a syllable
  - 8 depth in edges or nodes → answered: edges, root = 0
  - 9 model license → decided: personal, non-commercial use; user-supplied CC BY-NC-SA UDPipe 1 model accepted; route still chosen by the M3b spike
  - 10 no model in CI → answered: no `--model-path` = surface only + notice; unusable `--model-path` = exit 2
  - 11 downloader → decided: none in this spec (new Non-Goal)
  - 12 nominalization false positives → answered: lemma, length ≥ 7, NOUN, pybiber-seeded stoplist
  - 13 diagnostic per sentence or per rule → answered: per sentence

## Review — pass 1 (2026-09-30)

- Verdict: FAILED
- Date: 2026-09-30
- Reviewer: fresh-context Claude (`spec-review1`); repairs applied by `spec-repair` on main's decisions of 2026-09-30.
- Blocking findings (finding → repair):
  - B1 the intro example contradicted M3b 5 and M5 1 (no `--model-path`, invented layout, 3 of 5 metrics) → replaced by the verbatim output of a run with `--model-path` (all five metric lines, exit 1); only the paths and the line are illustrative.
  - B2 sentences too long for the model were unspecified → new M3b criterion 1 (limit from `config.json`, surface metrics kept, one stderr notice with `PATH:LINE:COL`, pieces and limit, `absent (too long for the model)`); M3b 3 "within the model's limit"; M3a 1 "its tokens or that it is too long".
  - B3 the exit contract was incomplete → M5 2 lists every exit-2 cause (unreadable or non-UTF-8 file, model or ONNX Runtime not loadable, a sentence that cannot be parsed, named as `PATH:LINE:COL`), 2 over 1, usage error 2; the unreachable JSON-serialization exit is an Implementation Detail.
  - B4 "self-contained" and "readable model" were untrue or undefined → intro drops "self-contained"; Scope names ONNX Runtime ≥ 1.17 (`ORT_DYLIB_PATH` or the loader path); new Definition *Model*; M3b 6 names the directory and cause, processes no file, exits 2.
  - B5 M3b 2 was untestable and stale ("BEFORE model download", "only where the license permits") → README and spec state the license and the non-commercial training-data caveat; no-redistribution stays with the Non-Goals.
  - B6 MDD and depth assumed one root → Definitions *projected root* and *Content dependency*; M3a 2–4 divide by the number of content dependencies, the file MDD pools parsed sentences, depth counts from a projected root, MDD absent without a content dependency.
  - B7 center-embedding's "the subject's head verb" and "a clausal dependent" were ambiguous → M3a 6 as built: subject precedes its head in the content projection (any part of speech), the subtree of some content token with a clausal relation lies strictly between them.
  - B8 per-sentence values were promised but not output → intro and Scope: vernier reports each file's metrics and every flagged sentence; per-sentence values are library API (`analysis::analyze*`); M2 lead, M4 2 and M4 3 say so.
  - B9 the CLI surface was never listed → new M5 criterion 8 (both subcommands, all options and defaults as `--help` shows); M1 8 reduced to `clap` derive.
  - B10 the file-level metrics were not enumerated → new Definition *File metrics* (table label ↔ JSON key, when absent); M5 3 gives the JSON schema, M5 5 refers to the definition.
  - B11 the `-es` suffix rule read as stem+`ce`+`es` → M2 7 lists the literal endings (`ces`, `ges`, …, as in `sentences`, `packages`).
- Non-blocking findings (finding → repaired where):
  - N1 "words strictly between" ≠ *Word* → M3a 6 counts parser tokens whose form has a letter or digit.
  - N2 stale parentheticals → removed from M5 2, M3b 2, M1 8.
  - N3 UDPipe remnants in Provenance → marked superseded; the UDPipe-backend conditions (example test with GUM/ParTUT, README warning, model license) live in BACKLOG item 10.
  - N4 non-EARS FOR/BEFORE criteria → M1 4, M2 3, M3a 9, M5 6 ubiquitous; M3b 1 (spike) moved to Provenance as a fact (`7f60be9`).
  - N5 "at least 7 letters long" applied to the word → M4 1: the lemma has at least 7 letters.
  - N6 *Prose span* did not fit plain text; M1 2 omitted character references and images → both repaired.
  - N7 "readable" undefined → M1 6/7: readable as UTF-8; not UTF-8 is unreadable.
  - N8 center-embedding has no maximum → M5 1 "its maximum where it has one, and the flag, if raised"; `none`, too-long and no-dependency lines stated.
  - N9 colour rule unverified → M5 Implementation Detail marked unverified on a terminal (by-hand check owed, HANDOVER).
  - N10 clauses count content tokens only → M3a 5.
- Consequence: M3b is `IN PROGRESS`; its new criterion 1 stays `[ ]` because no automated test covers the stderr notice line (`eprintln!` in `src/main.rs` `process`). A by-hand run on 2026-09-30 printed it as specified. A unit test is owed; main is adding it.
- Residue: the user still has to confirm the as-built choices behind B6/B7 (HANDOVER, "M3a decisions to confirm"). A re-review (pass 2) is owed.

## Review — pass 1, follow-up (2026-09-30)

- The pass-1 repair (8723cc6) had written three code defects into the spec as built; that weakened a requirement (A2), so the code was fixed instead and the intended requirements restored:
  - passive voice was 0, not absent, when no sentence was parsed → fixed, M4 4 restored (5623baa, 84afe70, audit 014);
  - an absent MDD named the wrong reason in the table and the diagnostic → typed `Absence` (`no parse`, `too long for the model`, `no content dependency`), labels bound in File metrics, M3a 3 and M5 1 (54c7592, b9e0fa4, audit 015).
- The owed unit test for M3b criterion 1's notice exists (`each_too_long_sentence_gets_the_specified_notice`, b7bbf3d); M3b is `IMPLEMENTED` again.
- An empty file analyzed with a model reports MDD and passive voice absent (`no parse`): no sentence was parsed (main, 2026-09-30).
- Next: pass 2 by a fresh-context reviewer.

## Review — pass 2 (2026-09-30)

- Verdict: FAILED
- Date: 2026-09-30
- Reviewer: fresh-context Claude (`spec-review2`) at `f787be0`; repairs applied by `pass2-fix` on main's decisions of 2026-09-30.
- Blocking findings (finding → repair (commit)):
  - R2-B1 a closed stdout panicked with exit 101 (`println!`) → every stdout write goes through a fallible writer, a failed write exits 2 with one stderr line and no panic, M5 2 says so; tests `a_closed_stdout_exits_2_without_a_panic` (tests/cli.rs, seen 101 first) and `a_failed_write_exits_2` (src/main.rs); audit 016 (bf80c10); `clippy::print_stdout` denied outside tests (a9c1dfa).
  - R2-B2 "exit 0" (M1 6, M5 5) and "SHALL print one JSON document" (M5 3) contradicted the exit-2 paths → "unless M5 criterion 2 requires 2"; M5 3 applies when no usage error or model-load failure ends the run, and a file that cannot be read or has a sentence that cannot be parsed has no object (aa002d5); the `analyze` and `check` help say they exit 2 on an error (1f33921).
  - R2-B3 block prose also joins directly across a backslash escape → Definitions *Block prose* names it (`a\*b` reads `a*b`, verified in `inline_markup_joins_spans_directly`).
  - R2-B4 the user's Q-B answer "fixed advice per rule" had no criterion → new M5 criterion 9: one `= help:` line per flag in flag order, a fixed text per flag stated verbatim, `help` in JSON flags (schema_version 1), compact unchanged; the intro example re-run; audit 017 (a0d5e69).
  - R2-B5 M3a 3's "never `no parse`" covered files without a parsed sentence → the reason `no content dependency` is for a parsed sentence and a file with at least one parsed sentence; a file without one reads `no parse`.
  - R2-B6 `end_line`/`end_column`, `code` and `message` were unstated → M5 3 states them; stating the end exposed a defect: the end landed past closing markup when more prose followed (`*hi.* Then`), so the last character is mapped now; the M5 6 property covers ends and the one-line underline; audit 018 (aa002d5).
  - R2-B7 as-built choices on projected roots, depth and center-embedding were unconfirmed → one `[NEEDS CLARIFICATION: … Fallback: as built.]` marker each in M3a 2, 4 and 6, quoting the text before review pass 1; Provenance points at them.
- Non-blocking findings (finding → repair):
  - R2-N1 normative text without SHALL, unwanted behaviour as WHEN → SHALL added in M3a 3, M3b 1, M4 1, M4 4, M5 1, M5 3; M1 7, M2 4, M3a 8, M3b 6 use IF … THEN.
  - R2-N2 usage error and unparsable-sentence exit 2 untested → `a_usage_error_exits_2` (tests/cli.rs) and `a_sentence_that_cannot_be_parsed_exits_2` (src/main.rs, a test parser, no model) (bf80c10).
  - R2-N3 the model's label form was wrong (FEATS holds `|`) and incomplete → Definitions *Model*: labels 0 to n − 1 without a gap, UPOS before the first `|`, DEPREL after the last, label 0 reserved, goeswith, a root and another relation label required, the position limits.
  - R2-N4 *Block prose* covered only paragraphs → new Definition *Block*: a paragraph or a tight list item's text, split at every block-level start or end; in plain text a run of non-blank lines.
  - R2-N5 the tree check was attributed to the metric functions → M3a 8: `DependencyTree::new` returns the error (and lists all five causes).
  - R2-N6 the diagnostic's "verb" can be an adjective; the help said "from the root" → M3a 6 says a diagnostic calls the head the verb whatever its part of speech; the `--max-tree-depth` help says "from a projected root" and M3a 4 requires it, tested by `max_tree_depth_help_says_edges` (1f33921).
  - R2-N7 M5 3 "per file that was read and reported" narrowed plan-M5's "per readable file" → decision (main, 2026-09-30): a file with a sentence that cannot be parsed is not reported; exit 2.
  - R2-N8 Provenance omitted pass 1's follow-up → a line names audits 014 and 015 and the notice test.
  - R2-N9 the `no parse` causes omitted a file without sentences analyzed with a model → File metrics rows and M4 4 name "the file has no sentence".
- Repairs without a commit named are spec text, in the commit that adds this section.
- Consequence: every criterion keeps its tick; each is verified by a named test, the new M5 9 by `each_flag_carries_its_fixed_advice`, `the_text_ends_with_one_help_line_per_flag`, `the_text_gives_each_flags_advice_in_order` and `check_prints_a_cognitive_overload_diagnostic`.
- Residue: the three M3a markers await the user; a re-review (pass 3) is owed.
- Amended (main, 2026-09-30): M5 9 prints one `= help:` line per distinct flag kind, in first-occurrence order, so two center-embeddings get one line; JSON keeps `help` on every flag; test `two_center_embeddings_get_one_help_line` (src/diagnostic.rs).

## Review — pass 3 (2026-09-30)

- Verdict: FAILED
- Date: 2026-09-30
- Reviewer: fresh-context Claude (`spec-review3`, fresh context) at `cd086d3`; repairs applied by `pass3-fix` on main's decisions of 2026-09-30.
- Blocking findings (finding → repair (commit)):
  - R3-B1 a failed stderr write panicked with exit 101 (`eprintln!`; `2>/dev/full`, a closed stderr pipe) → every stderr write goes through a fallible writer (`say` in src/main.rs), a failed one is dropped and leaves the exit code to the other clauses; M5 2 has an IF … THEN clause for it; `clippy::print_stderr` denied outside tests and listed in ENGINEERING §8; tests `a_failed_stderr_write_keeps_the_exit_code` (tests/cli.rs: a closed stderr pipe and `/dev/full` for `check`, `check --format json` and `analyze` on a missing file, and `check` on a flagged file; seen 101 first) and its twin in src/main.rs (a refusing writer); audit 019 extends audit 016's rule to every standard stream (d30a683).
  - R3-B2 the load checked only part of *Model*: a `config.json` that did not match the graph passed, and every file then failed at its first sentence → *Model* states the graph contract (inputs exactly `input_ids` and `attention_mask`, a tensor output `logits` whose last dimension, where the graph states it, is the label count; a dynamic one is checked on every parse); `OnnxParser::load` checks it through the session's metadata (`check_graph`, `LoadError::Graph`), which states the width for the tested model (`[-1, -1, 2561]`), so M3b 6 holds as written; tests `a_graph_that_breaks_the_contract_is_refused`, `a_graph_with_the_contracts_inputs_and_width_fits` (src/onnx.rs), `a_model_whose_labels_do_not_match_its_logits_is_refused` (tests/onnx.rs, a four-label copy of `VERNIER_TEST_MODEL` under `CARGO_TARGET_TMPDIR`, seen loading first) and `a_model_whose_labels_do_not_match_its_graph_processes_no_file` (tests/model.rs); audit 021 (ed4c071).
  - R3-B3 *Model* allowed `max_position_embeddings` > `pad_token_id` + 1, so M3b 1's limit could be negative and was clamped to `max 0` → *Model* requires `max_position_embeddings` > `pad_token_id` + 4 (a limit of at least 1); `limits_from_config` refuses anything else as unusable (exit 2); tests `a_limit_below_one_piece_is_refused` (src/decoder.rs, a law over positions and pad ids) and `a_model_without_room_for_a_piece_is_named_and_exits_2` (tests/cli.rs), both seen failing first; audit 020 (ffe2727).
- Non-blocking findings (finding → repair):
  - N1 no test asserted `vernier: cannot write to stdout: <cause>` → `a_closed_stdout_exits_2_without_a_panic` asserts exactly that line (the cause of `EPIPE`) besides the no-model notice, and `a_failed_write_exits_2` asserts the whole stderr (d30a683).
  - N2 "a closed stdout exits 2" was false for `>&-` → Provenance's pass-2 line reads "a write to a stdout pipe whose reader has gone exits 2"; M5 2 says a stdout or stderr closed by the shell reads as `/dev/null`, verified by `a_closed_descriptor_reads_as_dev_null` (tests/cli.rs, `>&-`, `2>&-` and both: exit 1) (d30a683).
    The pass-2 block's R2-B1 line above ("a closed stdout panicked") is to be read as "a write to a stdout pipe whose reader has gone panicked".
  - N3 the underline counts display columns → M5 6 says so (a wide character takes two carets; line and column count scalar values); witness `the_underline_counts_display_columns` (src/diagnostic.rs: `漢字 is here now.`, columns 1 to 16, 17 carets) (b49b19a).
  - N4 M3a 6's marker bundled three decisions; M3a 5 "content tokens only" had none → three markers in M3a 6 (subject before head; head of any part of speech; tokens counted by letter or digit) and one in M3a 5, each quoting the text before review pass 1 with `Fallback: as built`.
  - N5 M5 3 "read and reported" was main's narrowing, not the user's → a marker in M5 3 (plan-M5 said every readable file; `Fallback: as built`); M5 3 names "parsed or located".
  - N6 M5 8 → "two subcommands besides clap's `help`", "as `vernier <COMMAND> --help` shows their names and defaults" (the `--max-mdd` bound is not in the help; `rejects_a_non_positive_or_non_finite_max_mdd` tests it).
  - N7 the abbreviation match was unstated → *Sentence*: the segment's last whitespace-separated token, leading non-alphanumerics stripped, equals a listed abbreviation case-sensitively (`(Dr.` merges; `etc.)` and `no.` end a sentence); witnesses `each_listed_abbreviation_merges`, `lowercase_no_does_not_merge` and the new `an_abbreviation_before_a_closing_bracket_ends_the_sentence` (src/sentence.rs, b49b19a).
  - N8 *Block prose* / *Block* → "every character of a span keeps its source position, while an inserted space has none and is never a sentence's first or last character"; a plain-text block is "one byte range of the file".
  - N9 `ExamineError::Locate` was an unlisted exit-2 cause → M5 Implementation Details list it (`cannot locate a sentence in PATH: <cause>`) beside the JSON-serialization exit, both unreached by a test.
  - N10 M4 3 → after a form the parser changed, a next form that is a prefix of the changed token's word, or equal to it, matches at that word, so a passive's position can be another word's (audit 009; `the_known_limit_of_a_substitution`).
  - N11 EARS → M3a 6 "a diagnostic SHALL call the head the verb"; M5 2 opens "WHEN no sentence is flagged, … SHALL exit 0, and WHEN at least one sentence is flagged, it SHALL exit 1"; M3b 2's model-license facts moved to Provenance, the criterion says the spec states them there.
- Repairs without a commit named are spec text, in the commit that adds this section.
- Consequence: every criterion keeps its tick, each verified by a named test (above, and in the pass-1 and pass-2 blocks); `just verify` passes, and the model-gated tests pass with `VERNIER_TEST_MODEL` set.
- Residue: seven `[NEEDS CLARIFICATION]` markers await the user (M3a 2, 4, 5, three in 6; M5 3); a re-review (pass 4, the last PROCESS allows) is owed.
- Amended (main, 2026-09-30): the graph contract of *Model* includes the element types (`input_ids` and `attention_mask` tensors of int64, `logits` a tensor of float32), checked at load by `check_graph` (`GraphError::InputType`, `GraphError::LogitsType`); tests `a_graph_that_breaks_the_contract_is_refused` and `a_graph_with_the_contracts_inputs_types_and_width_fits` (src/onnx.rs); the same gap as R3-B2, so audit 021 covers it.

## Review — pass 4 (2026-09-30)

- Verdict: FAILED
- Date: 2026-09-30
- Reviewer: fresh-context Claude (`spec-review4`, fresh context) at `582b9b1` (the same tree as `c6c3f15` after a history rewrite; review pass 1's commit `8723cc6` is now `f2d5974`); repairs applied by `pass4-fix` on main's decisions of 2026-09-30.
- Blocking findings (finding → repair (commit)):
  - R4-B1 M5 1 "underlining the sentence over all its lines" was false for a sentence over 8 or more source lines, which annotate-snippets folded (`...`) because the whole file went to `Snippet::source` with default folding → the renderer gets only the sentence's lines, with `line_start` and folding off (`sentence_lines` in src/diagnostic.rs); tests `check_shows_every_line_of_a_tall_sentence` (tests/cli.rs, `tests/fixtures/tall.md`, a sentence over 14 lines, every line shown and no `...`; seen folded first) and the property `the_text_shows_every_line_of_the_sentence` (src/diagnostic.rs, 1 to 30 lines; seen failing at 8 with folding on); audit 022 (4225653).
  - R4-B2 M5 2's stdout-failure clause was false for clap's help and version (`vernier check --help >/dev/full` and `vernier --version >/dev/full` exited 0 with an empty stderr) → `main` parses with `Cli::try_parse`, and `show_refusal` (src/main.rs) writes clap's rendering through vernier's writers: help and version on stdout (exit 0, or exit 2 with `vernier: cannot write to stdout: <cause>`), a usage error on stderr (exit 2), colour chosen as clap chooses it (`anstream`, `ColorChoice::Auto`); `clap::Parser::parse`, `parse_from` and `clap::error::Error::exit`, `print` are denied in clippy.toml and listed in ENGINEERING §8; M5 2 names help and version and clap's message on stderr; tests `help_or_version_into_a_failed_stdout_exits_2` (tests/cli.rs, eight command lines into a closed pipe and `/dev/full`; seen exiting 0 first), `help_or_version_is_clap_s_text`, `a_usage_error_is_clap_s_text_on_stderr` (tests/cli.rs), `help_or_version_is_written_through_the_fallible_stdout` and `a_usage_error_is_written_through_the_fallible_stderr` (src/main.rs) (072d30f); audit 023, not audit 019 again: 019's rule was right and applied to every write vernier makes, while these writes were clap's, which its lint cannot reach (d23f454).
  - R4-B3 M5 2's closed list of exit-2 causes omitted two that Implementation Details and M5 3 name → M5 2 lists "the JSON document cannot be serialized, or a sentence cannot be located in its file (these two: Implementation Details; no test reaches either)" (5679347).
  - R4-B4 M3b 6 "names a directory that is not a usable model" had narrowed the text before review pass 1, "a missing or unusable model" (`git show f2d5974^:docs/specs/001-vernier/spec.md`, line 132), and a regular file was reported as `does not exist` → M3b 6 covers a path that does not exist, a path that is not a directory and a directory that is not a usable model, each named with its cause; `LoadError::NotADirectory`, and `LoadError::NotAFile` for a model file that is not a regular file, from one metadata look (`kind_of` in src/onnx.rs); tests `a_missing_or_non_directory_model_path_names_its_cause` (tests/cli.rs, the exact stderr line for a missing path and a regular file; seen saying `does not exist` for the file first), `a_model_path_that_is_a_file_is_not_a_directory` and `a_model_file_that_is_a_directory_is_not_a_file` (tests/onnx.rs); audit 024, whose rule compares a repair's criterion diff against the text before review pass 1 (f91a6d0; spec text 5679347).
  - R4-B5 Scope's per-sentence values as library API only was main's decision in pass 1 (B8) without a marker, while before review pass 1 the spec said vernier "reports how hard each sentence is to read" and M2 and M4 said "per sentence and per file" → a `[NEEDS CLARIFICATION]` marker in Scope, in the reviewer's words, `Fallback: as built` (5679347).
- Non-blocking findings (finding → repair (commit)):
  - N1 M3b 1 "the sentence SHALL NOT change the exit code" was untested → `a_too_long_sentence_keeps_the_exit_code` (src/main.rs: a parser that finds every sentence too long; `check` 1 on `long.md`, else 0, the same codes as without a parser, in every format; seen failing with a too-long sentence counted as flagged) (4d2b3f1).
  - N2 *Position* did not say where a line ends → "A line ends after each `\n`; a `\r` is a character of its line", verified by `every_boundary_converts_to_the_scanned_position` (src/position.rs, lone `\r` among the generated line ends) (5679347).
  - N3 M5 6 did not say how a tab is shown → a tab is shown as four spaces and takes four carets, whatever its column; witness `a_tab_is_shown_four_columns_wide` (src/diagnostic.rs) (4d2b3f1; spec text 5679347).
  - N4 EARS → M5 2 "IF the shell closes stdout or stderr (`>&-`, `2>&-`), THEN the tool SHALL write to it as to `/dev/null`" (`a_closed_descriptor_reads_as_dev_null`); M4 3 "each passive SHALL also carry"; M4 1 "WHEN a sentence has no parse … WHEN it has a parse" (5679347).
  - N5 M4 3's known limit (audit 009) cannot arise with the ONNX parser, whose forms are slices of the sentence (`tokens` in src/decoder.rs; property P10 in src/decoder.rs) → M4 3 says so (5679347).
  - N6 M5 3's marker covered JSON only, while such a file is reported in no format (`process` returns no report; `a_sentence_that_cannot_be_parsed_exits_2` checks every format) → the marker in the reviewer's words (5679347).
  - N7 *Model*'s per-parse check was untested and compares the whole shape → *Model* states `[rows, width, labels]`; `check_logits`, extracted unchanged from `run_rows`, is tested by `a_parse_checks_the_whole_shape_of_the_logits` (src/onnx.rs) (4d2b3f1; spec text 5679347).
  - N8 *Block prose* names strikethrough, superscript and subscript, which never occur because src/prose.rs enables only tables, math and the two metadata blocks → *Block prose* says so, and M1 Implementation Details say no other option is enabled (5679347).
  - N9 `docs/examples/github-actions.yml` said exit 2 "when a file cannot be read" → "2 on any error (spec 001 M5 criterion 2)"; `tests/examples.rs` stays green (5679347).
- Consequence: every ticked criterion is verified by a named test (above, and in the pass-1 to pass-3 blocks), except M5 2's two exit-2 causes of R4-B3, which the criterion itself says no test reaches; `just verify` passes, and the model-gated tests pass with `VERNIER_TEST_MODEL` set.
- Residue: see the next section.

## Review — stopped after pass 4 (2026-09-30)

- Verdict: FAILED (PROCESS: stop after four passes; there is no pass 5).
- The residue goes to Clarification with the user:
  - The pass-4 repairs above, which no review has re-checked: R4-B1 (4225653), R4-B2 (072d30f, d23f454), R4-B3 (5679347), R4-B4 (f91a6d0, 5679347), R4-B5 (5679347), N1, N3 and N7 (4d2b3f1), N2, N4, N5, N6, N8 and N9 (5679347), and this record.
  - Eight open `[NEEDS CLARIFICATION]` markers in `spec.md`, each `Fallback: as built`:
    1. Scope, per-sentence values: the command line shows a sentence's values only in its diagnostic; before pass 1 the spec reported every sentence.
    2. M3a criterion 2: several projected roots; MDD divides by N − R and is absent without a content dependency (before pass 1: N − 1, one root).
    3. M3a criterion 4: depth counts from a projected root, content tokens only (before pass 1: from the root, any token).
    4. M3a criterion 5: only content tokens count as clauses (before pass 1: any token).
    5. M3a criterion 6: the subject must precede its head (before pass 1: either order).
    6. M3a criterion 6: the head may be of any part of speech and is still called the verb (before pass 1: the head verb).
    7. M3a criterion 6: the distance counts parser tokens with a letter or digit, not *Word*s (before pass 1: words).
    8. M5 criterion 3: a readable file with a sentence that cannot be parsed or located is reported in no format (plan-M5: the JSON lists every readable file).
