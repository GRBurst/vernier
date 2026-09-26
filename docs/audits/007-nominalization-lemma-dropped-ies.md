# Audit 007 — the nominalization lemma rule dropped the researched `-ies` case

**Severity:** low
**Tags:** spec, nominalization, research

## Symptom
While planning M4, the first criterion read "without a parse: the word with a plural `-s` removed".
Under that rule `activities` becomes `activitie` and `capacities` becomes `capacitie`: neither ends in `-ity`, so every plural `-ity` nominalization went uncounted without a parse.
The criterion also left open which tokens are candidates when a parse exists, what the ratio's denominator is then, and where a passive's "position" points.

## Root cause
The same as audit 004: when the answered markers were written into the spec, [research/metrics.md §4](../specs/001-vernier/research/metrics.md) recommendation 1 ("strip plural `-s`/`-ies`") was carried over only in part.
The rule of audit 004 was applied to the Block-prose definition at the time, not to the rest of the spec.

## Rule
Audit 004's rule applies to every milestone, checked again when that milestone is planned: each research recommendation, one by one, against the criterion text.
Fixed in place (spec still `Draft`, CONSTITUTION D4): the surface lemma reads `-ies` as `-y` and drops one final `-s` unless it follows `s`; with a parse, candidates are `NOUN` tokens with their lemma; the ratio's words are M2's words; a passive's position is the first character of its verb, found by in-order matching of token forms.
