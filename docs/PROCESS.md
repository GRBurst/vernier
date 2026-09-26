# The Increment Loop

Every increment of work follows this loop. Governed by [CONSTITUTION.md](CONSTITUTION.md).
The unit of work is the **milestone** (D5). An open question at any phase pauses the work and goes to the user (D6).

## Ceremony tiers (D2)

| Tier | Scope | Phases |
| :--- | :--- | :--- |
| `full-spec` | New capability, contract change | 0–7 |
| `spec-delta` | Incremental change to a specced capability | 1–7 on a delta |
| `direct-patch` | Fix, typo, dependency bump; no contract change | 5 + commit |

## Roles (D4)

- **Coordinator** — owns scope: drafts specs, slices tasks, routes failures.
- **Implementor** — executes one task: touches only its named files, never edits a test to make it pass.
- **Verifier** — `just verify` first, judgment second; judgment review runs in a fresh context.
  A failure goes back to the Coordinator (re-scope or re-plan), never straight back to the Implementor. Retry cap: 3 per task, then defer it to `docs/BACKLOG.md` with a trigger.

## Phases

Every phase has an exit gate; the tag names who decides it.

### Phase 0 — Research

Check what the spec depends on that nobody has verified: a crate's existence and API, a formula's source, a model's license.
Record each finding as a committed note or measurement, never as memory.
**Exit (judgment):** every external fact a criterion relies on is verified or marked `[NEEDS CLARIFICATION]`.

### Phase 1 — Spec

Draft or extend `docs/specs/<NNN>-<slug>/spec.md` in the format `just spec-check` enforces: `**Status:**`, `**Scope:**`, `**Non-Goals:**`, then milestones `## M<id> — <title> (Status: PLANNED)` with an anchor and EARS acceptance criteria.
Mark every unknown `[NEEDS CLARIFICATION: … Fallback: …]`. Commit the draft before review, so the reviewer reads a fixed file.
**Exit (deterministic):** `just spec-check` passes.

### Phase 2 — Review and human gates

1. **Adversarial review** in a fresh context: the reviewer gets the spec and the repository, never the authoring session, and must name concrete defects. A finding is *blocking* when repairing it changes what a criterion requires or shows the spec says something false. Repair, commit, re-review; stop when a pass has no blocking finding, or after 4 passes — record what is left in the spec's `gates.md`.
2. **Clarification** (human): every `[NEEDS CLARIFICATION]` answered; answers written into the spec.
3. **Spec-Approval** (human): the user confirms the criteria are unambiguous, testable and complete. The spec's `**Status:**` becomes `Approved`, after which `just spec-check` refuses any remaining marker.
4. **Comprehension** (human): the user restates the change in their own words, and answers three questions a fresh-context reviewer wrote from the spec (not from the authoring session, which holds the answers).

**Exit (human):** all three verdicts recorded in `docs/specs/<NNN>-<slug>/gates.md`, one dated block each (append-only).

### Phase 3 — Plan

Slice the milestone into tasks: objective, files touched, boundaries. A task touching more than 5 files is split.
If the milestone will not fit one session, split it now and record the split in the spec.
**Exit (judgment):** every criterion maps to a task; every task names its files.

### Phase 4 — Implement

Open with one or two plain sentences on what is being built (D7). Test first: a failing test, then the code.
When a technical fact invalidates the spec: **halt**, record it (delta, and an ADR if architectural), re-plan — never silently adapt the spec or the tests.
**Exit (judgment):** each task lands as an atomic commit naming the milestone and tier.

### Phase 5 — Verify

`just verify` (spec-check and its tests, `cargo fmt --check`, `clippy -D warnings` with the complexity gate, `cargo test`), plus the milestone's own checks.
**Exit (deterministic):** all gates green. **Exit (judgment):** the tool runs end to end; a criterion is ticked only when verified.

### Phase 6 — Handover

Rewrite `docs/HANDOVER.md` in three steps (D7): preconditions verified (exact commands) → what changed → how to check it, then open tasks, next action, and known-bad approaches.
Set milestone `Status:` honestly: `IMPLEMENTED` until the user reviewed it, `DONE` after.
Hand over at every milestone boundary, not only at session end.
**Exit (deterministic):** a clean commit that includes the handover.

### Phase 7 — Retro

Record what the process cost and where it rubbed; friction becomes a backlog item or a delta.
**Exit (judgment):** at least one recorded friction item, or the words "none found".

## Deltas (D3)

After a spec is approved, a change is written as `docs/specs/<NNN>-<slug>/history/delta-<SSS>-<slug>.md`:

````markdown
# Delta 001 — Short summary of the change

**Target:** ../spec.md
**Status:** PROPOSED

## MODIFIED

### M2 — Surface readability engine

<the full replacement text of the milestone>

## ADDED / REMOVED

<same form>
````

A delta runs phases 1–7 like a spec. At **fold-back**, the approved delta's blocks replace the spec's text in one commit and its status becomes `FOLDED`; a delta replaced by another becomes `SUPERSEDED by delta-<SSS>`. Deltas are never deleted.

## ADRs

An architectural decision (a parser route, a dependency with native code, permitting `unsafe`) gets `docs/adr/<NNNN>-<slug>.md` with `Status: Proposed | Accepted | Superseded`, and sections Context, Decision, Consequences. It is append-only once `Accepted`.
