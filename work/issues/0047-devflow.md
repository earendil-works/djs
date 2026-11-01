# 0047: Development flow efficiency: why the machine is spinning, and proposals

status: done
agent: devflow
branch: issue/0047-devflow
depends-on: -
touches: work/issues/0047-devflow.md (the report), work/scratch/0047/ (measurements). No changes to shared scripts, Cargo.toml or code without coordinator approval.

## Goal
The owner reports that the machine is "spinning like mad": we recompile a lot and run
too many tests. Investigate where the CPU time goes in our development flow and come
back with concrete, measured proposals. This is an **investigation**: report first. The
coordinator decides what to adopt, and implementation follows as separate steps.

## Context (coordinator's observations)
- 18 cores. Four implementation agents work in parallel, each in its own worktree with
  its own `target/` (1.6–2.5 GB each), so every dependency and djs-core change is
  compiled four times or more.
- `scripts/gate.sh` runs on every merge (`scripts/merge-to-main.sh` rebases, then gates).
  It covers: fmt; clippy (native, all targets); `cargo test --workspace`; a wasm32 build
  of djs-core; wasm32 release clippy; bgen --check; a djs-wasm debug build; node tests;
  wire and reader cross-language tests (more cargo test invocations); difftest with
  suspension and GC stress; and the full test262 active group (32k tests) in debug.
- djs-prelude's build.rs compiles djs-core for the host to build the base image. D1330
  hashes djs-core's sources, so any djs-core change rebuilds the base and everything
  downstream (djs-cli, difftest, test262, wasm). djs-wasm builds compile djs-core twice
  (host for build.rs, and wasm32).
- `[profile.dev.package.djs-core] opt-level = 1` (D010).
- Agents also run `cargo test --workspace`, test262 and difftest themselves while
  iterating, often in release, plus benchmarks.

## What to deliver
- [x] Measurements: wall and CPU time of each gate step (cold and warm cache), of an
      incremental rebuild after touching one file in djs-core / compiler / builtins, and
      of `cargo test --workspace`. Which crates rebuild and why (`cargo build -v`, or
      `CARGO_LOG=cargo::core::compiler::fingerprint=info` for the reasons). How many
      times djs-core is compiled per gate run. Disk use per worktree.
- [x] Proposals, each with the expected saving and its risk. Candidates to evaluate (not
      exhaustive):
  - a shared `CARGO_TARGET_DIR` across worktrees (with its lock contention), or sccache
    (only if it is already installed; do not install anything);
  - splitting the gate into a fast tier (always, per merge) and a slow tier (periodic, or
    by the coordinator), and/or running test262/difftest in release with a cached build;
  - reducing duplicate compilation (the build.rs host compile of djs-core, profiles that
    differ only slightly so cargo rebuilds: dev vs test vs release vs wasm, debuginfo
    settings, `--all-targets` clippy vs test building the same things twice);
  - whether the D1330 source hash is too eager (for example hashing only what the image
    depends on);
  - agent habits: what agents should run while iterating (targeted tests: `cargo test -p
    djs-core <filter>`, a filtered test262 run) vs before merging (the gate). Propose a
    short "how to test while iterating" section for AGENTS.md;
  - limiting parallelism (`CARGO_BUILD_JOBS`, `nice`) so four agents don't oversubscribe
    18 cores;
  - anything else you find.
- [x] A recommended set: what to change first, in order, with the measured or estimated
      effect.

## Notes
- You get your own worktree for experiments (`.worktrees/0047-devflow`). Measure there,
  not in other agents' worktrees, and don't kill other agents' processes.
- Machine load from other agents makes timings noisy. Repeat them and say how noisy they
  were.
- Use `scripts/timeout.sh` for anything that runs engines or test suites.

## Log
- 2026-11-01 devflow: rebased the branch onto main (it was cut before the issue commit).
  Measured in this worktree with its own target dirs (`target/`, and
  `work/scratch/0047/tgt` for runs under a logging `RUSTC_WRAPPER`). Tools in
  `work/scratch/0047/`: `timed-gate.sh` (gate.sh with every step under `/usr/bin/time`),
  `rustc-log.sh` (logs crate, kind, target, opt-level, debuginfo, seconds per rustc call),
  `sample.sh` + `agg2.py` (a `ps` sampler every 3 s over ~35 min; CPU per process from
  its cumulative cputime). Raw data: `*.tsv`, `unit-times.txt`, `e2e-gc-cli.txt`.
