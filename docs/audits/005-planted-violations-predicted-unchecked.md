# Audit 005 — two planted violations in plan-M2 could not produce their predicted red

**Severity:** low
**Tags:** plan, testing, mutation

## Symptom
Two of the planted violations in [plan-M2.md](../specs/001-vernier/plan-M2.md) did not fail the check the plan named.
- #1, "try `ia` before `.y[aeiou]`", left P3 green. No two of the nine syllable-heuristic alternatives can match at the same starting position, so reordering them is an equivalent mutation. It changes none of the 117,493 CMUdict words.
- #5, "`Some(NaN)` for zero sentences", failed only the unit witness, not P6. P6's `any_counts` generator almost never drew `sentences == 0`.

## Root cause
The planner wrote each violation from the intended behaviour without checking two things:
- whether the mutation changes the observable output at all (equivalence);
- whether the property's generator reaches the input where the mutation shows (reachability).

## Rule
Before listing a planted violation, name one concrete input on which the mutated code gives a different result. Then check that the named test contains that input, or that its generator is biased towards it.
Fixed during M2's implementation. #1 got a substitute (advance by 1 instead of the match length, so matches overlap), which fails P3 on a 21-word witness table (`cronyism`, `dandyism`, `mccarthyism`, `shiyuan`). The #1 box stays unticked with a note. P6's generator is now biased towards zero counts and fails 3 out of 3 runs when #5 is planted.
