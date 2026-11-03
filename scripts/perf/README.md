# Performance anatomy tools (0081)

Tools for counting where djs spends machine instructions, natively and in wasm (D016:
wasm32 is the primary target). Instruction counts (`/usr/bin/time -l`: "instructions
retired", per process, all threads) repeat to <0.5% even on a loaded machine; cycles
and wall time do not, so compare instructions first. Everything is macOS/arm64 and uses
only what is installed (lldb, samply, node, llvm-objdump from Xcode, wasm-objdump).

Builds and outputs go to `$DJS_PERF_DIR` (default `work/scratch/perf`, gitignored).
Run `scripts/perf/setup.sh` first. Workloads are `scripts/perf/w/*.js` (codemode bodies).
qjs is `~/Development/dquickts/qjs`; quickjs-wasi (pi's engine) is imported read-only
from `~/Development/pi-private/node_modules/quickjs-wasi`.

| tool | what |
| --- | --- |
| `measure.py [--djs BIN]... names` | native djs vs qjs per workload (instructions, cycles, peak memory) |
| `wmeasure.py [--wasm X[@SRCROOT]]... [--node-flag=--no-liftoff] names` | djs-wasm vs quickjs-wasi in node, minus an empty run; the counts include V8's TurboFan tier-up of each module (~0.5 G for djs, ~0.3 G for quickjs-wasi; TESTING.md "Instruction ratios to quickjs-wasi"). `X.wasm@SRCROOT` runs a build with its own checkout's `wasm-run.ts` and JS heap reader (a build with another heap layout fails with LayoutMismatch otherwise). A failed run (an exception, a LayoutMismatch, no timing line) stops the measurement with its error; measure.py and perelem.py also stop on a failed run. 0159 measured twins under node's default tiering: `--no-liftoff` lowers the noise but hides costs that only Liftoff code shows |
| `micro.py` / `wmicro.py [--wasm A --wasm B]` | instructions per JS-level operation (loop body x10 minus x0), natively vs qjs and in wasm vs quickjs-wasi; the cases are in `cases.py`; several `--wasm` builds compare side by side (A/B) |
| `wasm-compile.ts`, `wcompile.py [--qjs] --wasm X... files` | compile-only time (MB/s) and instructions per compile and per source byte in the wasm build, vs quickjs-wasi (0092) |
| `perelem.py [--djs B].. [--wasm W].. [--noqjs] [-r R] [names]` | instructions per array element of the iteration builtins, for native and wasm builds side by side (0100) |
| `mem-perturb.sh [-n N] [--root DIR] WASM names` | peak wasm memory as a distribution over N perturbed runs (D2048): use it for every memory claim, since one run shows allocator placement, not policy |
| `wasm-run.ts`, `qjs-wasm-run.mjs` | run a workload in the wasm engines (`ONCE=1`, `FUEL`, `REPS`); `node --cpu-prof` on them profiles |
| `cpuprof.py file.cpuprofile` | self time by function of a node CPU profile (wasm function names) |
| `prof.py name` | samply profile of the native build: self time by inlined function and line, buckets, malloc/memcpy by caller |
| `instr.py [--self-test]` | builds an instrumented copy of the engine (in `$DJS_PERF_DIR/instr`): `DJS_PROF=1 djs run --body x.js` prints bytecodes per opcode and function, split into user code and bytecode builtins and into `run_hot` and `step`; a time split from a 50 µs sampling thread (user bytecode, bytecode builtins with the leaf intrinsics they call, `@native` builtins, collections); intrinsic calls, native state arms, frames, allocations, collections. Builtins are named by their registry paths. Patches are anchored on exact source text and the script fails, listing each anchor that moved, when one does not apply; `--self-test` also runs a script that must move every counter (0155). The split is of the native build. A Rust leaf intrinsic counts for the kind of the bytecode that called it, and a `@fast` entry for its caller's (usually user code). Function rows count "entries" at pc 0, so a loop whose head is a function's first instruction counts its iterations there too |
| `trace.sh case.js skip n out` + `trace_report.py out [--show OP]` | exact instruction trace through lldb (300 steps/s), split per bytecode: instructions, loads, stores, branches and categories per op; `BYFUNC=op` breaks one op down by function; `libsym.sh` names system-library code in a trace |
| `wasm_arm.py func.dis table entry` | reads one `br_table` arm of a wasm function (e.g. a `fast_step` arm of `run_loop`) |
| `nobounds.py` | experiment copy with unchecked arena accesses (native + wasm), to measure the checks |
| `model/` | a standalone dispatch-cost model of the run loop in three styles (`cargo run --release -- a|b|c N`) |

Build-specific constants: `trace.py`'s `TRACE_BOUNDARY` is the address of the `b.lt`
after the run loop's fuel check (`if self.fuel <= 0`, run.rs `run_loop`), and it reads
`Exec.pc`/`code`/`code_len` at offsets 0x28/0x30/0x34 and the arena Vec pointer at
heap+8. Find them in `llvm-objdump -d -l --dsym=$DJS_PERF_DIR/djs.dSYM` of `run_loop`
after a rebuild (the fast_step entry loads pc and code_len right after the check).

Instruction counts of node include V8's background compilers; for ops of a few hundred
instructions, single runs of wmicro vary by up to ~10% with tier-up timing, so judge
small per-op changes with whole workloads (wmeasure.py, n ≥ 3).