- Noise: load average swung between 5 and 201 during the session (other agents plus a
  second project's `node --test` run with 17 workers, Chrome, VS Code). The same gate step
  varied up to 4x in wall time (test262 3.4 s quiet, 18 s loaded) and CPU seconds inflate
  under contention too. Numbers below say "quiet" (load < 10) or "loaded" where it
  matters; where I repeated a measurement I give the range. M5 Max, 18 cores (6 + 12).
- Found and reported during the session: a 51-minute orphaned `e2e` test binary at 100 %
  CPU in 0033's worktree (started without timeout.sh; the coordinator has since made
  timeout.sh kill the whole process group).

## Findings

### F1. The gate is ~85 s / ~230 CPU-s quiet; >75 % of it is test execution, not compiling
Per step, wall s / CPU s (user+sys), this worktree:

| step | cold | warm (no change) | after editing interp/run.rs (loaded) |
|---|---|---|---|
| fmt | 0.5 / 0.4 | 0.3 / 0.3 | 0.3 / 0.3 |
| clippy (native, all targets) | 9.3 / 25 | 0.1 / 0.1 | 4.2 / 9.4 |
| cargo test --workspace | 62 / 154 | 46–60 / 76–80 | 56 / 115 |
| wasm32 build djs-core | 6.4 / 15.6 | 0.0 | 3.5 / 7.1 |
| clippy wasm32 release | 4.1 / 6.5 | 0.1 | 3.1 / 5.0 |
| bgen --check | 0.7 | 0.3–0.5 | 0.4 |
| djs-wasm build | 7.7 / 24.5 | 0.0–1.4 | 7.1 / 14.0 |
| node tests, wire/reader cross | 1.3 / 2.6 | 1.0 / 2.0 | 1.7 / 2.1 |
| **difftest (+suspend, +gc-stress)** | 23 / 115 | 22–24 / 102–118 | 27 / 93 |
| test262 active (debug) | 5.3 / 45 | 3.5–5.3 / 31–45 | 18 / 71 |
| **total** | **121–141 / 388–405** | **84–89 / 224–246** | **122 / 319** |

On a warm cache the compile share is ~25 CPU-s; difftest (~100 CPU-s), the djs-core test
binaries (~85 CPU-s) and test262 (~35 CPU-s) are the rest. D010 recorded the whole gate
at ~46 s, and the e2e test at ~3.4 s; both have grown since.

### F2. difftest: ~85 % of its gate cost is cases that hit the 10 s timeout under stress
Plain run of all 191 cases: 0.3–1.0 s. With `--suspend-every 97 --gc-stress`: 22 s wall,
87–117 CPU-s. The stress modes time out (10 s each) on 5–11 case/mode pairs, depending on
load: `builtins/array-methods.js`, `builtins/base64.js`, `builtins/proxy.js`,
`dquickts/builtin/test_map.js`, `dquickts/builtin/test_rope.js`, `errors/other-errors.js`.
Timed-out pairs are "not compared", so this CPU checks nothing, and the set of compared
cases changes with machine load. Their real cost, release build, one at a time:
gc-stress array-methods 8.8 s, base64 9.6 s, test_map 27 s, test_rope >120 s,
other-errors >120 s; suspend test_map 56 s, other-errors 33 s, test_rope >60 s.
The same run with those 6 cases left out: **3.7–8.8 s wall, 20–22 CPU-s** (loaded),
nothing timed out. Saving: **~70–95 CPU-s and ~13–18 s wall per gate**.

### F3. The e2e test is one serial `#[test]`: 33–53 s wall, the critical path of `cargo test`
`cargo test` runs test binaries one after another; `e2e` is a single `#[test]` that loops
over 52 cases × 3 modes on one thread. Per mode (quiet): plain 0.4 s, suspend 7.7 s,
gc-stress 25 s. In gc-stress mode one case, `iteration_runtime.js` (its "big spread" of
2000 objects across GC safepoints), is ~10.6 s of ~24.5 s (CLI measurement); next are
collections 3.2 s, json 2.4 s, proxies 1.9 s. e2e is ~75 % of `cargo test --workspace`
wall time and ~40 % of the gate's. It also has no fuel or time limit per case, so an
infinite loop in a case hangs until the outer timeout (the 51-minute hang above).

