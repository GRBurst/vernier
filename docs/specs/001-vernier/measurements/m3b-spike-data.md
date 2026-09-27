# M3b parser spike: data

Condensed copy of the numbers that [research/m3b-parser-spike.md](../research/m3b-parser-spike.md) cites.
The raw artifacts stay in `.sdd/m3b-spike/` (untracked scratch); each block below names the file it was copied from.
Blocks are copied verbatim or reduced to their measured lines (the repository path is shortened to `<repo>`); nothing was re-computed for this file.

Machine: AMD Ryzen 7 PRO 5850U (8 cores, 16 threads), CPU governor `powersave`, inside the devenv shell (Rust 1.98.1).
Sample: `parser-sample.md` in this directory, fed one sentence per line (50 sentences, 864 words).
Reference: UDPipe 2, LINDAT REST service, model `english-ewt-ud-2.17-251125`.
"Cold" runs follow `posix_fadvise(DONTNEED)` on the model files; "page cache before" is `fincore`'s cached size.
Per-sentence statistics: the median over rounds for each sentence, then median, p95 (nearest rank) and range over the 50 sentences.

## Sample

### Segmentation and word counts

Source: `.sdd/m3b-spike/logs/rerun-0927-sample-check.txt`.

```text
# vernier HEAD bb2ea70 target/debug/vernier analyze parser-sample.md
parser-sample.md: 12 prose spans, 864 words
  50 sentences, 1222 syllables, 68 complex words
  Flesch Reading Ease 69.64, Flesch-Kincaid Grade 7.84, Gunning Fog 10.06, average sentence length 17.28
  nominalization ratio 0.010 (9 of 864 words), passive voice absent (no parse)
# spike split (vernier-head copy) vs sentences.txt
identical
sentences=50 words=864 min=2 median=18 max=65
sorted: [2, 2, 3, 4, 4, 4, 4, 5, 5, 5, 6, 6, 6, 7, 7, 8, 13, 13, 13, 15, 16, 17, 17, 17, 17, 18, 19, 19, 19, 20, 21, 21, 21, 21, 21, 22, 22, 22, 23, 24, 24, 24, 24, 25, 25, 28, 31, 43, 46, 65]
```

## Builds

### Clean release builds

Source: `.sdd/m3b-spike/logs/{udpipe,onnx-download,onnx-nixlink,onnx-nixdyn}-build.log`.

```text
udpipe: "cargo build --release --offline"; Finished `release` profile [optimized] target(s) in 2m 41s; user 455.11 s; CPU 298%; max RSS 585368 KiB  [logs/udpipe-build.log]
onnx download-binaries: "cargo build --release"; Finished `release` profile [optimized] target(s) in 1m 06s; user 738.31 s; CPU 1171%; max RSS 777784 KiB  [logs/onnx-download-build.log]
onnx nix linked: "cargo build --release --offline"; Finished `release` profile [optimized] target(s) in 1m 07s; user 698.79 s; CPU 1075%; max RSS 774292 KiB  [logs/onnx-nixlink-build.log]
onnx nix load-dynamic: "cargo build --release --offline"; Finished `release` profile [optimized] target(s) in 1m 08s; user 702.92 s; CPU 1066%; max RSS 771920 KiB  [logs/onnx-nixdyn-build.log]
```

### How `ort-sys` found ONNX Runtime

Source: `.sdd/m3b-spike/target-onnx-{download,nixlink}/release/build/ort-sys-*/output`.

```text
[target-onnx-download/release/build/ort-sys-3ae9d7b9738c82fc/output]
[ort-sys] [DEBUG] Using prebuilt binaries
cargo:rustc-cfg=pyke
[ort-sys] [DEBUG] looking for prebuilt binaries matching feature set: (no features)
[ort-sys] [DEBUG] downloading from 'https://cdn.pyke.io/0/pyke:ort-rs/ms@1.28.0/x86_64-unknown-linux-gnu.tar.lzma2'; tls_provider=Rustls, root_certs=WebPki
cargo:rustc-link-lib=stdc++
cargo:rustc-link-search=native=<repo>/.sdd/m3b-spike/onnx/../xdg-cache/dfbin/x86_64-unknown-linux-gnu/e454f710f8a49f53aa5b4ff51e3454ae1835777e431c6c35c5255ce6f205fd68
cargo:rustc-link-lib=static=onnxruntime
[target-onnx-nixlink/release/build/ort-sys-7fd1873de9222847/output]
cargo:rustc-link-lib=onnxruntime
cargo:rustc-link-search=native=/nix/store/x46rnziynpalr4phm6yh0x46s42xq5kw-onnxruntime-1.27.1/lib
```

## Sizes

### Stripped binaries, symbol shares, native libraries, models

