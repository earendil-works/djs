# djs

djs is a small JavaScript and TypeScript engine written in `no_std` Rust. It is built
mainly for wasm32, where it replaces QuickJS in pi's codemode: model-written scripts that
call tools, process their results and print or return data. Its design follows from that
use (ARCH.md):

- **All VM state is one byte arena addressed by `u32` offsets**: objects, strings,
  bytecode, frames, the job queue, pending host calls. Suspending a VM inside a process
  is free; creating one is a copy of a prebuilt image.
- **Suspend anywhere.** The interpreter never recurses on the Rust stack for JS calls.
  It can stop between any two instructions (fuel, a host call) and resume,
  also inside getters, callbacks of builtins, promise jobs, generators and async
  functions.
- **Cheap data passing.** Values go in through a compact wire format and come out through
  direct reads of the heap from the host, with no serialization step in the VM.
- **A compiler without an AST** (single pass to stack bytecode, like QuickJS), with
  TypeScript erased while parsing.
- **Builtins in a strict TypeScript subset**, compiled to bytecode for the base snapshot
  and, for the hot ones, to resumable Rust state machines.
- **Persistence as code**: a run waiting on its tools is written out as a continuation
  dump, a plain JavaScript program that recreates it, which any later djs (or node)
  runs (docs/continuation-dumps.md).
- **Deterministic by construction**: the clock, randomness and weak references come from
  the host, and the same inputs give byte-identical heap images, natively and in wasm.

The behavior reference and test oracle is dquickts (a QuickJS fork), including its error
messages.

## Status

