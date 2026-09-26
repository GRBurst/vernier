# Audit 008 — the surface nominalization lemma missed possessives

**Severity:** low
**Tags:** spec, nominalization, segmentation

## Symptom
Reading the M4 plan against the code, the implementer noticed that UAX #29 keeps `decision's` as a single word.
Under the surface lemma rule of audit 007, that word's lemma is `decision'`, which does not end in `-ion`, so the word goes uncounted without a parse.
With a parse, `'s` is a separate token and `decision` is counted.
So the two paths disagreed on ordinary prose.

## Root cause
The lemma rule was written against the word list of the research spike (dictionary forms, no clitics).
Nobody checked it against the word segmentation vernier actually uses (UAX #29 word bounds, which keep apostrophe clitics inside the word).

## Rule
A rule that operates on words is checked against the output of vernier's own word splitter, not only against dictionary forms.
Fixed in place (spec still `Draft`, CONSTITUTION D4): the surface lemma first drops a trailing possessive `'s`, `’s`, `'` or `’`, then applies the plural rules.
