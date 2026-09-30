# Spec 001 — vernier: Markdown readability and syntactic-complexity analyzer

**Status:** Draft

`vernier` is a Linux CLI in Rust that reads Markdown or plain-text files, keeps only the prose, and reports each file's metrics and every sentence that breaks a rule, with its line and column.
It computes classical surface metrics (Flesch Reading Ease, Flesch-Kincaid Grade, Gunning Fog) and syntactic metrics from a Universal Dependencies parse (mean dependency distance, tree depth, subordinate clauses, center-embedding), plus nominalization density and passive voice.
Each sentence that breaks a rule is rendered like a compiler diagnostic, as one compact line, or as JSON for CI.

Example (M5), with a model:

```text
$ vernier check --model-path models/roberta-base-english-ud-goeswith-onnx docs/architecture.md
warning[CognitiveOverload]: Sentence exceeds human working-memory capacity
  --> docs/architecture.md:42:1
   |
42 | The proposal, which the executive committee rejected after extensive deliberation, caused significant delays.
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = metrics:
     - words: 13 (max 25)
     - mean dependency distance: 2.67 (max 3.00)
     - tree depth: 4 edges (max 5)
     - subordinate clauses: 1 (max 2)
     - center-embedding: subject "proposal" separated from verb "caused" by 8 words (CenterEmbedding)
   = help: Move the clause between the subject and its verb after the verb, or make it a sentence of its own.

$ echo $?
1
```

This is the verbatim output of this command (2026-09-30), run on a file `docs/architecture.md` whose line 42 is the M3a example sentence, with the tested model (M3b) in `models/roberta-base-english-ud-goeswith-onnx`.

**Scope:**

- English prose in `.md` and plain-text files; one binary, no Python, JVM or PyTorch at runtime.
- Two commands: `vernier analyze <FILE>...` (report) and `vernier check <FILE>...` (lint, exit code).
- Syntactic metrics need a model (see Definitions) supplied by the user with `--model-path`, and ONNX Runtime ≥ 1.17 as a shared library, loaded from `ORT_DYLIB_PATH` or else as `libonnxruntime.so` from the loader's path (M3b); without `--model-path` neither is needed.
- The command line reports each file's metrics (see *File metrics*) and a diagnostic for every sentence that breaks a rule.
  The per-sentence values (surface scores, nominalization ratio, passive constructions with their positions, syntactic metrics) are library API (`analysis::analyze`, `analysis::analyze_parsed`); the command line shows a sentence's words and syntactic metrics only in its diagnostic, and the other per-sentence values not at all.

**Non-Goals:**

- THE tool SHALL NOT analyze languages other than English in this spec.
- THE tool SHALL NOT bundle a parser model in the binary or the repository (the user supplies it; license in M3b).
- THE tool SHALL NOT rewrite the user's text; suggestions are fixed advice per rule (M5 criterion 9), never generated sentences.
- THE tool SHALL NOT read a configuration file in this spec; thresholds come from flags only.
- THE tool SHALL NOT download a parser model in this spec; a model reaches the tool only through `--model-path`.

**Provenance:**

