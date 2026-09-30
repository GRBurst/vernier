# Audit 016 — vernier panicked on a closed stdout

**Severity:** medium
**Tags:** cli, io, exit-code, review

## Symptom
`(vernier check --max-sentence-len 1 f*.md; echo "exit $?" >&2) | head -1` printed `failed printing to stdout: Broken pipe (os error 32)` with a backtrace and exited 101, although M5 criterion 2 lists only the exit codes 0, 1 and 2.
Review pass 2 of spec 001 found it (R2-B1).

## Root cause
Every stdout write in `src/main.rs` (`print_report`, `print_document`) used `println!`, which panics when the write fails.
Rust ignores `SIGPIPE`, so a closed pipe is not a signal but a write error, and `println!` turns that outcome into a panic, the construct D10 forbids for outcomes.
The lints ban `unwrap`, `expect` and `panic!`, but not the macros that panic inside std; and no test ran the binary with a stdout that fails, so every exit-code test passed with an always-writable pipe.
The exit contract listed each input that can fail (file, model, sentence) but not the output.

## Rule
Write stdout through a fallible writer (`writeln!` on `io::stdout().lock()`, or a `W: Write` passed in), propagate the `io::Error` with `?`, and map it to a typed outcome: exit 2 and one stderr line (`vernier: cannot write to stdout: <cause>`).
Never use `println!` or `print!` in the shell; when an exit contract is written, list the output's failure as well as each input's.
Tested by `a_closed_stdout_exits_2_without_a_panic` (tests/cli.rs, a pipe whose reader was dropped before the run) and `a_failed_write_exits_2` (src/main.rs, a writer that refuses every write).