Source: `.sdd/m3b-spike/logs/rerun-0927-sizes.txt` (strip and `nm` run 2026-09-27).

```text
# strip --strip-all (llvm strip 21) of target-*/release binaries, 2026-09-27
 27,044,008 B  spike-onnx
  4,612,952 B  spike-onnx-nixdyn
  4,628,064 B  spike-onnx-nixlink
  1,271,200 B  spike-udpipe
  1,271,520 B  spike-udpipe-preseg
  1,481,576 B  vernier
== target-udpipe/release/spike-udpipe
  symbols total=1.17MB ufal(UDPipe C++)=0.66MB onnxruntime=0.00MB tokenizers=0.00MB
  needed: libstdc++.so.6 libc.so.6 ld-linux-x86-64.so.2 libm.so.6 libgcc_s.so.1
== target-onnx-download/release/spike-onnx
  symbols total=24.15MB ufal(UDPipe C++)=0.00MB onnxruntime=15.51MB tokenizers=0.70MB
  needed: libc.so.6 ld-linux-x86-64.so.2 libm.so.6 libstdc++.so.6 libgcc_s.so.1
== target-onnx-nixlink/release/spike-onnx-nixlink
  symbols total=3.30MB ufal(UDPipe C++)=0.00MB onnxruntime=0.01MB tokenizers=0.70MB
  needed: libc.so.6 ld-linux-x86-64.so.2 libm.so.6 libonnxruntime.so.1 libgcc_s.so.1
== target-vernier/release/vernier
  symbols total=1.09MB ufal(UDPipe C++)=0.00MB onnxruntime=0.00MB tokenizers=0.00MB
  needed: libgcc_s.so.1 libc.so.6 ld-linux-x86-64.so.2
 28,536,824 B  onnxruntime/lib/libonnxruntime.so.1.27.1
  5,033,530 B  target-udpipe/release/build/udpipe-rs-ab9a3b1093bb1913/out/libudpipe.a
105,481,448 B  xdg-cache/dfbin/x86_64-unknown-linux-gnu/e454f710f8a49f53aa5b4ff51e3454ae1835777e431c6c35c5255ce6f205fd68/libonnxruntime.a
 16,309,608 B  models/english-ewt-ud-2.5-191206.udpipe
  8,880,020 B  models/english-gum-ud-2.5-191206.udpipe
  6,479,962 B  models/english-lines-ud-2.5-191206.udpipe
  5,740,109 B  models/english-partut-ud-2.5-191206.udpipe
    256,897 B  models/rbeg-onnx/config.json
504,367,078 B  models/rbeg-onnx/onnx/model.onnx
  2,358,748 B  models/rbeg-onnx/tokenizer.json
```

## Timings

### Timing batch, 2026-09-27 01:41–01:49

Source: `.sdd/m3b-spike/logs/timings.log` (script `.sdd/m3b-spike/run-timings.sh`).