- Drafted from the user's project description of 2026-09-26 ("Project Specification: Standalone Markdown Cognitive Readability & Syntactic Complexity Analyzer in Rust").
- Decided by the user on 2026-09-26: the center-embedding distance counts the words strictly between subject and verb (the example's "9" became 8); the parser choice (udpipe vs ONNX via `ort`) stays open until a spike at the start of M3b; the model license is checked before M3b and no model is bundled.
  The open parser choice is superseded by the decision of 2026-09-27 below.
- Clarified by the user on 2026-09-26 (evidence: [open-questions report](2026-09-26-vernier-open-questions-answered.md), [research/](research/)): suggestions are fixed advice per rule (no criterion carried this answer until review pass 2 added M5 criterion 9); vernier is for personal, non-commercial use, so the CC BY-NC-SA 4.0 UDPipe 1 English model is acceptable when the user supplies it; no downloader; the parser route is still chosen by the M3b spike. The other ten markers were answered by research and accepted by the user.
  Superseded for the default parser by the decision of 2026-09-27 below; the UDPipe 1 model's acceptance applies to a UDPipe backend, now [BACKLOG](../../BACKLOG.md) item 10.
- Decided by the user on 2026-09-27 (evidence: [research/m3b-parser-spike.md](research/m3b-parser-spike.md)): the ONNX route (`ort` with the RoBERTa goeswith UD model) is the default parser, because the user's use is not time-critical and prefers accuracy (UAS 94.8 % against 83.0 % for UDPipe 1, and it parses the M3a example right); the spike's timings are a tendency, not isolated reproducible benchmarks; a user-selectable backend (UDPipe 1, or different models per metric) is a follow-up, with its conditions in [BACKLOG](../../BACKLOG.md) item 10.
- The M3b spike report, comparing the `udpipe` route and the ONNX-via-`ort` route on build inside devenv, binary size, per-sentence time on a committed sample and output on the M3a example sentence, was committed in `7f60be9` before any parser code; the user chose the ONNX route from it (above).
- Clarified in place while planning M5 on 2026-09-27 (spec `Draft`; evidence: [research/m5-rendering-spike.md](research/m5-rendering-spike.md), [plan-M5.md](plan-M5.md#spec-clarifications-t0-in-place-spec-draft)): which metrics a diagnostic lists, compact's code and message, JSON for both commands, `analyze`'s table under M1's line, positions equal across formats, the example's location, and the renderer and JSON crates.
- Revised in place after adversarial review pass 1 on 2026-09-30 (spec `Draft`; findings and repairs in [gates.md](gates.md)): criteria whose text had drifted from as-built decisions now state the behaviour of the code (multiple projected roots, sentences too long for the model, the exit contract, the CLI surface, the file metrics, the intro example).
  Its follow-up fixed two code defects the repair had written into the spec instead, passive voice 0 without a parse ([audit 014](../../audits/014-passive-voice-zero-without-a-parse.md)) and an absent MDD naming the wrong reason ([audit 015](../../audits/015-absent-mdd-names-the-wrong-reason.md)), and tested M3b criterion 1's notice.
- Revised in place after adversarial review pass 2 on 2026-09-30 (spec `Draft`; findings and repairs in [gates.md](gates.md)): a closed stdout exits 2 without a panic ([audit 016](../../audits/016-panic-on-a-closed-stdout.md)), the fixed advice of Q-B became M5 criterion 9 ([audit 017](../../audits/017-clarification-answer-lost-before-the-criteria.md)), a sentence's end no longer lands past closing markup ([audit 018](../../audits/018-a-sentence-end-past-closing-markup.md)), and the exit-0 and JSON promises, end positions, block prose and the model's labels now state the code.
  The as-built choices on projected roots, depth and center-embedding, which the user has not confirmed, carry `[NEEDS CLARIFICATION]` markers in M3a criteria 2, 4 and 6.

## Definitions

- **Word:** a UAX #29 word segment containing at least one alphabetic or numeric character; punctuation and whitespace segments are not words.
- **Block:** in a Markdown file, the prose spans of one paragraph or of one tight list item's text (a list item without a paragraph), in a list item or blockquote too; every start or end of a block-level element, such as a nested list, ends the current block. In a plain-text file, a block is a maximal run of non-blank lines (a blank line holds only whitespace), as one span of the file without its final line break.
- **Block prose:** a block's spans joined in source order: two consecutive spans are joined directly when nothing but the start or end of inline emphasis, strong, strikethrough, superscript, subscript or link markup, or the backslash of a backslash escape (`a\*b` reads `a*b`), lies between them, and with one space otherwise (a line break, dropped inline code or HTML, a dropped character reference, image or autolink). Every line feed and carriage return is read as a space, and every character keeps its source position.
- **Sentence:** a UAX #29 sentence segment of one block's prose containing at least one word, where a segment ending in an abbreviation from the committed list (`Mr.` `Mrs.` `Ms.` `Dr.` `Prof.` `St.` `e.g.` `i.e.` `etc.` `vs.` `Fig.` `No.` `cf.`) is merged with the next segment. This split is authoritative: each sentence goes to the parser pre-segmented, so sentences and positions are the same with or without a model.
- **Prose span:** in a Markdown file, the text of a `Text` event inside a paragraph or list item (also within a blockquote) and inside no dropped element (M1), with its byte range in the source file, where the source slice at that range equals the event's text (so a character reference such as `&amp;` gives no span); in a plain-text file, the whole file is one prose span (the count of M1's summary line), split into blocks as *Block* says.
- **Position:** 1-based line and 1-based column, the column counted in Unicode scalar values (as rustc does).
- **Content token:** a parsed token whose `upostag` is not `PUNCT`; content tokens are renumbered 1..N in sentence order before any distance is measured; a content token whose head is a `PUNCT` token is re-attached to its nearest non-`PUNCT` ancestor (this renumbered, re-attached tree is the *content projection*).
  A content token that is the root, or whose ancestors are all `PUNCT`, is a *projected root*; a sentence can have several projected roots.
- **Content dependency:** a content token that is not a projected root, with its head in the content projection; a sentence with N content tokens and R projected roots has N − R content dependencies.
- **Clausal relation:** a `deprel` whose part before any `:` is one of `advcl`, `acl`, `csubj`, `ccomp` (so `acl:relcl` counts).
- **Model:** a directory holding `config.json` (with `id2label`, `max_position_embeddings` and `pad_token_id`, below), `tokenizer.json` (with the special tokens `<s>`, `</s>` and `<mask>`) and `onnx/model.onnx`, of a Universal Dependencies goeswith token-classification model exported to ONNX.
  Its `id2label` numbers the labels 0 to n − 1 without a gap, each a string with at least one `|`: the part before the first `|` is the UPOS, the part after the last `|` the DEPREL, and the FEATS between them may contain `|` itself (`UPOS|FEATS|DEPREL`).
  Label 0 is reserved and never predicted; the labels must include `X|_|goeswith` and, besides label 0, a label ending in `|root` and a relation label that neither ends in `|root` nor is `X|_|goeswith`.
  `max_position_embeddings` and `pad_token_id` are non-negative integers, the first greater than the second plus 1.
  The model vernier is tested with is `ghotriw/roberta-base-english-ud-goeswith-onnx` (M3b).
- **File metrics:** the metrics `analyze` reports per file, each with its label in the text table and its key in the JSON `metrics` object; an absent metric reads `absent (<reason>)` in the table and is `null` in JSON (the key is always present).

  | Table label | JSON key | Absent when (table reason) |
  |---|---|---|
  | prose spans (M1's summary line) | `prose_spans` | never |
  | words (M1's summary line) | `words` | never |
  | sentences | `sentences` | never |
  | syllables | `syllables` | never |
  | complex words | `complex_words` | never |
  | Flesch Reading Ease | `flesch_reading_ease` | the file has no sentence (`no sentences`) |
  | Flesch-Kincaid Grade | `flesch_kincaid_grade` | the file has no sentence (`no sentences`) |
  | Gunning Fog | `gunning_fog` | the file has no sentence (`no sentences`) |
  | average sentence length | `average_sentence_length` | the file has no sentence (`no sentences`) |
  | mean dependency distance | `mean_dependency_distance` | no sentence was parsed: no `--model-path`, the file has no sentence, or every sentence too long for the model (`no parse`); or the parsed sentences have no content dependency (`no content dependency`) |
  | nominalization ratio, shown with its count as `0.077 (1 of 13 words)` | `nominalization_ratio`, and the count `nominalizations` (never absent) | the file has no word (`no words`) |
  | passive voice | `passives` | no sentence was parsed: no `--model-path`, the file has no sentence, or every sentence too long for the model (`no parse`) |

  Tree depth, subordinate clauses and center-embedding have no file-level value; they appear per sentence in diagnostics.

## M1 — CLI skeleton and prose extractor (Status: IMPLEMENTED)
<a id="M1"></a>

Parse Markdown with `pulldown-cmark`, keep only prose, and map every kept character back to its source position.

**Acceptance Criteria:**

- [x] WHEN `extract_prose` receives Markdown, THE extractor SHALL return only text from paragraphs, list items and blockquotes, each span carrying its source byte range.
- [x] THE extractor SHALL drop fenced and indented code blocks, inline code, HTML blocks and inline HTML, YAML (`---`) and TOML (`+++`) frontmatter, math, tables, headings, images with their alt text, and character references (`&amp;`, `&#169;`).
- [x] WHEN a link is extracted, THE extractor SHALL keep its link text and drop its URL; a bare autolink SHALL contribute no text.
- [x] THE slice of the source at each returned span's byte range SHALL equal the span text (property test over generated Markdown).
- [x] WHEN a byte offset is converted to a position, THE converter SHALL return the line and column that a scan of the source up to that offset yields (property test, including multi-byte characters and CRLF line ends).
- [x] WHEN `vernier analyze FILE` runs on a file that can be read as UTF-8 text, THE tool SHALL print per file the number of prose spans and words and exit 0, unless M5 criterion 2 requires 2.
- [x] IF a file cannot be read, or is not valid UTF-8, THEN THE tool SHALL name the file on stderr and exit 2.
- [x] THE CLI SHALL be defined with `clap` derive.

**Implementation Details:**

- `pulldown-cmark` options: tables, math, YAML and TOML metadata blocks enabled so they can be recognised and dropped; use `into_offset_iter()` for byte ranges.
- Plain-text files are one prose span each; `.md` and `.markdown` (any letter case) are Markdown, anything else is plain text.

## M2 — Surface readability engine (Status: IMPLEMENTED)
<a id="M2"></a>

Count words, sentences and syllables, and compute FRE, FKGL, Gunning Fog and average sentence length per file, and per sentence in the library API (`analysis::analyze`).

**Acceptance Criteria:**

- [x] THE engine SHALL compute FRE = 206.835 − 1.015·(words/sentences) − 84.6·(syllables/words), FKGL = 0.39·(words/sentences) + 11.8·(syllables/words) − 15.59 and Fog = 0.4·((words/sentences) + 100·(complex words/words)).
- [x] WHEN the counts are 100 words, 5 sentences and 150 syllables, THE engine SHALL return FRE 59.635 and FKGL 9.91 within 1e-9.
- [x] THE engine SHALL return a strictly lower FRE and a strictly higher FKGL when syllables increase while words and sentences stay fixed (property test).
- [x] IF a text has zero words or zero sentences, THEN THE engine SHALL report the metrics as absent rather than dividing by zero.
- [x] THE syllable counter SHALL return at least 1 for every word containing a letter, and exactly 1 for a word of digits only.
- [x] THE syllable counter SHALL agree on at least 90 % of a committed reference list of 500 frequent English words sampled from CMUdict (keeping CMU's license notice), measured and committed under `docs/specs/001-vernier/measurements/`.
- [x] THE engine SHALL count a word as complex when it has 3 or more syllables after subtracting one when the lower-cased word ends in `ing`, `ted`, `ded`, `ses`, `xes`, `zes`, `ches`, `shes`, `ces` or `ges` (a final suffix that is itself a syllable: `-ing`; `-ed` after `t` or `d`; `-es` after `s`, `x`, `z`, `ch`, `sh`, `c` or `g`, as in `sentences` and `packages`), and does not begin with an upper-case letter in a non-sentence-initial position (Gunning's rule for proper nouns).
- [x] WHEN a sentence has more than `--max-sentence-len` words (default 25), THE engine SHALL flag it as `LongSentence`.

**Implementation Details:**

- Word and sentence boundaries from `unicode-segmentation`.
- Syllables: a rule-based counter. Liang hyphenation is not used: it scored 63.0 % against the rule-based 93.8 % on 500 frequent CMUdict words ([research/metrics.md](research/metrics.md)).

## M3a — Syntactic metrics on a token graph (Status: IMPLEMENTED)
<a id="M3a"></a>

Compute the dependency metrics as pure functions over a list of tokens, tested on hand-built parses, with no parser involved yet.

**Acceptance Criteria:**

- [x] THE crate SHALL define `Token { id, form, lemma, upostag, head, deprel }` and a `Parser` trait returning, for one sentence, its tokens or that it is too long for the model (with its subword pieces and the model's maximum), so that the metrics depend on no parser implementation.
- [x] THE MDD function SHALL return the sum of |i − head(i)| over the sentence's content dependencies (i and head(i) numbered in the content projection), divided by the number of content dependencies; the file-level MDD SHALL be the total distance over all parsed sentences divided by their total number of content dependencies. [NEEDS CLARIFICATION: as built, a content token whose ancestors are all `PUNCT` becomes a projected root, so a sentence can have several; MDD divides by the number of content dependencies, N − R for N content tokens and R projected roots, and is absent when a sentence has no content dependency (criterion 3). Before review pass 1 this criterion said "the sum of |i − head(i)| over the N − 1 non-root content tokens, divided by N − 1", the file-level MDD divided "by (content tokens − sentences)", and criterion 3 made MDD absent when "a sentence has fewer than 2 content tokens". Do you confirm the as-built rule? Fallback: as built.]
- [x] WHEN a sentence has no content dependency (always when it has fewer than 2 content tokens), THE MDD function SHALL report MDD as absent, and the file-level MDD of a file with at least one parsed sentence SHALL be absent when its parsed sentences have none; the reason SHALL read `no content dependency`, never `no parse`, for such a sentence and such a file, while a file without a parsed sentence SHALL read `no parse` (see *File metrics* and M5).
- [x] THE depth function SHALL return the largest number of edges on a path from a projected root down to any content token (a sentence without a content dependency has depth 0); the `--max-tree-depth` help text SHALL say that depth counts edges from a projected root. [NEEDS CLARIFICATION: as built, depth counts from a projected root (Definitions), so a sentence with several projected roots takes its deepest path below any of them, and only content tokens count. Before review pass 1 this criterion said "the largest number of edges on a path from the root to any token (a root-only sentence has depth 0)". Do you confirm the as-built rule? Fallback: as built.]
- [x] THE clause counter SHALL return the number of content tokens whose `deprel` is a clausal relation.
- [x] WHEN a content token whose `deprel` is `nsubj` or `nsubj:pass` precedes its head in the content projection (a head of any part of speech, such as a copular adjective), and the subtree of some content token with a clausal relation lies strictly between the two in the content projection, THE detector SHALL report center-embedding with the subject's form, the head's form, and the number of parser tokens strictly between them whose form contains an alphabetic or numeric character, once per such subject; a diagnostic calls the head the verb (`verb "caused"`, M5), whatever its part of speech. [NEEDS CLARIFICATION: as built, the subject must precede its head (a subject after its head, as in a question or an inversion, is never reported), the head may be of any part of speech, so the diagnostic's "verb" can be an adjective, and the tokens counted between are the parser tokens whose form has a letter or digit. Before review pass 1 this criterion said "WHEN a clausal dependent's subtree lies wholly between a nominal subject (`nsubj`, `nsubj:pass`) and the subject's head verb", in either order, "THE detector SHALL report center-embedding with the subject, the verb and the number of words strictly between them". Do you confirm the as-built rule? Fallback: as built.]
- [x] WHEN the hand-built UD parse of "The proposal, which the executive committee rejected after extensive deliberation, caused significant delays." is given, THE detector SHALL report subject "proposal", verb "caused" and distance 8.
- [x] IF the input is not a tree (no token, ids out of order, a head out of range, several roots, or a cycle), THEN THE tree construction (`DependencyTree::new`) SHALL return an error, so no metric is computed from it.
- [x] THE depth of every well-formed tree with N ≥ 1 content tokens SHALL be at most N − 1, and its MDD, where present, at least 1 (property test over generated trees).
- [x] WHEN MDD exceeds `--max-mdd` (default 3.0), depth exceeds `--max-tree-depth` (default 5), or the clause count exceeds `--max-clauses` (default 2), THE engine SHALL flag the sentence with `HighMdd`, `DeepTree` or `ClauseOverload`; center-embedding SHALL be flagged `CenterEmbedding`.

## M3b — Parser integration and model handling (Status: IMPLEMENTED)
<a id="M3b"></a>

Put a real Universal Dependencies parser behind the `Parser` trait: the ONNX model of Definitions, run by ONNX Runtime through `ort`, the route a spike chose (Provenance).

**Acceptance Criteria:**

- [x] WHEN a sentence has more subword pieces than the model takes (the limit is `max_position_embeddings − pad_token_id − 4`, read from the model's `config.json`: 509 for the tested model), THE tool SHALL keep the sentence's surface metrics and flags, skip its syntactic metrics, print one notice on stderr, `vernier: PATH:LINE:COL: sentence too long for the model (N subword pieces, max M); syntactic metrics skipped`, with the position of the sentence's first character, and list its syntactic metrics in a diagnostic as `absent (too long for the model)`; the sentence SHALL NOT change the exit code.
- [x] THE README and this spec SHALL state the license of the chosen model and that part of its training data is licensed non-commercially. The chosen model, `ghotriw/roberta-base-english-ud-goeswith-onnx` (ONNX export of `KoichiYasuoka/roberta-base-english-ud-goeswith`), is declared MIT, but it is trained on UD English EWT and Atis (CC BY-SA 4.0) and on GUM, ParTUT and LinES (CC BY-NC-SA 4.0), so vernier treats it as non-commercial ([research/licenses.md](research/licenses.md)).
- [x] WHEN `--model-path` names a model, THE tool SHALL parse each sentence once and compute the M3a metrics of every sentence within the model's limit from the result.
- [x] WHEN the parsed M3a example sentence is analyzed, THE tool SHALL report center-embedding with subject "proposal" and verb "caused" (integration test in `tests/model.rs`, run when `VERNIER_TEST_MODEL` names a model: skipped with a printed reason when it is unset, failing when it is set but the model is unusable).
- [x] WHEN no `--model-path` is given, THE tool SHALL compute the surface metrics, print one notice on stderr that the syntactic metrics were skipped, and judge the exit code on surface rules only.
- [x] IF `--model-path` names a directory that is not a usable model, or ONNX Runtime cannot be loaded, THEN THE tool SHALL name the model directory and the cause on stderr, process no file, and exit 2.

## M4 — Nominalization and passive voice (Status: IMPLEMENTED)
<a id="M4"></a>

Count nominalizations (nouns derived from verbs or adjectives) and passive constructions.

**Acceptance Criteria:**

- [x] THE engine SHALL count as a nominalization a word whose lower-cased lemma ends in `-tion`, `-sion`, `-ment`, `-ance`, `-ence` or `-ity`, has at least 7 letters, and is not on the committed stoplist `data/nominalization-stoplist.txt` (seeded from pybiber, MIT, keeping its notice). Without a parse, the candidates SHALL be the sentence's words and a word's lemma the word with a trailing possessive `'s`, `’s`, `'` or `’` removed, then a plural `-ies` read as `-y`, or else one final `-s` removed unless it follows another `s`; with a parse, the candidates SHALL be the tokens tagged `NOUN`, with the parser's lemma, or, where the parser gives none (`_`, as the ONNX model does), the surface lemma of the token's form.
- [x] THE engine SHALL report per file, and per sentence in the library API, the nominalization ratio = nominalizations / words, the words counted as in M2 (with or without a parse); a file without words has no ratio.
- [x] WHEN a parse contains a token with `deprel` `aux:pass`, THE engine SHALL count its head as one passive construction (once per head) in the file's passive voice; in the library API each passive also carries its verb's form and position: the first character of the verb's form, found by matching the token forms in order against the sentence text, or the sentence's first character when a form is not found.
- [x] WHEN no sentence of a file was parsed (no `--model-path` was given, the file has no sentence, or every sentence was too long for the model), THE engine SHALL report the file's passive voice as absent, not as zero; otherwise the file's count SHALL be the sum over its parsed sentences, and in the library API a sentence without a parse SHALL have no passive count.

## M5 — Diagnostics, check mode and CI output (Status: IMPLEMENTED)
<a id="M5"></a>

Render findings like compiler diagnostics, add JSON and compact output, and make `check` usable as a CI gate.

**Acceptance Criteria:**

- [x] WHEN a sentence carries at least one flag, THE `check` command SHALL print, in the default `text` format, one `warning[CognitiveOverload]` diagnostic for it. The diagnostic SHALL point at the sentence's first character, SHALL underline the sentence from its first to its last character over all its lines (the source range of M5 criterion 3), and SHALL list every metric that a rule checks (words, mean dependency distance, tree depth, subordinate clauses, center-embedding) with its value, its maximum where it has one, and the flag, if raised. A syntactic metric of an unparsed sentence SHALL be listed as `absent (no parse)`, of a sentence too long for the model as `absent (too long for the model)`; the mean dependency distance of a parsed sentence without a content dependency SHALL be listed as `absent (no content dependency)`, and a parsed sentence without center-embedding SHALL list `center-embedding: none`.
- [x] THE `check` command SHALL exit 0 when no sentence is flagged and 1 when at least one is, in every format, unless an exit-2 cause of this criterion applies. IF any file cannot be read (including a file that is not valid UTF-8), the model or ONNX Runtime cannot be loaded, a sentence cannot be parsed (named on stderr as `PATH:LINE:COL` with the cause), or a write to stdout fails (such as a pipe whose reader has gone), THEN both commands SHALL exit 2, and 2 SHALL take precedence over 1. IF a write to stdout fails, THEN the tool SHALL stop at that write without a panic and say so in one stderr line, `vernier: cannot write to stdout: <cause>`. IF the command line is a usage error (clap, such as `vernier check` without files), THEN the tool SHALL exit 2.
- [x] WHEN `--format json` is given and the run is not ended early by a usage error or by a model or ONNX Runtime that cannot be loaded (M5 criterion 2, M3b criterion 6), THE tool (`analyze` and `check` alike) SHALL print, after all files, one JSON document with `schema_version` 1 and `files`, one object per file that was read and reported, with `path`, `metrics` (the file metrics under their JSON keys, see Definitions) and `diagnostics`; a file that cannot be read, or has a sentence that cannot be parsed, SHALL have no object. Each diagnostic SHALL have `code` (`CognitiveOverload`), `severity` (`warning`), `message` (`Sentence exceeds human working-memory capacity`), `line` and `column` (the sentence's first character, as in the text format), `end_line` and `end_column` (exclusive: the position just past the sentence's last character, which is never whitespace, so markup closing after that character lies outside; the text format's underline ends on the character before it) and `flags` (each with `name`, `message` and `help`, the flag's advice of M5 criterion 9).
- [x] WHEN `--format compact` is given, THE `check` command SHALL print one line per diagnostic as `path:line:col: code: message`, where the code is `CognitiveOverload` and the message joins the diagnostic's flag messages, each `FLAG: message`, with `; `.
- [x] THE `analyze` command SHALL print per file, in the `text` and `compact` formats, M1's summary line followed by a table with a `metric`/`value` header and one row per file metric with a table label below M1's line (see Definitions), in that order, an absent metric reading `absent (<reason>)`, and SHALL exit 0 whatever the metrics are, unless M5 criterion 2 requires 2.
- [x] THE rendered line and column of every diagnostic SHALL point at the sentence's first character in the source, and SHALL be the same in the text, compact and JSON formats; its JSON `end_line` and `end_column` SHALL point just past the sentence's last character, and a one-line underline of the text format SHALL run from the column to the character before that end (property test over generated documents).
- [x] THE repository SHALL contain a GitHub Actions example under `docs/examples/` that runs `vernier check` and fails the job on exit 1.
- [x] THE CLI SHALL offer two subcommands, `vernier analyze [OPTIONS] <FILES>...` and `vernier check [OPTIONS] <FILES>...`, each requiring at least one file and accepting the options `--format text|json|compact` (default `text`), `--max-sentence-len` (default 25), `--max-mdd` (a finite number above 0, default 3), `--max-tree-depth` (default 5), `--max-clauses` (default 2) and `--model-path` (no default), as `vernier <COMMAND> --help` shows them.
- [x] WHEN a diagnostic is printed in the `text` format, THE `check` command SHALL end it with one `= help: <advice>` line per flag of the diagnostic, in the order of its flags (as in JSON `flags` and the compact message), so a sentence with two center-embeddings gets two; the advice SHALL be the fixed text of the flag, never a quotation of the sentence: `LongSentence` "Split the sentence into shorter ones.", `HighMdd` "Put words that belong together closer, e.g. the verb near its subject.", `DeepTree` "Flatten nested phrases, or move some into a sentence of their own.", `ClauseOverload` "Move a subordinate clause into a sentence of its own.", `CenterEmbedding` "Move the clause between the subject and its verb after the verb, or make it a sentence of its own."; the `compact` format SHALL carry no advice.

**Implementation Details:**

- Rendering through `annotate-snippets` (chosen over `miette`, whose content depends on the terminal: [research/m5-rendering-spike.md](research/m5-rendering-spike.md)); colour only when stdout is a terminal and `NO_COLOR` is unset or empty, with the same text either way (unverified on a terminal; a by-hand check is owed, HANDOVER).
- Exit 2 also follows when the JSON document cannot be serialized; no test reaches that path.
- A file that cannot be read, or holds a sentence that cannot be parsed, is named on stderr and not reported; the other files still are.
- JSON through `serde` + `serde_json`, derived on output types that exist only for the document.
