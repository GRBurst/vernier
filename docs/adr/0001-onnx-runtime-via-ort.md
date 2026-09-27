# ADR 0001 — ONNX Runtime as a native dependency, through `ort`

**Status:** Proposed
**Date:** 2026-09-27
**Spec:** [001 M3b](../specs/001-vernier/spec.md#M3b), plan [plan-M3b.md](../specs/001-vernier/plan-M3b.md)

## Context

The user chose the ONNX route for M3b on 2026-09-27: the RoBERTa goeswith UD model (`ghotriw/roberta-base-english-ud-goeswith-onnx`) run by ONNX Runtime, a C++ library, through the Rust crate `ort` ([spike report](../specs/001-vernier/research/m3b-parser-spike.md)).
PROCESS.md asks for an ADR for "a dependency with native code, permitting `unsafe`"; ENGINEERING §2 asks for one for any dependency with native code.
D9 forbids `unsafe` unless an ADR permits it for a named vernier module.

Facts (verified 2026-09-27):
- `ort` 2.0.0-rc.13 wraps the ONNX Runtime C API with `unsafe` blocks inside the crate; with `load-dynamic` it opens the library with `libloading` (also `unsafe` inside that crate).
- ort's *implicit* load (on the first API call, path from `ORT_DYLIB_PATH`) panics when the library is missing or wrong (`expect("Failed to load ONNX Runtime dylib")`, `ort` src/lib.rs:234). `ort::init_from(path)` does the same load and returns `Result<_, LoadDynamicError>` (`Dlopen`, `MissingApi`, `BadVersion`).
- A build that links ONNX Runtime has `NEEDED libonnxruntime.so.1`, so the binary does not start without the library, even for surface-only use; a `load-dynamic` build needs only libc, libm and libgcc (`readelf -d` on the spike binaries).
- Without an `api-*` feature, `ort` asks for API version 17, so any ONNX Runtime ≥ 1.17 loads.
- The pinned nixpkgs (`devenv.lock`) provides `onnxruntime` 1.27.1 (MIT).

## Decision

1. vernier depends on `ort` with `default-features = false`, features `std` and `load-dynamic`, and on `tokenizers` (pure Rust, no native code). No other native dependency is added.
2. vernier itself contains no `unsafe`: `unsafe_code = "forbid"` in `Cargo.toml` stays. The `unsafe` code this decision accepts lives in the dependencies `ort`, `ort-sys` and `libloading`, not in a vernier module, so no module is exempted from D9.
3. Only `src/onnx.rs` uses `ort`; the decoding (`src/mst.rs`, `src/decoder.rs`) is pure Rust on plain slices.
4. The runtime is loaded only when `--model-path` is given, once per run, through `ort::init_from(path)`. The path is `ORT_DYLIB_PATH` if set and non-empty, else `libonnxruntime.so` from the system loader's search path, resolved in `main`. A load failure is a typed error: vernier names the model directory and the cause on stderr and exits 2. ort's implicit load is never reached, because the first ort call is `init_from`.
5. The development environment provides the runtime: `devenv.nix` adds `onnxruntime` and sets `ORT_DYLIB_PATH` to its `libonnxruntime.so`.

## Consequences

- A user without a model needs neither the model nor ONNX Runtime; `analyze` and `check` run surface-only.
- A user with a model must supply ONNX Runtime ≥ 1.17 (set `ORT_DYLIB_PATH` or install it on the loader path). The README says so.
- ONNX Runtime runs in-process on user text; a crash inside it (a defect in C++) would bring vernier down with it, which no Rust type can prevent. Malformed models are rejected by ONNX Runtime with an error that `ort` returns as `ort::Error`, mapped to exit 2.
- A defect-level panic remains possible inside `ort` (for example `assert!(!base.is_null())` after a successful load, or `current_exe()` failing when the path is relative). vernier passes an absolute path where it can and treats such a panic as a defect, not an outcome.
- In `ort` 2.0.0-rc.13 a failed `init_from` still marks the library as loaded (`util/once_lock_std.rs`), so a second load attempt in the same process would use a library that was never loaded. vernier loads once per run and exits 2 on failure; any retry or second backend must not rely on a failed load being repeatable.
- A second backend (UDPipe, BACKLOG 10) needs its own ADR.
