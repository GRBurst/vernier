# Agent Instructions (canonical, harness-agnostic)

`vernier` is a Rust CLI that measures how hard Markdown prose is to read. It is built spec-first.
This file is always loaded; every line must pass the test *would removing it cause an agent to act incorrectly?*

## Read order (on session start)

1. `docs/CONSTITUTION.md` — axioms and decisions. Binding.
2. `docs/PROCESS.md` — the increment loop, tiers, gates. Binding.
3. `docs/HANDOVER.md` — where the last session stopped; a bare "continue" means: resume from its "Next action" (D7).
4. The spec `docs/HANDOVER.md` names as in flight, with its `PROPOSED` deltas in `history/`; `docs/BACKLOG.md` for what's next.
5. Before writing code: `docs/ENGINEERING.md` — how code is written (test first, types, errors, banned constructs). Binding (D10).

## Hard rules

- Never bypass a gate; record friction instead of working around it (A7).
- Never edit, weaken, or delete a spec requirement or a test to make something pass (A2).
- Run `just verify` before claiming anything works (B3).
- A task touching more than 5 files is split; a milestone fits one session and lands runnable, at a handover + commit boundary (A3, D5).
- If a "why" is unclear or a question is open — at any phase — stop and ask the user; never guess intent (D6).
- Before implementing, brief the user in one or two plain sentences; report as preconditions verified → what changed → exact check commands, with a minimal example (D7).
- Commit messages state the problem solved and end with the tier: `(full-spec)`, `(spec-delta)` or `(direct-patch)` (D2, D8).
- End every session with an updated `docs/HANDOVER.md` and a clean commit (A5).
- `.sdd/` is untracked scratch; anything the user asks to persist goes into a committed file (B5).
- Tests state laws and properties, not just constants (A4). Cognitive complexity ≤ 10 per function; no `unsafe` without an ADR (D9).
- Test first, pure core, typed outcomes; no `unwrap`/`expect`/`panic!` outside tests — an `#[allow]` carries a `// why:` (D10).
- Every corrected mistake gets a committed `docs/audits/<NNN>-<slug>.md` before the fix counts as done (D11).
- Never create or modify global cargo, rustup or user configs; everything runs through devenv (B4).

## Commands

Run inside the pinned environment (`devenv shell`, or `devenv shell -- just verify`).

- `just verify` — all deterministic gates (spec-check + its tests, fmt, clippy, cargo test)
- `just spec-check [files]` — spec house-style gate
- `just lint` / `just test` / `just fmt-check` — the Rust gates one by one
