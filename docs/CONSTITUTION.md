# Constitution

The binding root contract of this repository. Every agent session and every human contribution is governed by it.

**Mutability:** append-only. An amendment adds a new numbered entry; a superseded entry is marked `SUPERSEDED by <id>` in place, never deleted.
Every rule carries its *Why*: the reason tells you whether a situation justifies bending the rule — and any bend is still recorded (A7).

Adapted on 2026-09-26 from the constitution of the SDD framework repository, trimmed to one project.

## Axioms

<a id="A1"></a>
- **A1 — Scientific method.**
  Work proceeds via validated preconditions, explicit hypotheses, and empirical tests; uncertainty is marked and resolved before it propagates.
  *Why:* agents confidently produce plausible-but-wrong output; empirical tests catch it before it compounds.
<a id="A2"></a>
- **A2 — Specs are contracts.**
  A spec requirement or a test is never edited, weakened, or deleted to make something pass.
  The history layer (deltas, ADRs, this constitution, gate records) is append-only; the baseline spec changes only at a reviewed fold-back (D4).
  *Why:* the strongest agent failure mode is moving the target to match the shot.
<a id="A3"></a>
- **A3 — Small incremental deliverables.**
  Work is sliced into dependency-ordered increments; only the next one is specced in detail. A task touching more than 5 files is split.
  *Why:* uncertainty grows with planning horizon; small slices bound both error and rework.
<a id="A4"></a>
- **A4 — Property and law testing over hardcoded values.**
  Tests validate behavioral laws, invariants, and structural properties, not particular constants; a hand-computed example is allowed as one witness beside a property, never instead of it.
  *Why:* a hardcoded expectation validates one input and is trivially gamed.
<a id="A5"></a>
- **A5 — Clean session handover.**
  Every session persists verified state, hypotheses, and open tasks in `docs/HANDOVER.md` and ends with a clean commit. The commit is the memory boundary.
  *Why:* context windows die; the commit is the only memory that survives.
<a id="A6"></a>
- **A6 — Pragmatic stage phasing.**
  Build the core first; defer features until a trigger fires, but keep their one-line seams (IDs, `Status:` fields, traits) open.
  *Why:* speculative machinery never gets exercised, while a missing seam costs a rewrite.
<a id="A7"></a>
- **A7 — Ratchet.**
  Use the process as it exists; never bypass an existing gate. Friction is never worked around silently; it is recorded (backlog, delta, or ADR).
  *Why:* a gate bypassed once is a gate nobody trusts.

## Binding intent

<a id="B1"></a>
- **B1 — Harness-agnostic.** `AGENTS.md` is canonical; harness files (`CLAUDE.md`, ...) only point to it.
  *Why:* harnesses churn; the process must outlive them.
<a id="B2"></a>
- **B2 — Deterministic tools over prompts.** Any job code can do deterministically is a script or gate, not an instruction. Prompts are for judgment.
  *Why:* code cannot be talked out of its verdict.
<a id="B3"></a>
- **B3 — Every step gated, deterministic first.** Every phase has an exit gate; `just verify` runs before any judgment review.
  *Why:* ungated steps are where agents fail silently, and cheap checks must not wait on expensive ones.
<a id="B4"></a>
- **B4 — Hermetic reproducibility.** Every gate runs inside the pinned devenv environment with one command; zero unpinned host tools. `devenv.lock` and `Cargo.lock` are committed.
  *Why:* a gate whose verdict depends on the machine is not evidence.
<a id="B5"></a>
- **B5 — Committed project-local knowledge.** What the project knows lives in committed files. `.sdd/` is untracked scratch; nothing load-bearing goes there.
  *Why:* per-user state dies with the session or the machine.

## Decision register

