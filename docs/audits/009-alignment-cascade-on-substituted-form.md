# Audit 009: token alignment cascaded after one substituted form

**Severity:** medium
**Tags:** plan, passive, alignment

## Symptom
The M4 plan aligned parser tokens to the sentence text by matching each form only at the cursor.
On a miss it returned `None` and left the cursor where it was.
The plan argued that "one odd form costs one position, not the rest".
That holds when the parser inserts a token that has no counterpart in the text.
It fails when the parser substitutes a form (`was` → `WAS`, `’` → `'`): the text's word is still at the cursor, so every later token misses as well.
The implementer measured it in T6:
- `PASSIVE_CONLLU` with `was` upper-cased aligns as `[Some(0), Some(4), None × 12]`, and both passives fall back to the sentence start.
- Across 256 generated documents, 2233 passives landed on the fallback and 322 on their verb.
- P12 still passed, because the fallback is allowed.

## Root cause
The planner reasoned about one failure mode only (insertion) and wrote a property (P9) that exercised only that mode.
The law checked "a miss leaves the cursor" but never "a miss costs exactly one position" under substitution.

## Rule
When a design claims to "degrade locally", enumerate each kind of disturbance: insertion, deletion, substitution.
Give each one a generator branch whose law states that only the disturbed position changes.
Fixed: after a miss at the cursor, the form may match at the next later word start. The new law P9″ covers substitution, and plant 7″ (cursor-only matching) must turn it red.

## Residual limit (found in T4b)
In-order matching cannot tell an inserted token, which must not consume a text word, from a substituted one, whose text word should be skipped.
After a miss, the cursor stays on the substituted token's word.
If the next form is a prefix of that word (or equal to it: `the the`, `' '`), the next form matches there: one offset lands one word early, and the tail realigns.
Law P9″ therefore assumes that the form after the substituted one is not a prefix of the substituted token's original form.
The unconditioned case is a witness of this known limit: `["'", "'"]` with token 0 substituted aligns as `[None, Some(0)]`.
