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
