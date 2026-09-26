# Every recipe runs inside the pinned devenv environment.

default:
    @just --list

# Rust formatting check (no rewrite)
fmt-check:
    cargo fmt --all -- --check

# Lints, warnings as errors, incl. the cognitive-complexity gate (clippy.toml)
lint:
    cargo clippy --all-targets --locked -- -D warnings

# Rust test suite
test:
    cargo test --all-targets --locked

# Spec house-style gate (fails closed when no spec exists)
spec-check *args:
    python3 tools/spec-check/spec_check.py {{args}}

# The spec linter's own tests
spec-check-test:
    python3 -m unittest discover -s tools/spec-check

# All deterministic gates; run before claiming anything works
verify: spec-check spec-check-test fmt-check lint test
    @echo "✓ all gates green"
