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
- Syntactic metrics need a Universal Dependencies model supplied by the user (`--model-path`) or fetched by the tool (M3b).

**Non-Goals:**

- THE tool SHALL NOT analyze languages other than English in this spec.
- THE tool SHALL NOT bundle a parser model in the binary or the repository (license open, see M3b).
- THE tool SHALL NOT rewrite the user's text; suggestions are fixed advice per rule, never generated sentences. [NEEDS CLARIFICATION: your example suggestion quotes a rewritten sentence; generating one needs a language model, which contradicts "no heavy runtime". Fallback: fixed advice text per rule, no generated rewrite.]
- THE tool SHALL NOT read a configuration file in this spec; thresholds come from flags only.

**Provenance:**

- Drafted from the user's project description of 2026-09-26 ("Project Specification: Standalone Markdown Cognitive Readability & Syntactic Complexity Analyzer in Rust").
- Decided by the user on 2026-09-26: the center-embedding distance counts the words strictly between subject and verb (the example's "9" became 8); the parser choice (udpipe vs ONNX via `ort`) stays open until a spike at the start of M3b; the model license is checked before M3b and no model is bundled.

## Definitions

- **Word:** a UAX #29 word segment containing at least one alphabetic or numeric character; punctuation and whitespace segments are not words.
- **Sentence:** a UAX #29 sentence segment of the extracted prose containing at least one word. [NEEDS CLARIFICATION: the parser also splits sentences; which split is authoritative? Fallback: UAX #29 sentences are the unit, and each one is handed to the parser as one pre-segmented sentence.]
- **Prose span:** a run of text from a `Text` event inside a paragraph, list item or blockquote, with its byte range in the source file.
- **Position:** 1-based line and 1-based column, the column counted in Unicode scalar values (as rustc does).
- **Content token:** a parsed token whose `upostag` is not `PUNCT`; content tokens are renumbered 1..N in sentence order before any distance is measured. [NEEDS CLARIFICATION: renumber after dropping punctuation, or keep the parser's ids? Fallback: renumber.]
- **Clausal relation:** a `deprel` whose part before any `:` is one of `advcl`, `acl`, `csubj`, `ccomp` (so `acl:relcl` counts).

## M1 — CLI skeleton and prose extractor (Status: PLANNED)
<a id="M1"></a>

Parse Markdown with `pulldown-cmark`, keep only prose, and map every kept character back to its source position.

**Acceptance Criteria:**

- [ ] WHEN `extract_prose` receives Markdown, THE extractor SHALL return only text from paragraphs, list items and blockquotes, each span carrying its source byte range.
- [ ] THE extractor SHALL drop fenced and indented code blocks, inline code, HTML blocks and inline HTML, YAML (`---`) and TOML (`+++`) frontmatter, math, tables, headings and image alt text. [NEEDS CLARIFICATION: headings and image alt text are not in your list of kept or dropped content. Fallback: both dropped.]
- [ ] WHEN a link is extracted, THE extractor SHALL keep its link text and drop its URL; a bare autolink SHALL contribute no text.
- [ ] FOR every returned span, THE slice of the source at its byte range SHALL equal the span text (property test over generated Markdown).
- [ ] WHEN a byte offset is converted to a position, THE converter SHALL return the line and column that a scan of the source up to that offset yields (property test, including multi-byte characters and CRLF line ends).
- [ ] WHEN `vernier analyze FILE` runs on a readable file, THE tool SHALL print per file the number of prose spans and words and exit 0.
- [ ] WHEN a file cannot be read, THE tool SHALL name the file on stderr and exit 2.
- [ ] THE CLI SHALL be defined with `clap` derive and accept the flags of M5 from this milestone on, even where a flag has no effect yet.

**Implementation Details:**

- `pulldown-cmark` options: tables, math, YAML and TOML metadata blocks enabled so they can be recognised and dropped; use `into_offset_iter()` for byte ranges.
- Plain-text files (not `.md`) are one prose span each. [NEEDS CLARIFICATION: detect plain text by extension? Fallback: `.md` and `.markdown` are Markdown, anything else is plain text.]

## M2 — Surface readability engine (Status: PLANNED)
<a id="M2"></a>

Count words, sentences and syllables, and compute FRE, FKGL, Gunning Fog and average sentence length per sentence and per file.

**Acceptance Criteria:**

- [ ] THE engine SHALL compute FRE = 206.835 − 1.015·(words/sentences) − 84.6·(syllables/words), FKGL = 0.39·(words/sentences) + 11.8·(syllables/words) − 15.59 and Fog = 0.4·((words/sentences) + 100·(complex words/words)).
- [ ] WHEN the counts are 100 words, 5 sentences and 150 syllables, THE engine SHALL return FRE 59.635 and FKGL 9.91 within 1e-9.
- [ ] FOR fixed words and sentences, THE engine SHALL return a strictly lower FRE and a strictly higher FKGL when syllables increase (property test).
- [ ] WHEN a text has zero words or zero sentences, THE engine SHALL report the metrics as absent rather than dividing by zero.
- [ ] THE syllable counter SHALL return at least 1 for every word containing a letter.
- [ ] THE syllable counter SHALL agree with a committed reference list of English words and syllable counts on a share of words the reference measurement records. [NEEDS CLARIFICATION: which reference (e.g. a CMUdict sample) and which agreement is enough? Fallback: 500 words sampled from CMUdict, agreement at least 90 %, measured and committed under `docs/specs/001-vernier/measurements/`.]
- [ ] THE engine SHALL count a word as complex when it has 3 or more syllables after removing a final `-ed`, `-es` or `-ing`, and is not capitalized in a non-sentence-initial position. [NEEDS CLARIFICATION: "common proper nouns" has no list. Fallback: the capitalization rule stated here stands in for it.]
- [ ] WHEN a sentence has more than `--max-sentence-len` words (default 25), THE engine SHALL flag it as `LongSentence`.

**Implementation Details:**

- Word and sentence boundaries from `unicode-segmentation`.
- Syllables: a rule-based counter first; `hyphenation` (Liang) only if it measures better, because hyphenation points undercount syllables near word edges.

## M3a — Syntactic metrics on a token graph (Status: PLANNED)
<a id="M3a"></a>

Compute the dependency metrics as pure functions over a list of tokens, tested on hand-built parses, with no parser involved yet.

**Acceptance Criteria:**

- [ ] THE crate SHALL define `Token { id, form, lemma, upostag, head, deprel }` and a `Parser` trait returning the tokens of one sentence, so that the metrics depend on no parser implementation.
- [ ] THE MDD function SHALL return the sum of |i − head(i)| over the N − 1 non-root content tokens, divided by N − 1.
- [ ] WHEN a sentence has fewer than 2 content tokens, THE MDD function SHALL report MDD as absent.
- [ ] THE depth function SHALL return the largest number of edges on a path from the root to any token (a root-only sentence has depth 0). [NEEDS CLARIFICATION: count edges (root = 0) or nodes (root = 1)? Fallback: edges.]
- [ ] THE clause counter SHALL return the number of tokens whose `deprel` is a clausal relation.
- [ ] WHEN a clausal dependent's subtree lies wholly between a nominal subject (`nsubj`, `nsubj:pass`) and the subject's head verb, THE detector SHALL report center-embedding with the subject, the verb and the number of words strictly between them.
- [ ] WHEN the hand-built UD parse of "The proposal, which the executive committee rejected after extensive deliberation, caused significant delays." is given, THE detector SHALL report subject "proposal", verb "caused" and distance 8.
- [ ] WHEN the input is not a tree (a cycle, several roots, or a head out of range), THE metric functions SHALL return an error instead of a value.
- [ ] FOR every well-formed tree, THE depth SHALL be at most N − 1 and the MDD at least 1 (property test over generated trees).
- [ ] WHEN MDD exceeds `--max-mdd` (default 3.0), depth exceeds `--max-tree-depth` (default 5), or the clause count exceeds `--max-clauses` (default 2), THE engine SHALL flag the sentence with `HighMdd`, `DeepTree` or `ClauseOverload`; center-embedding SHALL be flagged `CenterEmbedding`.

## M3b — Parser integration and model handling (Status: PLANNED)
<a id="M3b"></a>

Put a real Universal Dependencies parser behind the `Parser` trait, after a short spike decides which one.

**Acceptance Criteria:**

- [ ] BEFORE any parser code lands, THE milestone SHALL commit a spike report comparing the `udpipe` route and the ONNX-via-`ort` route on build inside devenv, binary size, per-sentence time on a committed sample, and output on the M3a example sentence; the user chooses the route from that report.
- [ ] BEFORE model download or loading code lands, THE milestone SHALL record the license of the chosen model in the README and in this spec; the tool SHALL download or load it only where that license permits the user's intended use. [NEEDS CLARIFICATION: the standard UDPipe UD 2.5 models are believed to be CC BY-NC-SA, not verified. Fallback: the user supplies the model; the tool never redistributes it.]
- [ ] WHEN `--model-path` names a readable model, THE tool SHALL parse each sentence and compute the M3a metrics from the result.
- [ ] WHEN the parsed M3a example sentence is analyzed, THE tool SHALL report center-embedding with subject "proposal" and verb "caused" (integration test, skipped with a printed reason when no model is present).
- [ ] WHEN no model is available, THE tool SHALL compute the surface metrics, print one notice on stderr that the syntactic metrics were skipped, and judge the exit code on surface rules only. [NEEDS CLARIFICATION: in CI this silently weakens `check`. Fallback as stated; alternative: exit 2 unless a `--surface-only` flag is given.]
- [ ] WHERE the downloader is enabled, THE tool SHALL store the model under `$XDG_CACHE_HOME/vernier/` (or `~/.cache/vernier/`), verify a pinned checksum, and never download during `check`. [NEEDS CLARIFICATION: is a downloader wanted at all, given the license question? Fallback: a separate `vernier model fetch` command.]

## M4 — Nominalization and passive voice (Status: PLANNED)
<a id="M4"></a>

Count abstract deverbal nouns and passive constructions.

**Acceptance Criteria:**

- [ ] THE engine SHALL count as a nominalization a word ending in `-tion`, `-sion`, `-ment`, `-ance`, `-ence` or `-ity` that is at least 7 letters long, is tagged `NOUN` when a parse exists, and is not on a committed stoplist. [NEEDS CLARIFICATION: the suffix test alone matches words like "nation" or "city". Fallback: length ≥ 7, NOUN tag and a stoplist in `data/nominalization-stoplist.txt`.]
- [ ] THE engine SHALL report per sentence and per file the nominalization ratio = nominalizations / words.
- [ ] WHEN a parse contains a token with `deprel` `aux:pass`, THE engine SHALL report its head verb as a passive construction with its position.
- [ ] WHEN no parse exists, THE engine SHALL report passive voice as absent, not as zero.

## M5 — Diagnostics, check mode and CI output (Status: PLANNED)
<a id="M5"></a>

Render findings like compiler diagnostics, add JSON and compact output, and make `check` usable as a CI gate.

**Acceptance Criteria:**

- [ ] WHEN a sentence carries at least one flag, THE `check` command SHALL print one `warning[CognitiveOverload]` diagnostic for it, pointing at the sentence's first character, underlining the sentence, and listing every metric with the flags it raised. [NEEDS CLARIFICATION: one diagnostic per sentence, or one per rule (`LongSentence`, `HighMdd`, ...)? Fallback: one per sentence, as in your example.]
- [ ] THE `check` command SHALL exit 0 when no sentence is flagged, 1 when at least one is, and 2 on an unreadable file or an unusable model.
- [ ] WHEN `--format json` is given, THE tool SHALL print one JSON document with `schema_version` 1, per-file metrics and a diagnostics array whose positions equal those of the text format.
- [ ] WHEN `--format compact` is given, THE tool SHALL print one line per diagnostic as `path:line:col: code: message`.
- [ ] THE `analyze` command SHALL print per file a table of the file-level metrics and exit 0 whatever the metrics are.
- [ ] FOR every diagnostic, THE rendered line and column SHALL point at the sentence's first character in the source (property test over generated documents).
- [ ] THE repository SHALL contain a GitHub Actions example that runs `vernier check` and fails the job on exit 1.

**Implementation Details:**

- Rendering through `miette` or `annotate-snippets`; choose the one whose output survives `NO_COLOR` and a non-TTY stdout unchanged in content.