<a id="D1"></a>
- **D1 — Locations and spec format.**
  Specs: `docs/specs/<NNN>-<slug>/spec.md` (three-digit ordinal, never reused). Deltas: `docs/specs/<NNN>-<slug>/history/delta-<SSS>-<slug>.md`. Gate verdicts: `docs/specs/<NNN>-<slug>/gates.md`. ADRs: `docs/adr/<NNNN>-<slug>.md`. Measurements: `docs/specs/<NNN>-<slug>/measurements/`.
  A spec's format is the one `tools/spec-check/spec_check.py` enforces; acceptance criteria use EARS phrasing (`WHEN …, THE … SHALL …`).
  *Why:* one fixed place per artifact makes every lookup a path, not a search.
<a id="D2"></a>
- **D2 — Ceremony tiers.**
  `full-spec` for a new capability or contract change; `spec-delta` for an incremental change to a specced capability; `direct-patch` (commit + gates only) for a fix with no contract change. The tier is named in every commit message, e.g. `(direct-patch)`; the commit-msg hook refuses a message without one.
  *Why:* full ceremony on trivial changes gets bypassed; naming the tier keeps the shortcut auditable.
<a id="D3"></a>
- **D3 — Spec mutability.**
  Before its Spec-Approval gate passes, a spec is revised in place. After that, a change is a delta, folded back into the spec only after review. Milestone `Status:` and ticking a verified criterion are the only sanctioned in-place edits of an approved spec.
  *Why:* append-only history keeps every "why"; a clean baseline keeps the spec readable.
<a id="D4"></a>
- **D4 — Agent topology.**
  One agent wears Coordinator, Implementor and Verifier roles in turn. Judgment review of a spec or a milestone runs in a fresh context (a subagent or new session) that cannot see the authoring session.
  *Why:* a context that authored a solution cannot see its flaws.
<a id="D5"></a>
- **D5 — Milestone discipline.**
  A milestone fits one session, lands runnable with all gates green, and ends at a handover + commit boundary. A milestone found too big mid-flight is split and the split recorded, never pushed through a degraded context.
  *Why:* gates certify complete units only, and context degradation is nonlinear.
<a id="D6"></a>
- **D6 — Ask immediately.**
  When the "why" of anything is unclear or an open question surfaces — in any phase — the work pauses and the question goes to the user. Intent is never guessed. An open question in a spec draft is written `[NEEDS CLARIFICATION: … Fallback: …]`.
  *Why:* a wrong guess costs a delta and rework; a question costs minutes.
<a id="D7"></a>
- **D7 — Plain reports with an example.**
  Every briefing, question and report is concise, plain, and carries a minimal example (a few lines of code, output, or before/after). Work is reported in three steps: preconditions verified → what changed → exact check commands.
  A bare "continue" from the user means: follow the `AGENTS.md` read order and resume from the handover's "Next action".
  *Why:* a human gate is only real if the human can grasp the point fast.
<a id="D8"></a>
- **D8 — Commit discipline.**
  Direct-to-main, atomic commits; the message states the problem solved and the tier; source and test change together when behavior changes.
  *Why:* problem-stating atomic commits are memory, rollback points and review units at once.
<a id="D9"></a>
- **D9 — Elegance gate.**
  Cognitive complexity per function ≤ 10 (`clippy.toml`, denied in `Cargo.toml`); `unsafe` is forbidden unless an ADR permits it for a named module.
  *Why:* complex functions are where agents hide defects and reviewers stop reading.
<a id="D10"></a>
- **D10 — Engineering guide.**
  `docs/ENGINEERING.md` is binding for all code: test first, a pure core with I/O in the shell, domain newtypes, outcomes explicit in the type, no `unwrap`/`expect`/`panic!` outside tests. Every rule a lint can check is denied in `Cargo.toml`; a local `#[allow]` carries a `// why:` comment.
  *Why:* agents default to the shortest code that compiles; stated rules plus lints make the well-typed path the easy one.
<a id="D11"></a>
- **D11 — Audits.**
  Every corrected mistake gets `docs/audits/<NNN>-<slug>.md` (three-digit ordinal, never reused; Symptom, Root cause, Rule) committed before the fix counts as done. Audits are append-only; a rule no longer valid is marked `SUPERSEDED by <NNN>`.
  *Why:* a mistake fixed without a written rule is made again by the next context.
  Added 2026-09-26; extends D1's artifact locations.
