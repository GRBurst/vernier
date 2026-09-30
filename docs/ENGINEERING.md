# Engineering Guide

How code is written in this repository. Binding by [CONSTITUTION.md](CONSTITUTION.md) D10; the loop it runs inside is [PROCESS.md](PROCESS.md).
Where a rule is mechanical it is a lint or a gate (B2), and the table in §8 names it; the rest is judgment a reviewer checks.
A deviation needs a stated reason next to it (`// why: …`) and, if architectural, an ADR.

## 1. Source of truth (in order)

1. **Code and tests** — behavior is what runs and what the suite asserts.
2. **Specs, deltas, ADRs** under `docs/` — planned and approved work; an approved delta supersedes the spec text it replaces.
3. **Audits** (`docs/audits/`) — known failure modes and the rules that prevent their regression.
4. **Constitution and process**, then this guide — how we work.

When code and spec disagree, that is a finding: halt and record it (PROCESS phase 4); never silently adapt either side.

## 2. Toolchain

- Everything is project-local and pinned by devenv (B4). Never create or modify global cargo, rustup or user configs; never `cargo install` onto the host.
- A new dependency is a Phase 0 fact: verify it exists, its API and its license before a criterion relies on it. A dependency with native code needs an ADR.

## 3. Test first

- **Red → green → clean**, strictly in that order. Write a failing test and watch it fail; write the minimum code that makes it pass; then refactor with the suite green.
- One failing test at a time. No code ahead of a test — no "we'll need this later".
- Every EARS criterion has at least one test. The test's doc comment restates the criterion as Given/When/Then, one `When` per test:

  ```rust
  /// Given a sentence whose subject and verb are 8 words apart
  /// When center-embedding is detected
  /// Then the detector reports subject "proposal", verb "caused", distance 8
  #[test]
  fn reports_the_subject_verb_gap_of_a_center_embedded_sentence() { … }
  ```

- Tests state laws and properties (A4); a hand-computed value is one witness beside a property, never instead of it. Example laws: a metric is invariant under Markdown markup that hides no prose; every reported position points into the original file.
- A guard (validation, threshold, limit) is proven only by a test that fails when the guard is removed.
- Test names state what the system does, not how: `rejects_an_empty_file`, not `test_parse_2`.

## 4. Clean code

- **Names are the spec's vocabulary.** A concept the spec names (`Sentence`, `Token`, `Diagnostic`, `DependencyDistance`) has exactly that name in code and tests; drift between them is a defect. No abbreviations beyond the domain's own (`UD`, `MDD`); booleans read as questions (`is_passive`); functions are verbs.
- **A function does one thing.** If describing it needs "and", split it. Functions fit on one screen; cognitive complexity ≤ 10 is a gate (D9).
- **No surprising side effects.** A query does not mutate, print or read files.
- **Comments say why, never what.** Delete commented-out code; history is in git.
- **Refactoring is its own phase and its own commit**: behavior unchanged, tests green before and after, steps small (rename → extract → move → inline). A refactor never rides along with a feature, and code outside the current change is not "cleaned up" in passing.

## 5. Types model the domain

