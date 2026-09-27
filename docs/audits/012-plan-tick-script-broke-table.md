# Audit 012 — a scratch tick script broke the M3b plan's planted-violation table

**Severity:** low
**Tags:** plan, tooling, markdown

## Symptom
In M3b, plants 1–12 were ticked by a scratch script (`.sdd/tick.py`). It wrote `… || [x] |` into rows 1–12 of the planted-violation table in [plan-M3b.md](../specs/001-vernier/plan-M3b.md). That put an empty cell into each of those rows, so they had one more column than the header.
The mistake was committed in the T2–T5 plan ticks and seen only while ticking plants 13–17 in T7.

## Root cause
The script cut the six characters ` [ ] |` from the end of a row and then appended `| [x] |`. The `|` it kept was doubled.
The script's output was never checked, and `just spec-check` does not count table columns.

## Rule
After editing a table with a script, look at its diff before committing (`git diff --word-diff`).
Fixed in M3b T7 (`ccadc73`): the rows read `| [x] |` again, and the script now appends ` [x] |`.
