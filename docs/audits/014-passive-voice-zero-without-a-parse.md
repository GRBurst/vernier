# Audit 014 — passive voice reported as 0 when no sentence was parsed

**Severity:** medium
**Tags:** analysis, passive, spec, review

## Symptom
With `--model-path`, a file in which every sentence is too long for the model reported passive voice `0` (JSON `"passives": 0`), although nothing was parsed and so nothing was measured.
Review pass 1's repair (8723cc6) then wrote the defect into M4 as a requirement ("0 when every sentence was too long for the model"), replacing the original "WHEN no parse exists, THE engine SHALL report passive voice as absent, not as zero".

## Root cause
`analysis::analyze_parsed` summed the passives of the parsed sentences and wrapped the sum in `Some` whenever a parser ran; a sum over zero parsed sentences is 0, so "a parser ran" stood in for "a sentence was parsed".
No test combined a parser with sentences it could not take, so the gap between the two conditions was never exercised; M3b added `TooLong` after M4's rule was written and tested only without a parser.
The review repair described as-built behaviour where the behaviour contradicted the original requirement, which A2 forbids.

## Rule
A file metric summed over parsed sentences is absent when no sentence was parsed, never the empty sum; test it with a parser that finds every sentence too long (`testing::EveryTooLong`).
When a review finds code and criterion disagree, restore the criterion's intent and fix the code; restate a criterion to match the code only when the user decides the behaviour is intended.
Fixed in `analysis::analyze_parsed`, with tests in `analysis`, `summary` and `json`; M4 criterion 4 and the *File metrics* row restored.
