# Spec 001 — open questions answered (2026-09-26)

**Subject:** the 13 `[NEEDS CLARIFICATION]` markers in [spec.md](spec.md), plus 3 new questions the research raised.
**Evidence:** two research passes on 2026-09-26, committed as [research/licenses.md](research/licenses.md) and [research/metrics.md](research/metrics.md). Every quote below comes from them, with its primary source.
**Not legal advice:** license readings marked *[interpretation]* are ours.

## What the spec is (summary)

`vernier` is a Rust CLI. It reads Markdown, keeps the prose, and reports per sentence:

- surface metrics: Flesch Reading Ease, Flesch-Kincaid grade, Gunning Fog;
- syntactic metrics from a Universal Dependencies (UD) parse: mean dependency distance (MDD), tree depth, clause count, center-embedding;
- nominalizations and passive voice.

It works like a compiler linter: `vernier check` exits 1 when a sentence is flagged.
There are 6 milestones: M1 (prose extraction, implemented), M2 (surface metrics), M3a (syntactic metrics on hand-built trees), M3b (a real parser plus a model), M4 (nominalizations, passive voice), M5 (diagnostics and output).
Every marker has a *fallback*: the answer we use if nobody decides otherwise.

## Verdict: which questions need you

| # | Marker (spec line) | Who decides | Answer |
| :--- | :--- | :--- | :--- |
| 1 | Suggestions: generated rewrite or fixed advice (Non-Goals) | **you** (intent) | fixed advice recommended |
| 9 | Model license (M3b) | **you** (intended use) | depends on Q-A below |
| 11 | Model downloader (M3b) | **you** (follows from 9) | depends on Q-A |
| 2 | Which sentence split is authoritative | research | UAX #29, with two refinements |
| 3 | Content tokens: renumber after removing punctuation | research | renumber |
| 4 | Headings and image alt text | fallback (implemented in M1) | dropped |
| 5 | Plain text detected by extension | fallback (implemented in M1) | `.md`/`.markdown` are Markdown |
| 6 | Syllable reference and agreement bar | research | 500 frequent CMUdict words, ≥ 90 % |
| 7 | "Complex word" without a proper-noun list | research | capitalization rule, refined |
| 8 | Tree depth in edges or nodes | research | edges, root = 0 |
| 10 | No model in CI | technical | fallback, sharpened |
| 12 | Nominalization false positives | research | fallback, plus lemma and stoplist |
| 13 | One diagnostic per sentence or per rule | your example decides it | per sentence |

**Only 1, 9 and 11 need you**, and 9 and 11 come down to one question: *who will run vernier, and for what?* The other ten are technical, and research settles them. Each one is below, so you can overrule any of them.

---

## Questions that need you

### Q-A (markers 9 and 11): what is the intended use, and which parser model is acceptable?

**The problem.** The syntactic metrics (M3a/M3b) need a trained English UD model. Every ready-made model we found carries a non-commercial condition, in its license or in its training data.

| Model | License | Runs from Rust | Catch |
| :--- | :--- | :--- | :--- |
| UDPipe 1, `english-ewt-ud-2.5-191206.udpipe` | CC BY-NC-SA 4.0 | yes (the `udpipe-rs` crate, C++ via FFI) | **no commercial use**; FFI means `unsafe` (needs an ADR, D9) |
| UDPipe 2 (UD 2.12–2.17) | CC BY-NC-SA 4.0 | **no** (Python + TensorFlow; 8.5 GB) | — |
| `roberta-base-english-ud-goeswith` as ONNX | MIT (as labelled) | yes, via the `ort` crate | 3 of its 5 training treebanks are NC-SA |
| spaCy `en_core_web_sm` | MIT | no | its labels are **not UD** (`nsubjpass`, not `nsubj:pass`) |
| a model we train ourselves on UD_English-EWT only | CC BY-SA 4.0 at most *[interpretation]* | yes, via ONNX | training work; accuracy unknown |

Evidence:

- LINDAT (UDPipe 1 models): *"This item is Publicly Available and licensed under: Creative Commons - Attribution-NonCommercial-ShareAlike 4.0 International (CC BY-NC-SA 4.0)"* — https://lindat.mff.cuni.cz/repository/xmlui/handle/11234/1-3131
- UDPipe README: *"the linguistic models are free for non-commercial use and distributed under the CC BY-NC-SA … license"* — https://github.com/ufal/udpipe
- EWT: *"licensed under a Creative Commons Attribution-ShareAlike 4.0 International License"*. GUM: *"licensed under the Creative Commons License Attribution-NonCommercial-ShareAlike 4.0 International"* — github.com/UniversalDependencies/UD_English-EWT, UD_English-GUM
- The goeswith model's `maker.py` trains on `["UD_English-EWT","UD_English-GUM","UD_English-ParTUT","UD_English-Lines","UD_English-Atis"]` — https://huggingface.co/KoichiYasuoka/roberta-base-english-ud-goeswith

