# Audit 017 — a clarification answer never reached a criterion

**Severity:** medium
**Tags:** spec, clarification, process, review

## Symptom
The user answered Q-B on 2026-09-26: suggestions are "fixed advice per rule" ([open-questions report](../specs/001-vernier/2026-09-26-vernier-open-questions-answered.md), Q-B, with a `= help:` line as its example).
Spec 001 carried the answer only in Non-Goal 3 and in Provenance; no criterion required advice, so no code printed it, and every milestone was ticked without it.
Review pass 2 found it (R2-B4), four milestones later.

## Root cause
The answer was written into the spec as a restriction (a Non-Goal: "never generated sentences") and as history (Provenance), which both read as done; the positive half, "there is fixed advice per rule", became nobody's criterion.
The Clarification gate checked that every `[NEEDS CLARIFICATION]` marker was gone, not that every answer had become a requirement, so removing the marker counted as carrying the answer.
The plans map criteria to tasks, so a requirement that is not a criterion never reaches a plan, a test or the code.

## Rule
At the Clarification gate, map each answer to the named criterion (milestone and number) or Non-Goal that carries it, and record the map in `gates.md`; an answer that asks for behaviour ("there is advice") needs a criterion, a Non-Goal carries only what the tool does not do.
An answer without a named carrier fails the gate.
Repaired by M5 criterion 9 (one `= help:` line per flag, fixed text per flag, JSON `help`), tested by `each_flag_carries_its_fixed_advice`, `the_text_ends_with_one_help_line_per_flag` and `the_text_gives_each_flags_advice_in_order` (src/diagnostic.rs), the JSON property in src/json.rs and `check_prints_a_cognitive_overload_diagnostic` (tests/cli.rs).