```text
- 01:41:19 load1=5.98 page cache before=0B :: spike-udpipe, rounds=5, 1 thread
  model_load=1362.178ms first_parse_call=4.801ms
  per sentence (median of rounds, n=50): median=16.583ms p95=49.196ms min=1.471ms max=61.037ms
  whole_sample_last_round=901.655ms; wall=6.10s maxrss=105480KiB cpu=98%
  peak_rss_after_load=105872KiB peak_rss_end=106008KiB
- 01:41:25 load1=6.06 page cache before=15.6M :: spike-udpipe, rounds=5, 1 thread
  model_load=1452.028ms first_parse_call=3.236ms
  per sentence (median of rounds, n=50): median=13.085ms p95=32.327ms min=1.152ms max=44.315ms
  whole_sample_last_round=669.101ms; wall=4.72s maxrss=105484KiB cpu=99%
  peak_rss_after_load=105924KiB peak_rss_end=106120KiB
- 01:41:30 load1=5.74 page cache before=15.6M :: spike-udpipe, rounds=5, 1 thread
  model_load=1224.641ms first_parse_call=3.243ms
  per sentence (median of rounds, n=50): median=12.963ms p95=32.334ms min=1.141ms max=43.389ms
  whole_sample_last_round=703.460ms; wall=4.51s maxrss=105908KiB cpu=99%
  peak_rss_after_load=105832KiB peak_rss_end=106088KiB
- 01:41:34 load1=5.44 page cache before=0B :: spike-udpipe-preseg, rounds=5, 1 thread
  model_load=1826.475ms first_parse_call=3.238ms
  per sentence (median of rounds, n=50): median=13.964ms p95=34.847ms min=1.281ms max=47.234ms
  whole_sample_last_round=756.886ms; wall=5.38s maxrss=105420KiB cpu=98%
  peak_rss_after_load=105900KiB peak_rss_end=106040KiB
- 01:41:40 load1=5.16 page cache before=15.6M :: spike-udpipe-preseg, rounds=5, 1 thread
  model_load=1201.284ms first_parse_call=3.400ms
  per sentence (median of rounds, n=50): median=13.372ms p95=33.785ms min=1.175ms max=44.373ms
  whole_sample_last_round=663.197ms; wall=4.51s maxrss=105812KiB cpu=99%
  peak_rss_after_load=105928KiB peak_rss_end=106064KiB
- 01:41:44 load1=4.99 page cache before=15.6M :: spike-udpipe-preseg, rounds=5, 1 thread
  model_load=1153.709ms first_parse_call=3.013ms
  per sentence (median of rounds, n=50): median=13.522ms p95=36.449ms min=1.127ms max=48.282ms
  whole_sample_last_round=656.366ms; wall=4.57s maxrss=105772KiB cpu=99%
  peak_rss_after_load=105892KiB peak_rss_end=106024KiB
- 01:41:49 load1=4.83 page cache before=0B+0B :: spike-onnx, rounds=2, threads=ORT default
  tokenizer+labels_load=147.944ms session_load=1557.938ms first_parse_call=40.438ms
  per sentence (median of rounds, n=50): median=464.133ms p95=2296.699ms min=30.970ms max=5237.377ms
  all samples: inference median=449.410ms, decoding median=9.485ms
  whole_sample_last_round=28667.007ms; wall=59.52s maxrss=1012884KiB cpu=924%
  peak_rss_after_load=689676KiB peak_rss_end=1012884KiB
- 01:42:48 load1=9.94 page cache before=481M+2.3M :: spike-onnx, rounds=2, threads=ORT default
  tokenizer+labels_load=103.109ms session_load=1218.144ms first_parse_call=42.874ms
  per sentence (median of rounds, n=50): median=456.945ms p95=2323.328ms min=30.221ms max=5373.702ms
  all samples: inference median=442.751ms, decoding median=9.726ms
  whole_sample_last_round=29113.078ms; wall=60.04s maxrss=1013272KiB cpu=914%
  peak_rss_after_load=689924KiB peak_rss_end=1013272KiB
- 01:43:48 load1=11.29 page cache before=481M+2.3M :: spike-onnx, rounds=2, threads=ORT default
  tokenizer+labels_load=182.454ms session_load=1643.209ms first_parse_call=58.741ms
  per sentence (median of rounds, n=50): median=437.556ms p95=2264.150ms min=35.912ms max=5179.101ms
  all samples: inference median=427.393ms, decoding median=9.787ms
  whole_sample_last_round=28820.670ms; wall=61.68s maxrss=983220KiB cpu=890%
  peak_rss_after_load=689208KiB peak_rss_end=983220KiB
- 01:44:50 load1=12.81 page cache before=0B+0B :: spike-onnx, rounds=1, threads=1
  tokenizer+labels_load=148.018ms session_load=1694.034ms first_parse_call=117.576ms
  per sentence (median of rounds, n=50): median=1141.872ms p95=6859.214ms min=57.036ms max=21868.936ms
  all samples: inference median=1310.300ms, decoding median=6.464ms
  whole_sample_last_round=96965.682ms; wall=98.94s maxrss=951496KiB cpu=98%
  peak_rss_after_load=689292KiB peak_rss_end=951496KiB
- 01:46:29 load1=7.73 page cache before=481M+2.3M :: spike-onnx, rounds=1, threads=1
  tokenizer+labels_load=100.989ms session_load=1158.672ms first_parse_call=108.648ms
  per sentence (median of rounds, n=50): median=1374.381ms p95=5754.969ms min=54.410ms max=16810.607ms
  all samples: inference median=1372.506ms, decoding median=6.809ms
  whole_sample_last_round=90182.654ms; wall=91.57s maxrss=989116KiB cpu=99%
  peak_rss_after_load=689984KiB peak_rss_end=989116KiB
- 01:48:01 load1=5.40 page cache before=not logged :: spike-onnx-nixlink, rounds=2, threads=ORT default
  tokenizer+labels_load=95.617ms session_load=1393.239ms first_parse_call=67.892ms
  per sentence (median of rounds, n=50): median=671.307ms p95=3853.313ms min=53.991ms max=6206.946ms
  all samples: inference median=574.737ms, decoding median=16.325ms
  whole_sample_last_round=40352.886ms; wall=81.04s maxrss=1079500KiB cpu=1326%
  peak_rss_after_load=693464KiB peak_rss_end=1079500KiB
```

### Re-run, 2026-09-27 09:30–09:32 (single runs, warm cache)

Source: `.sdd/m3b-spike/logs/rerun-0927-timings.log`.

