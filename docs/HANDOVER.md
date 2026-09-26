# Handover

## 1. Preconditions verified

- The starter kit was generated on 2026-09-26 outside this repository. In the source sandbox, `just spec-check`, `just spec-check-test`, `just fmt-check`, `just lint` and `just test` passed; `devenv shell` and the git hooks were **not** run there.
- Re-verify here first: `devenv shell -- just verify` must print `✓ all gates green`.

## 2. What exists

- Process documents: `AGENTS.md`, `docs/CONSTITUTION.md`, `docs/PROCESS.md`.
- Gates: `justfile` (`just verify`), `tools/spec-check/` (spec linter, stdlib Python, with tests), `clippy.toml` (complexity ≤ 10), git hooks in `devenv.nix` (pre-commit runs `just verify`; commit-msg requires a tier).
- A Cargo skeleton (`src/main.rs` prints the version) so every gate has something to judge.
- `docs/specs/001-vernier/spec.md` — **Draft**, not yet reviewed; 13 `[NEEDS CLARIFICATION]` markers, each with a fallback.

## 3. How to check

```sh
devenv shell -- just verify          # ✓ all gates green (13 clarify-open warnings are expected)
git log --oneline                    # first commit names its tier
```

## Open tasks

- Spec 001 phase 2: fresh-context review, then Clarification (answer the 13 markers), Spec-Approval, Comprehension; verdicts go to `docs/specs/001-vernier/gates.md`.

## Next action

Run phase 2 on spec 001: spawn a fresh-context reviewer over `docs/specs/001-vernier/spec.md` and the repository, repair blocking findings in place, then put the 13 open questions to the user.

## Known-bad approaches

- None yet.
