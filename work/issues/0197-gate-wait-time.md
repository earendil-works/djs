# 0197: Gate wall time vs CPU: remove the waits

status: done
agent: gate-speed
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

- 0191's 639 s gate: the critical path was the `target/` group run one job after another:
  build:tests 316 s → bgen 95 s (0.6 CPU-s) → doctests 228 s (9.7 CPU-s) = 639 s, all
  three mostly waiting for macOS's first-run check of fresh executables.
- Done (D2790–D2792): bgen leg dropped (djs-bgen's golden test now checks prelude-slim.js
  too, so it covers `bgen.sh --check`); `doctest = false` for every library and the
  doctests leg dropped; `test = false` for the five programs without unit tests; an
  always-run `targets` leg guards both (tested: a planted doctest and a planted #[test]
  fail it). One integration-test binary per package (`tests/it/`): djs-bgen 3→1, djs-cli
  2→1, djs-core 3→1 (e2e stays separate). The four wasm clippy combinations in parallel
  (124 s → ~45 s). Builds end when cargo does; pending first runs overlap the legs.
- New executables a core change runs in the workspace build: 24 → 16; the `target/`
  chain after build:tests (bgen, doctests) is gone.
- Lab measurements, same core change (a comment in compiler/mod.rs):
  - before (main): 138, 69, 78 s; after: 89, 77, 85, 65, 76 s. With syspolicyd quick
    (bgen 2 s, doctests 7–18 s in the before runs) the difference is small; the removed
    chain is what cost 0191 323 s when it was slow.
  - Warm policy A/B (same commit): legs start when cargo is done 77/85 s; waiting for all
    first runs 88/100 s; no early runs 105/105 s.
- Critical path now: build:tests (cargo ~28–55 s: djs-core, djs-prelude's build script
  (a fresh executable, checked before it runs) and its base image, the dependents) →
  test (13–21 s; djs_core's lib tests and pending first-run checks). The LTO size builds
  (~45–55 s) are next behind it.

## Result

The waiting chain is gone: no bgen or doctests jobs after the workspace test build, 8
fewer new executables per Rust change (no doctest or empty test binaries, one
integration-test binary per package), the wasm clippy combinations side by side, builds
that end when cargo does. A `targets` leg keeps `doctest = false`/`test = false` honest.
Integration tests run as `cargo test -p X --test it [module::]` (djs-core's e2e as
before). Measured: core change 138/69/78 s before, 65–89 s after in the lab; 0191's
worst case (639 s) lost its 323 s serial tail. Still open: macOS's first-run checks
(Developer Tools is off); with them on, the build:tests/test path would be roughly cargo
plus test run time.

## Follow-ups

- Developer Tools for the agents' app (owner).
- Each target directory compiles and runs djs-prelude's build script (a fresh executable
  per directory, ~20 per core change); fewer cargo configurations or a base image that
  does not need the build script would remove most of the remaining checks.
