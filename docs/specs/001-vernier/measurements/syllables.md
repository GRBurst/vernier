# Measurement — syllable counter vs CMUdict (spec 001 M2)

Criterion: [spec.md#M2](../spec.md#M2): the syllable counter agrees on at least 90 % of a committed reference list of 500 frequent English words sampled from CMUdict.

## Result

| Counter | Agreement | Rate |
| :--- | ---: | ---: |
| Rust `vernier::syllables::count_syllables` | 469 / 500 | 93.8 % |
| Python spike `regex_counter` (the ported heuristic) | 469 / 500 | 93.8 % |

The 90 % bar is met. At p = 0.9 and n = 500 the 95 % confidence half-width is ±2.6 points.
The Rust port misses exactly the words the Python spike misses, with the same counts (compared word by word), so the port is faithful on the whole list.

## Method

- **Reference.** A count is correct when it equals the syllable count of *any* CMUdict pronunciation of the word (the number of phones carrying a stress digit).
- **Sample.** The words are the CMUdict alphabetic entries in the 20,000 most frequent words of the hermitdave FrequencyWords `en_50k` list. Five hundred of them are drawn with `random.Random(2026)`, as the second 500-draw after one uniform draw over all of CMUdict. This is the sample the research measured ([research/metrics.md](../research/metrics.md) §6), so its 93.8 % is reproduced, not re-rolled.
- **Counter.** `max(1, vowel runs − exceptions + additions)` on the lowercased word, with ASCII `aeiouy` as vowels (datascience.stackexchange answer 89312), ported by hand to Rust.
- **Files.** [sample_cmudict.py](sample_cmudict.py) (stdlib only) draws the list and writes [cmudict-500.tsv](cmudict-500.tsv) (`word<TAB>counts`, the counts comma-joined in ascending order).
  The test `syllables::tests::agrees_with_cmudict_on_the_reference_list` embeds the TSV, asserts the 90 % bar, and prints the rate and the misses.

Reproduce:

```sh
cd .sdd/research            # holds cmudict.dict and en_50k.txt (not committed)
python3 ../../docs/specs/001-vernier/measurements/sample_cmudict.py
cd ../.. && cargo test syllables -- --nocapture   # prints "agreement: 469/500"
```

## Misses (31)

Each entry gives the word, vernier's count, and CMUdict's counts.

| Word | Counted | CMUdict |
| :--- | ---: | :--- |
| achievements | 4 | 3 |
| barefoot | 3 | 2 |
| basement | 3 | 2 |
| basque | 2 | 1 |
| coalition | 3 | 4 |
| duo | 1 | 2 |
| excitement | 4 | 3 |
| experiences | 4 | 5 |
| forehead | 3 | 2 |
| funnier | 2 | 3 |
| gimme | 1 | 2 |
| gps | 1 | 3 |
| harriet | 2 | 3 |
| initiative | 5 | 4 |
| karate | 2 | 3 |
| marion | 2 | 3 |
| rhythm | 1 | 2 |
| rio | 1 | 2 |
| scariest | 2 | 3 |
| seattle | 2 | 3 |
| shareholders | 4 | 3 |
| signore | 2 | 3 |
| solely | 2 | 3 |
| ticklish | 2 | 3 |
| timeline | 3 | 2 |
| touches | 1 | 2 |
| trenches | 1 | 2 |
| undoing | 2 | 3 |
| vittorio | 3 | 4 |
| warehouses | 4 | 3 |
| wholesome | 3 | 2 |

Most misses fall into three groups:

- a silent `e` inside a compound (`basement`, `timeline`, `wholesome`, `forehead`)
- adjacent vowels that are two syllables (`duo`, `rio`, `coalition`, `undoing`, `scariest`)
- `-ches`, which the heuristic does not treat as syllabic (`touches`, `trenches`)

The bar holds only for frequent words. A uniform CMUdict sample scores 88.8 % in the research, and technical vocabulary misses more often. A human should read this list once (plan-M2, coverage gap).

## Sources and licenses

- **CMUdict** (cmusphinx `cmudict.dict`), Copyright © 1993–2015 Carnegie Mellon University, BSD-2-style license. Its notice is committed next to the list as [CMUDICT-LICENSE](CMUDICT-LICENSE). The TSV redistributes the words and their syllable counts derived from it.
- **hermitdave FrequencyWords** `en_50k` (OpenSubtitles 2018), CC BY-SA 4.0, <https://github.com/hermitdave/FrequencyWords>. It served only as a frequency filter when choosing the words. No frequency list or count from it is committed.
