# Spec 001 — vernier: Markdown readability and syntactic-complexity analyzer

**Status:** Draft

`vernier` is a self-contained Linux CLI in Rust that reads Markdown or plain-text files, keeps only the prose, and reports how hard each sentence is to read.
It computes classical surface metrics (Flesch Reading Ease, Flesch-Kincaid Grade, Gunning Fog) and syntactic metrics from a Universal Dependencies parse (mean dependency distance, tree depth, subordinate clauses, center-embedding), plus nominalization density and passive voice.
Every finding points at a line and column in the original file, rendered like a compiler diagnostic, or as JSON for CI.

Example of the target behaviour (M5):

```text
$ vernier check docs/architecture.md
warning[CognitiveOverload]: Sentence exceeds human working-memory capacity
  --> docs/architecture.md:42:1
   |
42 | The proposal, which the executive committee rejected after extensive deliberation, caused significant delays.
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = Metrics:
     - Word Count: 13 words (OK)
     - Mean Dependency Distance (MDD): 3.67 (High, threshold <= 3.0)
     - Center-Embedding: YES (subject "proposal" separated from verb "caused" by 8 words)
$ echo $?
1
```

The metric values in this example are illustrative; the criteria below, not the example, bind the numbers.

**Scope:**

- English prose in `.md` and plain-text files; one binary, no Python, JVM or PyTorch at runtime.
- Two commands: `vernier analyze <FILE>...` (report) and `vernier check <FILE>...` (lint, exit code).
- Syntactic metrics need a Universal Dependencies model supplied by the user (`--model-path`, M3b).

**Non-Goals:**

- THE tool SHALL NOT analyze languages other than English in this spec.
- THE tool SHALL NOT bundle a parser model in the binary or the repository (the user supplies it; license in M3b).
- THE tool SHALL NOT rewrite the user's text; suggestions are fixed advice per rule, never generated sentences.
- THE tool SHALL NOT read a configuration file in this spec; thresholds come from flags only.
- THE tool SHALL NOT download a parser model in this spec; a model reaches the tool only through `--model-path`.

**Provenance:**