**Options:**

1. **Personal or non-commercial use only**: accept the NC-SA UDPipe 1 model. You download it yourself and pass `--model-path`; vernier never redistributes it. *(This is the spec's current fallback.)*
2. **Commercial use must be possible**: syntactic metrics only with a model we train on EWT (plus Atis). That is a new milestone before M3b, with an ONNX export. Until it exists, vernier ships surface metrics only.
3. **Undecided for now**: build M2, M3a and M4 (all parser-free) and decide before M3b. Nothing is lost, because M3a's metrics depend only on the `Parser` trait.

**Recommendation: 3 now, then 1 unless you need commercial use.** M2–M3a don't touch the model, so deciding later costs nothing.
**Downloader (marker 11):** with option 1, a `vernier model fetch` command that fetches from the original LINDAT URL and **prints the CC BY-NC-SA terms first** is fine *[interpretation: fetching from the source is not redistribution by us]*. The `udpipe-rs` crate already has a `download` feature that does exactly this. With option 2 we host our own model, so a downloader is simple. Recommendation: no downloader until the route is chosen. `--model-path` covers both.

Example of what option 1 would look like:

```text
$ vernier model fetch
The English EWT model is licensed CC BY-NC-SA 4.0 (non-commercial). Continue? [y/N]
```

### Q-B (marker 1): may suggestions quote a rewritten sentence?

**Spec text:** *"THE tool SHALL NOT rewrite the user's text; suggestions are fixed advice per rule, never generated sentences."*
Your original project description showed a suggestion that quoted a rewritten sentence. Generating one needs a language model, which contradicts "no Python, JVM or PyTorch at runtime" (spec Scope).

```text
= help: Split the sentence, or move the relative clause after the verb.      ← fixed advice (recommended)
= help: Try: "The executive committee rejected the proposal, which caused…"  ← generated, needs an LLM
```

**Recommendation: fixed advice per rule.** It is deterministic (B2), testable, and needs no model. The alternative, an optional external LLM hook, would be a separate spec later.

---

## Settled by research (overrule if you disagree)

### 2. Sentence: UAX #29 is authoritative

**Spec:** *"Sentence: a UAX #29 sentence segment of the extracted prose … [the parser also splits sentences; which split is authoritative?]"*

**Answer:** the UAX #29 split, and each sentence goes to the parser pre-segmented. The surface metrics and diagnostic positions must be the same with or without a model. UDPipe supports this: *"presegmented: the input file is assumed to be already segmented, with each sentence on a separate line"* (https://ufal.mff.cuni.cz/udpipe/1/users-manual).

**Two refinements the spec must add**, from a spike with `unicode-segmentation` 1.13.3: raw 15/20 tricky sentences, 18/20 with both refinements.

1. **Segment per paragraph, not per M1 span.** UAX #29 breaks after every line feed (*"SB4 ParaSep ÷"*, https://www.unicode.org/reports/tr29/), and M1 spans stop at soft line breaks, links and inline code. A hard-wrapped Markdown sentence would otherwise be split at every source line.
2. **A small abbreviation list** (`Mr. Mrs. Ms. Dr. Prof. St. e.g. i.e. etc. vs. Fig. No. cf.`). A segment ending in one of these merges with the next. UAX #29 itself says: *"They cannot detect cases such as '...Mr. Jones...'; more sophisticated tailoring would be required."*

```text
"Dr. Smith reviewed the draft. It was fine."
raw UAX #29: ["Dr. ", "Smith reviewed the draft. ", "It was fine."]   tailored: 2 sentences ✓
```

Trade-off: a sentence that really ends in "etc." merges with the next one.

### 3. Content tokens: drop punctuation, renumber 1..N

Liu's group, the source of the "MDD < 3" threshold, does exactly this. Yan & Liu 2022: *"delete all the punctuations … Since the position numbers are now relative references, they will change automatically after the rows containing punctuations were deleted"* (https://aclanthology.org/2022.paclic-1.10.pdf). Futrell et al. 2015: *"We do not count any nodes representing punctuation"* (PNAS).
Tools that keep punctuation (TextDescriptives, textcomplexity) give values that cannot be compared with that threshold.

### 4 and 5. Headings and alt text dropped; `.md`/`.markdown` = Markdown

Implemented in M1 on the fallbacks. Headings are labels, not sentences. Alt text describes an image, not the argument.
The extension check ignores letter case (`README.MD` is Markdown).

### 6. Syllable reference: 500 frequent CMUdict words, ≥ 90 %

Spike against the full CMUdict (117,493 words):

| Counter | 500 frequent words | 500 uniform words |
| :--- | :--- | :--- |
| rule-based (regex heuristic) | **93.8 %** | 88.8 % |
| Liang hyphenation (the `hyphenation` crate's method) | 63.0 % | 49.4 % |

Liang undercounts (6,512 under vs 38 over), so the spec's "hyphenation only if it measures better" resolves to: **not used**.
Sample the 500 words from *frequent* words. A uniform sample is full of names and loanwords and would fail a good counter.
**License:** CMUdict: *"Use of this dictionary for any research or commercial purpose is completely unrestricted"* (https://github.com/cmusphinx/cmudict). BSD-2 style: the committed sample keeps CMU's notice.

### 7. Complex words: the capitalization rule stands in for proper nouns

Gunning's own rules, via University of Missouri Extension CM201: *"Don't count words (a) that are capitalized; (b) that are combinations of short, easy words …; (c) that are verb forms made into three syllables by adding -ed or -es"*.
The spec rule matches (a) and (c). Refinement: remove a suffix only when it *is* a syllable. `created` (cre-at-ed) drops to 2 syllables, but `embarrassed` keeps its 3, because its `-ed` adds no syllable.
Hyphenated compounds are already split into two words by UAX #29.

### 8. Tree depth: edges, root = 0

This matches Jing & Liu 2015: *"designate its projection position as 0 … the path length traveling from the root to a certain node along the dependency edges"* (https://aclanthology.org/W15-2119.pdf).
It also makes "depth ≤ N − 1" exact. udapi counts the root word as 1, so the `--max-tree-depth` help text will say "edges".

### 10. No model available

**Spec fallback:** compute the surface metrics, print one notice, and judge the exit code on surface rules only.
Sharpened so CI is never weakened silently:

- `--model-path` given but unusable → exit 2. This is already M5: *"2 on an unreadable file or an unusable model"*.
- No `--model-path` → surface-only run with the notice. You opt in to syntax by naming a model.

```text
$ vernier check README.md
note: syntactic metrics skipped (no --model-path)
```

### 12. Nominalizations: suffix + length ≥ 7 + NOUN + stoplist, on the lemma

Spike on the 10,000 most frequent English words:

| Rule | Share of matches that are not nominalizations (weighted by frequency) |
| :--- | :--- |
| suffix only | 38 % |
| + length ≥ 7 (removes nation, city, moment, cement) | 28 % |
| + a stoplist of the 40 most frequent false positives | **6 %** |

Refinements:
- Match the **lemma**, so the plural `decisions` counts. Biber 1988, as replicated by MAT: *"Any noun ending in -tion, -ment, -ness, or -ity, plus the plural forms"*.
- Seed `data/nominalization-stoplist.txt` from pybiber's list (MIT, 57 entries: city, nation, moment, station, function, …).
- Reword *"abstract deverbal nouns"* to *"nominalizations (nouns derived from verbs or adjectives)"*, because 65 % of `-ity` words come from adjectives.
- NOMLEX is rejected: it has no written license, and it lists *station* and *question* as nominalizations anyway.

### 13. One diagnostic per sentence

Your example shows one `warning[CognitiveOverload]` per sentence, listing every metric. That also keeps the output one block per problem sentence rather than one per rule.

---

## New questions the research raised (answered technically)

| Question | Answer | Why |
| :--- | :--- | :--- |
| A content token whose head is a punctuation token (a parser error; UD forbids it) | re-attach it to the nearest non-punctuation ancestor | dropping the sentence (as Yan & Liu do) would silently lose warnings |
| Syllables of a numeric word (`2024`) | 1 | what textstat does; words include numbers by the spec's definition |
| File-level MDD | total distance / (tokens − sentences), token-weighted | Liu 2008 formula (2); a mean of sentence means over-weights short sentences |

## What happens next

1. You answer Q-A and Q-B.
2. The answers and the ten technical answers go into `spec.md` in one commit. The markers disappear, and the Clarification verdict goes to `gates.md`.
3. Review and approval (phase 2) stay owed, as in BACKLOG item 2.
