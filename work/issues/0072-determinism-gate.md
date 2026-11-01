# 0072: Determinism checks in the gate

status: blocked
agent: determinism
branch: issue/0072-determinism-gate
depends-on: part 2: 0067, 0071
touches: crates/djs-core/tests/e2e.rs, crates/djs-difftest/, scripts/determinism/, scripts/gate.sh

## Goal
From the determinism investigation (0055): see work/issues/0055-determinism-investigation.md,
"Proposed issues", for the full goal and acceptance criteria. Experiment tools are in
scripts/determinism/.

## Acceptance criteria
- [ ] Part 1 (now): collected-image hashes compared across the e2e modes, a native-vs-wasm image check with the twin drivers, difftest `--determinism`. Part 2: async delivery cases and raw-image equality.
- [ ] The criteria in 0055's report for this item.
- [ ] Ratchets unchanged or better, stress modes identical, decisions recorded.

## Log

- **Part 1 is merged; part 2 is blocked on 0071** (raw-image equality needs GC points and
  suspension records that don't depend on the slice). Part 2's async delivery cases
  already exist: 0067's vm tests run every inbox case under eager, one-at-idle and batch
  delivery × three slices × restore (`inbox_outcome`).
- e2e (crates/djs-core/tests/e2e.rs): `run_case` also returns the FNV-64 of the image
  after a final `Vm::collect`. Plain and suspension must agree. GC stress is excluded:
  `collections.js` differs only through its WeakMap (dead entries are cleared at other
  times and the table compacts differently; without the WeakMap lines the stress image
  matches). All 62 cases pass.
- difftest `--determinism`: `RunResult::image` (djs: the image hash after a final collect,
  skipped for timeouts and crashes), `Job::now` (one clock reading per case for the main
  run and its variants: the clock is heap data, and per-variant wall-clock readings made
  images differ). The flag adds `--suspend-every 1009` (plus 97 when no
  `--suspend-every` is given), and in the runner a stress variant with the same transcript
  but another image is a mismatch. Checked that it bites: a suspended variant with a clock
  1 ms off is reported for every case. Runner test with an "ImageLiar" engine. The gate's
  difftest line has `--determinism`: about +25 s wall at -j 6 (68 s → 93 s); all
  identical.
- Native vs wasm: the ABI driver moved from scripts/determinism/probe to
  `crates/djs-difftest/src/bin/djs-abidrive.rs` (djs-difftest now depends on djs-wasm;
  the driver uses `unsafe` for the export buffers' raw addresses, with SAFETY comments).
  Its twin is `scripts/determinism/abidrive.mjs`. `scripts/determinism/native-vs-wasm.sh
  <wasm>` runs floats_body, general_body, deep_body and weakref at slices 1e6 and 997 and
  diffs the outputs: build hash, raw and collected image hashes, stops, result. In the
  gate after the js tests, against the debug wasm (7 s). OK.
- TESTING.md (outside `touches`, two sentences in the difftest section) and D1720.

## Follow-ups

- Part 2 after 0071: compare *raw* images too (e2e plain vs suspend, difftest
  `--determinism`, native-vs-wasm already compares raw at two slices per build but not
  across slices).

## Result

Part 1: the gate now fails if a suspended run (e2e suspension mode, difftest slices 97 and
1009 with restores) leaves a different heap image after a final collection than the
plain run, or if native and wasm builds differ in images, fuel or results
(`scripts/determinism/native-vs-wasm.sh`, `djs-abidrive`). GC stress stays an
output-only comparison (D1720). Part 2 (raw images) waits for 0071.