```text
- 09:30:41 load1=2.50 page cache before=not logged :: spike-udpipe-preseg, rounds=5, 1 thread
  model_load=1267.886ms first_parse_call=3.541ms
  per sentence (median of rounds, n=50): median=12.181ms p95=32.769ms min=1.081ms max=41.756ms
  whole_sample_last_round=565.397ms; wall=4.45s maxrss=105612KiB cpu=98%
  peak_rss_after_load=105988KiB peak_rss_end=106204KiB
- 09:30:46 load1=2.54 page cache before=not logged :: spike-onnx, rounds=1, threads=ORT default
  tokenizer+labels_load=94.064ms session_load=1360.512ms first_parse_call=33.248ms
  per sentence (median of rounds, n=50): median=417.490ms p95=2203.807ms min=25.540ms max=5313.191ms
  all samples: inference median=440.493ms, decoding median=9.213ms
  whole_sample_last_round=29215.569ms; wall=30.79s maxrss=983572KiB cpu=882%
  peak_rss_after_load=691984KiB peak_rss_end=983572KiB
- 09:31:17 load1=6.68 page cache before=not logged :: spike-onnx-nixlink, rounds=1, threads=ORT default
  tokenizer+labels_load=99.001ms session_load=1281.205ms first_parse_call=119.443ms
  per sentence (median of rounds, n=50): median=582.411ms p95=2462.200ms min=35.993ms max=6628.962ms
  all samples: inference median=567.327ms, decoding median=15.977ms
  whole_sample_last_round=39067.898ms; wall=40.60s maxrss=1027504KiB cpu=1347%
  peak_rss_after_load=689920KiB peak_rss_end=1027504KiB
- 09:32:25 load1=7.78 page cache before=not logged :: spike-onnx-nixlink, rounds=1, threads=8
  tokenizer+labels_load=115.002ms session_load=1135.420ms first_parse_call=42.989ms
  per sentence (median of rounds, n=50): median=395.108ms p95=2049.340ms min=26.448ms max=4687.412ms
  all samples: inference median=387.407ms, decoding median=11.253ms
  whole_sample_last_round=26706.147ms; wall=28.10s maxrss=1018584KiB cpu=754%
  peak_rss_after_load=691704KiB peak_rss_end=1018584KiB
```

### Time against sentence length (re-run)

Source: `.sdd/m3b-spike/logs/rerun-0927-length-bins.txt` (from `rerun-0927-len-*.tsv`).

```text
words	n	udpipe_preseg_ms(median)	onnx_download_default_ms(median)	onnx_nixlink_default_ms(median)	udpipe_ms_per_word	onnx_ms_per_word
1-8	16	3.3	58.1	114.8	0.66	11.6
9-19	13	11.9	370.8	581.8	0.70	21.8
20-25	16	16.9	669.4	885.6	0.77	30.4
26-31	2	22.6	1087.2	1632.8	0.77	36.9
32-99	3	35.7	2815.9	3086.9	0.78	61.2
longest (65 words): [(41.756, 5313.191, 6628.962)]
sum udpipe 632 ms; sum onnx 29216 ms; sum nixlink 39068 ms
```

## Agreement with the UDPipe 2 reference

### Whole sample, EWT model and ONNX

Source: `.sdd/m3b-spike/logs/rerun-0927-compare.txt` (`target-common/release/compare out-udpipe2-ref.conllu …`).

```text
=== ref vs out-udpipe.conllu
sentence 3: center embeddings ref [] vs route [("people", "left")]
sentence 5: center embeddings ref [("Residents", "confused"), ("glass", "belonged")] vs route []
sentence 9: center embeddings ref [("proposal", "caused")] vs route [("committee", "caused")]
sentence 18: center embeddings ref [("Okafor", "joined")] vs route [("Dr.", "joined")]
sentence 27: invalid tree: Some(IdOutOfOrder { position: 9, id: 1 })
sentence 42: center embeddings ref [("groups", "left")] vs route []
sentence 44: center embeddings ref [("sheets", "gather")] vs route []
sentences=50 invalid_trees=1 tokenization_mismatch=0
tokens(matched sentences)=974 UAS=80.7% LAS=78.7% UAS_no_punct=84.7%
metric agreement over 49 valid sentences: depth 34, clauses 43, center-embeddings 43, MDD exact 23, all four 23; mean |ΔMDD| 0.242
=== ref vs out-udpipe-preseg.conllu
sentence 3: center embeddings ref [] vs route [("people", "left")]
sentence 5: center embeddings ref [("Residents", "confused"), ("glass", "belonged")] vs route []
sentence 9: center embeddings ref [("proposal", "caused")] vs route [("committee", "caused")]
sentence 18: center embeddings ref [("Okafor", "joined")] vs route [("Dr.", "joined")]
sentence 42: center embeddings ref [("groups", "left")] vs route []
sentence 44: center embeddings ref [("sheets", "gather")] vs route []
sentences=50 invalid_trees=0 tokenization_mismatch=0
tokens(matched sentences)=974 UAS=83.0% LAS=81.0% UAS_no_punct=87.0%
metric agreement over 50 valid sentences: depth 35, clauses 43, center-embeddings 44, MDD exact 23, all four 23; mean |ΔMDD| 0.241
=== ref vs out-onnx.conllu
sentence 18: center embeddings ref [("Okafor", "joined")] vs route [("Dr.", "joined")]
sentence 48: tokenization differs (23 vs 22 tokens)
sentence 48: center embeddings ref [("Anyone", "asked")] vs route [("Anyone", "asked")]
sentences=50 invalid_trees=0 tokenization_mismatch=1
tokens(matched sentences)=951 UAS=94.8% LAS=92.1% UAS_no_punct=96.7%
metric agreement over 50 valid sentences: depth 40, clauses 48, center-embeddings 48, MDD exact 31, all four 29; mean |ΔMDD| 0.089
```

