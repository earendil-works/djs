# Performance anatomy tools (0081)

Tools for counting where djs spends machine instructions, natively and in wasm (D016:
wasm32 is the primary target). Instruction counts (`/usr/bin/time -l`: "instructions
retired", per process, all threads) repeat to <0.5% even on a loaded machine; cycles
and wall time do not, so compare instructions first. Everything is macOS/arm64 and uses
only what is installed (samply, node, wasm-objdump).

Builds and outputs go to `$DJS_PERF_DIR` (default `work/scratch/perf`, gitignored).
Run `scripts/perf/setup.sh` first. Workloads are `scripts/perf/w/*.js` (codemode bodies).
qjs is `~/Development/dquickts/qjs`; quickjs-wasi (pi's engine) is imported read-only
from `~/Development/pi-private/node_modules/quickjs-wasi`.

| tool | what |
| --- | --- |
| `measure.py [--djs BIN]... names` | native djs vs qjs per workload (instructions, cycles, peak memory) |
| `wmeasure.py [--wasm X[@SRCROOT]]... [--node-flag=--no-liftoff] names` | djs-wasm vs quickjs-wasi in node, minus an empty run; the counts include V8's TurboFan tier-up of each module (~0.5 G for djs, ~0.3 G for quickjs-wasi; TESTING.md "Instruction ratios to quickjs-wasi"). `X.wasm@SRCROOT` runs a build with its own checkout's `wasm-run.ts` and JS heap reader (a build with another heap layout fails with LayoutMismatch otherwise). A failed run (an exception, a LayoutMismatch, no timing line) stops the measurement with its error; measure.py also stops on a failed run. 0159 measured twins under node's default tiering: `--no-liftoff` lowers the noise but hides costs that only Liftoff code shows, and it makes V8 compile another `run_hot` (0268): `--node-flag=--wasm-sync-tier-up` keeps the shipped code and makes the tier-up deterministic |
| `micro.py` / `wmicro.py [--wasm A --wasm B]` | instructions per JS-level operation (loop body x10 minus x0), natively vs qjs and in wasm vs quickjs-wasi; the cases are in `cases.py`; several `--wasm` builds compare side by side (A/B) |
| `wasm-compile.ts`, `wcompile.py [--qjs] --wasm X... files` | compile-only time (MB/s) and instructions per compile and per source byte in the wasm build, vs quickjs-wasi (0092) |
| `mem-perturb.sh [-n N] [--root DIR] WASM names` | peak wasm memory as a distribution over N perturbed runs (D2048): use it for every memory claim, since one run shows allocator placement, not policy |
| `slope.py [-n N] [--qjs] [--per K] --wasm A [--wasm B]... [names]` | steady-state instructions per loop iteration in the wasm build: each case of `s/` (a body with the bound `N_ITER`) at N and 2N iterations, difference / N, under `--wasm-sync-tier-up`. Start-up, compiling and the Liftoff phase cancel; two identical builds agree to ~0.05% at N = 10M, so it resolves a single machine instruction per bytecode (0268). `--qjs` adds quickjs-wasi as a column with each build's ratio to it, `--qjs2` Bellard's QuickJS as the engine comparison builds it (`vendor/engines/build/quickjs.wasm`, the fastest of the QuickJS builds: the one the ops table's ratios are against); `--per K` gives instructions per operation for bodies written K times, minus the first case (an empty body) |
| `hotcode.sh NAME [workload]`, `v8arm.py code.txt [Op...]`, `v8arm.py code.txt --paths Op [SIG]` | what V8 makes of `run_hot`: a names build, V8's TurboFan code of the function under default tiering, and per opcode the path from the jump table back to the loop header (dispatch block, shared tail, back-edge moves, calls and stack traffic per arm). Run it before and after every change to `run_hot` and diff the summaries: an edit that changes what is live around the loop shows up as moves or stack loads on the path every bytecode shares (0268, D3391). `--paths` lists every call-free path of one arm with its length: the cost of a data-dependent arm's hits, which the one static path does not show (0286: `get_field`'s own-property hit) |
| `wasm-run.ts`, `qjs-wasm-run.mjs`, `qjs-built-run.mjs` | run a workload in the wasm engines (`ONCE=1`, `FUEL`, `REPS`); `node --cpu-prof` on them profiles |
| `prof.py name` | samply profile of the native build: self time by inlined function and line, buckets, malloc/memcpy by caller |
| `instr.py [--self-test]` | builds an instrumented copy of the engine (in `$DJS_PERF_DIR/instr`): `DJS_PROF=1 djs run --body x.js` prints bytecodes per opcode and function, split into user code and bytecode builtins and into `run_hot` and `step`; a time split from a 50 µs sampling thread (user bytecode, bytecode builtins with the leaf intrinsics they call, `@native` builtins, collections); intrinsic calls, native state arms, frames, allocations, collections. Builtins are named by their registry paths. Patches are anchored on exact source text and the script fails, listing each anchor that moved, when one does not apply; `--self-test` also runs a script that must move every counter (0155). The split is of the native build. A Rust leaf intrinsic counts for the kind of the bytecode that called it, and a `@fast` entry for its caller's (usually user code). Function rows count "entries" at pc 0, so a loop whose head is a function's first instruction counts its iterations there too |
| `model/` | a standalone dispatch-cost model of the run loop in three styles (`cargo run --release -- a|b|c N`) |

Instruction counts of node include V8's background compilers; for ops of a few hundred
instructions, single runs of wmicro vary by up to ~10% with tier-up timing, so judge
small per-op changes with slope.py (steady state) and whole workloads (wmeasure.py,
n ≥ 3). Do not judge them under `--no-liftoff`: V8 then compiles another `run_hot`
(TESTING.md "Measuring the tier-0 loop").
