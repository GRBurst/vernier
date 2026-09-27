# vernier

Readability and syntactic-complexity analyzer for Markdown, written in Rust.
Status: spec 001 is a draft. M1 (prose extraction) and M2 (sentences, syllables, Flesch Reading Ease, Flesch-Kincaid Grade, Gunning Fog, `check` flags `LongSentence`) are implemented. M3a (mean dependency distance, tree depth, clause count and center-embedding, computed from a UD parse; no parser is wired to the command line until M3b) is implemented too, and so is M4 (nominalization ratio; passive voice from a parse, reported absent without one), and M5 (compiler-style diagnostics, `--format text|compact|json`). The parser (M3b) is not wired in yet: its route is chosen from `docs/specs/001-vernier/research/m3b-parser-spike.md`. See `docs/HANDOVER.md`.

```sh
vernier analyze README.md                  # per file: M1 line + metric table; exit 0
vernier check README.md                    # warning[CognitiveOverload] per flagged sentence; exit 1 if any, 2 if a file is unreadable
vernier check --format compact README.md   # path:line:col: CognitiveOverload: LongSentence: …
vernier check --format json README.md      # one JSON document, schema_version 1
```

Colour appears only on a terminal with `NO_COLOR` unset or empty. A CI example is in `docs/examples/github-actions.yml`.

```sh
devenv shell          # pinned toolchain: rust, just, python3, git hooks
just verify           # all gates
```

How work happens here: `AGENTS.md` → `docs/PROCESS.md`.

## Parser model license

No model is bundled. The UDPipe Universal Dependencies models are CC BY-NC-SA 4.0 (non-commercial; verified,
`docs/specs/001-vernier/research/licenses.md`). Syntactic metrics (spec 001 M3b) need such a model, passed with
`--model-path`; without one, vernier reports surface metrics only.

The syllable counter's reference list is sampled from CMUdict (BSD-style; notice in
`docs/specs/001-vernier/measurements/CMUDICT-LICENSE`).

The nominalization stoplist `data/nominalization-stoplist.txt` is seeded from pybiber (MIT; notice in
`data/nominalization-stoplist.PYBIBER-LICENSE`).
