# Audit 018 — a sentence's end landed past closing markup

**Severity:** low
**Tags:** positions, markdown, diagnostics, spec

## Symptom
In `He said *hi.* Then left.` the first sentence's JSON `end_column` was 14, just past the closing `*`, and the text underline covered the `*`.
In `She went **home.**`, the end was just past the `.`, before the `**`.
The same sentence end had two meanings, depending on whether more prose followed in the block; review pass 2 (R2-B6) found `end_line`/`end_column` undefined, and stating them exposed this.

## Root cause
`sentence()` in `src/sentence.rs` mapped the block offset *past* the last character through `Block::source_offset`.
When the next span of the block starts at that block offset (the space after `*hi.*` begins the next span), the offset maps to that span's start, after the closing markup; at the block's end it maps into the last span.
The property tests checked only that the range is ordered and in the source, and `tests/positions.rs` checked only starts, so the end was never compared with the sentence's text.

## Rule
Map the last character of a range, never the offset past it, and add its length: a one-past-the-end offset belongs to no span.
A position property states both ends against the source: the character at the start is the text's first, the character before the end is its last.
Repaired in `sentence()`; tested by `a_source_range_ends_before_closing_markup` and `sentences_are_trimmed_ordered_and_disjoint` (src/sentence.rs) and `every_format_points_at_the_sentence_start` (tests/positions.rs), whose generator now ends sentences inside emphasis.
