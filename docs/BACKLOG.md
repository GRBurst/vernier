# Backlog

Ordered; only the top item gets a full spec (A3). Every item has an owner or a trigger.

1. **Spec 001 — vernier core** — in flight, see `docs/HANDOVER.md`.
2. **Configuration file** (`vernier.toml` for thresholds and ignores) — out of spec 001 by its Non-Goals. Trigger: a user runs `check` in CI with non-default flags on more than one repository.
3. **Inline suppression** (e.g. `<!-- vernier-ignore -->`) — Trigger: the first false positive that has no fix in the text.
4. **Languages other than English** — Trigger: a UD model for a second language is licensed for the intended use and requested.
5. **Performance budget** — the project description says "fast" with no number. Trigger: M3b's spike measures per-sentence parse time; then fix a budget as a criterion.
6. **CI workflow for this repository** (run `just verify` on push inside devenv). Trigger: the first push to a remote.
