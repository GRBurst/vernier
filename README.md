# vernier

Readability and syntactic-complexity analyzer for Markdown, written in Rust.
Status: spec 001 is a draft. M1 (prose extraction) and M2 (sentences, syllables, Flesch Reading Ease, Flesch-Kincaid Grade, Gunning Fog, `check` flags `LongSentence`) are implemented. M3a (mean dependency distance, tree depth, clause count and center-embedding, computed from a UD parse; no parser is wired to the command line until M3b) is implemented too, and so is M4 (nominalization ratio; passive voice from a parse, reported absent without one), and M5 (compiler-style diagnostics, `--format text|compact|json`). M3b wires in a UD parser (ONNX via `ort`, chosen from `docs/specs/001-vernier/research/m3b-parser-spike.md`): with `--model-path` vernier also reports mean dependency distance, tree depth, subordinate clauses, center-embedding and passive voice. See `docs/HANDOVER.md`.

```sh
vernier analyze README.md                  # per file: M1 line + metric table; exit 0
vernier check README.md                    # warning[CognitiveOverload] per flagged sentence; exit 1 if any, 2 if a file is unreadable
vernier check --format compact README.md   # path:line:col: CognitiveOverload: LongSentence: …
vernier check --format json README.md      # one JSON document, schema_version 1
```

With a parser model (see the license section below):

```sh
git clone https://huggingface.co/ghotriw/roberta-base-english-ud-goeswith-onnx model   # needs git-lfs; ~507 MB
export ORT_DYLIB_PATH=/path/to/libonnxruntime.so   # ONNX Runtime >= 1.17; devenv sets it
vernier check --model-path model docs/architecture.md
```

The model directory must contain `config.json`, `tokenizer.json` and `onnx/model.onnx`. Without `--model-path`, vernier prints one notice on stderr and reports surface metrics only (no ONNX Runtime needed). A missing or unusable model or runtime exits 2. Parsing is slow (about 0.5 s per sentence, ~0.8 GB RAM); a sentence longer than the model's 509 subword pieces keeps its surface metrics and reads `absent (too long for the model)`.

Colour appears only on a terminal with `NO_COLOR` unset or empty. A CI example is in `docs/examples/github-actions.yml`.

```sh
devenv shell          # pinned toolchain: rust, just, python3, git hooks
just verify           # all gates
```

How work happens here: `AGENTS.md` → `docs/PROCESS.md`.

## Parser model license

No model is bundled or downloaded. Syntactic metrics (spec 001 M3b) need the ONNX Universal Dependencies model
`ghotriw/roberta-base-english-ud-goeswith-onnx` (an export of `KoichiYasuoka/roberta-base-english-ud-goeswith`), passed
with `--model-path`; without one, vernier reports surface metrics only. The model is declared MIT, but it is trained on
UD English EWT and Atis (CC BY-SA 4.0) and on GUM, ParTUT and LinES (CC BY-NC-SA 4.0), so treat it as usable for
personal, non-commercial work only (`docs/specs/001-vernier/research/licenses.md`). vernier never redistributes it.
(The UDPipe 1 models, a possible later backend, are CC BY-NC-SA 4.0.)

The syllable counter's reference list is sampled from CMUdict (BSD-style; notice in
`docs/specs/001-vernier/measurements/CMUDICT-LICENSE`).

The nominalization stoplist `data/nominalization-stoplist.txt` is seeded from pybiber (MIT; notice in
`data/nominalization-stoplist.PYBIBER-LICENSE`).
