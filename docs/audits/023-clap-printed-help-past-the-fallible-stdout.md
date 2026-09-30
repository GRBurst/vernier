# Audit 023 — clap printed the help and the version past vernier's fallible stdout

**Severity:** medium
**Tags:** cli, io, exit-code, clap, review

## Symptom
`vernier check --help >/dev/full` and `vernier --version >/dev/full` exited 0 with an empty stderr, although M5 criterion 2 says a failed write to stdout exits 2 with `vernier: cannot write to stdout: <cause>`.
Review pass 4 of spec 001 found it (R4-B2), two passes after audit 016 and one after audit 019 had routed vernier's own writes through fallible writers.

## Root cause
`main` called `Cli::parse()`, which on a help, a version or a usage error prints through clap's own writer and exits the process; clap's `Error::exit` swallows a failed write ("Swallow broken pipe errors") and exits 0 for a help or the version.
Audits 016 and 019 fixed and linted vernier's printing macros (`print_stdout`, `print_stderr`), but no lint sees a dependency that writes to a standard stream on vernier's behalf, and no test ran `--help` or `--version` with a failing stdout.
This is not audit 019 again: 019's rule (every stream through a fallible writer) was right and was applied to every write vernier makes; the writes that escaped were clap's, which the rule's lint cannot reach.

## Rule
No dependency writes to a standard stream on vernier's behalf: take its rendering and write it through vernier's own writers.
`main` calls `Cli::try_parse`, and `show_refusal` (src/main.rs) writes clap's rendered help or version on stdout (exit 0, or exit 2 with the stdout-failure line when the write fails) and a usage error on stderr (exit 2, a failed write dropped as `say` does), each stream wrapped in `anstream::AutoStream` with `ColorChoice::Auto`, as clap's own printing does, so the text and its colour are clap's.
Enforced by `disallowed-methods` in clippy.toml (`clap::Parser::parse`, `parse_from`, `clap::error::Error::exit`, `print`), listed in ENGINEERING §8.
Tested by `help_or_version_into_a_failed_stdout_exits_2` (tests/cli.rs, a closed pipe and `/dev/full`, seen exiting 0 first), `help_or_version_is_clap_s_text` and `a_usage_error_is_clap_s_text_on_stderr` (tests/cli.rs, clap's rendering exactly, plain on a pipe and coloured under `CLICOLOR_FORCE=1`), and the unit tests `help_or_version_is_written_through_the_fallible_stdout` and `a_usage_error_is_written_through_the_fallible_stderr` (src/main.rs).
`anstream` 1 (MIT OR Apache-2.0, pure Rust) became a direct dependency; it was already in Cargo.lock as clap's.
