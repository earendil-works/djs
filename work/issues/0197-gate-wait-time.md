# 0197: Gate wall time vs CPU: remove the waits

status: open
agent: -
branch: issue/0197-gate-waits
depends-on: 0183
guide: -
touches: scripts/gate.py, Cargo.toml / crate manifests (doctest/test flags), crates/*/tests layout (combining test binaries), TESTING.md

## Goal
0191's gate (a Rust change): 639 s wall for 1,329 CPU-s, with legs that mostly wait:
doctests 228 s wall / 9.7 CPU-s (3 ignored doctests), bgen 95 s / 0.6 CPU-s, build:tests
316 s / 220 CPU-s, native vs wasm 54 s / 3.3 CPU-s. The waits are macOS first-exec
notarization checks of every freshly linked binary (0183's finding; Developer Tools is
still disabled: `DevToolsSecurity -status`). Until the owner enables it, cut the number of
fresh executables and the waiting legs:
- doctests: `doctest = false` for crates without doctests (with a guard that fails if one
  is added), or drop the leg if no crate has runnable doctests;
- bgen leg: avoid a fresh binary per run (reuse the test binary, or run it from an
  already-checked binary);
- combine integration-test crates into one binary per package where it costs nothing
  (0183 follow-up), `test = false` for binaries without unit tests (with a guard);
- check which legs wait on others needlessly (the critical path) and reorder.
Measure a Rust-change gate before/after (same commit), report the critical path.

## Log

## Result
