# 0053: Dev-flow test fixes (from 0047)

status: done
agent: devflow
branch: issue/0053-devflow-test-fixes
depends-on: 0047
touches: crates/djs-difftest/, crates/djs-test262/ (job count from DJS_JOBS, the qjs result cache), crates/djs-core/tests/e2e.rs, the two slow djs-core unit tests, tests/corpus/ (no-stress headers), scripts/gate.sh (only if needed for the above)

## Goal
Implement devflow's proposals 2, 3, 4 and 6 from work/issues/0047-devflow.md (1, 5 and 7
are adopted as D012). Measure the gate before and after.

## Acceptance criteria
- [x] difftest: a `// difftest: no-stress` directive. Apply it to the six cases that only
      time out under the stress modes (array-methods, base64, proxy, test_map, test_rope,
      other-errors), so the compared set is the same on every run. The stress runs of
      all other cases stay hard failures on divergence.
- [x] e2e: parallel cases, a per-case fuel cap (a hung case fails in seconds), and a
      smaller `iteration_runtime.js`.
- [x] The two slow djs-core unit tests: limit fuel-1 suspension to the first N stops, as
      e2e does.
- [x] The test262/difftest runners default `-j` to `DJS_JOBS` when set (the repo's
      `.cargo/config.toml` sets 6; the gate sets full width).
- [x] A qjs result cache keyed by the qjs binary's hash and the test262 pin, so qjs runs
      don't repeat.
- [x] Gate wall/CPU time before and after on a quiet machine, recorded in the Log.

