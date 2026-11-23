# The host interface: djs-wasm and the Sandbox API

ARCH.md "Host Interface" and "Execution API". djs runs in a host (node, a browser, any wasm
runtime) as `djs_wasm.wasm`, a `wasm32-unknown-unknown` cdylib (`crates/djs-wasm`) with a
thin export layer over `djs_core::vm::Vm`. `js/src/sandbox.ts` (package `@earendil-works/djs`, js/README.md) is
its TypeScript client. Values go in through the wire format and come out through direct heap
reads (docs/wire.md, both halves).

```sh
scripts/build-wasm.sh              # target/wasm32-unknown-unknown/wasm/djs_wasm.wasm, stripped, with the dumper
scripts/build-wasm.sh --keep-names # with the name section (profiling, scripts/wasm-funcs.py)
node --test js/test/*.test.ts    # sandbox.test.ts needs a build ($DJS_WASM overrides the path)
scripts/npm-pack.sh                # the npm package: target/npm/earendil-works-djs-VERSION.tgz
scripts/npm-test.sh TARBALL node   # its smoke test (also bun, deno, js-tests, browser)
```

The npm package (D3105–D3109, D3602) has one entry point, `.` (`src/index.ts`): the whole
API with the package's one module, wasm/djs.wasm, as the default (`$DJS_WASM` overrides
it), and the dump runtime's `helpers`, `script` and `scriptFor`; besides it only
`./wasm/*.wasm` and `./package.json`. It finds its module with
`new URL("../wasm/djs.wasm", import.meta.url)`, which bundlers copy as an asset;
`Sandbox.defaultModule()` is the hook the entry point overrides.
`js/src/sandbox.ts` itself defaults to `$DJS_WASM`, else the repository's build when it
runs from source. The wire encoder and the reader are not exported.

The gate builds a debug djs-wasm and runs the node tests against it.