### Whole sample, other UDPipe 1 English models (re-run)

Source: `.sdd/m3b-spike/logs/rerun-0927-preseg-english-{partut,gum,lines}.txt`.

```text
== english-partut-ud-2.5-191206, presegmented, 1 round
--- metrics: MDD 32/12 = 2.667, depth 4, clauses 1, center embeddings [(proposal … caused, 8 words between)]
sentences=50 invalid_trees=0 tokenization_mismatch=2
tokens(matched sentences)=931 UAS=80.5% LAS=73.7% UAS_no_punct=83.5%
metric agreement over 50 valid sentences: depth 27, clauses 34, center-embeddings 41, MDD exact 18, all four 17; mean |ΔMDD| 0.241
== english-gum-ud-2.5-191206, presegmented, 1 round
--- metrics: MDD 30/12 = 2.500, depth 5, clauses 2, center embeddings [(proposal … caused, 8 words between)]
sentences=50 invalid_trees=0 tokenization_mismatch=3
tokens(matched sentences)=908 UAS=85.2% LAS=80.4% UAS_no_punct=86.7%
metric agreement over 50 valid sentences: depth 26, clauses 37, center-embeddings 43, MDD exact 19, all four 18; mean |ΔMDD| 0.224
== english-lines-ud-2.5-191206, presegmented, 1 round
--- metrics: MDD 36/12 = 3.000, depth 5, clauses 2, center embeddings [(which … caused, 7 words between), (committee … caused, 4 words between)]
sentences=50 invalid_trees=0 tokenization_mismatch=2
tokens(matched sentences)=931 UAS=83.9% LAS=77.9% UAS_no_punct=86.2%
metric agreement over 50 valid sentences: depth 29, clauses 36, center-embeddings 43, MDD exact 16, all four 15; mean |ΔMDD| 0.274
```

## The M3a example sentence

*The proposal, which the executive committee rejected after extensive deliberation, caused significant delays.*

The UDPipe 2 reference has no metrics line of its own: ONNX agrees with `EXAMPLE_CONLLU` on all four metrics (second block), so the reference's are ONNX's.
The GUM, ParTUT and LinES parses were first made at 01:26 with the default tokenizer (`logs/out-udpipe-{gum,partut,lines}.conllu`, compared in the second block) and again in the re-run with the presegmented one (shown below); the token lines are identical.

### vernier's M3a metrics per parser

Source: `.sdd/m3b-spike/logs/{udpipe-run1,udpipe-preseg-run1,onnx-full-r1,rerun-0927-preseg-english-*}.txt`.

```text
udpipe EWT (default tokenizer): MDD 36/12 = 3.000, depth 5, clauses 2, center embeddings [(committee … caused, 4 words between)]  [logs/udpipe-run1.txt]
udpipe EWT (presegmented): MDD 36/12 = 3.000, depth 5, clauses 2, center embeddings [(committee … caused, 4 words between)]  [logs/udpipe-preseg-run1.txt]
ONNX: MDD 32/12 = 2.667, depth 4, clauses 1, center embeddings [(proposal … caused, 8 words between)]  [logs/onnx-full-r1.txt]
udpipe GUM (re-run): MDD 30/12 = 2.500, depth 5, clauses 2, center embeddings [(proposal … caused, 8 words between)]  [logs/rerun-0927-preseg-english-gum.txt]
udpipe ParTUT (re-run): MDD 32/12 = 2.667, depth 4, clauses 1, center embeddings [(proposal … caused, 8 words between)]  [logs/rerun-0927-preseg-english-partut.txt]
udpipe LinES (re-run): MDD 36/12 = 3.000, depth 5, clauses 2, center embeddings [(which … caused, 7 words between), (committee … caused, 4 words between)]  [logs/rerun-0927-preseg-english-lines.txt]
```

