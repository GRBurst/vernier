# Research: metric definitions for spec 001 (vernier)

Researcher: agent `metric-research`, 2026-09-26/27. Scope: the `[NEEDS CLARIFICATION]` markers of
`docs/specs/001-vernier/spec.md` that concern metric definitions (Content token, depth, Fog complex
words, nominalizations, Sentence, syllable reference).
Scratch material (downloaded sources, scripts, raw outputs) is in `.sdd/research/` and `.sdd/uax29-spike/`
(untracked). Source code was read from the default branches as of today; versions are given.

## Summary of recommendations

| # | Question | Recommendation |
|---|---|---|
| 1 | MDD punctuation | Exclude `PUNCT` and **renumber** content tokens 1..N (spec fallback stands). This is what Liu's group and Futrell et al. do. Divide by N − 1, as in Liu 2008 formula (1). |
| 2 | Tree depth | **Edges, root = 0** (spec fallback stands), like Jing & Liu 2015 (MHD) and `textcomplexity`. Say so in the flag help: udapi and TRUNAJOD count the root word as 1. |
| 3 | Fog complex words | Keep the spec rule (≥ 3 syllables, not capitalized unless sentence-initial, suffix rule). Subtract a suffix only when it is a syllable. Hyphenated compounds are handled already because UAX #29 splits at hyphens. Note that textstat does something different. |
| 4 | Nominalizations | Suffix + length ≥ 7 + `NOUN` + stoplist (spec fallback stands). Match on the lemma, or strip plural `-s`, so plurals count. Seed the stoplist from pybiber (MIT, 57 entries) and this spike. Do not use NOMLEX: it has no licence and its entries include words like *station* and *question*. Rename the metric: many `-ity`/`-ence` words come from adjectives, not verbs. |
| 5 | Sentences | UAX #29 is authoritative, and each sentence goes to the parser pre-segmented (spec fallback stands). UDPipe supports this (`--tokenizer=presegmented`). Two mandatory tailorings: turn soft line breaks into spaces before segmenting, and use a small abbreviation merge list. With both, the spike went from 15/20 to 18/20. |
| 6 | Syllables | A regex counter (not Liang hyphenation) gets 93.8 % of a 500-word sample of common words right against CMUdict; Liang gets 63 %. A 90 % threshold is reasonable if the sample comes from frequent words. It is not reasonable for a uniform CMUdict sample, where the same counter scores 88.8 %. |

---

## 1. Mean Dependency Distance: punctuation and numbering

### What the literature does

- **Liu 2008** defines MDD over "words". It does not state in the formula section how punctuation is handled.
  Quote (§2, preprint p. 5): "let W1...Wi...Wn be a word string … adjacent words have a DD of 1 … MDD(the sentence) = 1/(n−1) Σ|DDi| … Here n is the number of words in the sentence … there is generally one word (the root verb) without a governor, whose DD is therefore defined as zero."
  A footnote about the treebank statistics shows that punctuation was left out: "Excluding all punctuations, Romanian has also a very short MSL." (fn. 7).
  <https://pdfs.semanticscholar.org/b6b9/cf00698a76d7a1e5ba58baa92d8799366813.pdf>
