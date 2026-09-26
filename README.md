# vernier

Readability and syntactic-complexity analyzer for Markdown, written in Rust.
Status: spec 001 is a draft. M1 (prose extraction) and M2 (sentences, syllables, Flesch Reading Ease, Flesch-Kincaid Grade, Gunning Fog, `check` flags `LongSentence`) are implemented. See `docs/HANDOVER.md`.

```sh
vernier analyze README.md          # per file: spans, words, sentences, syllables, scores
vernier check README.md            # path:line:col: LongSentence: …; exit 1 if any flag, 2 if a file is unreadable
```

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
