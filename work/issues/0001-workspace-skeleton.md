# 0001: Workspace skeleton and process

status: done
agent: coordinator
branch: main
depends-on: -
touches: Cargo.toml, crates/, scripts/, work/, docs/, AGENTS.md, TESTING.md, .gitignore

## Goal
Cargo workspace, the gate, the merge script and the work tracking files.

## Result
- Workspace with `crates/djs-core` (no_std, empty) and `crates/djs-cli` (`djs` binary stub).
- `scripts/gate.sh` (fmt, clippy -D warnings, tests, wasm32 build of djs-core).
- `scripts/merge-to-main.sh` (lock, rebase, gate, ff-merge).
- AGENTS.md, TESTING.md, docs/decisions.md, work/README.md, work/BOARD.md.
