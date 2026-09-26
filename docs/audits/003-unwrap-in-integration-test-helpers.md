# Audit 003 — `unwrap` in an integration-test helper failed the lint gate

**Severity:** low
**Tags:** clippy, tests, lints

## Symptom
`just lint` failed with `clippy::unwrap_used` on `tests/cli.rs` helpers (`vernier`, `expected_line`), although `clippy.toml` sets `allow-unwrap-in-tests = true`.

## Root cause
`allow-unwrap-in-tests` exempts only `#[test]` functions and `#[cfg(test)]` modules.
A plain helper function in an integration-test crate (`tests/*.rs`) is neither, so the crate-wide deny applies to it.

## Rule
A helper outside a `#[test]` function that unwraps carries `#[allow(clippy::unwrap_used)]` on that function with a `// why:` naming this audit — or returns a `Result` instead. Never allow it for the whole test crate.