### Agreement with `EXAMPLE_CONLLU` (src/testing.rs)

Source: `.sdd/m3b-spike/logs/rerun-0927-example-compare.txt`.

```text
== EXAMPLE_CONLLU (src/testing.rs) vs logs/onnx-example.conllu
tokens(matched sentences)=16 UAS=93.8% LAS=93.8% UAS_no_punct=100.0%
metric agreement over 1 valid sentences: depth 1, clauses 1, center-embeddings 1, MDD exact 1, all four 1; mean |ΔMDD| 0.000
== EXAMPLE_CONLLU (src/testing.rs) vs logs/out-udpipe-gum.conllu
tokens(matched sentences)=16 UAS=75.0% LAS=75.0% UAS_no_punct=76.9%
metric agreement over 1 valid sentences: depth 0, clauses 0, center-embeddings 1, MDD exact 0, all four 0; mean |ΔMDD| 0.167
== EXAMPLE_CONLLU (src/testing.rs) vs logs/out-udpipe-lines.conllu
tokens(matched sentences)=16 UAS=50.0% LAS=43.8% UAS_no_punct=61.5%
metric agreement over 1 valid sentences: depth 0, clauses 0, center-embeddings 0, MDD exact 0, all four 0; mean |ΔMDD| 0.333
== EXAMPLE_CONLLU (src/testing.rs) vs logs/out-udpipe-partut.conllu
tokens(matched sentences)=16 UAS=93.8% LAS=93.8% UAS_no_punct=100.0%
metric agreement over 1 valid sentences: depth 1, clauses 1, center-embeddings 1, MDD exact 1, all four 1; mean |ΔMDD| 0.000
== EXAMPLE_CONLLU vs sentence 9 of out-udpipe.conllu
tokens(matched sentences)=16 UAS=50.0% LAS=50.0% UAS_no_punct=61.5%
metric agreement over 1 valid sentences: depth 0, clauses 0, center-embeddings 0, MDD exact 0, all four 0; mean |ΔMDD| 0.333
== EXAMPLE_CONLLU vs sentence 9 of out-udpipe-preseg.conllu
tokens(matched sentences)=16 UAS=50.0% LAS=50.0% UAS_no_punct=61.5%
metric agreement over 1 valid sentences: depth 0, clauses 0, center-embeddings 0, MDD exact 0, all four 0; mean |ΔMDD| 0.333
```

### CoNLL-U: UDPipe 2 reference (= `EXAMPLE_CONLLU`)

Source: `.sdd/m3b-spike/out-udpipe2-ref.conllu`, sentence 9.

```text
1	The	the	DET	DT	Definite=Def|PronType=Art	2	det	_	_
2	proposal	proposal	NOUN	NN	Number=Sing	13	nsubj	_	SpaceAfter=No
3	,	,	PUNCT	,	_	8	punct	_	_
4	which	which	PRON	WDT	PronType=Rel	8	obj	_	_
5	the	the	DET	DT	Definite=Def|PronType=Art	7	det	_	_
6	executive	executive	ADJ	JJ	Degree=Pos	7	amod	_	_
7	committee	committee	NOUN	NN	Number=Sing	8	nsubj	_	_
8	rejected	reject	VERB	VBD	Mood=Ind|Number=Sing|Person=3|Tense=Past|VerbForm=Fin	2	acl:relcl	_	_
9	after	after	ADP	IN	_	11	case	_	_
10	extensive	extensive	ADJ	JJ	Degree=Pos	11	amod	_	_
11	deliberation	deliberation	NOUN	NN	Number=Sing	8	obl	_	SpaceAfter=No
12	,	,	PUNCT	,	_	2	punct	_	_
13	caused	cause	VERB	VBD	Mood=Ind|Number=Sing|Person=3|Tense=Past|VerbForm=Fin	0	root	_	_
14	significant	significant	ADJ	JJ	Degree=Pos	15	amod	_	_
15	delays	delay	NOUN	NNS	Number=Plur	13	obj	_	SpaceAfter=No
16	.	.	PUNCT	.	_	13	punct	_	SpacesAfter=\n
```

### CoNLL-U: udpipe, EWT (default and presegmented tokenizer give the same parse)

Source: `.sdd/m3b-spike/out-udpipe.conllu`, sentence 9.

