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
