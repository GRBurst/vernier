# Audit 015 — an absent mean dependency distance named the wrong reason

**Severity:** low
**Tags:** analysis, diagnostic, summary, spec, review

## Symptom
With `--model-path`, `analyze` on a file whose sentences were parsed but have no content dependency (`Go.` and `Stop!`) read `mean dependency distance  absent (no parse)`, although both sentences were parsed.
A diagnostic of a parsed sentence with two content tokens that are both projected roots (`Yes no.` with both words headed by the punctuation root) read `mean dependency distance: absent (fewer than 2 content tokens)`, which is false for that sentence.
Review pass 1's repair (8723cc6) then wrote the first defect into the *File metrics* row ("`no parse` in both cases") instead of fixing it.

## Root cause
The reason for an absent metric was not a value: each renderer wrote a string literal at the place it found `None`, so the literal named the cause its author had in mind rather than the cause the data held.
`FileSummary.mean_dependency_distance` was an `Option<f64>`, so "no sentence parsed" and "parsed, no content dependency" were the same `None`, and `summary::values` could only print one reason; the diagnostic's literal (M5) assumed a sentence lacks a content dependency only below 2 content tokens, but M3a's content projection makes every content token with only `PUNCT` ancestors a projected root, so a sentence with ≥ 2 content tokens can have none.
No test rendered either case: every parsed fixture had a content dependency.
This is not audit 014's cause (an empty sum standing in for "nothing parsed"): there the value was wrong, here the value was right and its reason was lost.

## Rule
The reason for an absent metric comes from a typed outcome (`analysis::Absence`, rendered by its `Display`), never from a literal chosen in a renderer; a type that can be absent for several reasons carries the reason (`Result<_, Absence>`), not `Option`.
When a definition changes what can make a metric absent, add a witness for each cause to the renderers' tests.
Fixed in `analysis` (`Absence`, `Syntax::parsed_or_absence`, `SentenceSyntax::mean_dependency_distance`, `FileAnalysis::mean_dependency_distance`), `summary`, `diagnostic` and `json`, with the test parser `testing::ProjectedRoots`; the *File metrics* row, M3a criterion 3 and M5 criterion 1 now bind the labels.