- **Newtypes over primitives.** A value with an invariant is a tuple struct with a private field and a smart constructor; every downstream function can then trust it.

  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
  pub struct GradeLevel(f64);            // field private to the module

  impl GradeLevel {
      pub fn new(raw: f64) -> Result<Self, MetricError> {
          if raw.is_finite() { Ok(Self(raw)) } else { Err(MetricError::NotFinite(raw)) }
      }
  }
  ```

- Value objects derive `PartialEq`/`Eq` — structural equality is their definition. Something with identity compares by its id only.
- **Transformations return new values**: take `self` (or `&self`) and return a value; `&mut self` needs a reason.
- **Iterator pipelines over mutable accumulators**:

  ```rust
  let long_sentences: Vec<&Sentence> = doc.sentences().filter(|s| s.word_count() > limit).collect();
  ```

## 6. Functional core, imperative shell

- The analysis — prose extraction, sentence splitting, metrics, rule evaluation — is pure: same input, same output, no I/O.
- File reading, model loading, the clock, environment variables, stdout/stderr and exit codes live in the shell (`main` and the CLI layer). The shell reads, calls the core, and writes.
- Ambient input is passed in as an argument or a trait (`Parser`, a clock), never read inside the core.
- Operations are idempotent where the domain allows (analyzing a file twice yields the same report); a state transition that can repeat gets a test for double application.

## 7. Outcomes are explicit in the type

Apply this to every value that may be missing, in order:

1. Can it be made mandatory by construction? → make it mandatory.
2. Is absence normal and needs no explanation? → `Option<T>`.
3. Does the failure mean something to the caller? → `Result<T, E>` with a closed error enum.
4. Are several states being represented? → an `enum`, one variant per state.
5. Is it optional only because an external format allows it? → convert at the boundary; it never travels into the core.

- `Option` is never a silent failure channel: if two ways of coming back empty need two different tests, they need two different variants.
- Optional fields that depend on each other or on a flag (`Option<Receipt>` + `is_done: bool`, "only set when …") become an `enum`.
- Eliminate exhaustively: `match` every variant of our own enums with no `_` arm, so a new variant breaks the build where it must be handled. No arbitrary fallback value for a missing one — a default is a domain decision and needs a test.
- Propagate with `?`; the happy path reads top to bottom.
- **Errors**: one `thiserror` enum per module, one variant per distinct failure, carrying the data the caller needs. `anyhow` only in `main`, never as a core function's error type. Both crates are added when first needed.
- **Panics are for defects**, not outcomes: a broken internal invariant may panic; user input, files and models never cause one.

## 8. Banned constructs

| Construct | Why | Instead | Enforced by |
| :--- | :--- | :--- | :--- |
| `unwrap()`, `expect()` | a typed failure becomes a crash | `?`, or a match | `clippy::unwrap_used`, `expect_used` |
| `panic!`, `todo!`, `unimplemented!` on a reachable path | unchecked control flow | return `Err` | `clippy::panic`, `todo`, `unimplemented` |
| `dbg!` | debug output leaks into the product | a test assertion | `clippy::dbg_macro` |
| `println!`, `print!` | panics when stdout is closed (audit 016) | `writeln!` on a `W: Write`, the error propagated | `clippy::print_stdout` |
| `eprintln!`, `eprint!` | panics when stderr is closed or full (audit 019) | `writeln!` on a `W: Write`; a failed stderr write is dropped with a `// why:` and never changes the exit code | `clippy::print_stderr` |
| `unsafe` | escapes every guarantee | nothing, unless an ADR permits a named module (D9) | `unsafe_code = "forbid"` |
| `RefCell`, `Mutex`, `static mut`, global state in the core | hidden mutable state | pass state in, return it out | review |
| `SystemTime::now()`, `env::var`, file I/O in the core | ambient input | an argument or a trait, filled by the shell | review |

Tests may use `unwrap`, `expect` and `panic` (`clippy.toml`). Anywhere else, an `#[allow(clippy::…)]` sits on the smallest item possible with a `// why:` comment.

## 9. Audits — memory against regression

Every time a mistake is corrected — a wrong assumption, a broken build, a failed review finding, a toolchain deviation — a new `docs/audits/<NNN>-<slug>.md` is committed before the fix counts as done (D11).
`NNN` is three digits, strictly increasing, never reused. One failure mode per file:

```markdown
# Audit 001 — Mistake name

**Severity:** low | medium | high
**Tags:** parser, cargo, nix, process

## Symptom
What was observed.

## Root cause
Why it actually happened.

## Rule
An imperative, checkable rule that prevents the regression — and, where possible, the gate that now enforces it.
```

Before starting work in an area, grep `docs/audits/` for its tags.
