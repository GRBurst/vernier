# vernier

Readability and syntactic-complexity analyzer for Markdown, written in Rust.
Status: spec 001 is a draft; M1 (prose extraction, `vernier analyze` span and word counts) is implemented. See `docs/HANDOVER.md`.

```sh
devenv shell          # pinned toolchain: rust, just, python3, git hooks
just verify           # all gates
```

How work happens here: `AGENTS.md` → `docs/PROCESS.md`.

## Parser model license

No model is bundled. The standard UDPipe Universal Dependencies models are believed to be
CC BY-NC-SA (non-commercial); this is **not yet verified** and is checked before spec 001 M3b.

---

## Setting up from the starter kit (delete this section after the first commit)

```sh
mkdir -p ~/projects/vernier
cp -a <sdd-repo>/.sdd/rust-starter/. ~/projects/vernier/   # the trailing /. copies .gitignore too
cd ~/projects/vernier
git init
devenv shell          # first run creates devenv.lock and installs the git hooks
just verify           # expect: 13 clarify-open warnings, then "✓ all gates green"
git add -A
git commit -m "bootstrap: the project had no process, gates or spec (full-spec)"
```

Then start a session in your agent harness and say **continue**: it reads `AGENTS.md`,
then `docs/HANDOVER.md`, whose next action is the review of spec 001.

What the kit changes compared to the SDD framework it was taken from:
the process keeps its phases, tiers and human gates, but drops the framework's research
corpus, report pipeline, writer/reader models, measurement schema and its ~40 repo-specific
checks. The spec linter is ~200 lines of stdlib Python instead of ~27k.
