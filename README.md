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

Measured at issue 0095 (main after 0091). TESTING.md and docs/host.md have the details.

**Conformance.**
- test262 (at dquickts' pinned commit): djs passes **38,069 of the 38,109** tests in the
  active group (99.9%; at 0097 every test outside the skip groups is active). The 40 failures are listed with reasons in
  `tests/test262/expected-failures.txt`; 35 of them follow from djs being always strict
  (TESTING.md "Known differences").
  dquickts' qjs fails 4 active tests under the same runner. Skipped by design: Intl,
  Temporal, SharedArrayBuffer/Atomics, Annex B, sloppy mode, direct eval, modules.
- The differential corpus (codemode scripts, console and result formatting, error
  messages, TypeScript, dquickts' own tests): **224 of 230** cases match qjs; 1 known
  failure (a dquickts test that needs SharedArrayBuffer/Atomics, a non-goal) and 5
  skipped as non-goals.
- pi's codemode test suite on djs: 221 of 226 tests pass (last run at 0066). The
  failures follow from djs being always strict, or from tests that expect the
  quickjs-wasi fingerprint.

**Speed in wasm** (node, V8's wasm tier). Machine instructions per workload, djs-wasm vs
quickjs-wasi (pi's engine), from `scripts/perf/wmeasure.py`; lower is better:

| workload | djs-wasm | quickjs-wasi | ratio |
| --- | ---: | ---: | ---: |
| tight loop | 8.8G | 15.7G | 0.56 |
| property access (prop1) | 3.5G | 5.5G | 0.63 |
| strings (split, slice, join, ...) | 3.3G | 6.0G | 0.56 |
| arrays | 13.6G | 16.9G | 0.81 |
| JSON round trips | 1.3G | 2.2G | 0.61 |
| JSON with Dates | 1.0G | 1.9G | 0.52 |
| for-of | 7.5G | 7.2G | 1.05 |
| calls | 5.1G | 4.1G | 1.26 |
| push/pop | 6.0G | 4.8G | 1.26 |
| a pi-like script | 10.8G | 7.3G | 1.48 |
| forEach | 12.5G | 6.1G | 2.06 |
| map/filter/reduce (higher-order) | 18.1G | 4.8G | 3.80 |

pi's own benchmark (`js/bench/pi.bench.ts`, a tool-heavy script with 61 calls, through
pi's adapter): 4.2 ms on djs (reusing the prelude image) vs 5.3 ms on quickjs-wasi; a
script start is 1.0 ms vs 3.9 ms. Callback-heavy builtin code is the main remaining gap
(work/issues/0081-perf-anatomy.md).

**Size and startup.** `djs_wasm.wasm` is 1,515 KB (531 KB gzip, 426 KB brotli): 935 KB
of code and 576 KB of data, of which 398 KB is the base snapshot (the initialized realm
with all builtins). Creating a VM copies that snapshot: a few µs. A `Sandbox` (instance
plus VM) takes about 60-140 µs.

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
builtins/            the builtins in the TypeScript subset (builtins/README.md)
js/                  the host package: Sandbox, wire encoder, heap reader, pi adapter
tests/               corpus, test262 lists, benchmarks, pi fixtures
scripts/             gate, merge, build, bench, perf and determinism tools
docs/                design notes and the decision log
work/                issues, board and process
```

## Documentation

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