The numbers below are generated (`djs-docs regen`, D2101): counts from the repository
(test262 grouping and ratchets, the corpus) on every merge, measurements from
`tests/scores/status.tsv`, which `scripts/status.py` records with the commit they were
measured at (`scripts/status.py` without arguments lists what it measures and how; the
guide's status page lists every record). Figures inside sentences are generated too
(inline `n` markers, D3070).

**Conformance.**

<!-- gen:status conformance -->
- test262 (at dquickts' pinned commit): djs passes **38,069 of the 38,109** tests in the
  active group (99.9%; 53,597 tests in all, the rest skipped by design). The 40 known
  failures are listed with reasons in `tests/test262/expected-failures.txt`; dquickts'
  qjs fails 4 active tests under the same runner.
- The differential corpus (codemode scripts, console and result formatting, error
  messages, TypeScript, dquickts' own tests): **621 of 632** cases match qjs; 6 known
  failures (`tests/corpus/expected-failures.txt`) and 5 skipped as non-goals.
- pi's codemode test suite on djs: 221 of 226 tests pass in the default mode, in the
  next mode and with a dump at every idle point (measured at 7167411f0f, 2026-11-05).
<!-- /gen -->

Skipped by design: Intl, Temporal, SharedArrayBuffer/Atomics, Annex B, sloppy mode,
direct eval, modules. Most known failures follow from djs being always strict (TESTING.md
"Known differences"); pi's failures from that, or from tests that expect the quickjs-wasi
fingerprint.

**Speed in wasm** (node, V8's wasm tier). djs-wasm vs quickjs-wasi (pi's engine), from
`scripts/perf/wmeasure.py`:

<!-- gen:status speed -->
| workload | djs-wasm | quickjs-wasi | ratio |
| --- | ---: | ---: | ---: |
| tight loop | 5.8G | 15.3G | 0.38 |
| property access (prop1) | 2.4G | 5.4G | 0.45 |
| strings (split, slice, join, ...) | 2.2G | 5.8G | 0.38 |
| arrays | 9.5G | 16.3G | 0.58 |
| JSON round trips | 1.2G | 2.0G | 0.60 |
| JSON with Dates | 0.9G | 1.7G | 0.52 |
| for-of | 5.1G | 6.9G | 0.73 |
| calls | 2.9G | 3.9G | 0.75 |
| push/pop and string methods | 3.8G | 2.9G | 1.33 |
| a pi-like script | 5.8G | 6.9G | 0.84 |
| forEach | 5.4G | 5.8G | 0.93 |
| map/filter/reduce (higher-order) | 7.8G | 4.5G | 1.73 |

Machine instructions per workload (`scripts/perf/w/`), measured at 7167411f0f,
2026-11-05; lower is better.
<!-- /gen -->

pi's own benchmark (`js/bench/pi.bench.ts`, through pi's adapter, `scripts/status.py pi`):
a tool-heavy script with 61 calls takes <!--n pi/bench/tool-heavy/djs .1-->2.4<!--/n--> ms on djs (reusing the prelude image)
vs <!--n pi/bench/tool-heavy/qjs .1-->4.5<!--/n--> ms on quickjs-wasi, a script start <!--n pi/bench/start/djs .2-->0.14<!--/n--> ms vs <!--n pi/bench/start/qjs .1-->2.9<!--/n--> ms (<!--n pi/bench/start/djs-prelude .1-->4.9<!--/n--> ms on djs
when it runs the prelude). Callback-heavy builtin code is the main remaining gap
(work/issues/0081-perf-anatomy.md).

**Size and startup.**

<!-- gen:status size -->
`djs_wasm.wasm` is 969 KB (422 KB gzip, 356 KB brotli): 821 KB of code, 98 KB of data,
and the base snapshot (the initialized realm with all builtins, 186 KB) coded to 46 KB
in a custom section. The slim build is 843 KB (357 KB gzip); the dump module, the
shipped build with the dumper, 1,836 KB (754 KB gzip). Module sizes measured at
035457334f-dirty, 2026-11-05.
<!-- /gen -->

Creating a VM copies the base snapshot: about <!--n startup/vm-new-us .1-->2.5<!--/n--> µs in wasm (D1990). A `Sandbox`
(instance plus VM) takes about <!--n startup/sandbox-us .0-->113<!--/n--> µs, <!--n startup/sandbox-run-us .0-->142<!--/n--> µs with its first run
(`js/bench/sandbox.bench.ts`, `scripts/status.py startup`).

## Building and running

Requirements: stable Rust (edition 2024) with the `wasm32-unknown-unknown` target and the
`rust-src` component (the wasm module builds core and alloc from source), node
22.6 or newer for the JS host package (it runs TypeScript directly), and, for the
oracle-based tests, dquickts' `qjs` at `~/Development/dquickts/qjs` (or `$DJS_QJS`).

```sh
cargo build                                  # the workspace
cargo run -p djs-cli -- run file.js          # run a Script (prints console output)
cargo run -p djs-cli -- run --body file.js   # run a codemode async function body
cargo run -p djs-cli -- run --ts file.ts     # TypeScript
cargo run -p djs-cli -- repl                 # REPL with fuel, suspension and heap commands
cargo run -p djs-cli -- disasm file.js       # bytecode listing
cargo run -p djs-cli -- strip-types file.ts  # the source with its types blanked
scripts/build-wasm.sh                        # target/wasm32-unknown-unknown/wasm/djs_wasm.wasm
```

`djs run` also takes `--gc-stress`, `--suspend-every N` and `--backend bytecode|native`
(docs/cli.md).

From JavaScript (`js/`, package `@djs/host`, docs/host.md):

```ts
import { loadModule, Sandbox } from "@djs/host";

const sb = await Sandbox.create({
  asyncFunctions: { search: (q) => mySearch(q as string) },
  memoryLimit: 64 << 20,
  frozenBuiltins: true,                           // dumps need them (D2836)
  dumpModule: await loadModule("djs-dump.wasm"),  // only to persist runs (scripts/build-wasm.sh --dump)
});
const result = await sb.runAsync("const hits = await search(q); return hits.length;", { q: "djs" });
const run = sb.begin("return (await search(q)).length", { q: "djs" });  // a run the host drives
const dump = run.dump();                     // while it waits: a dump, a plain JavaScript Script
```

`js/pi/vm.ts` implements pi's codemode VM contract (`startVm`) on djs, running pi's
unmodified prelude (docs/host.md "pi codemode").

## Testing

```sh
cargo test                                   # unit and integration tests
scripts/fetch-test262.sh                     # once: vendor/test262 at the pinned commit
cargo run --release -p djs-test262 -- run --engine djs --group active \
    --expected tests/test262/expected-failures.txt       # test262 against the ratchet
cargo run --release -p djs-difftest -- --engine djs --ratchet   # the corpus against qjs
node --test js/test/*.test.ts                # the JS host package (needs a djs-wasm build)
```

Everything that runs an engine should run under `scripts/timeout.sh`. TESTING.md
explains the layers: test262 with skip/later/active groups and a ratchet, the
differential corpus in pi's codemode model, and the stress modes (suspension through heap
images, GC stress, both builtin backends, heap-image determinism, native versus wasm).

`scripts/gate.sh` runs everything that must pass before a change reaches `main`;
`scripts/merge-to-main.sh` rebases, regenerates generated files, runs the gate and
fast-forwards `main` (work/README.md). Benchmarks: `scripts/bench.sh` (engine track and
size), `scripts/perf/` (instruction counts natively and in wasm).

## Layout

```
crates/djs-core      the engine: lexer, compiler, heap, GC, interpreter, builtins' Rust parts (no_std)
crates/djs-bgen      the builtin compiler: TS subset -> prelude JS and Rust state machines
crates/djs-prelude   builds the base snapshot from bgen's output
crates/djs-wasm      the wasm32 build and its integer ABI
crates/djs-cli       the `djs` command
crates/djs-test262   the test262 runner
crates/djs-difftest  the differential corpus runner
crates/djs-bench     benchmarks
crates/djs-docs      the guide's generator and checker (docs/guide)
builtins/            the builtins in the TypeScript subset (builtins/README.md)
js/                  the host package: Sandbox, wire encoder, heap reader, pi adapter
tests/               corpus, test262 lists, benchmarks, pi fixtures
scripts/             gate, merge, build, bench, perf and determinism tools
docs/                design notes and the decision log
work/                issues, board and process
```

## Documentation

- **docs/guide/**: *How djs works*, the guide for humans: the design and its trade-offs,
  with live examples. `cargo run -p djs-docs -- build` writes the site to
  `target/docs-site/` (live examples need `scripts/build-wasm.sh` first; serve the
  directory over HTTP for them).
- **ARCH.md**: the architecture: goals, the arena, values and heap, compiler,
  interpreter, builtins, host interface, snapshots, and what was planned but replaced.
- **TESTING.md**: test262, the differential corpus, stress modes, known differences, the
  gate.
- **docs/decisions.md**: every design decision in order (append-only);
  **docs/decisions-index.md** groups them by topic.
- **docs/compiler.md**: the compiler and bytecode design note.
- **docs/objects.md**: property access, inline caches, shapes and dictionaries.
- **docs/host.md**: the Sandbox API, the wasm ABI, the pi adapter, numbers.
- **docs/wire.md**: the inbound wire format and the outbound heap reader.
- **docs/cli.md**: the `djs` command and the REPL.
- **builtins/README.md**: the builtin language and its tags.
- **AGENTS.md** and **work/README.md**: how work is organized (issues, worktrees, merging).