Rust hosts do not go through wasm: they link `crates/djs-embed` (D3170), a std crate over
`djs_core::vm::Vm` whose API names no djs-core type (snapshots, instances, typed host
functions, plain data as `Data` or wire bytes, budgets, one `Error`; its crate docs are the
reference, the guide's rust-api.md the overview). What it needs from djs-core beyond the wasm
modules' needs sits behind djs-core's `embed` feature (D3180): the outbound wire encoder, the
uncounted UTF-8 string tag, `ExecOptions::fatal_out_of_memory` (D3181), `Vm::reset_to`
(D3182), `Vm::decode_wire`.

## The Sandbox API

```ts
import { Sandbox, loadModule } from "@earendil-works/djs";

const sb = await Sandbox.create({
  functions: { readFile: (p) => fs.readFileSync(p as string, "utf8") },
  asyncFunctions: { fetchJson: (url) => fetch(url as string).then((r) => r.json()) },
  // globals: { readFile: "readFile", fetchJson: "fetchJson", console: "console" }, // the default
  // bindings: { myTool: "@sdk/our-stuff#myTool" }, // body parameters bound to host functions
  console: myConsole,            // receives console.log/info/warn/error; default: console
  memoryLimit: 64 << 20,         // heap cap in bytes; default 4 GiB (see below)
  fuelSlice: 1_000_000,          // fuel per vm_run, i.e. how often the host gets control
  maxFuel: 1e9,                  // per run; default unlimited
  now: Date.now,                 // the VM's clock (ms), a function or a fixed time; default Date.now
  timeZoneOffset: 0,             // minutes east of UTC; default: the host's current offset
  unhandledRejections: "warn",   // or "throw", or "collect" (sb.takeUnhandledRejections())
  randomSeed: 42n,               // Math.random's 64-bit seed; default: from crypto.getRandomValues
  frozenBuiltins: false,         // builtins behave as frozen, with the override fix (D2304)
  deterministic: { startedAt, seed }, // repeatable runs (D1680): see below
});
sb.run("return readFile(path).length", { path: "README.md" });   // codemode function body
await sb.runAsync("return await fetchJson(url)", { url });       // host functions may return promises
const task = sb.start(code, params);                            // a task: suspend() into a dump while it waits
const run = sb.begin(code, params);                             // a run the host drives; run.dump() while idle
sb.dispose();
```

- `loadModule(location?)` compiles the module once per location and caches the
  `WebAssembly.Module`. Every sandbox gets its own instance and VM (`new WebAssembly.Instance`,
  about 30 µs), so a trap in one cannot affect another.
- `new Sandbox(opts)` is the synchronous form. It needs `opts.module`, or a finished
  `loadModule()` of the default location. `Sandbox.create` / `Sandbox.resume` load the
  module first.
- `run(code, params)` compiles `code` as a codemode async function body. The `params` are
  its parameters, sent in through the wire format. `return` gives the result, read out
  through the reader. Objects come back as plain JS objects. Values without a host form
  (functions, symbols, …) come back as `Opaque`. `console` is a global object whose methods
  are host functions.
- Host functions get host values (read from the VM) and return host values (sent through
  the wire format). A thrown host error is thrown in the VM at the call, as an Error of the
  same kind (TypeError, RangeError, otherwise Error) with the same message. A return value
  the wire format cannot carry (a function, a cycle) becomes such an error too.
- **Identities and bindings (D2970, D2972).** `functions` and `asyncFunctions` are keyed
  by the function's identity, any string (`readFile`, `tools.read`,
  `@sdk/our-stuff#myTool`): calls, journals, `run.calls` and dumps name the function by
  it, and the sandbox makes one function object per identity. An identity `ns.member` of
  two identifiers also makes the function member `member` of the namespace object `ns`
  (identity `ns`). Where scripts find them is separate: `globals` maps global names to
  identities of functions or namespaces, and `bindings` maps parameter names to them,
  which every codemode body gets after the run's `params` (`bindings: { myTool:
  "@sdk/our-stuff#myTool", sdk: "sdk" }`: the script calls `myTool("x")` and
  `sdk.fetch(u)`; the stand-in for imports). Without `globals`, one written rule:
  `console`, every namespace and every function whose identity is an identifier are
  globals of their own names, so `{ "tools.read": f }` makes the global `tools`; other
  identities are bound nowhere. A run parameter with a binding's name, or a binding or
  global to an identity the sandbox lacks, is a `misuse` error.
- `functions` are *sync* host functions: a call parks the VM until it returns. `run` needs
  them to return synchronously (a promise is a TypeError in the VM); `runAsync` awaits a
  promise while the VM stays parked at the call.
- `asyncFunctions` are *async* host functions (D1374), plain functions or `{ call, save,
  resume }` with hooks for dumps (below): a call returns a promise in the VM
  at once and the script goes on, so several calls are outstanding together (`Promise.all`
  over tools). `runAsync` starts each call as soon as `vm_run` returns, and when only
  pending calls are left (IDLE) awaits whichever settles first, passes its result back and
  runs on. The function may return a value or a promise (`run` accepts only values). A
  rejection or a thrown error rejects the VM promise with an Error of the same kind.
  Results are macrotasks (D1670): the VM applies one only when its job queue is empty,
  oldest first, and drains the queue before the next, so the output does not depend on
  `fuelSlice` or on when a result came back.
- The VM reads no clock (D1383, ARCH "Nondeterminism"): before every fuel slice the
  Sandbox sets `Date.now()` from `opts.now` (default `Date.now`; a number fixes it, for
  replays) and the local time zone from `opts.timeZoneOffset` (minutes east of UTC, no
  DST; default the host's offset at that time). Without a host clock `Date.now()` is 0.
- Deterministic mode (`deterministic: { startedAt, seed }`, D1680): the same code,
  parameters and results in the same order do the same thing on every run. `Math.random`
  is seeded from `seed`, the time zone is UTC, weak references are strong, and the clock starts at `startedAt`. It moves only when an async
  function's result is delivered: to `opts.now` at that moment (never backwards), applied
  when the VM takes the result from its inbox (D1670). Fuel slices and sync host
  functions do not move it. With a fixed `now` the clock is fully reproducible. Without `deterministic`, `randomSeed` (default: entropy) seeds a new
  sandbox and weak references are collected.
- Unhandled rejections (D1423): promises rejected with no handler once the VM's job queue
  is empty (after the run, after each async result, and whenever it waits on async host
  calls), as qjs reports them. `unhandledRejections: "warn"` (default) calls `console.warn("Possibly unhandled
  promise rejection:", "TypeError: x")`; `"throw"` fails the run with a `threw` error
  `Possibly unhandled promise rejection: TypeError: x`; `"collect"` keeps the reasons (host
  values) for `takeUnhandledRejections()`. A rejected result promise is the run's error,
  not an unhandled rejection.
- A promise as the result is unwrapped: its value, or `threw` with its reason; a promise
  that can never settle is a `threw` error.
- `defineFunction(name, f)`, `defineAsyncFunction(name, f)` and `setGlobal(name, value)`
  add functions and globals between runs; the functions are bound by the written rule
  above (a dotted name, `tools.read`, as a property of the global namespace object
  `tools`, D2632).
- Errors are `SandboxError` with a `kind`:
  - `threw`: an uncaught exception in the VM. `message` is the display form (`TypeError: x`);
    `value` is the value as read.
  - `syntax`: a compile error. `memory`: the heap cap ran out outside a run (inside a run,
    out of memory is a thrown `InternalError`, so `threw`), or the host could not provide
    memory below the cap during a run (`out of host memory`, D1690): the wasm memory could
    not grow. That is not a script error, scripts cannot catch it, and the call is
    abandoned.
  - `fuel`: `maxFuel` ran out. `trap`: the module trapped (a djs bug), with the panic
    message. `misuse`: a dump that does not load, a missing host function, a disposed
    sandbox.

  After a `trap` the sandbox is unusable. Every other error leaves it usable. A failed
  run that left the VM mid-call (`fuel`, or a host-side error escaping the run loop) is
  abandoned with `vm_abort`, and globals set earlier stay intact.
- **Choosing `memoryLimit` (D1690).** A JS `InternalError: out of memory` means the heap
  reached its cap, which depends only on the program and the cap. If the host cannot back
  the heap below the cap, the run ends with `memory` / `out of host memory` instead, and
  the outcome then depends on the machine. To keep out-of-memory deterministic, choose a
  cap the host can always provide. On wasm32 a collection needs the to-space (as large as
  the heap) and the heap's buffer grows by doubling, so the address space needs about 3x
  the cap: at most about **1 GiB** in a 4 GiB wasm32 memory, less if the embedder runs
  several VMs in one instance.
  A VM stopped out of fuel inside a regexp match may hold a progress record allocated past
  the cap (it does not count toward the cap, D1710, and is freed when the match resumes).
  A host `collect()` at exactly that point can then fail for lack of to-space; collect
  when the VM is idle or done instead.

### Tasks: suspending a run into a dump (D2770, D2771)

`runAsync` returns only at the end. A task is the same run, driven by the sandbox, that
can be suspended into a continuation dump while it waits on async host calls and
continued from it later, in another sandbox or process (examples/sleep-dump shows all of
it):

```ts
const sleep: AsyncHostFunction<{ due: number }> = {
  async call(ms) { const due = Date.now() + Number(ms); this.state = { due }; await until(due, this.signal); },
  save: (call) => call.state,                     // the default: the call's state, JSON data
  async resume({ due }, call) { await until(due, call.signal); }, // after a load
};
const opts = { asyncFunctions: { "tools.sleep": sleep } };
const task = (await Sandbox.create(opts)).start(code, params, { filename: "task.js" });
task.calls;                                 // the open calls: { id, fn, args, startedAt, state, signal }
const dump = await task.suspend();          // at the next idle point: save() per call, dump, stop
const again = await Sandbox.resume(dump, opts); // any time later, any process
await again.result;
```

- `start` runs the body like `runAsync` (the first step at once: a compile error throws)
  and calls the async host functions as the script calls them. `task.result` is the
  outcome; it rejects with a SandboxError (`threw`, `fuel`, ...).
- `suspend()` waits for the next idle point with no result left to deliver, takes each
  open call's state (its function's `save(call)`, by default `call.state`; JSON data) and
  writes the run as a dump, the states with their calls (`$djs.resume(id, name, args,
  state)`). The task stops: the calls' `signal`s abort,
  their late results are dropped, and `result` rejects with kind `suspended` (counted as
  handled). A dump that fails (a value that cannot be written, a `save` that throws, a
  state that is not JSON) rejects the `suspend` only, and the task goes on.
  `checkpoint()` dumps the same way and goes on. Both reject if the run ends first.
