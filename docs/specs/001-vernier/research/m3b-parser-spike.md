# M3b parser spike: `udpipe` route vs ONNX via `ort`

Spike for spec 001 M3b, first criterion: compare the two parser routes before any parser code lands; the user chooses the route from this report.
Spike run: 2026-09-27 01:16–01:53 (agent session that broke before writing this report).
Report reconstructed from its artifacts, with short single re-runs on 2026-09-27 09:27–09:35 (labelled *re-run*).
Not a decision: license and spec text changes are left to the next step.

Every number below cites its source.
Numbers are copied, each under its source path, into [measurements/m3b-spike-data.md](../measurements/m3b-spike-data.md) ("data" below); a citation names that file's section, with the raw log in parentheses.
The raw artifacts stay in `.sdd/m3b-spike/` (untracked scratch, B5); paths in `[…]` are relative to it.

## TL;DR

| | `udpipe` route (UDPipe 1 via `udpipe-rs` FFI, EWT model) | ONNX route (`ort` + RoBERTa goeswith model) |
|---|---|---|
| Builds in devenv today | yes, offline, no `devenv.nix` change | only with `download-binaries` (network at build time, breaks B4); B4-clean with nix `onnxruntime` added to devenv |
| Binary (stripped) | spike 1.27 MB; UDPipe code ≈ 0.66 MB of it | 27.0 MB static, or 4.6 MB + 28.5 MB `libonnxruntime.so` |
| Model the user supplies | 1 file, 16.3 MB | 3 files, 507 MB |
| Per sentence, median / p95 | 13–17 ms / 32–49 ms | 437–464 ms / 2.26–2.32 s (default threads); 1.14–1.37 s / 5.8–6.9 s (1 thread) |
| Whole sample (50 sentences, 864 words) | 0.57–0.90 s + 1.15–1.83 s model load | 28.7–29.2 s + 1.3–1.9 s load |
| Peak RSS | 104 MiB | 929–1054 MiB |
| Agreement with UDPipe 2 (UAS, sample) | 83.0 % (presegmented) | 94.8 % |
| M3a example sentence | **wrong**: center embedding (committee … caused, 4) | **right**: (proposal … caused, 8), all M3a metrics equal the reference |
| Lemmas / XPOS | yes | no (`_`); M4's "parser's lemma" is unavailable |
| Decoding | transition-based (Parsito), beam 5 | MST (Chu-Liu/Edmonds), as the model's `ud.py` |
| `unsafe` in vernier | none; in dependency (`udpipe-rs`, 15 lines) | none; in dependency (`ort`) |

