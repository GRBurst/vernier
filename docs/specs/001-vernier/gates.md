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