- `Sandbox.resume(dump, opts)` makes a sandbox from `opts` (the same host functions, by
  name), loads the dump and drives it as a task (`task.sandbox` is the new sandbox;
  `sb.resume(dump)` does it in an existing fresh one). Each open call continues by its
  function's hooks: `resume(state, call)` with the saved state; `call` again for a
  function without `resume`; a plain function's call is rejected (an Error naming the
  function), since the sandbox cannot know whether calling it again is safe.
- **Dumps** (`"djs-dump";`, D2830, D2890): the sandbox writes its dumps (tasks',
  `Run.dump`, `dump`) as ordinary Scripts in which every suspended function is a
  continuation, the rest of its code from where it waits. Dumps need frozen builtins
  (`frozenBuiltins`, D2836): in a sandbox without them every dump throws a `misuse`
  SandboxError (`dumps need frozen builtins`). `Sandbox.resume`, `loadRun` and `load`
  run a dump with the global object `$djs` defined, which holds the
  dump runtime's helpers (`vm_dump_runtime`, D2903) and the hooks: the dump calls
  `$djs.seed(lo, hi, next)` first (Math.random's state where the run used it, restored
  with `vm_restore_math_random` as one that ran, and the id of the next host call where it
  is not one past the highest reopened call's; `$djs.seed(next)` for the id alone, or no
  call, D2837) and `$djs.resume(id, name, args, state)` for each open call, which opens call `id` of the
  host function `name` again under the same id (`vm_open_call`) and gives the dump its
  promise. A host value the dump holds (a host function, a namespace object such as
  `tools` or `console`) is written as `$djs.host(name)`, by its identity rather than by
  where it is bound (D2971), and the sandbox answers with the object it made for that
  name, so `===` holds across the load. The calls opened that way are the loaded calls above, with their states, and
  go to the hooks as described. The hooks are served only while the Script runs to its
  first stop (an idle point, or the end); then the sandbox sets the id counter from
  `next` (`vm_set_next_call_id`), so the run's later calls get the ids the uninterrupted
  run gives them (D2897), and deletes `$djs` and the dump's `$dump` (D2916). A dump
  names the codemode body it continues as `$djs.source(name, hash, params)` instead of
  carrying its text again (D2882): the sandbox compiles the script `name` it ran (this
  process's scripts, or the `sources` option's) as it runs bodies, the one with that hash
  (`scriptHash`, the same as `vm::script_hash`); a host without it gets a clear error, and the
  `inlineSources` option writes the text instead. Sync host
  functions the dump's code calls meanwhile are called (they must return
  synchronously). The Script's completion value is the run's promise. Loading a dump
  needs no dumper and no frozen builtins: it is code, and node runs it
  too, given the same host functions and a `host` hook that returns them
  (examples/sleep-dump: `run.sh --node`).
- The hooks see a `HostCall`: `id`, `fn`, `args`, `startedAt` (this sandbox's
  `Date.now()` when it started or resumed the call), `state` (set it any time; a resumed
  call starts with its saved state, so it is saved again by default) and `signal`
  (aborted when the task is suspended or the sandbox disposed). It is `this` of `call`.
- One task or run at a time per sandbox; `dispose()` ends a task in progress (`misuse`).

### Host-driven runs (D2630, D2633–D2635)

A host that wants to decide every call itself drives the run instead:

```ts
const run = sb.begin(code, params, { filename: "task.js" }); // to the first idle point
run.state;                                  // "idle" | "done" | "threw"
run.calls;                                  // [{ id, fn: "sleep", args: [5000] }]: open, not called
run.settle(id, { ok: true, value });        // a result (ok false: rejects with value)
await run.advance();                        // to the next idle point; run.value / run.error at the end
const text = run.dump({ [id]: { due } });   // only while idle (and frozen builtins): a dump, with call states
const again = (await Sandbox.create({ asyncFunctions })).loadRun(text); // same calls, same ids
again.calls;                                // [{ id, fn: "sleep", args: [5000], state: { due } }]
```

- A host-driven run never calls async host functions: it lists them in `run.calls`, and
  the host answers with `settle`, in any order (results are still macrotasks, D1670).
  Sync host functions are called as in `run`. The sandbox runs nothing else meanwhile.
- `dump(states)` writes the idle VM as a dump (with frozen builtins, see above): its data, the suspended
  activations, the open calls (by the host's function names, with the states given for
  them, JSON data by call id) and the run's result promise, named `run`. `loadRun` loads
  the dump into a fresh sandbox made with the same host functions, by name, and
  continues: `again.calls` are the open calls with their ids and states. What the host
  does with them (deliver a recorded result, issue the call again, reject it) is its own
  policy; no hooks run.
- `sb.load(text)` loads a dump into an existing sandbox and returns `{ calls }`.
- `run`, `runAsync`, `replay` and tasks are this loop with the host's part built in (D2634): they
  call each async host function as soon as the script calls it, pass results back before
  every fuel slice, and wait at idle points. `begin(code, params, { journal })` records a
  host-driven run (its `settle`s in order, a check at every idle point), and `replay` runs
  that journal like a `runAsync` one. A dump names the run's result without taking a
  handle, so it leaves the VM byte for byte as it was (D2633): a recorded run can be
  dumped and still replays.

**One module (D3600).** djs ships one module, and it has the dumper (cargo feature `dump`,
with lazy functions and the compile cache it turns on): `scripts/build-wasm.sh` builds it, and
every host, the ones that persist runs and the ones that never do, loads the same file. It
costs the module about 335 KB gzip (767.6 KB against 432.9 KB without the dumper, measured
at 0294; the owner accepted the growth, D3601). The Sandbox needs that module: the
feature stays a build knob for size measurements, not a configuration hosts use.

**Storing dumps (D2703).** A dump is plain JavaScript text and stays that way: the
format is a Script, not a container. Hosts compress it when they store it, with zstd at
level 3 (node: `zlib.zstdCompressSync(text, { params: { [zlib.constants.ZSTD_c_compressionLevel]: 3 } })`,
and `zstdDecompressSync` back): over the harvested corpus a dump is 6.1 KB raw and 1.7 KB
compressed at the median, 17 KB compressed at p99; a session holding a 1.8 MB log is
394 KB. That is a third of what the image containers took (5.1 KB at the median), with
no dictionary and no reference image to keep: any zstd reads it, and so does any later
djs once decompressed. Level 19 saves a fifth to a quarter more on megabyte dumps (big-log
394 → 299 KB), at a much slower compression.

### Journals: record and replay (D1730)

A deterministic sandbox can record a run and replay it later, for durability (restart a
run after a crash), debugging, or to check a build against another.

```ts
const journal: JournalEntry[] = [];
const result = await sb.runAsync(code, params, { journal });   // or sb.run(...)
// later, in a sandbox in the same state (a fresh one made the same way and given the same
// runs before), with any fuel slice:
await sb2.replay(journal);   // the same result; a mismatch is a `diverged` SandboxError
```

