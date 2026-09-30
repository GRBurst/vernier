# Audit 013 — criteria ticked as met while their text had drifted from as-built decisions

**Severity:** medium
**Tags:** spec, plan, process

## Symptom
Adversarial review pass 1 of spec 001 (2026-09-30, [gates.md](../specs/001-vernier/gates.md)) found eleven blocking defects in a spec whose M1–M5 criteria were all ticked `[x]`.
Several ticked criteria described behaviour the code does not have:
- M3a 2 divided by N − 1, while the code divides by the number of content dependencies, because the M3a plan made a content token with only `PUNCT` ancestors a projected root, so a sentence can have several roots.
- Nothing specified sentences too long for the model, although the M3b plan decided to skip them with a notice and `absent (too long for the model)`.
- M5 2 left out exit-2 causes and the rule that 2 wins over 1, although the code and `tests/cli.rs` implement both.
- The intro example showed an output layout and an exit code that no run produces.

## Root cause
Each of these decisions was made in a plan (`plan-M3a.md` "Decisions made", `plan-M3b.md`) or in code. The plan recorded it, but the criterion text stayed as it was.
When a criterion was ticked, its tests were checked, but its text was not re-read against the behaviour: the tests passed because they tested the as-built behaviour, not the criterion's words.
The intro example was marked illustrative and never compared with a run.

## Rule
If a plan decision changes observable behaviour or a definition, the criterion text is revised in the same commit (in place while the spec is `Draft`, else as a delta).
Before ticking a criterion, re-read its text against the behaviour. Tick it only if the text, not just the tests, is true of the code.
Repaired in the review pass 1 commit of 2026-09-30.