## Log
- devflow: difftest `no-stress` (D1530) on 8 cases: the issue's six plus
  `console/cyclic-array.js` and `language/bigint-ops.js`, which also timed out under stress
  by now. Measured per case, debug, `-j 1`, load ~25 (`case-times*.tsv` in scratch): the
  marked cases take 3–120+ s per stress mode, the slowest remaining one is
  `builtins/regexp-property-escapes.js` at 5.4 s (GC stress). Stressed runs now get 3x the
  time limit, and a stressed timeout is a hard failure again (it used to pass as "not
  compared", which also hid stress-only hangs). The dquickts importer writes the
  directive for its two cases (`NO_STRESS`; touches scripts/, minimal). TESTING.md
  documents the directive.
- Runners: `djs-test262` and `djs-difftest` default `-j` to `$DJS_JOBS`
  (`par::default_jobs`).
- e2e: plain runs of all cases first, then GC stress and suspension, on `DJS_JOBS` threads;
  per-run budget of 20M fuel, 100k output lines and 120 s. A case over budget in the
  plain run skips the stress modes: an infinite loop now fails in ~60 ms (checked with a
  temporary `for (;;) {}` case and a console.log loop). `iteration_runtime.js`'s spread
  went from 2000 to 300 objects (expected output regenerated with qjs via
  gen-expected.sh, identical to the hand edit): its GC stress run went from 23 s to 1.5 s.
  `DJS_E2E_CASE=<substring>` selects cases. e2e: 33–53 s → 6–8 s wall.
- Unit tests: `finish` (vm/tests.rs) suspends at the requested fuel for the first 1000
  stops, then in slices of 97. Lib test binary: 10.4 s / 45 CPU-s → 2.2 s / 19 CPU-s; the
  two slow tests 4–7 s → ~1 s.
- qjs cache (D1531): `vendor/qjs-cache/test262-v1-<hash>.tsv`. qjs active group: 25 s
  wall / 134 CPU-s → 3.7 s / 11 CPU-s on the second run.
- Not done: slowest-first scheduling in djs-test262. The slow tests (RegExp property
  escapes before 0056, the three TypedArray copyWithin "detached" tests now) sit in the
  middle of the alphabetical order, followed by ~19k fast `language/` tests, so they don't
  form the tail: the tail is bounded by one test (≤ 3–5 s debug under load).
- copyWithin "detached" tests: 2.9 s each in the debug runner at `-j 1` (3–5 s at
  `-j 18` under load); I did not see 26 s. The time goes to the harness looping over ~55
  constructor/arg-factory pairs with a 10,000-element array: `new Uint8Array(array)` of
  10k elements takes ~31 ms in debug (it walks the iterator protocol element by element;
  an ArrayBuffer + `set` takes ~8 ms), and `array.fill` ~10 ms. Follow-up below.
- Gate before/after (warm, `timed-gate.sh`; the machine never got quiet: load 7 at the
  start of each run, 30–125 by its end, so treat these as rough):

  | | wall s | CPU s | test step | difftest | test262 |
  |---|---|---|---|---|---|
  | before (main b8eefec), run 1 | 154 | 773 | 81 / 128 | 33 / 243 | 37 / 396 |
  | before, run 2 | 141 | 796 | 76 / 110 | 30 / 259 | 32 / 422 |
  | after, run 1 (load 88–125) | 145 | 709 | 72 / 140 | 20 / 121 | 44 / 441 |
  | after, run 2 | 103 | 670 | 29 / 117 | 15 / 116 | 54 / 430 |

  (step columns are wall / CPU s.) difftest: −50 % CPU, −50 % wall; the test step: −45 s
  wall at comparable load. test262 (~400 CPU-s here) is the RegExp property-escape cost
  that 0056 has since cut to ~90 CPU-s on main; it is the same on both sides.
- After rebasing onto main (0056 etc.): two new corpus cases also needed `no-stress`
  (`builtins/string-append.js` 5 s / 14 s, `builtins/normalize.js` 2 s / 7 s, debug,
  load ~20), so 10 cases are marked in total. Slowest remaining stressed case:
  `builtins/regexp-property-escapes.js` 5.4 s under GC stress (30 s limit). e2e slowest
  run: collections.js under GC stress, 5.3 s.
- Side task (coordinator/owner): 14-minute CPU sample of the whole machine, see
  "Machine sample" below.

### Machine sample (11:5x–12:0x, 273 samples every 3 s, `work/scratch/0053/sample/`)
All processes: mean 10.0 busy cores (p50 9.6, p90 16.9, max 18.6). CPU by owner,
cumulative cputime per process:

| | CPU s | cores avg |
|---|---|---|
| non-djs (Chrome 2.6 cores mean, max 12.9; WindowServer 0.2; the rest small) | 3227 | 3.9 |
| gates under the merge lock (heap, num, determinism, lexer; back-to-back) | 2065 | 2.5 |
| agents by hand: full `--ratchet --suspend-every 97 --gc-stress` difftest (determinism 2x, heap 2x, infra 1x release) | 1438 | 1.8 |
| agents by hand: full active test262 (heap debug 1x, infra release 2x) | 454 | 0.6 |
| agents by hand: cargo test (e2e at 100 % for 40–100 s each, unit tests) | ~500 | 0.6 |
| num: `detprobe` determinism probes (debug, fuel slices down to 1) | 259 | 0.3 |
| infra: `djs-bench --record engine` (release, on the busy machine) | 93 | 0.1 |
| builds (rustc, clippy) outside gates | ~350 | 0.4 |

- Caps: respected. Agent runs had `DJS_JOBS=6 RUST_TEST_THREADS=6` and `-j 6` and peaked
  at ~600 %; agent rustc peaked at 2–5 cores; gates ran at 18 as intended. Gates were
  serialized (one at a time), but there was a gate running for nearly the whole window,
  and two merges waited 3–5 minutes for the lock.
- Hangs/orphans: none. An idle `djs repl` in the main checkout (24 min, 0 % CPU).
- Waste: the hand-run full stress difftests (~300 CPU-s each, mostly stress timeouts that
  this issue removes) and full test262 active runs are gate steps run by hand: ~1890
  CPU-s, a third of all djs CPU. The release `--record` bench on a loaded machine
  records unreliable numbers. The serial e2e (8 runs, ~600 CPU-s at one core each) goes
  away with this issue.

## Follow-ups
- TypedArray from an Array walks the iterator protocol element by element (~3 µs per
  element in debug); a fast path for arrays with the original %ArrayIteratorPrototype%.next
  would make test262's TypedArray harness loops (copyWithin "detached" tests and others)
  several times faster. For the typed-array owner.
- `crates/djs-core/tests/regexp_fuel.rs` `suspended_match_resumes_from_the_image`: 6.6 s
  in debug (an image round trip every 97 fuel through a 2^14 backtracking match), now the
  slowest djs-core test binary after e2e. For num (0050/0056).
- GC stress cost is broad: a median corpus case takes 0.18 s under `--gc-stress` vs 0.08 s
  suspended, since every safepoint copies the whole heap including the base image. A
  cheaper GC stress (e.g. skipping the immutable base, or collecting every Nth safepoint)
  would cut the remaining difftest stress CPU (~100 s per gate) further. Engine work.
- AGENTS.md: say explicitly that the full stress difftest and the full active test262 run
  are gate steps (filter while iterating), and that `scripts/bench.sh --record` needs a
  quiet machine (or have bench.sh refuse `--record` above a load threshold).

## Result
Landed:
- difftest: `// difftest: no-stress <reason>` skips the stress variants for a case; 10
  corpus cases carry it, with their measured times. Stressed runs get 3x the time limit
  and a stressed timeout is a hard failure again (D1530, amends D1347). The dquickts
  importer writes the directive (`NO_STRESS`).
- e2e (`crates/djs-core/tests/e2e.rs`): parallel on `DJS_JOBS` threads, plain runs first,
  a per-run budget (20M fuel, 100k lines, 120 s); a runaway case fails in well under a
  second and skips its stress runs. `DJS_E2E_CASE=<substring>` and `DJS_E2E_MODE` select.
  `iteration_runtime.js` spreads 300 objects instead of 2000.
- vm unit tests: fuel-1 suspension for the first 1000 stops, then slices of 97.
- `djs-test262` / `djs-difftest`: `-j` defaults to `$DJS_JOBS` (else the CPU count).
- `djs-test262 --engine qjs` caches results in `vendor/qjs-cache/` (D1531); `--no-cache`.

Effect (noisy machine, see the Log): difftest gate step −50 % CPU (~240 → ~120 CPU-s) and
wall; `cargo test` wall −45 s (e2e 33–100 s → 6–20 s, lib unit tests 10 s → 2 s); a repeated
qjs active run 134 → 11 CPU-s. Known gaps: GC stress is still ~0.2 s per small case (see
Follow-ups); the test262 runner keeps alphabetical order (no slowest-first; not needed
now).
