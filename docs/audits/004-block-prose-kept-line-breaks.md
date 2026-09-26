# Audit 004 — the Block-prose definition dropped a researched tailoring

**Severity:** medium
**Tags:** spec, sentences, uax29

## Symptom
While planning M2, the spec's *Block prose* definition made a plain-text file one block and said nothing about line breaks.
UAX #29 breaks a sentence after every line feed and carriage return (SB4, `ParaSep ÷`), so every hard-wrapped line of a plain-text file would have become its own sentence, and a paragraph without final punctuation would have run into the next.

## Root cause
When the answered markers were written into the spec (commit c8404d0), only half of the research's sentence tailoring was carried over.
[research/metrics.md](../specs/001-vernier/research/metrics.md) names two mandatory tailorings: turn soft line breaks into spaces, and merge abbreviations.
The spec kept the merge list and the "join spans with one space" rule, which covers Markdown (whose `Text` events hold no line breaks), but not the line-break rule, which plain text needs.

## Rule
When research is written into a spec, check every recommendation of the research section against the new text, one by one, for each input format the spec names.
Fixed in place (spec still `Draft`, CONSTITUTION D4): line feeds and carriage returns in block prose read as spaces; in plain text, blank lines separate blocks.
