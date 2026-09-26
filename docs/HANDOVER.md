# Handover

## 1. Preconditions verified

- 2026-09-26, inside the devenv shell of this repository: `just verify` printed `✓ all gates green`, before and after this session's changes.
- The new lint gates catch what they should: a temporary `unwrap()`, `expect()`, `panic!`, `todo!`, `unimplemented!` and `dbg!` in `src/main.rs` each failed `just lint` with its named clippy lint. The same `unwrap`/`expect`/`panic` inside `#[cfg(test)]` passed (0 errors).
- Nested `devenv shell -- …` fails in the agent sandbox (`Permission denied`). The session already runs inside the devenv shell (`DEVENV_ROOT` is set), so run `just verify` directly.

## 2. What exists

- Process documents: `AGENTS.md`, `docs/CONSTITUTION.md` (now D1–D11), `docs/PROCESS.md`, and **new** `docs/ENGINEERING.md`, the binding engineering guide (D10). It covers test first with EARS criteria restated as Given/When/Then, clean code, newtypes, a pure core with I/O in the shell, outcome types, errors (`thiserror` per module, `anyhow` only in `main`), banned constructs, and audits.
- **New** `docs/audits/` (D11): every corrected mistake gets a numbered file. `001-sandbox-tmp-unreadable.md` is the first.
- Gates: `justfile` (`just verify`), `tools/spec-check/`, `clippy.toml` (complexity ≤ 10; unwrap/expect/panic allowed in tests), and `Cargo.toml` lints. These now also deny `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented` and `dbg_macro`. Git hooks run from `devenv.nix`.
- A Cargo skeleton (`src/main.rs` prints the version).
- `docs/specs/001-vernier/spec.md` — **Draft**, not yet reviewed; 13 `[NEEDS CLARIFICATION]` markers, each with a fallback.

## 3. How to check

```sh
just verify                          # ✓ all gates green (13 clarify-open warnings are expected)
grep -n 'D10\|D11' docs/CONSTITUTION.md
grep -n 'unwrap_used' Cargo.toml     # the guide's §8 table names each lint
```

## Open tasks

- Spec 001 phase 2: fresh-context review, then Clarification (answer the 13 markers), Spec-Approval, Comprehension. Verdicts go to `docs/specs/001-vernier/gates.md`.

## Next action

Run phase 2 on spec 001: spawn a fresh-context reviewer over `docs/specs/001-vernier/spec.md` and the repository, repair blocking findings in place, then put the 13 open questions to the user.

## Known-bad approaches

- Scratch files in `/tmp` inside the agent sandbox; use `.sdd/` (audit 001).
- Committing from the agent sandbox when commits are GPG-signed: `~/.gnupg` is on nono's deny list, so signing fails. The user commits, or the session gets a grant.
