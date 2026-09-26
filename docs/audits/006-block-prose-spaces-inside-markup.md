# Audit 006 — block prose put a space wherever inline markup split the text

**Severity:** medium
**Tags:** spec, prose, sentences, parser-input

## Symptom
While wiring the parser into the engine (M3a T9), the fixture `**The proposal**, which …` reached the sentence splitter as `The proposal , which …`.
The spec's *Block prose* definition joined consecutive prose spans with one space, always.
`pulldown-cmark` ends a `Text` event at every inline tag and at some plain characters, so the rule also produced `the  executive  committee` (double spaces) for `the **executive** committee`, `un believ able` (three words) for `un*believ*able`, `a [ b` for `a [b` and `a *b` for `a\*b`.
Word and syllable counts changed with markup that does not change the rendered text, and M3b's parser would have tokenized text no reader sees.

## Root cause
The definition was written from the M1 span list, where each span is independent, without asking what text lies *between* two spans in the event stream.
The M2 laws (P8–P11) checked positions and word sets, both of which the extra space leaves intact; no law compared a block with and without inline markup.

## Rule
A definition that joins or splits source text states what happens at every boundary kind the parser can produce, and the plan carries an invariance law over the constructs that must not change the result.
Fixed in place (spec still `Draft`, CONSTITUTION D4): spans are joined directly when only inline emphasis, strong, strikethrough, superscript, subscript or link delimiters lie between them, and with one space otherwise.
Follow-up F1 in [plan-M2.md](../specs/001-vernier/plan-M2.md) carries the markup-invariance law.