### F4. djs-core unit tests: two tests are a third of the CPU
djs_core lib tests: 7.4 s wall, 38 CPU-s (quiet). Slowest single tests (quiet):
`vm::tests::async_generators_queue_requests` 9.6 CPU-s,
`vm::tests::for_await_over_sync_async_and_generator_iterables` 7.8 CPU-s, then
async_functions_await_through_jobs 2.6, fixed_precision_exponential… 2.3,
function_kinds_have_their_prototypes 1.9. Both top tests go through
`run_with_suspendables`, which runs fuel 1 (an image round trip at **every** instruction)
for the whole program; e2e already caps that at the first 1000 stops, then fuel 997.

### F5. Compilation: djs-core is compiled 9 times per gate, but incremental makes it cheap
rustc calls per gate (logging wrapper), djs-core units:

| # | step | unit | cold s | incr s (run.rs edit) |
|---|---|---|---|---|
| 1 | clippy | check lib (host dev) | 3.4 | – |
| 2 | clippy | **full build** of lib as djs-prelude's build-dep (check can't run build.rs) | 6.4 | 2.5 |
| 3 | clippy | check lib --test | 2.7 | 0.7 |
| 4 | test | lib (host dev; shared by build-dep and normal dep) | 7.2 | 2.7 |
| 5 | test | lib --test (unit tests) | 12.0 | 4.4 |
| 6 | wasm-core / djs-wasm | wasm32 dev lib (reused by step djs-wasm) | 5.4 | 3.2 |
| 7 | clippy wasm | check wasm32 release (opt 3) | 2.2 | 1.6 |
| 8 | clippy wasm | host build-dep for release (opt 0, debuginfo 1) | 2.8 | 2.1 |
| 9 | djs-wasm | host build-dep again (opt 1, debuginfo 2; not shared with #4) | 6.9 | 5.7 |

Sum of rustc wall seconds per gate: cold 76 s (djs-core 49 s), incremental 20–34 s
(djs-core 10–23 s). Units 2, 8 and 9 exist only because djs-prelude's build.rs needs a
host djs-core in each profile/target: ~10 s of rustc per incremental gate. Not a big lever.

The iteration loop: after one edit in djs-core, `cargo test -p djs-core --no-run` rebuilds
djs-core twice (lib, needed by djs-prelude/e2e, and lib --test), in parallel: quiet 3–7 s
wall / 9–26 CPU-s, loaded 23–33 s wall / 39–48 CPU-s. Editing a `*tests.rs` file rebuilds
only the test harness: 1.2 CPU-s (D1330's exclusion works). A release rebuild of
difftest+test262 after the same edit: 4.3 s wall / 22 CPU-s quiet (djs-core opt 3, plus a
host opt-0 djs-core for build.rs), vs 3.3 s / 3.3 CPU-s for dev.

### F6. The D1330 source hash costs nothing extra
The base snapshot (djs-prelude build.rs) reruns because djs-core, its build-dependency,
was rebuilt; cargo does that with or without SOURCE_HASH. Running the build script takes
0.01–0.04 s (debug and release), the djs-prelude crate 0.1 s. Narrowing the hash would
save nothing measurable.

### F7. Release builds of test262 / difftest buy little because djs-core is opt 1 in dev
test262 active group, repeated, quiet: dev 3.4–4.0 s wall / 35–39 CPU-s; release
2.5–2.8 s / 27–28 CPU-s. difftest plain: dev 0.3 s, release 0.2 s. But keeping a release
build current costs ~22 CPU-s per djs-core edit, a second djs-core in the incremental-less
release profile, and 150–280 MB per worktree. `debug = 1` in `[profile.release]` doesn't
measurably slow builds (44–47 vs 42–45 CPU-s cold); it costs 45 MB of disk.

### F8. Where the machine's CPU went (35-minute `ps` sample)
Busy cores per 3 s sample, djs agents (not me) vs everything else:

| | mean | p50 | p90 | max |
|---|---|---|---|---|
| djs agents | 3.7 | 1.7 | 11.5 | 16.4 |
| everything else (another project's node tests, Chrome, VS Code, biome…) | 3.9 | 2.6 | 8.8 | 16.9 |

The djs load is bursty: every runner (test262, difftest, libtest, cargo) uses all 18
cores, so one agent's test262 run takes the whole machine for its duration. All p90+
bursts were a `djs-test262` or `djs-difftest` process at 1200–1500 % CPU. The biggest djs
consumer in the window was agents' **release test262 runs**: 10 processes, 1836 CPU-s,
more than all rustc processes of all agents together (~1250 CPU-s). Several were wide:
`run --engine djs` with no filter (all groups: 307–316 CPU-s, 37–64 s each), RegExp +
String trees (167–269 CPU-s each), and repeated `--engine qjs` runs of the same groups
(the qjs children aren't in these numbers; the active group under qjs measured 10 s wall,
118 CPU-s quiet). qjs results only change when qjs or the test262 pin changes. One release
active-group run took 265 CPU-s / 24 s under load, vs 27 CPU-s quiet.

Merges happen 3–13 times per hour (reflog), so gates add roughly 0.3–1 core on average,
in bursts that take all 18 cores. Most of the remaining djs CPU is agents' own iteration.

### F9. Disk and cold builds
Per worktree `target/`: 0.6–2.7 GB (main checkout 2.6 GB); `incremental/` is 60–75 % of
it. One gate from cold: 1.1 GB. A new worktree pays a cold build of every profile it uses:
~150 CPU-s for the gate's set (`cargo test` ~77, clippy ~25, wasm ~50) plus ~45 for
release. Seeding a new worktree's target with an APFS clone (`cp -c -R`, 2 s, no extra
disk) saved only 15 % (64 vs 77 CPU-s): crate hashes match across worktrees, but the
incremental caches are tied to absolute paths.

### F10. A shared CARGO_TARGET_DIR is harmful here
Unit hashes for workspace crates are identical across worktrees (`libdjs_core-558d…` shows
up in four of them): cargo hashes the workspace-relative path. In one shared target dir,
all worktrees would use the same djs-core artifacts and incremental caches with different
sources. That means constant rebuilds back and forth, and, since freshness is decided by
mtimes, a risk of linking another branch's djs-core. On top, one build lock would
serialize five agents. The only real sharing would be `libm` (3.4 rustc-s). sccache isn't
installed, and it skips incremental workspace crates anyway. **Don't do it.**

## Proposals
Ranked by expected saving per unit of risk. "Gate" savings are per merge (3–13/h); the
iteration savings repeat many times per agent per hour.

1. **Iteration habits: a "how to test while iterating" section in AGENTS.md** (text below).
   Main lever on the agent CPU in F8: no release builds for test262/difftest, filtered
   runs, no qjs re-runs, no manual gate before merge-to-main. Expected: the release test262
   share (~1.5 cores average in the sample) mostly goes away, plus ~22 CPU-s per edit for
   not keeping release current. Risk: none for correctness; benchmarks still use release.
2. **difftest: skip the stress modes for cases that can't finish** (F2), with case
   metadata such as `// difftest: no-stress` (or `no-suspend` / `no-gc-stress`) on the 6
   cases, and listing them as skipped in the stress line. Saving: ~70–95 CPU-s and
   ~13–18 s wall per gate, and a deterministic set of compared cases. Risk: those 6 cases
   lose stress coverage they don't have today anyway (they time out). Better long term: make
   suspend/gc-stress cheaper on big heaps (engine work, separate issue).
3. **e2e: parallel and bounded** (F3). Run cases on `available_parallelism()` threads
   (`std::thread::scope`, collect failures), or at least one `#[test]` per mode. Shrink the
   gc-stress cost of `iteration_runtime.js` (fewer than 2000 objects still crosses many
   safepoints), and give each case a fuel cap so a hang fails in seconds. Expected:
   e2e from 33–53 s to ~11 s (bounded by iteration_runtime), ~3–5 s with the case shrunk;
   gate and `cargo test` wall time −30…45 s. CPU roughly −15 s (the shrunk case). Risk: low;
   the output check doesn't change.
4. **Unit tests: cap fuel-1 suspension in `run_with_suspendables`** as e2e does (first
   1000 stops at fuel 1, then 997) (F4). Saving: ~15 CPU-s per djs-core test run, unit
   binary wall 7.4 s → ~3 s. Risk: low; it still gets thousands of image round trips.
5. **Cap per-agent parallelism, keep the gate at full width.** Agents export
   `CARGO_BUILD_JOBS=6` and `RUST_TEST_THREADS=6`, and pass `-j 6` to djs-test262 /
   djs-difftest (or a `DJS_JOBS` env both runners read), optionally under
   `nice -n 10`. The gate (merge-to-main, holds the lock, on the
   critical path) keeps full width. Saving: no CPU-s, but bursts drop from 18 to 6 cores per
   agent, and wall-clock timeouts (difftest 10 s, test262 20 s) stop firing because of
   contention. Risk: single-agent iteration gets slower (test262 active ~4 s → ~10 s).
6. **Cache qjs oracle results** (F8). `djs-test262 run --engine qjs` and difftest's qjs
   side are deterministic for a given qjs binary + test262 pin. Store per-test results
   in the primary checkout's `vendor/` (keyed by qjs binary hash + test262 commit) and use
   them unless `--fresh`. Saving: ~120 CPU-s per qjs active-group run, more for wider
   groups. Risk: low (key mismatch → rerun).
7. **Reuse worktrees** (F9): a fixed pool of agent slots (`.worktrees/slot-N`) where the
   coordinator does `git switch -c issue/NNNN-slug main` instead of `worktree add/remove`.
   The target dir and incremental caches survive. Saving: ~150–200 CPU-s and 1–2 min
   wall per new issue, and stable disk (no 1–2.7 GB churn per issue). Risk: process change
   for the coordinator; a slot must be clean before switching (merge-to-main already
   requires that).
8. **Small gate trims (optional).** Drop the separate `wasm-core` step (djs-wasm builds the
   same wasm32 djs-core; 0 CPU saved, one fewer step). Leave clippy `--all-targets`, the wasm
   release clippy and the build.rs host builds as they are: together ~10–15 rustc-s per
   incremental gate, and they catch real things.

Not recommended:
- Shared `CARGO_TARGET_DIR` / sccache (F10).
- Narrowing the D1330 hash (F6): saves nothing.
- Splitting the gate into fast/slow tiers. After 2–4 the gate would be ~35–45 s wall /
  ~110 CPU-s quiet, and test262 active is only 3.5–5 s. Splitting would let regressions
  into main for little gain. Revisit if the gate goes back above ~2 min quiet.
- Release test262/difftest in the gate: saves ~1 s and ~10 CPU-s but costs a release
  rebuild of djs-core (~22 CPU-s) on every merge.
- `debug = 0` for release, or other debuginfo changes: no measurable compile difference.

### Draft for AGENTS.md: "Testing while iterating"
```
## Testing while iterating

The gate runs once, inside scripts/merge-to-main.sh. Don't run scripts/gate.sh yourself
before merging; while iterating, run only what your change can break:

- Unit tests of one area: `cargo test -p djs-core --lib compiler::` (2–3 s).
- e2e: `DJS_E2E_MODE=plain cargo test -p djs-core --test e2e`; the stress modes once
  before merging.
- test262: dev build, filtered: `cargo run -q -p djs-test262 -- run --engine djs
  built-ins/RegExp`. djs-core is opt-level 1 in dev; the active group takes ~4 s in dev vs
  ~3 s in release, and a release rebuild costs more than that. Don't run the unfiltered
  default (all groups, ~300 CPU-s) to check a change.
- difftest: `cargo run -q -p djs-difftest -- --engine djs <filter>`; add the stress flags
  for the cases you touched.
- qjs results don't change: read tests/test262/oracle-failures.txt and the corpus
  .expected files instead of re-running qjs.
- Use --release only for benchmarks (scripts/bench.sh) and timing work.
- Leave cores for the others: `export CARGO_BUILD_JOBS=6 RUST_TEST_THREADS=6`, `-j 6`
  for the runners. Everything that runs an engine goes under scripts/timeout.sh.
```

### Recommended order
1. AGENTS.md section + per-agent job caps (1, 5): no code, immediate; addresses the
   biggest measured consumer (agents' wide release test262 runs) and the 18-core bursts.
2. difftest no-stress metadata for the 6 cases (2): small change in djs-difftest + 6
   corpus headers; −70–95 CPU-s and −15 s per gate.
3. e2e parallel + fuel cap + shrink iteration_runtime (3) and the unit-test fuel cap
   (4): one small djs-core test issue; −30–45 s wall per gate and per `cargo test`,
   −30 CPU-s per run, and no more silent e2e hangs.
4. qjs result cache (6) and worktree slots (7) when convenient.

Together (2–4): gate ~85 s / ~230 CPU-s → ~35–45 s / ~110 CPU-s quiet; `cargo test
--workspace` ~46 s → ~12 s.

## Result
Investigation only; no code, script or Cargo.toml changes. Findings and proposals are
above; measurement tools and raw data are in `work/scratch/0047/` (gitignored).