**Recommendation: the `udpipe` route**, with a presegmented-tokenizer patch to `udpipe-rs`.
It is 25–60× faster, uses a tenth of the memory, and builds hermetically today.
Its price is lower parse quality, and with the EWT model it gets the M3a example sentence wrong, so the M3b integration-test criterion ("subject proposal, verb caused") fails with the model the spec names.
Details and the alternatives are in [Recommendation](#recommendation).

## Setup

- Machine: AMD Ryzen 7 PRO 5850U, 8 cores / 16 threads, CPU governor `powersave` (read at 09:27 and logged with each re-run in [data: Re-run](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-timings.log`); not logged during the 01:41 batch).
- Toolchain: devenv shell, Rust 1.98.1 (`.sdd/m3b-spike/logs/onnx-download-build.log` line 1), `CC=gcc` (gcc 15.3.0) and clang 21.1.8 on `PATH`.
- Sample: `docs/specs/001-vernier/measurements/parser-sample.md`, 50 sentences, 864 words, 2–65 words per sentence, median 18 ([data: Sample](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-sample-check.txt`)).
  The spike fed the sentences pre-segmented, one per line, from `[sentences.txt]`, which vernier's splitter produced from that file (identical, same log).
- Reference: UDPipe 2 via the LINDAT REST service, model `english-ewt-ud-2.17-251125` ([data: CoNLL-U: UDPipe 2 reference](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/udpipe2-rest.json`), `[out-udpipe2-ref.conllu]`); its 50 `# text` lines equal `[sentences.txt]`, so the input was one sentence per line (request parameters not logged); its sentence 9 is token-for-token `EXAMPLE_CONLLU` of `src/testing.rs` (only the MISC of the final `.` differs: `SpacesAfter=\n` vs `SpaceAfter=No`).
  UDPipe 2 is a parser, not gold: "agreement" below is agreement with a stronger parser, not accuracy.
- Metrics: the spike linked a copy of vernier taken before M4 (`[vernier-head/]`); `src/dependency.rs` and `src/syntax.rs` are identical to HEAD `bb2ea70`, so the M3a numbers are what vernier computes today.

### Spike binaries

| Binary | Source | What it does |
|---|---|---|
| `spike-udpipe` | `[udpipe/]`, `udpipe-rs = "=0.2.0"` | Loads the `.udpipe` model, parses each line of the sample `rounds` times, writes CoNLL-U, prints timings and the example's M3a metrics. Uses the crate's hard-wired default tokenizer, which may re-split a sentence. |
| `spike-udpipe-preseg` | `[udpipe-preseg/]`, path dep `[udpipe-rs-preseg/]` | Same, with `udpipe-rs` 0.2.0 patched in one line: `src/udpipe_wrapper.cpp` line 89 `new_tokenizer(model::DEFAULT)` → `new_tokenizer("presegmented")` (the crate's own MIT/Apache wrapper, not an MPL UDPipe file). |
| `spike-onnx` | `[onnx/]`, `ort = "=2.0.0-rc.13"` with `download-binaries`, `tokenizers = "=0.23.2"` | Model `ghotriw/roberta-base-english-ud-goeswith-onnx`; Rust port of the model's `ud.py` decoding (masked-token batch, goeswith constraint, Chu-Liu/Edmonds, single-root fix, subword merge). Args: rounds, threads (0 = ORT default). |
| `spike-onnx-nixlink` | `[onnx-nixlink/]`, `ort` without download, links nix `onnxruntime` 1.27.1 | Same code; dynamic link against `/nix/store/…-onnxruntime-1.27.1/lib` ([data: How `ort-sys` found ONNX Runtime](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/target-onnx-nixlink/release/build/ort-sys-*/output`)). |
| `spike-onnx-nixdyn` | `[onnx-nixdyn/]`, `ort` feature `load-dynamic` | Same code minus `TIMES_OUT`; loads `libonnxruntime.so` at run time via `ORT_DYLIB_PATH`. Built and run on the example only (`.sdd/m3b-spike/logs/x1.conllu`, `.sdd/m3b-spike/logs/x2.conllu`), not timed. |
| `compare`, `split`, `wordcounts` | `[common/src/bin/]` | Tree validity, UAS/LAS and M3a-metric agreement against the reference; the sample splitter; word counts. |

Timing method (`[udpipe/src/main.rs]`, `[onnx/src/main.rs]`): wall time of one `parse` call per sentence, model already loaded; the first call of a run is reported apart and excluded.
"Per sentence" below is the median over rounds for each of the 50 sentences, then the median and p95 (nearest rank) over the 50; with n = 50, p95 is the third-longest sentence (43–65 words).
"Cold" = run 1 of each series, after `posix_fadvise(DONTNEED)` on the model files (`[evict.py]`); `fincore` confirmed 0 B cached ([data: Timing batch](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/timings.log`)).

## (a) Build inside devenv

| Route | Command | Clean release build | Network | B4 | Source |
|---|---|---|---|---|---|
| udpipe | `cargo build --release --offline` | 2:41.6 wall, 455 s user, 298 % CPU, 585 MB peak RSS | none | yes | [data: Builds](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/udpipe-build.log`) |
| ONNX, `download-binaries` | `cargo build --release` | 1:06.4 wall, 738 s user, 1171 % CPU | build script downloads `libonnxruntime.a` (105 MB, ONNX Runtime 1.28.0) from `cdn.pyke.io` | **no** | [data: Builds](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/onnx-download-build.log`); [data: How `ort-sys` found ONNX Runtime](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/target-onnx-download/release/build/ort-sys-*/output`); [data: Sizes](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-sizes.txt`) |
| ONNX, nix linked | `cargo build --release --offline`, `ORT_LIB_LOCATION` or `ORT_LIB_PATH` → nix store (which one is not logged) | 1:07.9 wall, 699 s user | none | yes, once `onnxruntime` is in `devenv.nix` | [data: Builds](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/onnx-nixlink-build.log`) |
| ONNX, nix `load-dynamic` | `cargo build --release --offline` | 1:08.8 wall, 703 s user | none | yes at build; run time needs `ORT_DYLIB_PATH` | [data: Builds](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/onnx-nixdyn-build.log`) |

- udpipe compiles the 324 vendored UDPipe C++ files with the C++ compiler devenv already provides; the `devenv.nix` comment "clang / stdenv.cc and pkg-config" is not needed today.
- `udpipe-rs` 0.2.0 depends on `ureq` unconditionally (no feature gate, registry `Cargo.toml`), so `ureq`, `rustls` and `ring` are compiled ([data: Builds](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/udpipe-build.log`)) even though none of their symbols reach the binary (`nm`: 0 matches).
  `docs/specs/001-vernier/research/licenses.md` §1 calls this a `download` feature; in 0.2.0 it is not optional.
- The download build is hash-addressed in the cache (`xdg-cache/dfbin/…/e454f7…/libonnxruntime.a`) but is fetched by a build script outside `devenv.lock` and `Cargo.lock`, and it fails in an offline or sandboxed build.
- The nix linked binary carries a `RUNPATH` into the nix store ([data: Sizes](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-sizes.txt`)), so it runs inside devenv/nix; outside nix a user needs a matching `libonnxruntime.so`.
- The nixpkgs revision that supplied `onnxruntime` 1.27.1 is not logged (`.sdd/m3b-spike/logs/nix-ort.log` holds only `exit 0`); whether `devenv.lock`'s nixpkgs has the same version: not checked.
- Build times are not comparable across routes: udpipe compiles a C++ library, ONNX links a prebuilt one; udpipe's build ran at 298 % CPU against about 1100 % for the others.

## (b) Binary size

Stripped with `strip --strip-all` (llvm strip 21); re-stripped on 2026-09-27 from the `target-*` binaries ([data: Sizes](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-sizes.txt`), `.sdd/m3b-spike/logs/restrip/`), byte-identical to the spike's own `.sdd/m3b-spike/logs/*.stripped` for `spike-udpipe` and `spike-onnx`.

| Binary | Stripped size | Needed at run time | Of which (symbol sizes, unstripped) |
|---|---|---|---|
| `vernier` (pre-M4 copy, baseline) | 1,481,576 B | – | – |
| `spike-udpipe` | 1,271,200 B | libstdc++ | UDPipe C++ (`ufal::`) 0.66 MB |
| `spike-udpipe-preseg` | 1,271,520 B | libstdc++ | same |
| `spike-onnx` (static ORT 1.28.0) | 27,044,008 B | libstdc++ | ONNX Runtime 15.5 MB, `tokenizers` 0.70 MB |
| `spike-onnx-nixlink` | 4,628,064 B | `libonnxruntime.so.1` = 28,536,824 B | `tokenizers` 0.70 MB |
| `spike-onnx-nixdyn` | 4,612,952 B | same `.so`, via `ORT_DYLIB_PATH` | `tokenizers` 0.70 MB |

- The spike binaries link only the part of vernier they use, so they are smaller than `vernier` itself; vernier + udpipe was not built.
  Estimate: about +0.7–1.0 MB on vernier's 1.48 MB (UDPipe symbols 0.66 MB plus `libstdc++` glue); not measured.
- `.sdd/m3b-spike/logs/spike-onnx-download.stripped` (made at 01:31 from the first build) is 22,909,424 B, not 27.0 MB; the binary was rebuilt at 01:53 after `TIMES_OUT` was added; the cause of the 4 MB gap was not determined, and the table uses the reproducible value.
- Models ([data: Sizes](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-sizes.txt`)): `english-ewt-ud-2.5-191206.udpipe` 16,309,608 B; ONNX `model.onnx` 504,367,078 B + `tokenizer.json` 2,358,748 B + `config.json` 256,897 B.

## (c) Per-sentence time on the committed sample

UDPipe 1 is single-threaded: its runs used 98–99 % CPU ([data: Timing batch](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/timings.log`)), so "1 thread" and "default threads" are the same configuration.

### Timing batch 01:41–01:49 ([data: Timing batch](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/timings.log`), script `[run-timings.sh]`)

| Run | Load avg (1 min) | Model load | Per sentence median | p95 | max | Whole sample | Peak RSS | CPU |
|---|---|---|---|---|---|---|---|---|
| udpipe, run 1 cold | 5.98 | 1362 ms | 16.6 ms | 49.2 ms | 61.0 ms | 902 ms | 103.5 MiB | 98 % |
| udpipe, run 2 | 6.06 | 1452 ms | 13.1 ms | 32.3 ms | 44.3 ms | 669 ms | 103.6 MiB | 99 % |
| udpipe, run 3 | 5.74 | 1225 ms | 13.0 ms | 32.3 ms | 43.4 ms | 703 ms | 103.6 MiB | 99 % |
| udpipe-preseg, run 1 cold | 5.44 | 1826 ms | 14.0 ms | 34.8 ms | 47.2 ms | 757 ms | 103.6 MiB | 98 % |
| udpipe-preseg, run 2 | 5.16 | 1201 ms | 13.4 ms | 33.8 ms | 44.4 ms | 663 ms | 103.6 MiB | 99 % |
| udpipe-preseg, run 3 | 4.99 | 1154 ms | 13.5 ms | 36.4 ms | 48.3 ms | 656 ms | 103.5 MiB | 99 % |
| ONNX default threads, run 1 cold | 4.83 | 148 + 1558 ms | 464 ms | 2297 ms | 5237 ms | 28.7 s | 989 MiB | 924 % |
| ONNX default threads, run 2 | 9.94 | 103 + 1218 ms | 457 ms | 2323 ms | 5374 ms | 29.1 s | 990 MiB | 914 % |
| ONNX default threads, run 3 | 11.29 | 182 + 1643 ms | 438 ms | 2264 ms | 5179 ms | 28.8 s | 960 MiB | 890 % |
| ONNX 1 thread, run 1 cold | 12.81 | 148 + 1694 ms | 1142 ms | 6859 ms | 21869 ms | 97.0 s | 929 MiB | 98 % |
| ONNX 1 thread, run 2 | 7.73 | 101 + 1159 ms | 1374 ms | 5755 ms | 16811 ms | 90.2 s | 966 MiB | 99 % |
| ONNX nixlink default threads, cold | 5.40 | 96 + 1393 ms | 671 ms | 3853 ms | 6207 ms | 40.4 s | 1054 MiB | 1326 % |

- ONNX model load = tokenizer/labels load + session load; RSS after session load is 673–677 MiB in every ONNX run (`peak_rss_after_load`, KiB in the log).
- udpipe: 5 rounds per run; ONNX: 2 rounds (default threads, nixlink) or 1 round (1 thread), because the long sentences take seconds.
- ONNX time is inference: median inference 427–449 ms against median decoding 9.5–9.8 ms (default threads, [data: Timing batch](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/timings.log`)).

### Re-run 2026-09-27 09:30–09:32, single runs, warm cache ([data: Re-run](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-timings.log`))

| Run | Load avg | Model load | Median | p95 | max | Whole sample | CPU |
|---|---|---|---|---|---|---|---|
| udpipe-preseg, 5 rounds | 2.50 | 1268 ms | 12.2 ms | 32.8 ms | 41.8 ms | 565 ms | 98 % |
| ONNX download, default threads, 1 round | 2.54 | 94 + 1361 ms | 417 ms | 2204 ms | 5313 ms | 29.2 s | 882 % |
| ONNX nixlink, default threads, 1 round | 6.68 | 99 + 1281 ms | 582 ms | 2462 ms | 6629 ms | 39.1 s | 1347 % |
| ONNX nixlink, 8 threads, 1 round | 7.78 | 115 + 1135 ms | 395 ms | 2049 ms | 4687 ms | 26.7 s | 754 % |

Noise: the batch ran at load average 5–13, partly caused by the multi-threaded ONNX runs themselves; the udpipe re-run at load 2.5 was 6–27 % faster than the batch (median 12.2 ms vs 13.0–16.6 ms).
The nix build (ORT 1.27.1) with default threads ran 1.4–1.5× slower (median) than the download build (ORT 1.28.0) twice, while using 13–13.5 cores; with 8 intra-op threads it matched or beat the download build (single run).
Even its pure-Rust decoding step got slower (median 16 ms vs 9–10 ms), which points to thread oversubscription rather than a slower runtime; not investigated further.

### Time against sentence length (re-run, [data: Time against sentence length](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-length-bins.txt`))

| Words per sentence | Sentences | udpipe-preseg median | ms per word | ONNX default median | ms per word |
|---|---|---|---|---|---|
| 1–8 | 16 | 3.3 ms | 0.66 | 58 ms | 11.6 |
| 9–19 | 13 | 11.9 ms | 0.70 | 371 ms | 21.8 |
| 20–25 | 16 | 16.9 ms | 0.77 | 669 ms | 30.4 |
| 26–31 | 2 | 22.6 ms | 0.77 | 1087 ms | 36.9 |
| 32–65 | 3 | 35.7 ms | 0.78 | 2816 ms | 61.2 |
| 65 (longest) | 1 | 41.8 ms | 0.64 | 5313 ms | 81.7 |

UDPipe grows linearly with length; the ONNX model grows about quadratically.
That is architectural, not a runtime setting: the goeswith model runs one masked copy of the sentence per subword piece, a batch of *n* sequences of *n* + 3 tokens (`[onnx/src/main.rs]` `parse`, following `models/rbeg-onnx/ud.py` `_forward`).
Quantization or a GPU could shrink the constant; they cannot change the order (not measured).

## (d) Output on the M3a example sentence and the M3a metrics

*The proposal, which the executive committee rejected after extensive deliberation, caused significant delays.*

| Parser | Parse (key arcs) | MDD | Depth | Clauses | Center embedding | Source |
|---|---|---|---|---|---|---|
| UDPipe 2 reference (`EXAMPLE_CONLLU`) | proposal ←nsubj caused (root); rejected ←acl:relcl proposal | 32/12 = 2.667 | 4 | 1 | (proposal … caused, 8) | [data: Agreement with EXAMPLE_CONLLU](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-example-compare.txt`) (ONNX row agrees on all four) |
| udpipe, EWT (both tokenizers) | **proposal = root**; caused ←acl:relcl proposal; committee ←nsubj caused; which ←obj caused; rejected ←acl committee | 36/12 = 3.000 | 5 | 2 | **(committee … caused, 4)** | [data: M3a metrics per parser](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/udpipe-run1.txt`), [data: M3a metrics per parser](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/udpipe-preseg-run1.txt`) |
| ONNX | same heads as the reference for every content token; only the head of the first `,` differs (2 vs 8) | 32/12 = 2.667 | 4 | 1 | (proposal … caused, 8) | [data: M3a metrics per parser](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/onnx-full-r1.txt`), [data: Agreement with EXAMPLE_CONLLU](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-example-compare.txt`) |
| udpipe, GUM model (re-run) | proposal ←nsubj caused; committee ←acl:relcl proposal | 30/12 = 2.500 | 5 | 2 | (proposal … caused, 8) | [data: Agreement with the UDPipe 2 reference; M3a metrics per parser](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-preseg-english-gum.txt`) |
| udpipe, ParTUT model (re-run) | same content-token heads as the reference | 32/12 = 2.667 | 4 | 1 | (proposal … caused, 8) | [data: Agreement with the UDPipe 2 reference; M3a metrics per parser](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-preseg-english-partut.txt`) |
| udpipe, LinES model (re-run) | proposal = root, like EWT | 36/12 = 3.000 | 5 | 2 | (which … caused, 7), (committee … caused, 4) | [data: Agreement with the UDPipe 2 reference; M3a metrics per parser](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-preseg-english-lines.txt`) |

The UDPipe 1 EWT model reads the sentence as a garden path: "The proposal" becomes the root and "which … caused significant delays" its relative clause.
With default thresholds vernier would then flag a center embedding with the wrong subject (committee), and M3b's integration test "subject proposal, verb caused" would fail with the model the spec names.
All UDPipe 1 English models carry the same license (CC BY-NC-SA 4.0, research/licenses.md §1); GUM, ParTUT and LinES are also trained on NC treebanks.

### Whole sample against the UDPipe 2 reference ([data: Agreement with the UDPipe 2 reference](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-compare.txt`), [data: Agreement with the UDPipe 2 reference; M3a metrics per parser](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-preseg-*.txt`))

| Parser | Invalid trees | Tokenization differs | UAS | LAS | UAS w/o punct | Depth = | Clauses = | Center emb. = | MDD exact | All four = | Mean abs. ΔMDD |
|---|---|---|---|---|---|---|---|---|---|---|---|
| udpipe EWT, default tokenizer | 1 | 0 | 80.7 % | 78.7 % | 84.7 % | 34/49 | 43/49 | 43/49 | 23/49 | 23/49 | 0.242 |
| udpipe EWT, presegmented | 0 | 0 | 83.0 % | 81.0 % | 87.0 % | 35/50 | 43/50 | 44/50 | 23/50 | 23/50 | 0.241 |
| ONNX | 0 | 1 | 94.8 % | 92.1 % | 96.7 % | 40/50 | 48/50 | 48/50 | 31/50 | 29/50 | 0.089 |
| udpipe GUM, presegmented (re-run) | 0 | 3 | 85.2 % | 80.4 % | 86.7 % | 26/50 | 37/50 | 43/50 | 19/50 | 18/50 | 0.224 |
| udpipe ParTUT, presegmented (re-run) | 0 | 2 | 80.5 % | 73.7 % | 83.5 % | 27/50 | 34/50 | 41/50 | 18/50 | 17/50 | 0.241 |
| udpipe LinES, presegmented (re-run) | 0 | 2 | 83.9 % | 77.9 % | 86.2 % | 29/50 | 36/50 | 43/50 | 16/50 | 15/50 | 0.274 |

- UAS/LAS count only sentences whose tokens match the reference (974 tokens for EWT, 951 for ONNX, 908–931 for the others).
- The default tokenizer re-split sentence 27 at its colon ("…a clear pattern: the buildings…"), giving two trees for one vernier sentence, which vernier rejects (`IdOutOfOrder`); the presegmented patch removes this (`[out-udpipe.conllu]`, [data: Agreement with the UDPipe 2 reference](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-compare.txt`)).
  The spec requires pre-segmented input (*Sentence* definition), and `udpipe-rs` 0.2.0 offers no way to ask for it.
- ONNX's one tokenization difference is "cannot" (one token; UDPipe 2 splits it into `can` + `not` under a multiword line); ONNX emits no multiword tokens, no lemmas and no XPOS.
- Both routes name "Dr." rather than "Okafor" as the subject of sentence 18, where the reference has "Okafor".
- Flag agreement (HighMdd, DeepTree, ClauseOverload, CenterEmbedding per sentence) was not computed.

### Determinism

Every output repeated byte for byte ([data: Determinism](../measurements/m3b-spike-data.md); `cmp`, 2026-09-27): udpipe EWT 7 runs, udpipe-preseg 5 runs, ONNX 11 runs across 1 thread, default threads, 8 threads, ORT 1.28.0 static and ORT 1.27.1 nix (raw outputs: `.sdd/m3b-spike/out-*.conllu`, `.sdd/m3b-spike/logs/t-*.conllu`, `.sdd/m3b-spike/scratch/r-*.conllu`).

## ONNX decoding: MST, not greedy

The ONNX route decodes with a maximum spanning tree, as the model author's `ud.py` does: Chu-Liu/Edmonds over per-arc best-label scores, a goeswith constraint (subword arcs only rightwards and contiguous), then a single-root fix that re-runs Chu-Liu/Edmonds with non-chosen roots penalised (`models/rbeg-onnx/ud.py`, ported in `[onnx/src/main.rs]` `chu_liu_edmonds` and `decode`).

- Port against the original, on the same logits: `[parity.py]` runs the original `ud.py` postprocess on the logits the Rust binary dumped (`[dump/]`); its CoNLL-U is byte-identical to the Rust output ([data: ONNX decoder checks](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/out-onnx-python-udpy.conllu`) = `[out-onnx.conllu]`; re-run: [data: ONNX decoder checks](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-parity.out`)).
- Chu-Liu/Edmonds alone: 2000 random matrices (n = 1–29, seed 2026) through both implementations, 0 disagreements; 732 of them have a 2-cycle in the greedy argmax, so the contraction path is exercised (`[cle_fuzz.py]`, re-run [data: ONNX decoder checks](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-cle-fuzz.out`)).
- On the sample the extra machinery mattered in 1 of 50 sentences ("So are the taps.", 3 Chu-Liu/Edmonds calls, so greedy argmax heads are not a tree there); goeswith merges occurred in 17 of 50 ([data: ONNX decoder checks](../measurements/m3b-spike-data.md) (`.sdd/m3b-spike/logs/rerun-0927-parity-cov.out`)).
- Not checked: that the ONNX export gives the same logits as the original PyTorch model.

UDPipe 1, for comparison, is transition-based (Parsito) with beam search of width 5 by default (`vendor/udpipe/src/model/model_morphodita_parsito.cpp` line 127 in the `udpipe-rs` 0.2.0 crate).

## Licenses

Code, model and training data, per route; sources are research/licenses.md unless another file is cited.

| | udpipe route | ONNX route |
|---|---|---|
| Rust crate | `udpipe-rs` 0.2.0: MIT OR Apache-2.0 | `ort` / `ort-sys` 2.0.0-rc.13: MIT OR Apache-2.0; `tokenizers` 0.23.2: Apache-2.0 (registry `Cargo.toml`) |
| Native code | UDPipe 1.4.1-dev C++ (vendored in the crate): MPL-2.0, file-level copyleft; the crate's SPDX string omits it | ONNX Runtime: MIT (prebuilt by pyke for `download-binaries`, or nixpkgs) |
| Build-only extras | `ureq` MIT OR Apache-2.0, `ring` Apache-2.0 AND ISC, `rustls` (compiled, not linked) | download build: the same `ureq`/`rustls`/`ring` in `ort-sys`'s build script |
| Model | `english-ewt-ud-2.5-191206`: CC BY-NC-SA 4.0 (ÚFAL) | `ghotriw/roberta-base-english-ud-goeswith-onnx`: MIT declared (base model MIT) |
| Training data | UD_English-EWT 2.5: CC BY-SA 4.0 | EWT, Atis (CC BY-SA 4.0) + GUM, ParTUT, LinES (CC BY-NC-SA 4.0): an MIT label over NC data |
| Fits "personal, non-commercial, user supplies the model" | yes | yes, with the provenance caveat |

- The one-line presegmented patch touches `udpipe-rs`'s own `src/udpipe_wrapper.cpp` (MIT/Apache), not an MPL file; shipping it still means carrying a patched crate (vendored or `[patch]`), whose MPL-2.0 C++ files need their notices kept and their source available.
- Neither route redistributes a model; the spec's non-goals (no bundled model, no downloader) hold for both.
  The ONNX user must fetch three files (507 MB) and point `--model-path` at a directory; the udpipe user fetches one 16 MB file.
- Correction for the next step: licenses.md §1 says `udpipe-rs` "already has a `download` feature"; in 0.2.0 `ureq` is a mandatory dependency and `download_model` is always compiled.

## B4 hermeticity

- udpipe: hermetic today; every input is a crate in `Cargo.lock` plus the devenv C++ compiler.
- ONNX `download-binaries`: not hermetic; the build fetches a 105 MB static library from a third-party CDN, outside both lock files.
- ONNX nix linked: hermetic if `pkgs.onnxruntime` goes into `devenv.nix` (pinned by `devenv.lock`) with `ORT_LIB_LOCATION`, and the `ort` API feature matches the nixpkgs version (`api-27` for 1.27.1); binaries then depend on the nix store path at run time.
- ONNX nix `load-dynamic`: hermetic build, but the runtime library is found through `ORT_DYLIB_PATH` at run time, an environment contract vernier would have to document and test.
- A `cargo install vernier` user outside nix: udpipe needs only a C++ compiler; ONNX needs either the download (network in build) or a system ONNX Runtime of the matching version.

## `unsafe` / FFI and D9

- vernier's `Cargo.toml` sets `unsafe_code = "forbid"`, and neither route needs `unsafe` in vernier: the spike sources contain none (`grep`).
- The `unsafe` lives in the dependency: `udpipe-rs` has 15 lines with `unsafe` wrapping a C ABI over C++, including `unsafe impl Send for Model` (`[udpipe-rs-preseg/src/lib.rs]` line 290); `ort` has about 800 mentions over the ONNX Runtime C API.
- D9 names "an ADR for a named module"; a dependency is not a vernier module, so the letter of D9 is not triggered by either route.
  Its intent is: both routes run a C++ library in-process on user text.
- If the patched `udpipe-rs` is vendored into the repository, its `unsafe` code sits in the repository, and an ADR is clearly needed.
- Suggested ADR scope (next step): which crate may carry FFI, why, the patched file, and how an FFI failure maps to exit 2.

## Performance budget proposal (BACKLOG item 6)

Measured on the udpipe route (laptop, `powersave`, see above): model load 1.15–1.83 s, sentence median 12–17 ms, p95 32–49 ms, 0.64–0.78 ms per word, whole sample 0.57–0.90 s, peak RSS 104 MiB.
Proposed budget, about 1.5–2× headroom over those numbers:

| Scope | Budget |
|---|---|
| Model load, once per invocation (not per file) | ≤ 2.5 s cold |
| Per sentence | p95 ≤ 50 ms on the committed sample; no sentence of ≤ 70 words above 100 ms |
| Per document | parsing ≤ 1.5 s per 1000 words; `vernier check --model-path M parser-sample.md` ≤ 4 s wall |
| Memory | peak RSS ≤ 200 MiB |

- The ONNX route misses every line: 29 s for the sample, 5.3 s for the 65-word sentence, about 1 GiB RSS.
  A budget it could meet would be about 60 s per 1000 words and 10 s per sentence, which does not match "fast".
- Timing is not deterministic, so B3 argues against a `just verify` gate; the options are a measurement recipe whose result is recorded in `measurements/`, or an ignored test run on demand.

## Recommendation

**Choose the `udpipe` route** (UDPipe 1 through `udpipe-rs`, model supplied by the user).

Why:

- Speed: 25–60× faster per sentence (median 12–17 ms vs 395–671 ms) and per document (0.6–0.9 s vs 27–40 s), linear in sentence length; it meets the budget above with room to spare.
- Footprint: 104 MiB RSS vs about 1 GiB; one 16 MB model file vs 507 MB in three files; about +1 MB of binary vs +25 MB static or +3 MB plus a 28.5 MB shared library.
- Build: hermetic in today's devenv, offline, no `devenv.nix` change.
- Output completeness: lemmas and XPOS, which M4's parse path uses; the ONNX model gives neither.
- Code to own: a thin adapter; the ONNX route needs the ported decoder (about 250 lines incl. Chu-Liu/Edmonds, in `[onnx/src/main.rs]`), whose `chu_liu_edmonds` and `decode` would need splitting to meet the complexity limit of 10 (estimate; clippy not run on the spike).

The tradeoffs you accept with it:

- Parse quality: agreement with UDPipe 2 is 83 % UAS vs 95 % for ONNX; the four M3a metrics agree on 23 of 50 sentences vs 29, center embedding on 44 vs 48.
- The EWT model gets the M3a example wrong, so M3b's integration-test criterion fails with the model the spec names (see open question 3).
- `udpipe-rs` 0.2.0 cannot take pre-segmented input; you carry a one-line patch (vendored or `[patch.crates-io]`) or wait for an upstream change; the crate has one maintainer and 5 stars (research/licenses.md §3).
- `ureq`/`rustls`/`ring` are compiled into every build although unused.
- An in-process C++ library (MPL-2.0 part to be listed by hand in the license inventory).

When ONNX would be the better pick: if parse quality matters more than speed (for example, batch reports rather than an editor or CI loop), or if a faster UD model in ONNX form with clean training data turns up; the `Parser` trait keeps that door open for a later, optional backend.
If ONNX is chosen anyway, use the nix-linked `onnxruntime` with explicit intra-op threads = physical cores; never `download-binaries` (B4).

## Open questions for the user

1. Route: `udpipe` as recommended, or ONNX for quality?
2. Presegmented input: vendor the one-line patch to `udpipe-rs` (and offer it upstream), or write vernier's own thin FFI to UDPipe (more `unsafe`, in vernier)?
3. The M3a example with the EWT model: name a model that parses it right in the README and the test (GUM got the center embedding right and had the best UDPipe 1 UAS here; ParTUT matched all four metrics but agreed least over the sample), or change the M3b integration-test criterion?
4. ADR: one for accepting a C++ FFI dependency even though vernier itself stays `unsafe`-free, or only if the patched crate is vendored?
5. Performance budget: accept the numbers above, and as a gate or as a recorded measurement?
6. Is the unused `ureq`/`rustls`/`ring` build dependency of `udpipe-rs` acceptable, or should the patch also remove it?
7. Persistence: answered yes; the cited numbers are in [measurements/m3b-spike-data.md](../measurements/m3b-spike-data.md).
8. The repository has no `LICENSE` file although `Cargo.toml` declares `MIT OR Apache-2.0`, and parser-sample.md now points at that declaration; add the license files?

## Reproduce

From `.sdd/m3b-spike/`, inside `devenv shell` with `CARGO_HOME=$DEVENV_STATE/cargo`:

- Build: `cargo build --release --offline --manifest-path udpipe-preseg/Cargo.toml --target-dir target-udpipe-preseg` (likewise for the other crates; the nix linked variant needs `ORT_LIB_LOCATION` (or `ORT_LIB_PATH`) set to `$PWD/onnxruntime/lib`, the nix `load-dynamic` variant `ORT_DYLIB_PATH` at run time).
- Time: `./run-timings.sh` (about 8 minutes).
- Compare: `target-common/release/compare out-udpipe2-ref.conllu out-onnx.conllu`.
- Decoder parity: `py-numpy/bin/python3 parity.py dump out.conllu`, `py-numpy/bin/python3 cle_fuzz.py`.
