# Audit 010 — the JSON spike assumed serde_json reads its own floats back exactly

**Severity:** low
**Tags:** json, serde, testing, research

## Symptom
While implementing M5 T8, the property "every file's metrics in the document equal its summary's fields" failed on the single-sentence document `word.`.
The Flesch Reading Ease 121.22000000000003 was written correctly but read back by `serde_json::from_str` as 121.22000000000004.

## Root cause
The M5 rendering spike recorded that `serde_json` writes floats in their shortest round-trip form, and stopped there.
The default parser of `serde_json` is fast but not correctly rounded; exact parsing is the opt-in `float_roundtrip` feature.
Nobody checked the read side, which is what a test (or a JSON consumer) does.

## Rule
When a spike evaluates a serialization format, check the round trip (write, then read back and compare exactly) on real values, not only the writer.
Fixed in T8: the tests read the document with `serde_json`'s `float_roundtrip` feature enabled as a dev-dependency only (the product writes, it never parses), and the spike report now says so.