```text
1	The	the	DET	DT	Definite=Def|PronType=Art	2	det	_	_
2	proposal	proposal	NOUN	NN	Number=Sing	0	root	_	SpaceAfter=No
3	,	,	PUNCT	,	_	2	punct	_	_
4	which	which	PRON	WDT	PronType=Rel	13	obj	_	_
5	the	the	DET	DT	Definite=Def|PronType=Art	7	det	_	_
6	executive	executive	ADJ	JJ	Degree=Pos	7	amod	_	_
7	committee	committee	NOUN	NN	Number=Sing	13	nsubj	_	_
8	rejected	reject	VERB	VBN	Tense=Past|VerbForm=Part	7	acl	_	_
9	after	after	ADP	IN	_	11	case	_	_
10	extensive	extensive	ADJ	JJ	Degree=Pos	11	amod	_	_
11	deliberation	deliberation	NOUN	NN	Number=Sing	8	obl	_	SpaceAfter=No
12	,	,	PUNCT	,	_	7	punct	_	_
13	caused	cause	VERB	VBD	Mood=Ind|Tense=Past|VerbForm=Fin	2	acl:relcl	_	_
14	significant	significant	ADJ	JJ	Degree=Pos	15	amod	_	_
15	delays	delay	NOUN	NNS	Number=Plur	13	obj	_	SpaceAfter=No
16	.	.	PUNCT	.	_	2	punct	_	SpaceAfter=No
```

### CoNLL-U: ONNX

Source: `.sdd/m3b-spike/out-onnx.conllu`, sentence 9.

```text
1	The	_	DET	_	Definite=Def|PronType=Art	2	det	_	_
2	proposal	_	NOUN	_	Number=Sing	13	nsubj	_	SpaceAfter=No
3	,	_	PUNCT	_	_	2	punct	_	_
4	which	_	PRON	_	PronType=Rel	8	obj	_	_
5	the	_	DET	_	Definite=Def|PronType=Art	7	det	_	_
6	executive	_	ADJ	_	Degree=Pos	7	amod	_	_
7	committee	_	NOUN	_	Number=Sing	8	nsubj	_	_
8	rejected	_	VERB	_	Mood=Ind|Tense=Past|VerbForm=Fin	2	acl:relcl	_	_
9	after	_	ADP	_	_	11	case	_	_
10	extensive	_	ADJ	_	Degree=Pos	11	amod	_	_
11	deliberation	_	NOUN	_	Number=Sing	8	obl	_	SpaceAfter=No
12	,	_	PUNCT	_	_	2	punct	_	_
13	caused	_	VERB	_	Mood=Ind|Person=3|Tense=Past|VerbForm=Fin	0	root	_	_
14	significant	_	ADJ	_	Degree=Pos	15	amod	_	_
15	delays	_	NOUN	_	Number=Plur	13	obj	_	SpaceAfter=No
16	.	_	PUNCT	_	_	13	punct	_	SpaceAfter=No
```

### CoNLL-U: udpipe, GUM

Source: `.sdd/m3b-spike/logs/rerun-0927-preseg-english-gum.conllu`.

```text
1	The	the	DET	DT	Definite=Def|PronType=Art	2	det	_	_
2	proposal	proposal	NOUN	NN	Number=Sing	13	nsubj	_	SpaceAfter=No
3	,	,	PUNCT	,	_	7	punct	_	_
4	which	which	PRON	WDT	PronType=Rel	7	nsubj	_	_
5	the	the	DET	DT	Definite=Def|PronType=Art	7	det	_	_
6	executive	executive	ADJ	JJ	Degree=Pos	7	amod	_	_
7	committee	committee	NOUN	NN	Number=Sing	2	acl:relcl	_	_
8	rejected	reject	VERB	VBN	Tense=Past|VerbForm=Part	7	acl	_	_
9	after	after	ADP	IN	_	11	case	_	_
10	extensive	extensive	ADJ	JJ	Degree=Pos	11	amod	_	_
11	deliberation	deliberation	NOUN	NN	Number=Sing	8	obl	_	SpaceAfter=No
12	,	,	PUNCT	,	_	2	punct	_	_
13	caused	cause	VERB	VBD	Mood=Ind|Tense=Past|VerbForm=Fin	0	root	_	_
14	significant	significant	ADJ	JJ	Degree=Pos	15	amod	_	_
15	delays	delay	NOUN	NNS	Number=Plur	13	obj	_	SpaceAfter=No
16	.	.	PUNCT	.	_	13	punct	_	SpaceAfter=No
```

### CoNLL-U: udpipe, ParTUT

Source: `.sdd/m3b-spike/logs/rerun-0927-preseg-english-partut.conllu`.

