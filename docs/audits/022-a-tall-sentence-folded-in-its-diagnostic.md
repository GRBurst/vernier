# Audit 022 — a sentence over eight or more lines was folded in its diagnostic

**Severity:** medium
**Tags:** diagnostic, rendering, spec, review

## Symptom
M5 criterion 1 promises that the text diagnostic underlines the sentence "from its first to its last character over all its lines", but a sentence hard-wrapped over 8 or more source lines showed only its first four and last two lines, with `...` in place of the rest: a 12-line sentence showed lines 1–4, `...`, 11–12.
Review pass 4 of spec 001 found it (R4-B1).

## Root cause
`render_text` passed the whole file to `annotate_snippets::Snippet::source` and kept the crate's default, `fold(true)`, which shows a multi-line annotation's first and last lines and elides the middle once it is tall enough.
The width was already raised so that no long line is elided (`RENDER_WIDTH`), but the crate's second elision, over lines, was not looked for.
The only multi-line witness, `check_underlines_a_hard_wrapped_sentence`, had three lines, below the fold's threshold, so every test passed.

## Rule
Pass `annotate-snippets` only the sentence's own lines (`sentence_lines`, with `line_start` at the sentence's first line) and turn folding off, so every line of the sentence is shown and no other line is.
When a criterion says "all" of something a library renders, find every way the library can drop or elide it (width, lines, count), and test past each threshold.
Tested by `check_shows_every_line_of_a_tall_sentence` (tests/cli.rs, a sentence over 14 lines, seen folding first) and the property `the_text_shows_every_line_of_the_sentence` (src/diagnostic.rs, 1 to 30 lines, LF or CRLF, seen failing with folding on at 8 lines).
