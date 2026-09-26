# Audit 002 — Crate download wrote to the global ~/.cargo

**Severity:** medium
**Tags:** cargo, devenv, sandbox, nono

## Symptom
The first `cargo info pulldown-cmark` inside the devenv shell failed: `failed to open /home/pallon/.cargo/registry/cache/…: Permission denied`.
The nono sandbox grants `~/.cargo` read-only, and the devenv shell left `CARGO_HOME` unset, so cargo defaulted to the user's global registry.

## Root cause
`devenv.nix` pinned the toolchain but not cargo's home; every crate download depended on, and wrote into, per-user state outside the project — the very thing B4 forbids.

## Rule
`CARGO_HOME` is project-local: `devenv.nix` sets `env.CARGO_HOME = "${config.env.DEVENV_STATE}/cargo"` (under the untracked `.devenv/`).
An agent shell started before that line existed runs `export CARGO_HOME=$DEVENV_STATE/cargo` before any cargo command.