```text
1	The	the	DET	RD	Definite=Def|PronType=Art	2	det	_	_
2	proposal	proposal	NOUN	S	Number=Sing	13	nsubj	_	SpaceAfter=No
3	,	,	PUNCT	FF	_	2	punct	_	_
4	which	which	PRON	PR	PronType=Rel	8	obj	_	_
5	the	the	DET	RD	Definite=Def|PronType=Art	7	det	_	_
6	executive	executive	ADJ	A	Degree=Pos	7	amod	_	_
7	committee	committee	NOUN	S	Number=Sing	8	nsubj	_	_
8	rejected	reject	VERB	V	Mood=Ind|Person=3|Tense=Past|VerbForm=Fin	2	acl:relcl	_	_
9	after	after	ADP	E	_	11	case	_	_
10	extensive	extensive	ADJ	A	Degree=Pos	11	amod	_	_
11	deliberation	deliberation	NOUN	S	Number=Sing	8	obl	_	SpaceAfter=No
12	,	,	PUNCT	FF	_	2	punct	_	_
13	caused	cause	VERB	V	Mood=Ind|Person=3|Tense=Past|VerbForm=Fin	0	root	_	_
14	significant	significant	ADJ	A	Degree=Pos	15	amod	_	_
15	delays	delay	NOUN	S	Number=Plur	13	obj	_	SpaceAfter=No
16	.	.	PUNCT	FS	_	13	punct	_	SpaceAfter=No
```

### CoNLL-U: udpipe, LinES

Source: `.sdd/m3b-spike/logs/rerun-0927-preseg-english-lines.conllu`.

```text
1	The	the	DET	DEF	Definite=Def|PronType=Art	2	det	_	_
2	proposal	proposal	NOUN	SG-NOM	Number=Sing	0	root	_	SpaceAfter=No
3	,	,	PUNCT	Comma	_	13	punct	_	_
4	which	which	PRON	WH-REL	PronType=Int	13	nsubj	_	_
5	the	the	DET	DEF	Definite=Def|PronType=Art	7	det	_	_
6	executive	executive	NOUN	SG-NOM	Case=Nom	7	compound	_	_
7	committee	committee	NOUN	SG-NOM	Number=Sing	13	nsubj	_	_
8	rejected	reject	VERB	PASS	Tense=Past|VerbForm=Part|Voice=Pass	7	acl	_	_
9	after	after	ADP		_	11	case	_	_
10	extensive	extensive	ADJ	POS	Degree=Pos	11	amod	_	_
11	deliberation	deliberation	NOUN	SG-NOM	Number=Sing	8	obl	_	SpaceAfter=No
12	,	,	PUNCT	Comma	_	8	punct	_	_
13	caused	cause	VERB	PAST	Mood=Ind|Tense=Past|VerbForm=Fin	2	acl:relcl	_	_
14	significant	significant	ADJ	POS	Degree=Pos	15	amod	_	_
15	delays	delay	NOUN	PL-NOM	Number=Plur	13	obj	_	SpaceAfter=No
16	.	.	PUNCT	Period	_	2	punct	_	SpaceAfter=No
```

## ONNX decoder checks

### Chu-Liu/Edmonds fuzz: Rust port vs `ud.py`

Source: `.sdd/m3b-spike/logs/rerun-0927-cle-fuzz.out` (`cle_fuzz.py`, seed 2026, n = 1–29).

```text
matrices=2000 with_2cycle_in_greedy=732 disagreements=0
```

### Whole-sample decoding parity on the same logits

Source: `.sdd/m3b-spike/logs/rerun-0927-parity.out` (`parity.py dump …`; the spike's own copy: `logs/out-onnx-python-udpy.conllu` = `out-onnx.conllu`).

```text
parity rerun: python ud.py == rust out-onnx
```

### Branch coverage on the sample

Source: `.sdd/m3b-spike/logs/rerun-0927-parity-cov.out` (`parity_cov.py`; 49 of 50 sentences needed 1 Chu-Liu/Edmonds call, lines omitted).

```text
So are the taps. pieces 5 words 5 cle_calls 3
sentences with goeswith merges: 17
```

## Determinism

Source: `cmp` on 2026-09-27 of the files named; each group is byte-identical.

```text
udpipe EWT, default tokenizer: out-udpipe.conllu = logs/out-udpipe-{2,3,4}.conllu = logs/t-udpipe-{1,2,3}.conllu
udpipe EWT, presegmented: out-udpipe-preseg.conllu = logs/t-udpipe-preseg-{1,2,3}.conllu = scratch/r-udp.conllu
ONNX: out-onnx.conllu = logs/t-onnx-default-{1,2,3}.conllu = logs/t-onnx-1thread-{1,2}.conllu = logs/t-onnx-nixlink-default.conllu
      = logs/out-onnx-dumprun.conllu = scratch/r-onnx.conllu = scratch/r-onnx-nix.conllu = scratch/r-onnx-nix8.conllu
```
