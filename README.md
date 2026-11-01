# djs

djs is a small JavaScript and TypeScript engine written in `no_std` Rust. It is built
mainly for wasm32, where it replaces QuickJS in pi's codemode: model-written scripts that
call tools, process their results and print or return data. Its design follows from that
use (ARCH.md):

- **All VM state is one byte arena addressed by `u32` offsets**: objects, strings,
  bytecode, frames, the job queue, pending host calls. Snapshotting a VM means copying the
  bytes; restoring one means copying them back.
- **Suspend anywhere.** The interpreter never recurses on the Rust stack for JS calls.
  It can stop between any two instructions (fuel, a host call, a snapshot) and resume,
  also inside getters, callbacks of builtins, promise jobs, generators and async
  functions.
- **Cheap data passing.** Values go in through a compact wire format and come out through
  direct reads of the heap from the host, with no serialization step in the VM.
- **A compiler without an AST** (single pass to stack bytecode, like QuickJS), with
  TypeScript erased while parsing.
- **Builtins in a strict TypeScript subset**, compiled to bytecode for the base snapshot
  and, for the hot ones, to resumable Rust state machines.
- **Deterministic by construction**: the clock, randomness and weak references come from
  the host, and the same inputs give byte-identical heap images, natively and in wasm.

The behavior reference and test oracle is dquickts (a QuickJS fork), including its error
messages.

## Status

The numbers below are generated (`djs-docs regen`, D2101): counts from the repository
(test262 grouping and ratchets, the corpus) on every merge, measurements from
`tests/scores/status.tsv`, which `scripts/status.py` records with the commit they were
measured at. TESTING.md and docs/host.md have the details.

**Conformance.**

<!-- gen:status conformance -->
- test262 (at dquickts' pinned commit): djs passes **38,069 of the 38,109** tests in the
  active group (99.9%; 53,597 tests in all, the rest skipped by design). The 40 known
  failures are listed with reasons in `tests/test262/expected-failures.txt`; dquickts'
  qjs fails 4 active tests under the same runner.
- The differential corpus (codemode scripts, console and result formatting, error
  messages, TypeScript, dquickts' own tests): **234 of 240** cases match qjs; 1 known
  failure (`tests/corpus/expected-failures.txt`) and 5 skipped as non-goals.
- pi's codemode test suite on djs: 221 of 226 tests pass (measured at 2026-11-01).
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
| tight loop | 6.6G | 15.7G | 0.42 |
| property access (prop1) | 2.7G | 5.5G | 0.49 |
| strings (split, slice, join, ...) | 3.1G | 6.0G | 0.51 |
| arrays | 10.8G | 16.8G | 0.64 |
| JSON round trips | 1.3G | 2.2G | 0.57 |
| JSON with Dates | 0.9G | 1.9G | 0.48 |
| for-of | 5.8G | 7.2G | 0.81 |
| calls | 3.3G | 4.0G | 0.81 |
| push/pop and string methods | 4.3G | 3.1G | 1.40 |
| a pi-like script | 10.0G | 7.3G | 1.38 |
| forEach | 9.0G | 6.1G | 1.49 |
| map/filter/reduce (higher-order) | 13.9G | 4.8G | 2.92 |

Machine instructions per workload (`scripts/perf/w/`), measured at d780463b0d,
2026-11-01; lower is better.
<!-- /gen -->

pi's own benchmark (`js/bench/pi.bench.ts`, a tool-heavy script with 61 calls, through
pi's adapter): 4.2 ms on djs (reusing the prelude image) vs 5.3 ms on quickjs-wasi; a
script start is 1.0 ms vs 3.9 ms (at 0095). Callback-heavy builtin code is the main
remaining gap (work/issues/0081-perf-anatomy.md).

**Size and startup.**

<!-- gen:status size -->
`djs_wasm.wasm` is 1,471 KB (525 KB gzip, 423 KB brotli): 901 KB of code and 566 KB of
data, of which 385 KB is the base snapshot (the initialized realm with all builtins).
Module sizes measured at d780463b0d, 2026-11-01.
<!-- /gen -->

Creating a VM copies the base snapshot: about 40 µs in wasm (D1990). A `Sandbox`
(instance plus VM) takes about 60-140 µs.

## Building and running

Requirements: stable Rust (edition 2024) with the `wasm32-unknown-unknown` target, node
22.6 or newer for the JS host package (it runs TypeScript directly), and, for the
oracle-based tests, dquickts' `qjs` at `~/Development/dquickts/qjs` (or `$DJS_QJS`).

```sh
cargo build                                  # the workspace
cargo run -p djs-cli -- run file.js          # run a Script (prints console output)
cargo run -p djs-cli -- run --body file.js   # run a codemode async function body
cargo run -p djs-cli -- run --ts file.ts     # TypeScript
cargo run -p djs-cli -- repl                 # REPL with fuel, suspension and image commands
cargo run -p djs-cli -- disasm file.js       # bytecode listing
cargo run -p djs-cli -- strip-types file.ts  # the source with its types blanked
scripts/build-wasm.sh                        # target/wasm32-unknown-unknown/wasm/djs_wasm.wasm
```

`djs run` also takes `--gc-stress`, `--suspend-every N` and `--backend bytecode|native`
(docs/cli.md).

From JavaScript (`js/`, package `@djs/host`, docs/host.md):

```ts
import { Sandbox } from "@djs/host";

const sb = await Sandbox.create({
  asyncFunctions: { search: (q) => mySearch(q as string) },
  memoryLimit: 64 << 20,
});
const result = await sb.runAsync("const hits = await search(q); return hits.length;", { q: "djs" });
const image = sb.bytes();                    // snapshot: VM image plus host function names
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
