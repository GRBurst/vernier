# Audit 019 — vernier panicked when a write to stderr failed

**Severity:** medium
**Tags:** cli, io, exit-code, review

## Symptom
`vernier check tests/fixtures/long.md 2>/dev/full >/dev/null` exited 101 with a panic instead of 1, and with stderr on a pipe whose reader had gone, `check`, `check --format json` and `analyze` on a missing file exited 101 instead of 2.
Review pass 3 of spec 001 found it (R3-B1), one pass after audit 016 had fixed the same panic on stdout.

## Root cause
Every stderr write in `src/main.rs` (the no-model notice, a model that cannot be loaded, an unreadable file, a sentence that cannot be parsed or located, a sentence too long for the model, the JSON document, a failed stdout write) used `eprintln!`, which panics when the write fails.
Audit 016's rule named `println!` and `print!` only, so its fix and its lint (`clippy::print_stdout`) covered one of the two standard streams; the same construct on the other stream was left as it was.
No test ran the binary with a stderr that fails.

## Rule
Write every standard stream through a fallible writer, never a printing macro: stdout as audit 016 says, stderr through `say` in `src/main.rs` (`writeln!` on a `W: Write`, stderr locked in `main`, a failed write dropped with a `// why:`, because the diagnostic cannot be reported anywhere else and the exit code already carries the outcome).
A failed stderr write never changes the exit code the other clauses give.
Enforced by `clippy::print_stderr` (denied outside tests, ENGINEERING §8) beside `clippy::print_stdout`; tested by `a_failed_stderr_write_keeps_the_exit_code` in tests/cli.rs (a closed stderr pipe and `/dev/full`, seen exiting 101 first) and in src/main.rs (a writer that refuses every write).
When a rule bans a construct on one stream, file or channel, check every sibling it applies to before closing the audit.
