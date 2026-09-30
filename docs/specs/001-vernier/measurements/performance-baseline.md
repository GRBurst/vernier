# Performance baseline — ONNX route (spec 001 M3b)

Recorded budget, accepted by the user on 2026-09-30 as a measurement, not a `just verify` gate (timing is noisy; BACKLOG 6 stays open for a binding criterion).
The spike's proposed budget (p95 ≤ 50 ms per sentence, RSS ≤ 200 MiB) was measured on the udpipe route and does not apply to the ONNX route the user chose.

| Scope | Budget | Measured 2026-09-28 (source: [plan-M3b.md, Measured](../plan-M3b.md#measured)) |
| :--- | :--- | :--- |
| Parsing per document | ≤ 60 s per 1000 words | `parser-sample.md`, 964 words: 39.2–43.8 s wall (3 runs) ≈ 41–45 s per 1000 words |
| Memory | peak RSS ≤ 1 GiB | 771–830 MB (`parser-sample.md`); 694–698 MB (`tests/fixtures/sample.md`) |
| No model | surface only, no runtime loaded | 0.00 s on `parser-sample.md` |

Conditions: laptop, 16 logical cores, `powersave` governor, ONNX Runtime 1.27.1 from nix, default intra-op threads, model `ghotriw/roberta-base-english-ud-goeswith-onnx`, `cargo build --release`.
These are tendencies, not isolated reproducible benchmarks.

How to re-measure:

```sh
cargo build --release
/usr/bin/env time -f '%e s %M KB' target/release/vernier check --format compact \
  --model-path "$VERNIER_TEST_MODEL" docs/specs/001-vernier/measurements/parser-sample.md
```

Known gap: the full run is about 1.4× slower than the spike's whole-sample time (28.7–29.2 s); not investigated (user, 2026-09-30).