A journal is an array of plain entries (JSON-serializable when the host's values are):

| entry | records |
| --- | --- |
| `{ type: "run", engine, code, params, hash }` | the start: the engine build (`djs_build_hash`), the code and parameters, the VM's canonical hash before the run |
| `{ type: "sync", fn, args, ok, value }` | a sync host function's call and outcome (`value` of an error: `{ name, message }`). Console output is not journaled |
| `{ type: "async", callId, fn, args, ok, value, now }` | an async host function's result, in delivery order, with the clock it carried |
| `{ type: "check", delivered, issued, hash }` | the VM was idle after `delivered` results, having made `issued` host calls, with this canonical hash |
| `{ type: "end", ok, value, delivered, issued, hash }` | the outcome (`ok` false: the error's display text) and the final counts and hash |

Replay answers every sync call from the journal (checking the function and arguments),
delivers the async results one per idle point in the recorded order with their clocks
(checking that the script has made that call with those arguments), and compares the
canonical hashes at the start, at the recorded idle points and at the end. It never calls
the host functions. Results are macrotasks (D1670), so only their order matters, not
when they arrived. That is why a replay at another fuel slice, or
delivering one result at a time where the recording got several at once, reaches the
same states. Raw value bits and offsets are never journaled: they depend on collection
points.

`sb.canonicalHash()` (`vm_canonical_hash`, `Vm::canonical_hash`) is FNV-1a (64 bits) of
the image a collection would leave, computed on a copy: the VM itself is unchanged, and
the hash does not depend on fuel slices or when the VM last collected.

### Images inside a process (D2700)

Heap images are not a persistence format: a run that must outlive its process is written
as a continuation dump. Images remain the engine's in-process mechanism: the base image
every VM is copied from, a native host's snapshots and the stress and determinism checks.
`vm_arena_ptr`/`vm_arena_len` read an image; a wasm host loads none. The REPL's `.heap`
reports on the live VM's heap.

#### What an image depends on (D3641)

An image holds numbers whose meaning is fixed by the engine: cell kinds and field offsets,
opcodes in every function's bytecode, intrinsic ids in the builtins' calls, realm slots,
object classes, native builtin indexes. The roots cell of every VM image carries the
engine build hash (`Roots::BUILD`, `vm::BUILD_HASH`: the layout, the opcode and intrinsic
tables, the sources and the features that change results), and loading checks it first:
`Vm::from_bytes` refuses another build's image with `LoadError::OtherBuild`. The same hash
(`djs_build_hash`) keys Sandbox journals.

## The ABI

The module exports `memory`, `__stack_pointer` and one function, `djs(op, a, b, c, d, e, f, t)`
(D3320): it runs operation `op` (numbered by `ops!` in crates/djs-wasm/src/lib.rs and `OPS` in
js/src/abi.ts) with integer arguments `a`-`f` and the `f64` `t`, which only `vm_set_now` and
`vm_resolve_host_call_at` take, and returns an `i32`. This document calls the operations by
name, as if each were an export; `djsExports(module)` (js/src/abi.ts) makes them functions by
those names. Operation 0 is `djs_abi_version` in every ABI; `djs_has(op)` tells whether the build
has an operation. One export because V8 makes a function object for each exported function of
each instance: 75 exports were a quarter of `new WebAssembly.Instance`. There are no imports:
the module needs nothing from the host, so `WebAssembly.instantiate(module, {})` works
anywhere. `djs_abi_version()` returns the ABI version (`ABI_VERSION` in
crates/djs-wasm/src/lib.rs), bumped on any incompatible change; the Sandbox checks it.

**Buffers.** Data crosses the boundary through two buffers in linear memory.
- `djs_staging(len) -> ptr` makes the *staging* buffer at least `len` bytes. The host writes
  into it, then calls the export that consumes it.
- The *out* buffer is filled by exports that return data, at `djs_out_ptr()` /
  `djs_out_len()`.

Both stay valid until the next call into the module. Any call can grow memory, which
detaches `memory.buffer`, so the host must re-derive every typed array view after every
call.

**Numbering.** VMs are numbered from 1. Values that outlive a call are heap handles (GC
roots), numbered from 1: the handle index + 1. As an argument, handle 0 means `undefined`;
as a result, 0 means failure, with the reason as UTF-8 text in the out buffer. The host
frees handles with `vm_handle_free`. Using a freed VM or handle is a trap.

| export | does |
| --- | --- |
| `vm_new(limit, live, flags) -> vm` | a new VM: a copy of the base snapshot (D1321); `limit` = heap cap in bytes, 0 = default; `live` = the heap size to reserve room for (0: none). Flags: 1 = frozen builtins (D2304); 0 with a message for an unknown flag |
| `vm_frozen_builtins(vm) -> 0/1` | whether the VM's builtins are frozen (part of the image) |
| `vm_new_compiled(limit) -> vm` | builds the realm and compiles the prelude instead (only with the `compiled-prelude` feature; tests) |
| `vm_free(vm)` | drop the VM |
| `vm_arena_ptr(vm)`, `vm_arena_len(vm)` | the arena (the image, and the reader's view) |
| `vm_collect(vm) -> 0/-1` | run the GC |
| `vm_compile_file(vm, goal, name_len, params_len, src_len) -> h` | compile staging: `name_len` bytes of the file name stack traces show (none: `sandbox.js`; 0049), `params_len` bytes of `\n`-separated parameter names (goal 1, async body), then the source; goal 0 is a Script; `goal \| 2` parses TypeScript (0048), `\| 4` with legacy `experimentalDecorators`, `\| 8` also with `emitDecoratorMetadata` (D1651) |
| `vm_global(vm) -> h`, `vm_new_object(vm) -> h` | objects |
| `vm_new_host_function(vm, fn_id, name_len) -> h` | a function that parks the VM with `fn_id` |
| `vm_new_async_host_function(vm, fn_id, name_len) -> h` | a function that returns a promise and adds a pending call (D1374) |
| `vm_define(vm, obj, name_len, value, flags) -> 0/-1` | define a property (name in staging): non-enumerable, enumerable with `flags` bit 0 |
| `vm_decode_wire(vm, len) -> h` | decode wire data from staging (docs/wire.md) |
| `vm_string_reserve(vm, len) -> h` | a new one-byte string of `len` zero units, whose `len` data bytes (at `vm_arena_ptr` + the value's offset from `vm_handle_bits` + 16) the host fills, one Latin-1 unit per byte (ASCII UTF-8 as is), before anything allocates (D2551) |
| `vm_new_error(vm, kind, msg_len) -> h` | kind 0 Error, 1 TypeError, 2 RangeError, 3 ReferenceError, 4 SyntaxError, 5 InternalError |
| `vm_handle_bits(vm, h)` | out = the value's 64 bits as u32 lo, hi (for `readValue`) |
| `vm_handle_free(vm, h)` | release a handle |
| `vm_display(vm, h) -> len` | out = the value as `djs run`'s console.log shows it |
| `vm_call(vm, f, this, argc) -> 0/-1` | start a call; argument handles as u32 LE in staging |
| `vm_run(vm, fuel) -> status` | 0 DONE, 1 THREW, 2 OUT_OF_FUEL, 3 HOST_CALL, 4 IDLE, 5 HOST_OUT_OF_MEMORY (the host could not back the heap below its cap; the call is abandoned and the VM is idle, D1690), 6 NEED_STACK (elastic stack only, D3280: move the stack and `vm_run(vm, 0)` again) |
| `vm_fuel_used(vm) -> u32` | the fuel the last `vm_run` used (saturated): a host that polls by fuel counts it across sync host calls (D2653) |
| `vm_set_now(vm, ms: f64)` | the clock `Date` reads, ms since the epoch (D1383); set it before every `vm_run` |
| `vm_set_time_zone_offset(vm, minutes)` | the local time zone, minutes east of UTC (default 0) |
| `vm_canonical_hash(vm) -> 0/-1` | out = the canonical hash (u32 lo, hi) of the VM's state, computed on a collected copy (D1730) |
| `vm_set_weak_refs(vm, mode) -> 0/-1` | WeakRef and FinalizationRegistry (D1680): 0 collected (weak), 1 strong (no collection is observable); set it on a new VM |
| `vm_set_random_seed(vm, lo, hi)` | reseed `Math.random` (xorshift64*, as QuickJS) with `hi << 32 \| lo`; a new VM has a fixed default seed (0049) |
| `vm_abort(vm)` | abandon the current call (out of fuel or parked): the VM is idle and usable again |
| `vm_result(vm) -> h` | the DONE/THREW value, once |
| `vm_outcome(vm) -> h` | the entry call's outcome while no frame is active, also at IDLE (a function the host called while async calls are pending); any number of times; out[0] = 1 if it threw; 0 if none (D2650) |
| `vm_dump(vm, len) -> 0/1` | (feature `dump`) a dump (a plain Script, D2830) of the idle VM in out; the VM needs frozen builtins (D2836). Staging, six lines: host globals and names to dump (comma-separated), the host's function identities in id order, the values to name as `handle:name` (tab-separated; handle 0 names the entry call's outcome), the host roots as `handle:name` (tab-separated; written as `$djs.host(name)`, D2971) and the open calls' states as `call_id:byte_len` (tab-separated), then the states' JSON texts one after another (D2770). 1: refused, reasons in out. It needs no base image: the module has its base's builtin names (D3020). Loading a dump is running it as a Script: no export (D2890) |
| `vm_open_call(vm, parked, fn_id) -> 0/-1` | while parked on the sync call `parked`, a plain dump's `$djs.resume(id, name, args, state)`: opens async call `id` of host function `fn_id` with `args`' elements under that id, and returns its promise from `parked` (`Vm::open_host_call`, D2890). The call is not reported by `vm_new_host_calls`; the next new call gets a higher id. -1: the reason in out (an open id, not an array), for the host to throw at the call |
| `vm_dump_runtime(vm, len) -> h` | a new object holding the plain dump runtime's helpers (D2903), evaluated from staging[0..len], the runtime's Script (js/src/dump-runtime.js `scriptFor(dump)`, the helpers the dump names, or `script`, all of them; the module does not carry it): the host adds its hooks (`seed`, `resume`) and defines it as the global `$djs` before it runs a plain dump. Runs the VM, so call it while the VM is not running. 0: out of memory or not the runtime's text |
| `vm_set_dump_source_refs(vm, on)` | whether `vm_dump` names codemode bodies by `$djs.source` (1, the default) or writes their text (0) (D2882) |
| `vm_set_next_call_id(vm, id)` | the id the next host call gets: after a plain dump's load, its `$djs.seed`'s `next` or one past the highest reopened call, whichever is larger (`Vm::set_next_call_id`, D2897) |
| `vm_restore_math_random(vm, lo, hi)` | a plain dump's `$djs.seed(lo, hi)`: `Math.random`'s state `hi << 32 \| lo`, as one that ran, so that the next dump writes it again (`Vm::restore_math_random`, D2837) |
| `vm_host_call(vm) -> argc` | out = `[call_id, fn_id, this lo, hi, (arg lo, hi)*]` |
| `vm_host_call_info(vm, call_id) -> argc` | `vm_host_call` for any pending call (async ones) |
| `vm_new_host_calls(vm) -> n` | out = the ids (u32) of the async calls made since the last check |
| `vm_resolve_host_call(vm, call_id, ok, value) -> 0/-1` | return (`ok` 1) or throw (`ok` 0) `value`; for an async call, fulfill or reject its promise: the result goes into the VM's inbox and `vm_run` applies it when the job queue is empty (D1670) |
| `vm_resolve_host_call_at(vm, call_id, ok, value, now: f64) -> 0/-1` | the same, and `Date` reads `now` (ms) from when the result is applied: a virtual clock per result (D1670) |
| `vm_promise_state(vm, h) -> state` | 0 not a promise, 1 pending, 2 fulfilled, 3 rejected; settled: out = a handle (u32) to the result, and the promise counts as handled |
| `vm_take_unhandled_rejections(vm) -> n` | out = handles (u32) to the reasons of promises rejected with no handler, oldest first (D1423) |
| `djs_build_hash() -> u32` | the engine build hash: identifies the engine build (0049), including its sources, cargo features and libm version (D1750); a VM image's roots cell holds it (D3641) |
| `djs_base_ptr()`, `djs_base_len()` | the base snapshot in the module's memory (read-only). The module ships it coded (D2252): the first call (or `vm_new`) decodes it, unless the host injected it. The pointer is valid until the next `vm_new`: the instance's first VM takes the bytes as its arena (D2060), and a later call decodes them again. 0 if there is nothing to decode (no `djs_base_coded_ptr` bytes) |
| `djs_base_coded_ptr(len) -> ptr` | room for the coded base, the module's custom section `djs_base` (D2541: not in linear memory); the host writes the section there when it has no decoded base to inject (`provideCodedBase`, js/src/base.ts). Without either, there is no base and `vm_new` fails. 0 if the instance has its base or coded base already, or `len` is not the section's length |
| `djs_base_inject_ptr(len) -> ptr` | room for the base snapshot that another instance of the same module decoded (its `djs_base_ptr` bytes), so that this one need not decode (D2252); 0 if the instance has its base already or `len` is not the base's length |
| `djs_base_inject() -> 1/0` | takes the bytes written at `djs_base_inject_ptr` as the base if their length and hash match the coded base's; 0: refused, the instance decodes its own |
| `djs_panic_ptr()`, `djs_panic_len()` | the last panic message, after a trap |
| `djs_debug_panic()` | trap on purpose (tests) |
| `djs_elastic_stack(on)` | the elastic stack (D3280): a compile nesting past the small stack's limits makes `vm_compile*` return `-1` and `vm_run` NEED_STACK instead of trapping |
| `djs_stack(len) -> end` | a `len`-byte block (multiple of 16) for `__stack_pointer`, returns its end; `djs_stack(0)` frees it. js/src/trap.ts `elasticStack` drives it |

A run, as the Sandbox does it:

1. compile → `f`; `vm_call(f)`;
2. loop on `vm_run(slice)`:
   - OUT_OF_FUEL: continue, or give up;
   - HOST_CALL: `vm_host_call`, read the arguments with the reader, call the host function,
     then encode the result and `vm_decode_wire` it (a large ASCII string:
     `vm_string_reserve` and copy its bytes there), or make it an error with
     `vm_new_error`; then `vm_resolve_host_call`;
   - IDLE: only async host calls are pending (or nothing was called): wait for one,
     then `vm_resolve_host_call` it and run again;
   - DONE / THREW: `vm_result`, read the value, free the handle (a promise result:
     `vm_promise_state`).

   After every `vm_run`, `vm_new_host_calls` lists the async calls to start.

`vm_run` runs the microtask queue too (D1370): after the entry call returns, it runs the
promise jobs, and DONE comes only once the queue is empty and no async host call is
pending. An uncaught exception is THREW at once.

**Async results are macrotasks (D1670).** `vm_resolve_host_call` on an async call can come
at any time (after OUT_OF_FUEL, while parked on a sync call, at IDLE, several at once).
The result waits in the VM's inbox, part of the heap and of images. Whenever the job
queue runs empty, `vm_run` takes the oldest result, settles its promise and drains the
queue again, then takes the next. IDLE means: no jobs, no results waiting, async calls
pending. So what a script does depends only on the order of the results, not on when the
host delivered them, or the fuel slice (as I/O callbacks in an
event loop).

`vm_take_unhandled_rejections` lists rejected promises nobody handled, as qjs reports
them; check it after DONE, THREW or IDLE. Each time the job queue runs empty is a
checkpoint: a rejection still unhandled then stays reported even if a later result's
code handles it, so taking them after several results gives the same list as taking
them after each one. (`vm_promise_state` on a settled promise withdraws it, D1423.)

The raw value bits from `vm_host_call` and `vm_handle_bits` stay valid until the next
`vm_run` or collection (the GC moves objects). Read them right away.

**Traps.** The module is built with `panic = "abort"`. A panic (an engine bug, or a misused
ABI) records a message, then traps. On wasm32 djs-wasm is `no_std` with its own panic
handler (D2332): debug modules record the panic's message, the shipped (release) module a
fixed one ("djs-wasm panicked ..."), because formatting would keep core::fmt and every
panic message in the module; a native or debug build reproduces the real message. The
shipped module also carries no panic locations (D2333). The host sees a
`WebAssembly.RuntimeError`, reads `djs_panic_*`, and must discard the instance. The Sandbox
does that and reports `kind: "trap"`. Hostile JS cannot cause a panic: the heap is bounds
checked, and limits are thrown errors.

**Allocator.** djs-wasm uses std, so it gets Rust's built-in dlmalloc for
wasm32-unknown-unknown (D1231). Arenas are `Vec<u8>`s that dlmalloc places in linear
memory. A freed VM's memory is reused but not returned: wasm memory cannot shrink.

## Numbers

Node 25, Apple M-series, the `wasm` profile at opt-level `s` (D1232), medians.

**Size.** 1,515 KB at 0095 (after 0091): code 935 KB, data 576 KB of which the base
snapshot is 398 KB; 531 KB gzip -9, 426 KB brotli -q 11. (At 0057 it was 1,241 KB; the
growth since is mostly the generated `@native` builtins, `@fast` entries and Unicode
tables, each justified by measurements in its decision.) The `wasm` profile strips the name section
(D1575) and links with `--compress-relocations` (D1570). Where the bytes go and what could
shrink them: work/issues/0052-bloat-investigation.md. The size track (`scripts/bench.sh
size`) records `size/djs-wasm ...` (file, code, data, names, gzip, brotli) next to the old
probe. The ARCH budget (400 KB) is meant after `wasm-opt`, which is not installed here.

**Startup** (`node js/bench/sandbox.bench.ts`; measured at 0057 on a quiet machine. At
0095, under load 12: compile 1.5 ms, instantiate 66 µs, `vm_new` 4.8 µs,
`Sandbox.create` 142 µs, `run("return 1")` 9.7 µs):

| step | time |
| --- | ---: |
| `WebAssembly.compile` (once per process) | 0.6 ms |
| instantiate | 39 µs |
| `vm_new` (a copy of the base snapshot, D1321) | 0.4 µs (146 µs compiling the prelude) |
| `Sandbox.create` (instance + VM + console) | 58 µs |
| `run("return 1")` (compile + call + run) | 4.4 µs |
| `Sandbox.create` + first run | 80 µs |

Since the base snapshot, instantiating the module is most of a sandbox's startup.

**The coded base (D2252).** Since 0122 the module carries the base snapshot coded
(djs-lzrc: 79 KB instead of 385 KB, for the size budget D020), and an instance decodes it
at its first VM: about 5 ms, once. Every sandbox is its own instance, so the host decodes
once per compiled module instead: `js/src/base.ts`'s `provideBase` (right after
instantiating) copies the base an earlier instance of the module decoded into the new
one, which checks its length and hash, and `rememberBase` keeps it, per
`WebAssembly.Module`. The Sandbox does this. A later instance then costs
about 0.15 ms more than with the uncoded base (the copy and its check), and a host that
never injects still works (each instance decodes its own).

**The base becomes the arena (D2060).** The first `vm_new` of an instance takes the
decoded or injected base as its arena and grows it in place (it ends where memory ends,
D2053); the instance keeps no copy. Hosts therefore call `rememberBase` before `vm_new`
(a host that reads `djs_base_ptr` after it makes the instance decode again), and inject
only into instances that create a VM, not into ones that restore an image. An instance
that creates a second VM decodes the base once more, from the coded section handed over
again (the first VM's `vm_new` drops the coded base, D3360), and keeps it for its further
VMs.
Measured: every sandbox uses 0.31 MiB less wasm memory (an empty Sandbox
1.50 → 1.19 MiB), and later `Sandbox.create` is
about 20 µs faster (no copy).
| 1000-item tool result in, a loop over it, an object out | 0.6 ms |

**The boundary in wasm** (`node js/bench/wasm.bench.ts <dir>`, datasets from
`wire.bench.ts`; ms). This is the measurement docs/wire.md deferred to djs-wasm.

Inbound: encode into linear memory and decode into a fresh VM, against `JSON.stringify` +
`encodeInto` + a native JSON parse in the VM (these numbers were taken with an export
that parsed staged JSON text; the bench now runs the VM's `JSON.parse` on the text as a
wire string):

| dataset | encode | stringify + encodeInto | decode | parse_json | wire total | JSON total |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 10k similar objects | 2.70 | 1.84 | 5.21 | 9.01 | **7.9** | 10.9 |
| one 1 MiB ASCII string | 0.02 | 0.14 | 0.08 | 0.75 | **0.10** | 0.89 |
| 1 MiB text, some non-ASCII | 0.20 | 1.13 | 0.67 | 2.05 | **0.87** | 3.18 |
| tool results | 0.67 | 0.66 | 1.14 | 2.10 | **1.81** | 2.76 |
| 500k numbers | 2.84 | 5.27 | 2.15 | 13.23 | **5.0** | 18.5 |

Outbound: `readValue` on the arena, against `JSON.stringify` in the VM + reading the string
+ `JSON.parse`:

| dataset | readValue | stringify in the VM | read JSON + JSON.parse | JSON total |
| --- | ---: | ---: | ---: | ---: |
| 10k similar objects | **3.25** | 6.12 | 3.41 | 9.5 |
| one 1 MiB ASCII string | **0.06** | 0.13 | 0.34 | 0.48 |
| 1 MiB text, some non-ASCII | **0.03** | 3.03 | 0.94 | 3.97 |
| tool results | **0.45** | 1.93 | 1.03 | 2.96 |
| 500k numbers | **3.00** | 24.26 | 5.47 | 29.7 |

In wasm the designs win by more than they did natively:
- inbound: 1.4x on objects, 1.5x on tool results, 3.7x on numbers and text, 9x on one big
  string;
- outbound: 2.9x to 10x on structured data, and 130x on non-ASCII text, which reads at
  memcpy speed.

The decoder runs 1.8x (objects) to 2.7x (one string) slower than natively. The reader is
host code and runs at the same speed.

## Gaps

- **Host refs** (ARCH "Data passing") are not built: pass-through data such as images is
  copied in and out.
- **Persistence** is by continuation dumps (D2560): another sandbox or process loads one
  or refuses it with a reason; a later djs only once the dump format is declared stable
  (D3643). A run computing between idle points cannot be persisted (D2561): give it
  fuel to reach one, or rerun it.
- **Sync host functions** run one at a time (the VM parks); use `asyncFunctions` for
  concurrent calls.
