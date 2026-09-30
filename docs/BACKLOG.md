# Backlog

Ordered; only the top item gets a full spec (A3). Every item has an owner or a trigger.

1. **Spec 001 — vernier core** — in flight, see `docs/HANDOVER.md`.
2. **Phase 2 owed for spec 001** — the user directed implementation before review and the human gates (`gates.md`, 2026-09-26). Clarification passed 2026-09-26; Review, Spec-Approval and Comprehension remain. Trigger: before any M1+ milestone is marked `DONE`, or when the user next reviews.
3. **Configuration file** (`vernier.toml` for thresholds and ignores) — out of spec 001 by its Non-Goals. Trigger: a user runs `check` in CI with non-default flags on more than one repository.
4. **Inline suppression** (e.g. `<!-- vernier-ignore -->`) — Trigger: the first false positive that has no fix in the text.
5. **Languages other than English** — Trigger: a UD model for a second language is licensed for the intended use and requested.
6. **Performance budget** — the project description says "fast" with no number. The user chose the ONNX route (accuracy over speed) and, on 2026-09-30, accepted a loose budget as a recorded measurement, not a gate: ≤ 60 s per 1000 words, peak RSS ≤ 1 GiB (`docs/specs/001-vernier/measurements/performance-baseline.md`). Trigger for a binding criterion: a faster backend (item 10) or a user reporting that a run is too slow.
7. **CI workflow for this repository** (run `just verify` on push inside devenv). Trigger: the first push to a remote.
8. ~~**Plan location in D1**~~ — resolved 2026-09-26: the user confirmed it; CONSTITUTION D12.
9. ~~**Agent sessions cannot commit**~~ — resolved 2026-09-26: the user chose unsigned commits; CONSTITUTION D13.
10. **Selectable parser backend** — the user chose ONNX as default (2026-09-27) and wants users to be able to configure the parser later: a UDPipe 1 backend (≈ 30× faster, gives lemmas, 83 % UAS; vendored `udpipe-rs` with the presegmented one-line patch; an ADR for the C++ dependency), possibly a different model per metric. When it comes, its M3a-example test accepts the GUM or ParTUT model and the README warns that EWT parses the example wrong (spike report). Trigger: M3b is DONE and a user asks for speed; interacts with item 3 (configuration file).