- Drafted from the user's project description of 2026-09-26 ("Project Specification: Standalone Markdown Cognitive Readability & Syntactic Complexity Analyzer in Rust").
- Decided by the user on 2026-09-26: the center-embedding distance counts the words strictly between subject and verb (the example's "9" became 8); the parser choice (udpipe vs ONNX via `ort`) stays open until a spike at the start of M3b; the model license is checked before M3b and no model is bundled.
- Clarified by the user on 2026-09-26 (evidence: [open-questions report](2026-09-26-vernier-open-questions-answered.md), [research/](research/)): suggestions are fixed advice per rule; vernier is for personal, non-commercial use, so the CC BY-NC-SA 4.0 UDPipe 1 English model is acceptable when the user supplies it; no downloader; the parser route is still chosen by the M3b spike. The other ten markers were answered by research and accepted by the user.
- Decided by the user on 2026-09-27 (evidence: [research/m3b-parser-spike.md](research/m3b-parser-spike.md)): the ONNX route (`ort` with the RoBERTa goeswith UD model) is the default parser, because the user's use is not time-critical and prefers accuracy (UAS 94.8 % against 83.0 % for UDPipe 1, and it parses the M3a example right); the spike's timings are a tendency, not isolated reproducible benchmarks; a user-selectable backend (UDPipe 1, or different models per metric) is a follow-up, and when UDPipe comes its example test accepts the GUM or ParTUT model and the README warns that EWT parses the example wrong.
- Clarified in place while planning M5 on 2026-09-27 (spec `Draft`; evidence: [research/m5-rendering-spike.md](research/m5-rendering-spike.md), [plan-M5.md](plan-M5.md#spec-clarifications-t0-in-place-spec-draft)): which metrics a diagnostic lists, compact's code and message, JSON for both commands, `analyze`'s table under M1's line, positions equal across formats, the example's location, and the renderer and JSON crates.

## Definitions

- **Word:** a UAX #29 word segment containing at least one alphabetic or numeric character; punctuation and whitespace segments are not words.
- **Block prose:** the prose spans of one paragraph (in a list item or blockquote too), joined in source order: two consecutive spans are joined directly when nothing but the start or end of inline emphasis, strong, strikethrough, superscript, subscript or link markup lies between them, and with one space otherwise (a line break, dropped inline code or HTML, a dropped character reference, image or autolink). Every line feed and carriage return is read as a space, and every character keeps its source position. In a plain-text file, blank lines (lines of only whitespace) separate blocks.
- **Sentence:** a UAX #29 sentence segment of one block's prose containing at least one word, where a segment ending in an abbreviation from the committed list (`Mr.` `Mrs.` `Ms.` `Dr.` `Prof.` `St.` `e.g.` `i.e.` `etc.` `vs.` `Fig.` `No.` `cf.`) is merged with the next segment. This split is authoritative: each sentence goes to the parser pre-segmented, so sentences and positions are the same with or without a model.
- **Prose span:** a run of text from a `Text` event inside a paragraph, list item or blockquote, with its byte range in the source file.
- **Position:** 1-based line and 1-based column, the column counted in Unicode scalar values (as rustc does).
- **Content token:** a parsed token whose `upostag` is not `PUNCT`; content tokens are renumbered 1..N in sentence order before any distance is measured; a content token whose head is a `PUNCT` token is re-attached to its nearest non-`PUNCT` ancestor.
- **Clausal relation:** a `deprel` whose part before any `:` is one of `advcl`, `acl`, `csubj`, `ccomp` (so `acl:relcl` counts).

## M1 — CLI skeleton and prose extractor (Status: IMPLEMENTED)
<a id="M1"></a>

Parse Markdown with `pulldown-cmark`, keep only prose, and map every kept character back to its source position.

**Acceptance Criteria:**

- [x] WHEN `extract_prose` receives Markdown, THE extractor SHALL return only text from paragraphs, list items and blockquotes, each span carrying its source byte range.
- [x] THE extractor SHALL drop fenced and indented code blocks, inline code, HTML blocks and inline HTML, YAML (`---`) and TOML (`+++`) frontmatter, math, tables, headings and image alt text.
- [x] WHEN a link is extracted, THE extractor SHALL keep its link text and drop its URL; a bare autolink SHALL contribute no text.
- [x] FOR every returned span, THE slice of the source at its byte range SHALL equal the span text (property test over generated Markdown).
- [x] WHEN a byte offset is converted to a position, THE converter SHALL return the line and column that a scan of the source up to that offset yields (property test, including multi-byte characters and CRLF line ends).
- [x] WHEN `vernier analyze FILE` runs on a readable file, THE tool SHALL print per file the number of prose spans and words and exit 0.
- [x] WHEN a file cannot be read, THE tool SHALL name the file on stderr and exit 2.
- [x] THE CLI SHALL be defined with `clap` derive and accept the flags of M5 from this milestone on, even where a flag has no effect yet.

**Implementation Details:**

- `pulldown-cmark` options: tables, math, YAML and TOML metadata blocks enabled so they can be recognised and dropped; use `into_offset_iter()` for byte ranges.
- Plain-text files are one prose span each; `.md` and `.markdown` (any letter case) are Markdown, anything else is plain text.

## M2 — Surface readability engine (Status: IMPLEMENTED)
<a id="M2"></a>

Count words, sentences and syllables, and compute FRE, FKGL, Gunning Fog and average sentence length per sentence and per file.

**Acceptance Criteria:**

- [x] THE engine SHALL compute FRE = 206.835 − 1.015·(words/sentences) − 84.6·(syllables/words), FKGL = 0.39·(words/sentences) + 11.8·(syllables/words) − 15.59 and Fog = 0.4·((words/sentences) + 100·(complex words/words)).
- [x] WHEN the counts are 100 words, 5 sentences and 150 syllables, THE engine SHALL return FRE 59.635 and FKGL 9.91 within 1e-9.
- [x] FOR fixed words and sentences, THE engine SHALL return a strictly lower FRE and a strictly higher FKGL when syllables increase (property test).
- [x] WHEN a text has zero words or zero sentences, THE engine SHALL report the metrics as absent rather than dividing by zero.
- [x] THE syllable counter SHALL return at least 1 for every word containing a letter, and exactly 1 for a word of digits only.
- [x] THE syllable counter SHALL agree on at least 90 % of a committed reference list of 500 frequent English words sampled from CMUdict (keeping CMU's license notice), measured and committed under `docs/specs/001-vernier/measurements/`.
- [x] THE engine SHALL count a word as complex when it has 3 or more syllables after subtracting one for a final suffix that is itself a syllable (`-ing`; `-ed` after `t` or `d`; `-es` after `s`, `x`, `z`, `ch`, `sh`, `ce` or `ge`), and is not capitalized in a non-sentence-initial position (Gunning's rule for proper nouns).
- [x] WHEN a sentence has more than `--max-sentence-len` words (default 25), THE engine SHALL flag it as `LongSentence`.

**Implementation Details:**

- Word and sentence boundaries from `unicode-segmentation`.
- Syllables: a rule-based counter. Liang hyphenation is not used: it scored 63.0 % against the rule-based 93.8 % on 500 frequent CMUdict words ([research/metrics.md](research/metrics.md)).

## M3a — Syntactic metrics on a token graph (Status: IMPLEMENTED)
<a id="M3a"></a>

Compute the dependency metrics as pure functions over a list of tokens, tested on hand-built parses, with no parser involved yet.

**Acceptance Criteria:**

- [x] THE crate SHALL define `Token { id, form, lemma, upostag, head, deprel }` and a `Parser` trait returning the tokens of one sentence, so that the metrics depend on no parser implementation.
- [x] THE MDD function SHALL return the sum of |i − head(i)| over the N − 1 non-root content tokens, divided by N − 1; the file-level MDD SHALL be the total distance over all sentences divided by (content tokens − sentences).
- [x] WHEN a sentence has fewer than 2 content tokens, THE MDD function SHALL report MDD as absent.
- [x] THE depth function SHALL return the largest number of edges on a path from the root to any token (a root-only sentence has depth 0); the `--max-tree-depth` help text SHALL say that depth counts edges.
- [x] THE clause counter SHALL return the number of tokens whose `deprel` is a clausal relation.
- [x] WHEN a clausal dependent's subtree lies wholly between a nominal subject (`nsubj`, `nsubj:pass`) and the subject's head verb, THE detector SHALL report center-embedding with the subject, the verb and the number of words strictly between them.
- [x] WHEN the hand-built UD parse of "The proposal, which the executive committee rejected after extensive deliberation, caused significant delays." is given, THE detector SHALL report subject "proposal", verb "caused" and distance 8.
- [x] WHEN the input is not a tree (a cycle, several roots, or a head out of range), THE metric functions SHALL return an error instead of a value.
- [x] FOR every well-formed tree, THE depth SHALL be at most N − 1 and the MDD at least 1 (property test over generated trees).
- [x] WHEN MDD exceeds `--max-mdd` (default 3.0), depth exceeds `--max-tree-depth` (default 5), or the clause count exceeds `--max-clauses` (default 2), THE engine SHALL flag the sentence with `HighMdd`, `DeepTree` or `ClauseOverload`; center-embedding SHALL be flagged `CenterEmbedding`.

## M3b — Parser integration and model handling (Status: PLANNED)
<a id="M3b"></a>

Put a real Universal Dependencies parser behind the `Parser` trait, after a short spike decides which one.

**Acceptance Criteria:**

- [x] BEFORE any parser code lands, THE milestone SHALL commit a spike report comparing the `udpipe` route and the ONNX-via-`ort` route on build inside devenv, binary size, per-sentence time on a committed sample, and output on the M3a example sentence; the user chooses the route from that report (chosen 2026-09-27: ONNX via `ort`; see Provenance).
- [ ] BEFORE model download or loading code lands, THE milestone SHALL record the license of the chosen model in the README and in this spec; the tool SHALL load it only where that license permits the user's intended use (personal, non-commercial), and SHALL never redistribute it. The chosen model, `ghotriw/roberta-base-english-ud-goeswith-onnx` (ONNX export of `KoichiYasuoka/roberta-base-english-ud-goeswith`), is declared MIT, but it is trained on UD English EWT and Atis (CC BY-SA 4.0) and on GUM, ParTUT and LinES (CC BY-NC-SA 4.0), so vernier treats it as non-commercial ([research/licenses.md](research/licenses.md)).
- [ ] WHEN `--model-path` names a readable model, THE tool SHALL parse each sentence and compute the M3a metrics from the result.
- [ ] WHEN the parsed M3a example sentence is analyzed, THE tool SHALL report center-embedding with subject "proposal" and verb "caused" (integration test, skipped with a printed reason when no model is present).
- [ ] WHEN no `--model-path` is given, THE tool SHALL compute the surface metrics, print one notice on stderr that the syntactic metrics were skipped, and judge the exit code on surface rules only.
- [ ] WHEN `--model-path` names a missing or unusable model, THE tool SHALL name it on stderr and exit 2.

## M4 — Nominalization and passive voice (Status: IMPLEMENTED)
<a id="M4"></a>

Count nominalizations (nouns derived from verbs or adjectives) and passive constructions.

**Acceptance Criteria:**

- [x] THE engine SHALL count as a nominalization a word whose lower-cased lemma ends in `-tion`, `-sion`, `-ment`, `-ance`, `-ence` or `-ity`, is at least 7 letters long, and is not on the committed stoplist `data/nominalization-stoplist.txt` (seeded from pybiber, MIT, keeping its notice). Without a parse, the candidates are the sentence's words and a word's lemma is the word with a trailing possessive `'s`, `’s`, `'` or `’` removed, then a plural `-ies` read as `-y`, or else one final `-s` removed unless it follows another `s`; with a parse, the candidates are the tokens tagged `NOUN`, with the parser's lemma, or, where the parser gives none (`_`, as the ONNX model does), the surface lemma of the token's form.
- [x] THE engine SHALL report per sentence and per file the nominalization ratio = nominalizations / words, the words counted as in M2 (with or without a parse); a file without words has no ratio.
- [x] WHEN a parse contains a token with `deprel` `aux:pass`, THE engine SHALL report its head verb as a passive construction (once per head) with its position: the first character of the verb's form, found by matching the token forms in order against the sentence text, or the sentence's first character when a form is not found.
- [x] WHEN no parse exists, THE engine SHALL report passive voice as absent, not as zero.

## M5 — Diagnostics, check mode and CI output (Status: IMPLEMENTED)
<a id="M5"></a>

Render findings like compiler diagnostics, add JSON and compact output, and make `check` usable as a CI gate.

**Acceptance Criteria:**

- [x] WHEN a sentence carries at least one flag, THE `check` command SHALL print, in the default `text` format, one `warning[CognitiveOverload]` diagnostic for it. The diagnostic points at the sentence's first character, underlines the sentence over all its lines, and lists every metric that a rule checks (words, mean dependency distance, tree depth, subordinate clauses, center-embedding) with its value, its maximum and the flag it raised; a syntactic metric of an unparsed sentence is listed as `absent (no parse)`.
- [x] THE `check` command SHALL exit 0 when no sentence is flagged, 1 when at least one is, and 2 on an unreadable file or an unusable model, in every format (the unusable-model case is exercised once M3b loads models).
- [x] WHEN `--format json` is given, THE tool (`analyze` and `check` alike) SHALL print, after all files, one JSON document with `schema_version` 1 and, per readable file, its path, its metrics and a diagnostics array whose positions equal those of the text format.
- [x] WHEN `--format compact` is given, THE `check` command SHALL print one line per diagnostic as `path:line:col: code: message`, where the code is `CognitiveOverload` and the message joins the diagnostic's flag messages with `; `.
- [x] THE `analyze` command SHALL print per file, in the `text` and `compact` formats, M1's summary line followed by a table of the file-level metrics (one row per metric, `absent` where a metric is absent), and exit 0 whatever the metrics are.
- [x] FOR every diagnostic, THE rendered line and column SHALL point at the sentence's first character in the source, and SHALL be the same in the text, compact and JSON formats (property test over generated documents).
- [x] THE repository SHALL contain a GitHub Actions example under `docs/examples/` that runs `vernier check` and fails the job on exit 1.

**Implementation Details:**

- Rendering through `annotate-snippets` (chosen over `miette`, whose content depends on the terminal: [research/m5-rendering-spike.md](research/m5-rendering-spike.md)); colour only when stdout is a terminal and `NO_COLOR` is unset or empty, with the same text either way.
- JSON through `serde` + `serde_json`, derived on output types that exist only for the document.
