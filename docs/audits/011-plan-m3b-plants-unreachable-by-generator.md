# Audit 011 — two M3b planted violations were unreachable by their properties' generators

**Severity:** low
**Tags:** plan, testing, mutation, parser

## Symptom
Two of the planted violations in [plan-M3b.md](../specs/001-vernier/plan-M3b.md) left their named properties green.
- #8, "merge ignores the all-pieces-between-are-goeswith condition", passed P10 on every run. The goeswith constraint masks a goeswith arc that skips a piece unless that piece prefers goeswith from the same head. So the two merge rules differ only when Chu-Liu/Edmonds re-heads the piece in between, and random logits for up to 8 pieces never produced that case.
- #4, "`fits` uses `n + 2`", failed P6 in about 1 run of 5. It differs only at `max == n + 2`, and uniform limits in 0..1100 almost never hit that value.

## Root cause
Audit 005's rule was applied to the named input only. The plan named P10's "goeswith-heavy cases" and P6 "at `n = max − 2`", but it did not check that either generator produces those inputs. The witness named for #4 (`fits(510, 512)`) did catch it.

## Rule
Before listing a planted violation, also run the mutant once against the named property. A property counts as the check only if it fails in 3 of 3 runs; otherwise add a hand-built witness or bias the generator towards the input.
Fixed in M3b T5 with two changes. #8 has a hand-built witness, `decoder::a_goeswith_piece_after_a_non_goeswith_piece_stays_a_word`. P6's generator now draws half of its limits within 5 of the piece count, and it fails 3 of 3 runs under #4.
