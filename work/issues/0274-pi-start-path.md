# 0274: pi's cached start: image-cache key, restore zero-fill, instantiation

status: done
agent: pstart
branch: issue/0274-pi-start-path
depends-on: -
touches: js/pi (engine.ts, vm.ts: the prelude image cache and its key, instance reuse), js/src (only what pi's start path calls: image restore), crates/djs-wasm (restore's zero-fill only), tests/pi, js/test
guide: pi.md
decisions: D3450-D3459

## Goal
pstart measured pi end to end (0271's log): a cached start is 0.15 ms, of which
instantiation 18%, the image-cache key 16% (`preludeKeyOf` JSON-stringifies the whole
prelude on every start because pi passes a new `data` each run and the WeakMap misses),
the restore's zero-fill 10.6%, the engine's own work under 10%.
1. Key the prelude image cache on a per-prelude hash (or the prelude's identity plus a
   cheap content hash), so a start doesn't stringify the prelude.
2. The restore's zero-fill: zero only what the image doesn't overwrite (or what an earlier
   session dirtied, with D3023's instance reuse), measured.
3. Instantiation in pi's path: check D3023's instance reuse hits on the measured start; if
   not, why not.
Dumps at idle points (dump 46%, load 43% of ~7.5 ms) are out of scope: the owner paused
dump performance work (0215); list findings as follow-ups for the owner.

## Acceptance criteria
- [x] cached start time and instructions, main vs branch (js/bench/pi.bench.ts, the pi tests)
- [x] pi's tests pass in all modes; no behavior change for pi
- [x] gate green; decisions recorded; pi.md updated if the start story changes

## Shared context (all of 0256-0260)
The owner asked to push both memory and speed as far as they go (2026-11-21). Background,
fresh profiles and the ranked idea list: work/scratch/perfreview/ of the primary checkout
(BRIEF.md, IDEAS.md; prof.ts and cat.py profile the codemode workloads in wasm). Read
IDEAS.md's section for your issue first; its numbers are estimates, not measurements.

Five issues run in parallel (0256 per-VM/per-byte costs, 0257 idle memory, 0258 GC,
0259 builtin calls, 0260 byte kernels). Stay in your `touches:`; announced overlaps are in
each issue's Notes.

Measuring (D016: wasm32 under node is the yardstick):
- run time and memory per workload: build the shipped module in your worktree
  (`scripts/build-wasm.sh`, or `--keep-names` for profiles) and run
  `DJS_WASM=$PWD/target/wasm32-unknown-unknown/wasm/djs_wasm.wasm node js/bench/engines/run.ts --engines djs --reps 25`
  before and after, on main's build and yours, at load below ~4 (`uptime`; say the load).
  Memory claims: scripts/perf/mem-perturb.sh (D2048).
- instructions: scripts/perf/wmeasure.py / wmicro.py (TESTING.md "Instruction ratios").
- Report per workload: ms and MiB, main vs branch, and the module's gzip bytes.

Size: the shipped module is ~1.3 KB gzip under the D020 budget (ceilings, D3266). Report
the bytes of every change; find offsets where you can. If a measured win needs more room
than that, stop and tell the coordinator with the numbers: do not raise ceilings yourself.

Never weaken determinism (collection points a function of the program, D1710), suspension
anywhere, images, or the unsafe rules (AGENTS.md).

## Log
- Harness: work/scratch/0274/st.ts (N cached starts of `return 1` through js/pi/vm.ts with
  pi-private's PRELUDE_SOURCE and two tools), ins.sh (whole-process instructions, N=2000 minus
  N=0, min of 3, --wasm-sync-tier-up), ab.sh (interleaved wall time, min of 7).
- Profile of a cached start on main (names build, 50 µs sampling): preludeKeyOf 33%, renew
  13.6% (zeroing the pooled instance's 1.1 MB), restore 9% (copying the image in), staging's
  `Vec::resize` 3.1%; compile and run of the body ~25%.
- D3450 (image key: prelude by number): 1,077K → 853K instructions.
- D3451 (staging not zeroed): −28K. +40 B gz (429,747 → 429,787 after rebasing onto
  e0c804e4), under the ratchet ceiling 429,790; the coordinator allowed up to +40.
- D3452 (renew skips where the image goes; the staging address is remembered per pool and
  memory size, a mismatch drops the instance): −19K instructions, 470 KB less memset.
  js/test/pi-pool.test.ts checks that no marker of an earlier session survives a pooled
  restore; it fails with the skip range left unzeroed.
- Instance reuse (item 3): D3023's pool hits on every cached start of an in-thread launcher
  (one instance for any number of sessions). The 18% "Instance" in 0271's profile was my
  harness, and js/bench/pi.bench.ts has the same bug: a script that ends before `startVm`
  resolves (`return 1`) was never disposed, so its instance never went back to the pool and
  every start made a new one. pi's launchers dispose on settle; pi.bench.ts now does too.
- Result on e0c804e4 vs branch: cached start 1,089K → 800K instructions (−26.5%); wall (load
  50-60, interleaved, min of 7): 51.5 → 30.7 µs (−40%); wall times are load-sensitive.
- Tests: js tests (dev modules), pi tests in pi, next, next with a dump at every idle point
  (dump module) and slim --no-lockdown modes: 0 new failures. Guide: pi.md's prose is
  unchanged (the start story is the same); its numbers come from status.tsv, to be recorded
  on a quiet machine (`scripts/status.py pi`), not now (load 50-120).

## Follow-ups
- pi's default sandbox (CodemodeSandbox, pi-private runtime/host.ts) runs every script in a
  new worker, which starts with an empty module-level image cache and pool: every
  worker-launched script runs the prelude cold (~4 ms of djs work) plus worker startup. The
  cache only helps the inline and remote launchers. A cross-worker image needs a shared
  buffer the host passes: e.g. djs's `loadQuickJSWasm` (the swapped src/wasm.ts, pi's
  `CodemodeWasmModule` is an opaque `object`) could return `{ module, cache:
  SharedArrayBuffer }`, which structured clone carries into every worker, and the adapter
  could keep the prelude image there. Measured per worker execute: 136-260 ms wall under
  load 25-60 (mostly node's worker startup and module loading; TypeScript stripping in the
  test setup is 19% of it, the published package ships .js).
- Dumps at every idle point (owner-paused, 0271's profile): ~7.5 ms per idle point, writing
  the dump 46% (`Dumper::finish`: plain::write 18%, react_upto 12%), loading it 43% (compile
  32%, parse 18%), scaling with the result strings the session holds.
- What is left of a cached start (~800K instructions): compiling and running the body and
  pi's wrapper (~25%), copying the image in (~14%), zeroing the rest of the pooled
  instance's memory (~11%), JS host work (Encoder, outWords, setClock, Reader).

## Result
A cached pi start (in-thread launcher, prelude image reused) costs 26.5% fewer instructions:
1,089K → 800K on e0c804e4, ~51 → ~31 µs wall under load. D3450: the image cache key names
the prelude by a number instead of stringifying it. D3451: the image's staging buffer is
not zeroed before the host writes it (+40 B gz). D3452: a pooled instance's renew skips the
bytes the image overwrites. The pool hits on every cached start; pi.bench.ts now disposes
early-ending sessions as pi does. New test: js/test/pi-pool.test.ts (pool hygiene).
Known gap: pi's default worker-per-script sandbox never reaches the cache (follow-up).