- **Jing & Liu 2015** (Liu's group, Depling), "Mean Hierarchical Distance Augmenting Mean Dependency Distance": "Note that punctuation marks are rejected when measuring the MDD and MHD." <https://aclanthology.org/W15-2119.pdf>
- **Yan & Liu 2022** (PACLIC) says outright that punctuation is excluded **and positions are renumbered**:
  - "Punctuations are generally not included in the calculation of MDD." (fn. 4)
  - "The second step was to convert all the values in the ID and HEAD columns into a relative reference in Excel … The third step was then to delete all the punctuations, a treatment following Jiang & Liu (2015) and other previous studies … Since the position numbers are now relative references, they will change automatically after the rows containing punctuations were deleted."
  - They dropped sentences "where punctuations are heads of other tokens, which causes problems when deleting punctuations."

  <https://aclanthology.org/2022.paclic-1.10.pdf>
- **Futrell, Mahowald & Gibson 2015** (PNAS, 37 languages): "We calculate the length of a single dependency arc as the number of words between a head and a dependent, including the dependent … We do not count any nodes representing punctuation or root nodes, nor arcs between them."
  Because punctuation nodes are not counted as words, positions are effectively renumbered.
  <https://pmc.ncbi.nlm.nih.gov/articles/PMC4547262/>
- **Chen 2026** (arXiv 2609.04223) compares preprocessing choices across UD treebanks. Its canonical specification is "punctuation excluded", and it requires "a connected acyclic dependency tree after punctuation removal". It also shows that the punctuation choice changes cross-treebank agreement, so the choice must be documented. <https://arxiv.org/pdf/2609.04223>
- **UDW 2026** (122 UD languages): "MDD: Mean absolute distance |pos(head) − pos(dep)| over non-punctuation tokens, excluding root dependencies (Liu et al., 2017)." <https://aclanthology.org/2026.udw-1.14.pdf>
- **Gibson's DLT** does not measure in words at all: "it is assumed that 1 EU is consumed for each new discourse referent in the intervening region". Punctuation is irrelevant to it, and it is not the MDD formula. <https://tedlab.mit.edu/tedlab_website/researchpapers/Gibson_2000_DLT.pdf>
- **UD guideline**, which justifies the renumbering: "Tokens with the relation punct always attach to content words (except in cases of ellipsis) and can never have dependents." <https://universaldependencies.org/u/dep/punct.html>

### What tools do

- **TextDescriptives 2.8.4** (`components/dependency_distance.py`) keeps punctuation and uses **original spaCy indices**: `dep_dist = abs(token.head.i - token.i)`; the root gets 0.
  The sentence mean is `np.mean` over **all** tokens, including the root and punctuation. That is N in the denominator, not N − 1, which the docs describe as "follows Oya, 2011".
  The maintainers acknowledge that Liu's formula differs and give a snippet for it (issue #77).
  <https://github.com/HLasse/TextDescriptives/blob/main/src/textdescriptives/components/dependency_distance.py>, <https://github.com/HLasse/TextDescriptives/issues/77>
- **textcomplexity 0.11.0** (Proisl) keeps punctuation. The skip is commented out: `# if relation == "punct": continue`. It uses original positions and averages over edges, which is N − 1.
  Its `--ignore-punct` flag only affects "surface-based and pos-based complexity measures".
  <https://github.com/tsproisl/textcomplexity/blob/master/textcomplexity/utils/conllu.py>

### Recommendation

Keep the spec fallback: **exclude `upostag == PUNCT`, renumber content tokens 1..N, and compute Σ|i − head(i)| / (N − 1)**.

Reasons:
1. It matches Liu's own group (Jing & Liu 2015; Yan & Liu 2022, following Jiang & Liu 2015) and Futrell et al. 2015. These are the sources that the threshold "MDD ≤ 3" comes from. Liu 2008 reports "a threshold of less than 3 words".
2. It keeps the property "MDD ≥ 1".
3. Tools that keep punctuation (TextDescriptives, textcomplexity) produce values that cannot be compared with Liu's threshold.

Two edges the spec should state:
- A content token whose head is a `PUNCT` token is a parser error (UD forbids it). Yan & Liu drop such sentences. I recommend re-attaching the token to the nearest non-PUNCT ancestor. Report MDD as absent only if no such ancestor exists (a PUNCT root). The alternative, reporting absent for the whole sentence, silently loses warnings. **Decision for main/user.**
- The formula averages per sentence. Liu's text-level formula (2), Σ|DD| / (n − s), is token-weighted, not a mean of sentence means. If a file-level MDD is reported, say which of the two it is. Liu's formula (2) is the one that matches the literature.

## 2. Tree depth: edges or nodes

| Source | Convention | Quote / evidence |
|---|---|---|
| Jing & Liu 2015 (MHD) | **edges, root = 0** | "we take the root of a syntactic tree as a reference point and designate its projection position as 0 … the path length traveling from the root to a certain node along the dependency edges, is defined as 'hierarchical distance (HD)'" <https://aclanthology.org/W15-2119.pdf> |
| textcomplexity 0.11.0 | **edges, root = 0** | `longest_shortest_path`: "Longest shortest path from the root vertex, i.e. depth of the tree", computed with networkx `shortest_path_length` from the root word; a 1-node graph returns 0 <https://github.com/tsproisl/textcomplexity/blob/master/textcomplexity/dependency.py> |
| udapi 0.5.2 (UD's Python API) | **nodes, root word = 1** | "depth: depth in the dependency tree (technical root has depth=0, highest word has depth=1)"; `block/demo/complexity.py` reports `max_depth` this way <https://github.com/udapi/udapi-python/blob/master/udapi/core/node.py> |
| TRUNAJOD 2.0 | **nodes, root = 1** | "The ``ROOT`` of the sentence is considered level 1." (`surface_proxies.get_word_depth`) <https://github.com/dpalmasan/TRUNAJOD2.0/blob/master/src/TRUNAJOD/surface_proxies.py> |
| NLTK `Tree.height` (constituency; used by lingfeat and textcomplexity for parse-tree height) | nodes | "The height of a tree containing no children is 1" (docstring). <https://github.com/nltk/nltk/blob/develop/nltk/tree/tree.py> |
| TextDescriptives, L2SCA (Lu 2010) | no depth measure | TextDescriptives has no depth component. L2SCA has 14 constituency-based measures, none of them depth. <https://sites.psu.edu/xxl13/l2sca/> |

**Recommendation: edges, root = 0** (spec fallback). This matches the dependency-distance literature the spec already relies on (Liu's MHD) and the graph-theoretic height, and makes "depth ≤ N − 1" exact.
The default `--max-tree-depth 5` is interpreted in edges. In udapi's convention the same trees would read 6. Put "(edges; a one-word sentence has depth 0)" in the flag's help text. Also decide whether depth is computed on the content-token tree, which is my recommendation for consistency with MDD. PUNCT tokens are always leaves, so this only matters when punctuation is the deepest node.

## 3. Gunning Fog "complex words"

### Gunning's rules

Gunning's 1952 book is not online. The most-cited paraphrase, from the University of Missouri Extension (CM201, "Clear Writing"):
"Count the number of words with three or more syllables in the sample. Don't count words (a) that are capitalized; (b) that are combinations of short, easy words (such as 'bookkeeper' or 'butterfly'); (c) that are verb forms made into three syllables by adding -ed or -es (such as 'created' or 'trespasses')."
<https://extension.missouri.edu/publications/cm201>

Wikipedia widens this to "proper nouns, familiar jargon, or compound words. Do not include common suffixes (such as -es, -ed, or -ing) as a syllable". The `-ing` comes from a secondary source (readabilityformulas.com), not from Gunning.
<https://en.wikipedia.org/wiki/Gunning_fog_index>

### Implementations

- **textstat 0.7.12**: `gunning_fog` counts `difficult_words` with `syllable_threshold = 3` for `en`. A word is difficult if it is **not in the ~2,940-word Dale–Chall easy list** and has ≥ 3 syllables.
  - No proper-noun rule, no compound rule, no suffix rule. The Dale–Chall filter is not part of Gunning's definition.
  - Syllables come from CMUdict, falling back to Pyphen.

  <https://github.com/textstat/textstat/blob/main/textstat/backend/validations/_is_difficult_word.py>, `metrics/_gunning_fog.py`, `utils/constants.py`
- **py-readability-metrics 1.4.4**: `is_gunning_complex = syllable_count >= 3 and not (self._is_proper_noun(t) or self._is_compound_word(t))`, with `_is_proper_noun = token[0].isupper()` (the POS check is commented out). It therefore also drops capitalized sentence-initial words. A compound is any token containing `-`. No suffix rule.
  <https://github.com/cdimascio/py-readability-metrics/blob/master/readability/text/analyzer.py>
- **quanteda.textstats** `FOG`: plain words with ≥ 3 syllables. The comment says the refinements need a POS-tagged text: "If the text was POS-tagged accordingly, proper nouns and combinations of only easy words will not be counted as hard words, and the syllables of verbs ending in '-ed', '-es' or '-ing' will be counted without these suffixes."
  <https://github.com/quanteda/quanteda.textstats/blob/master/R/textstat_readability.R>

### Recommendation (no proper-noun list)

Keep the spec rule. Its capitalization test is exactly Gunning's "capitalized" rule, minus the mistake py-readability-metrics makes on sentence-initial words. Two precisions:
1. **Subtract a suffix only when it is a syllable.** Complex means `syll(word) − s ≥ 3`, where `s = 1` if the word ends in:
   - `-ing`;
   - `-ed` after `t`/`d` (*created*, but not *embarrassed*);
   - `-es` after `s`, `x`, `z`, `ch`, `sh`, or in `-ces`/`-ges` (*trespasses*, *boxes*, *changes*);

   and `s = 0` otherwise.
   Naively re-counting the stripped stem misfires on stems such as *creat*. Gunning's rule is only for -ed/-es. `-ing` is the Wikipedia widening. Keep it (the spec has it), but note where it comes from.
2. **Compounds**: UAX #29 splits `well-known` into `well`, `known`, so hyphenated compounds never reach ≥ 3 syllables as one word. Closed compounds (*bookkeeper*) cannot be detected without a lexicon. Accept this and document it.
   Also document the known error: sentence-initial proper nouns count as complex.
   All-caps acronyms are capitalized, so they are excluded mid-sentence. That is fine.

The spec's M2 formula check is correct: 100 words, 5 sentences, 150 syllables give FRE 59.635 and FKGL 9.91.

## 4. Nominalizations by suffix

### Published approaches

- **Biber 1988, as replicated by MAT (Nini)**: "Nominalizations (NOMZ) Any noun ending in -tion, -ment, -ness, or -ity, plus the plural forms. Although Biber (1988) does not mention that this variables was checked manually, it is likely that a stop list was used to avoid obviously erroneous tagging (e.g. city)." MAT v1.2 manual, <https://corpus.bfsu.edu.cn/MATv1.2Manual.pdf.pdf>
- **pybiber 0.3.1 (MIT)**: `pos == "NOUN"` and the token matches `tion$|tions$|ment$|ments$|ness$|nesses$|ity$|ities$`, not in `nominalization_stoplist`. The stoplist has 57 entries:
  > apartment(s), attention, business(es), capacity/capacities, city/cities, comment(s), condition(s), document(s), edition(s), element(s), environment(s), experiment(s), fiction(s), function(s), humanity, identity/identities, mention(s), moment(s), motion(s), nation(s), notion(s), pity, position(s), quality/qualities, section(s), solution(s), station(s), tradition(s), university/universities, witness(es)

  No length rule. <https://github.com/browndw/pybiber/blob/main/pybiber/parse_functions.py>, `biber_dict.py`, catalogued at <https://jtauber.github.io/clef/feature/NOMZ/>
- **Coh-Metrix 3.0**: the description of its 108 indices contains no nominalization index; searching the index description for "nominali" finds nothing. <https://cohmetrix.cn/cohmetrix.pdf>
- **NOMLEX**:
  - NYU, 1,025 entries. Licence text: "the 2001 version … is downloadable from this website … and freely available for use by all". There is no licence file and no redistribution terms. <https://nlp.cs.nyu.edu/nomlex/index.html>, archived copy at <https://web.archive.org/web/2024/https://nlp.cs.nyu.edu/nomlex/index.html>
  - NOMLEX-plus (4,417 NOM/NOMADJ entries) ships inside the NomBank 1.0 tarball. Its README has no licence for it; it mentions LDC licences only for related files. <https://nlp.cs.nyu.edu/meyers/NomBank.html>
  - NOMLEX also lists zero-derived and non-suffix nouns: `station`, `question`, `mention`, `authority`, `function` and `balance` are all NOM entries. A lookup still needs the suffix filter and still lets through what readers do not see as "zombie nouns".
- Spec vs Biber: the spec's suffix set adds `-sion`, `-ance`, `-ence` and drops `-ness`.

### Spike

The spike (`.sdd/research/nominal_spike.py`, `nominal_strict.py`) uses the hermitdave FrequencyWords `en_50k` list (OpenSubtitles 2018). Lemmas come from stripping plural `-s`/`-ies`; the six spec suffixes are applied.
An oracle labels each match:
- **deverbal**: NOMLEX/NOMLEX-plus `:VERB`, or a WordNet 3.1 noun→verb derivation pointer, where the base is shorter and shares the stem;
- **deadjectival**: the same test with `:ADJ` or a WordNet noun→adjective pointer;
- **other**: everything else.

The first, lenient version of the oracle (any WordNet derivation link) counted *city*, *cement* and *station* as deverbal. WordNet links conversions and reverse derivations too (*city* → *citify*), so I switched to the strict stem test.
I checked all 88 "other" words in the top 10k by hand; 6 were real nominalizations the oracle missed (*ability, solution, entrance, detention, discretion, necessity*). A few oracle "nominal" labels are also wrong (*television*, *community*), so read the figures as ±5 pp.

Results on the 10k most frequent lemmas (422 suffix matches), after the manual correction:

| Rule | Not a nominalization, share of types | Share of tokens |
|---|---|---|
| suffix only | 24.4 % (103/422) | 38.0 % |
| suffix + length ≥ 7 | 20.6 % (82/399) | 28.1 % |
| + stoplist of the 20 most frequent false positives | — | 12.5 % residual |
| + stoplist of the 40 most frequent false positives | — | 6.0 % residual |

- The length rule removes 21 types: *chance, moment, city, dance, France, nation, pity, vision, motion, fence, lance, notion, hence, potion, glance, cement, fiancé, ration, vanity, unity, Vance*. It costs only *action* and *option* (on the top 50k also *purity, fusion, stance, sanity, nudity, rarity, oddity*).
- The false positives that survive length ≥ 7 are lexicalized long nouns: *question, station, apartment, mission, condition, department, mention, science, audience, authority, university, sentence, section, document, circumstance, experiment, quality, passion, ambulance, advance, identity, comment, version, basement, charity, tradition, occasion, element, balance, influence, session, environment, function, instance …*.
- Per suffix (top 10k, length ≥ 7):
  - `-tion`: 87 % deverbal
  - `-ment`: 70 % deverbal
  - `-ence`: 32 % deverbal + 35 % deadjectival
  - `-ity`: 65 % deadjectival, 5 % deverbal

  `-ity` and much of `-ence`/`-ance` are **not deverbal**.
- The `NOUN` tag could not be measured, because the list has no POS. It is what removes verb uses of *comment, document, experiment, question, balance, commence, enhance*.
- Caveat: the corpus is subtitles. Technical Markdown will have *function, element, instance, version, section, document, environment, application* far more often, so the stoplist needs a pass over real docs.

### Recommendation

**Suffix + length ≥ 7 + `NOUN` + committed stoplist** (spec fallback), with these changes:
1. **Match on the lemma, or on the form with plural `-s`/`-ies` stripped.** The current text, "a word ending in -tion …", misses *decisions* and *activities*. Biber, MAT and pybiber all include plurals. With a parse, use `lemma`.
2. **Seed `data/nominalization-stoplist.txt`** from the pybiber list (MIT; keep the attribution) plus the spike's high-frequency false positives listed above. Then extend it after running `vernier` on real READMEs, and commit that measurement.
3. **Reword "abstract deverbal nouns" to "nominalizations (nouns derived from verbs or adjectives)"**, or drop `-ity`: 65 % of `-ity` hits come from adjectives. Biber counts `-ity` and `-ness` as nominalizations, so keeping `-ity` with the new wording is consistent with the literature.
   Adding `-ness` is optional. It is Biber's; its false positives are basically *business* and *witness*, both already in pybiber's stoplist.
4. **No NOMLEX lookup.** Its licence status is only "freely available for use by all", with no written terms, so committing derived data is doubtful. It has about 4k entries, so it misses newer technical words. And it still lets through *station*, *question* and *function*.

## 5. Sentence segmentation: UAX #29 vs the parser's splitter

### UDPipe accepts pre-segmented input

- UDPipe 1 user manual, tokenizer options: "presegmented: the input file is assumed to be already segmented, with each sentence on a separate line, and is only tokenized (respecting sentence breaks)". CLI: `--tokenizer=presegmented`.
  The manual also offers `ranges` ("for each token, a range in the original document is stored") and `--input=horizontal` ("each sentence on a separate line, with tokens separated by spaces").
  <https://ufal.mff.cuni.cz/udpipe/1/users-manual>
- API: `model::TOKENIZER_PRESEGMENTED`, `input_format::new_presegmented_tokenizer(input_format* tokenizer)`, `input_format::GENERIC_TOKENIZER_PRESEGMENTED`. <https://ufal.mff.cuni.cz/udpipe/1/api-reference>

### UAX #29's own caveat

"They cannot detect cases such as '...Mr. Jones...'; more sophisticated tailoring would be required to detect such cases." Also: "Break after paragraph separators. SB4 ParaSep ÷", where ParaSep = Sep | CR | **LF**.
UAX #29 revision 49 (Unicode 18.0.0), <https://www.unicode.org/reports/tr29/>

### Spike

`.sdd/uax29-spike`, `unicode-segmentation` 1.13.3, the version in vernier's Cargo.lock. Raw `unicode_sentences()` got **15/20** as expected.

| Case | Result |
|---|---|
| `Use a tool, e.g. vernier, to check prose.` | PASS (lowercase after `e.g.`) |
| `Dr. Smith reviewed the draft. It was fine.` | **FAIL**: `["Dr. ", "Smith reviewed the draft. ", "It was fine."]` |
| `The value rose to 3.14 percent in 2024.` | PASS (decimal) |
| `See Fig. 3 for details.` / `See No. 5 in the list.` | PASS (digit follows) |
| `The U.S. economy grew. Analysts were surprised.` | PASS |
| `It costs $4.50, i.e. less than before.` | PASS |
| `Mr. and Mrs. Jones arrived at 5 p.m. on Friday.` | **FAIL**: break after `Mrs. ` |
| `Version 1.2.3 fixes the bug. Upgrade now!` | PASS |
| `He asked "Why?" and left.` | **FAIL**: break after `"Why?" ` (SB8 protects only `.`) |
| ``Call `run()` first. Then call stop().`` | PASS |
| `Items include apples, pears, etc. and more.` | PASS |
| `Is it done? Yes. See the docs...then retry.` | PASS (3) |
| `The vs. comparison matters. Next one.` | PASS |
| `Read chapter 3 (pp. 10-12) before class.` | PASS |
| `We met at 3 a.m. Then we left.` / `Wait... Really?` | PASS (2 each) |
| `Use a crate, e.g. Serde, for this.` | **FAIL**: break before capitalized word |
| `This sentence is hard-wrapped\nacross two source lines.` | **FAIL**: LF is a paragraph separator (SB4) |

With two tailorings the spike got **18/20**:
1. Replace `\n` with a space before segmenting. This is byte-for-byte, so offsets are preserved.
2. Merge a segment into the next one when its last token is in `{mr., mrs., ms., dr., prof., st., e.g., i.e., etc., vs., fig., no., cf.}`.

The two remaining failures:
- `"Why?" and` stays split.
- `here.\nnext` becomes one sentence, which is what UAX #29 does for a lowercase continuation. That is acceptable.

The trade-off is that a sentence really ending in `etc.` gets merged with the next one.

### Recommendation

**UAX #29 sentences are authoritative, and each is sent to the parser as one pre-segmented sentence** (spec fallback).

Reasons:
1. Surface metrics (FRE sentence counts, `LongSentence`, diagnostic positions) must not change when a model is added or removed. With a model-independent split, `check` without a model and `check` with a model flag the same sentences.
2. UDPipe supports it directly (`presegmented`), and an ONNX route receives what we give it anyway.
3. One segmentation keeps positions simple.

The spec must add:
- **Segment per block, not per `Text` span.** M1 spans stop at soft breaks, inline code and links, so a hard-wrapped Markdown sentence arrives as several spans. Join the spans of one paragraph with single spaces and keep an offset map. Otherwise SB4 splits a sentence at every wrapped line: the spike's hard-wrap case fails without this.
- **An abbreviation merge list**, with the list and its trade-off in the spec. This is the tailoring UAX #29 itself anticipates.
- **The parser's tokens are not UAX #29 words.** For example, UD splits *don't* into *do* + *n't* and *U.S.* keeps its dots. Word Count and Fog use UAX #29 words; MDD and depth use parser tokens. State this so nobody expects N(MDD) to equal the word count.

## 6. Syllable counting against CMUdict

### Published numbers

On datascience.stackexchange (answer 89312), a regex heuristic "gets over 90% of cmudict correct … 0.9073751569397757 … For comparison, Pyphen is at 53.8% and the syllables function in the other answer is at 83.7%". That is an exact match against any pronunciation of all of NLTK's CMUdict.
<https://datascience.stackexchange.com/questions/23376/how-to-get-the-number-of-syllables-in-a-word>

meooow25/syllable, a small model trained on CMUdict: "has an accuracy of ~95%". <https://github.com/meooow25/syllable>

textstat does not count by rule for English. It looks the word up in CMUdict and falls back to Pyphen (`backend/counts/_count_syllables.py`).

### Spike

`.sdd/research/syllable_spike.py`: current cmusphinx `cmudict.dict`, 117,493 alphabetic entries. A count is a match if it equals any pronunciation. The seed is fixed.

| Counter | All CMUdict | 500 uniform from CMUdict | 500 from top-20k frequent ∩ CMUdict | Token-weighted (frequency) |
|---|---|---|---|---|
| naive vowel groups + silent-e | 83.8 % | 82.6 % | 85.2 % | 95.8 % |
| **regex heuristic** (SE 89312) | **91.2 %** | **88.8 %** | **93.8 %** | **97.7 %** |
| Liang, en_US patterns, min 2/3 (like `hyphenation`) | 51.5 % | 49.4 % | 63.0 % | 90.5 % |
| Liang, min 1/1 | 56.0 % | 53.2 % | 64.6 % | 84.1 % |

- This reproduces the published 90.7 % and 53.8 %.
- Liang **undercounts**: 6,512 under vs 38 over on the top 20k. That confirms the spec's worry, so hyphenation should not be used.
- On the top 20k, the ≥ 3-syllable decision that Fog needs agrees with CMUdict 96.2 % of the time for the regex counter and 87.9 % for Liang.
- The 95 % confidence interval at p = 0.9 and n = 500 is ±2.6 pp.
- Typical regex errors: *initiative, experiences, shareholders, excitement, basement, timeline, forehead, touches, seattle, karate*.

### Recommendation

Use a rule-based counter modelled on the regex heuristic. Commit a reference of **500 words sampled with a fixed seed from the frequent words in CMUdict**, for example CMUdict ∩ the top 20k of a frequency list, and require **≥ 90 %** agreement, a match being any pronunciation.
That leaves about 1.5 confidence intervals of margin for a counter of this quality (93.8 % measured).
Do **not** use a uniform CMUdict sample with a 90 % bar. It is full of names and loanwords, and this counter scores 88.8 % on it. If uniform sampling is preferred, use 85 %.

Licences:
- CMUdict is BSD-style (`Copyright (C) 1993-2015 Carnegie Mellon University … Redistribution and use in source and binary forms … are permitted`), so committing 500 entries with the notice is fine.
- hermitdave FrequencyWords is "CC-by-sa-4.0 for content". It is only used to choose the sample, but credit it in the measurement file.

Open detail for M2: the word definition includes numeric segments (`2024`, `3.14`), which contain no letter. textstat counts them as 1 (the Pyphen fallback gives 0 hyphens + 1). I recommend 1 syllable per numeric word, and saying so in the spec.

---

### Reproduce

```sh
cd .sdd/research && python3 syllable_spike.py && python3 nominal_strict.py
cd .sdd/uax29-spike && CARGO_HOME=$DEVENV_STATE/cargo cargo run -q [-- --tailored]
```

The downloads (CMUdict, en_50k, hyph_en_US.dic, NOMLEX, NomBank, WordNet 3.1) are in `.sdd/research/`. If the measurements should persist, they belong in `docs/specs/001-vernier/measurements/` (spec M2), which is not done here.

Network note: `nlp.cs.nyu.edu` served an incomplete TLS chain, so the NOMLEX and NomBank pages and files were fetched through web.archive.org. Everything else came directly from its source.
