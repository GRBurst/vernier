# Audit 001 — Scratch backup in /tmp could not be read back

**Severity:** low
**Tags:** sandbox, nono, process

## Symptom
A backup written with `cp src/main.rs /tmp/main.rs.bak` inside an agent session failed to restore: `cp: cannot open '/tmp/main.rs.bak' for reading: Permission denied`.
The edited `src/main.rs` stayed modified until it was restored from git.

## Root cause
The nono sandbox lets the session write to `/tmp` but not read those files back; the assumption "what I wrote I can read" was wrong.

## Rule
Keep agent scratch files in the untracked `.sdd/` (B5), never `/tmp`; restore tracked files with `git checkout -- <path>`, not from ad-hoc copies.
