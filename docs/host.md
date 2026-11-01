# The host interface: djs-wasm and the Sandbox API

ARCH.md "Host Interface" and "Execution API". djs runs in a host (node, a browser, any wasm
runtime) as `djs_wasm.wasm`, a `wasm32-unknown-unknown` cdylib (`crates/djs-wasm`) with a
thin export layer over `djs_core::vm::Vm`. `js/src/sandbox.ts` (package `@djs/host`) is its
TypeScript client. Values go in through the wire format and come out through direct heap
reads (docs/wire.md, both halves).

```sh
scripts/build-wasm.sh              # target/wasm32-unknown-unknown/wasm/djs_wasm.wasm, stripped
scripts/build-wasm.sh --keep-names # with the name section (profiling, scripts/wasm-funcs.py)
node --test js/test/*.test.ts    # sandbox.test.ts needs a build ($DJS_WASM overrides the path)
```

The gate builds a debug djs-wasm and runs the node tests against it.

## The Sandbox API

```ts
import { Sandbox, loadModule } from "@djs/host";

const sb = await Sandbox.create({
  functions: { readFile: (p) => fs.readFileSync(p as string, "utf8") },
  asyncFunctions: { fetchJson: (url) => fetch(url as string).then((r) => r.json()) },
  console: myConsole,            // receives console.log/info/warn/error; default: console
  memoryLimit: 64 << 20,         // heap cap in bytes; default 4 GiB
  fuelSlice: 1_000_000,          // fuel per vm_run, i.e. how often the host gets control
  maxFuel: 1e9,                  // per run; default unlimited
  now: Date.now,                 // the VM's clock (ms), a function or a fixed time; default Date.now
  timeZoneOffset: 0,             // minutes east of UTC; default: the host's current offset
  unhandledRejections: "warn",   // or "throw", or "collect" (sb.takeUnhandledRejections())
});
sb.run("return readFile(path).length", { path: "README.md" });   // codemode function body
await sb.runAsync("return await fetchJson(url)", { url });       // host functions may return promises
const snap = sb.bytes();                                         // VM image + host function names
const sb2 = await Sandbox.fromBytes(snap, { functions: { readFile } });
sb.dispose();
```

- `loadModule(location?)` compiles the module once per location and caches the
  `WebAssembly.Module`. Every sandbox gets its own instance and VM (`new WebAssembly.Instance`,
  about 30 µs), so a trap in one cannot affect another.
- `new Sandbox(opts, snapshot?)` is the synchronous form. It needs `opts.module`, or a
  finished `loadModule()` of the default location. `Sandbox.create` / `Sandbox.fromBytes`
  load the module first.
- `run(code, params)` compiles `code` as a codemode async function body. The `params` are
  its parameters, sent in through the wire format. `return` gives the result, read out
  through the reader. Objects come back as plain JS objects. Values without a host form
  (functions, symbols, …) come back as `Opaque`. `console` is a global object whose methods
  are host functions.
- Host functions get host values (read from the VM) and return host values (sent through
  the wire format). A thrown host error is thrown in the VM at the call, as an Error of the
  same kind (TypeError, RangeError, otherwise Error) with the same message. A return value
  the wire format cannot carry (a function, a cycle) becomes such an error too.
- `functions` are *sync* host functions: a call parks the VM until it returns. `run` needs
  them to return synchronously (a promise is a TypeError in the VM); `runAsync` awaits a
  promise while the VM stays parked at the call.
- `asyncFunctions` are *async* host functions (D1374): a call returns a promise in the VM
  at once and the script goes on, so several calls are outstanding together (`Promise.all`
  over tools). `runAsync` starts each call as soon as `vm_run` returns, and when only
  pending calls are left (IDLE) awaits whichever settles first, passes its result back and
  runs on. The function may return a value or a promise (`run` accepts only values). A
  rejection or a thrown error rejects the VM promise with an Error of the same kind.
- The VM reads no clock (D1383, ARCH "Nondeterminism"): before every fuel slice the
  Sandbox sets `Date.now()` from `opts.now` (default `Date.now`; a number fixes it, for
  replays) and the local time zone from `opts.timeZoneOffset` (minutes east of UTC, no
  DST; default the host's offset at that time). Without a host clock `Date.now()` is 0.
- Unhandled rejections (D1423): promises rejected with no handler once the VM's job queue
  is empty (after the run, and whenever it waits on async host calls), as qjs reports
  them. `unhandledRejections: "warn"` (default) calls `console.warn("Possibly unhandled
  promise rejection:", "TypeError: x")`; `"throw"` fails the run with a `threw` error
  `Possibly unhandled promise rejection: TypeError: x`; `"collect"` keeps the reasons (host
  values) for `takeUnhandledRejections()`. A rejected result promise is the run's error,
  not an unhandled rejection.
- A promise as the result is unwrapped: its value, or `threw` with its reason; a promise
  that can never settle is a `threw` error.
- `defineFunction(name, f)`, `defineAsyncFunction(name, f)` and `setGlobal(name, value)`
  add globals between runs.
- Errors are `SandboxError` with a `kind`:
  - `threw`: an uncaught exception in the VM. `message` is the display form (`TypeError: x`);
    `value` is the value as read.
  - `syntax`: a compile error. `memory`: the heap cap ran out outside a run (inside a run,
    out of memory is a thrown `InternalError`, so `threw`).
  - `fuel`: `maxFuel` ran out. `trap`: the module trapped (a djs bug), with the panic
    message. `misuse`: a bad snapshot, a missing host function, a disposed sandbox.

  After a `trap` the sandbox is unusable. Every other error leaves it usable. A failed
  run that left the VM mid-call (`fuel`, or a host-side error escaping the run loop) is
  abandoned with `vm_abort`, and globals set earlier stay intact.
- Snapshots: `bytes()` is `"DJSS"`, version 1 (u32 LE), the length of a JSON header,
  `{"functions": [names by fn_id]}`, then the VM image. `fromBytes` binds the host function
  ids to the implementations by name. It refuses a snapshot whose functions are not all
  provided. The image is the arena itself and is validated on load (`Vm::from_bytes`).

## The ABI

Exports take and return only `i32`s, except `vm_set_now`'s `f64`. There are no imports: the module needs nothing from
the host, so `WebAssembly.instantiate(module, {})` works anywhere. `djs_abi_version()`
returns `1` and is bumped on any incompatible change. The Sandbox checks it.

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
| `vm_new(limit) -> vm` | a new VM: a copy of the base snapshot (D1321); `limit` = heap cap in bytes, 0 = default |
| `vm_new_compiled(limit) -> vm` | builds the realm and compiles the prelude instead (only with the `compiled-prelude` feature; tests) |
| `vm_from_bytes(len, limit) -> vm` | restore the image in staging (takes the staging buffer over) |
| `vm_free(vm)` | drop the VM |
| `vm_arena_ptr(vm)`, `vm_arena_len(vm)` | the arena (the image, and the reader's view) |
| `vm_collect(vm) -> 0/-1` | run the GC |
| `vm_compile(vm, goal, params_len, src_len) -> h` | compile staging: `params_len` bytes of `\n`-separated parameter names (goal 1, async body) then the source; goal 0 is a Script; `goal \| 2` parses TypeScript (0048) |
| `vm_compile_file(vm, goal, name_len, params_len, src_len) -> h` | `vm_compile` with the file name stack traces show (staging: name, parameters, source; 0049) |
| `vm_global(vm) -> h`, `vm_new_object(vm) -> h` | objects |
| `vm_new_host_function(vm, fn_id, name_len) -> h` | a function that parks the VM with `fn_id` |
| `vm_new_async_host_function(vm, fn_id, name_len) -> h` | a function that returns a promise and adds a pending call (D1374) |
| `vm_define(vm, obj, name_len, value) -> 0/-1` | define a non-enumerable property (name in staging) |
| `vm_decode_wire(vm, len) -> h` | decode wire data from staging (docs/wire.md) |
| `vm_parse_json(vm, len) -> h` | parse UTF-8 JSON text from staging (`JSON.parse`) |
| `vm_json_stringify(vm, h) -> h` | `JSON.stringify` of plain data as a heap string |
| `vm_new_error(vm, kind, msg_len) -> h` | kind 0 Error, 1 TypeError, 2 RangeError, 3 ReferenceError, 4 SyntaxError, 5 InternalError |
| `vm_handle_bits(vm, h)` | out = the value's 64 bits as u32 lo, hi (for `readValue`) |
| `vm_handle_free(vm, h)` | release a handle |
| `vm_display(vm, h) -> len` | out = the value as `djs run`'s console.log shows it |
| `vm_call(vm, f, this, argc) -> 0/-1` | start a call; argument handles as u32 LE in staging |
| `vm_run(vm, fuel) -> status` | 0 DONE, 1 THREW, 2 OUT_OF_FUEL, 3 HOST_CALL, 4 IDLE |
| `vm_set_now(vm, ms: f64)` | the clock `Date` reads, ms since the epoch (D1383); set it before every `vm_run` |
| `vm_set_time_zone_offset(vm, minutes)` | the local time zone, minutes east of UTC (default 0) |
| `vm_set_random_seed(vm, lo, hi)` | reseed `Math.random` (xorshift64*, as QuickJS) with `hi << 32 \| lo`; a new VM has a fixed default seed (0049) |
| `vm_abort(vm)` | abandon the current call (out of fuel or parked): the VM is idle and usable again |
| `vm_result(vm) -> h` | the DONE/THREW value, once |
| `vm_host_call(vm) -> argc` | out = `[call_id, fn_id, this lo, hi, (arg lo, hi)*]` |
| `vm_host_call_info(vm, call_id) -> argc` | `vm_host_call` for any pending call (async ones) |
| `vm_new_host_calls(vm) -> n` | out = the ids (u32) of the async calls made since the last check |
| `vm_resolve_host_call(vm, call_id, ok, value) -> 0/-1` | return (`ok` 1) or throw (`ok` 0) `value`; for an async call, fulfill or reject its promise (in a job) |
| `vm_promise_state(vm, h) -> state` | 0 not a promise, 1 pending, 2 fulfilled, 3 rejected; settled: out = a handle (u32) to the result, and the promise counts as handled |
| `vm_take_unhandled_rejections(vm) -> n` | out = handles (u32) to the reasons of promises rejected with no handler, oldest first (D1423) |
| `djs_build_hash() -> u32` | the build hash of VM images: identifies the engine build (0049) |
| `djs_panic_ptr()`, `djs_panic_len()` | the last panic message, after a trap |
| `djs_debug_panic()` | trap on purpose (tests) |

A run, as the Sandbox does it:

1. compile → `f`; `vm_call(f)`;
2. loop on `vm_run(slice)`:
   - OUT_OF_FUEL: continue, or give up;
   - HOST_CALL: `vm_host_call`, read the arguments with the reader, call the host function,
     then encode the result and `vm_decode_wire` it, or make it an error with
     `vm_new_error`; then `vm_resolve_host_call`;
   - IDLE: only async host calls are pending (or nothing was called): wait for one,
     then `vm_resolve_host_call` it and run again;
   - DONE / THREW: `vm_result`, read the value, free the handle (a promise result:
     `vm_promise_state`).

   After every `vm_run`, `vm_new_host_calls` lists the async calls to start.

`vm_run` runs the microtask queue too (D1370): after the entry call returns, it runs the
promise jobs, and DONE comes only once the queue is empty and no async host call is
pending. An uncaught exception is THREW at once. `vm_take_unhandled_rejections` lists
rejected promises nobody handled, as qjs reports them; check it after DONE, THREW or
IDLE.

The raw value bits from `vm_host_call` and `vm_handle_bits` stay valid until the next
`vm_run` or collection (the GC moves objects). Read them right away.

**Traps.** The module is built with `panic = "abort"`. A panic (an engine bug, or a misused
ABI) records its message through a panic hook, then traps. The host sees a
`WebAssembly.RuntimeError`, reads `djs_panic_*`, and must discard the instance. The Sandbox
does that and reports `kind: "trap"`. Hostile JS cannot cause a panic: the heap is bounds
checked, and limits are thrown errors.

**Allocator.** djs-wasm uses std, so it gets Rust's built-in dlmalloc for
wasm32-unknown-unknown (D1231). Arenas are `Vec<u8>`s that dlmalloc places in linear
memory. A freed VM's memory is reused but not returned: wasm memory cannot shrink.

## Numbers

Node 25, Apple M-series, the `wasm` profile at opt-level `s` (D1232), medians.

**Size.** 1,241 KB (0057, after 0059): code 709 KB, data 528 KB of which the base snapshot
is 395 KB; 428 KB gzip -9, 345 KB brotli -q 11. The `wasm` profile strips the name section
(D1575) and links with `--compress-relocations` (D1570). Where the bytes go and what could
shrink them: work/issues/0052-bloat-investigation.md. The size track (`scripts/bench.sh
size`) records `size/djs-wasm ...` (file, code, data, names, gzip, brotli) next to the old
probe. The ARCH budget (400 KB) is meant after `wasm-opt`, which is not installed here.

**Startup** (`node js/bench/sandbox.bench.ts`):

| step | time |
| --- | ---: |
| `WebAssembly.compile` (once per process) | 0.6 ms |
| instantiate | 39 µs |
| `vm_new` (a copy of the base snapshot, D1321) | 0.4 µs (146 µs compiling the prelude) |
| `Sandbox.create` (instance + VM + console) | 58 µs |
| `run("return 1")` (compile + call + run) | 4.4 µs |
| `Sandbox.create` + first run | 80 µs |

Since the base snapshot, instantiating the module is most of a sandbox's startup.
| 1000-item tool result in, a loop over it, an object out | 0.6 ms |

**The boundary in wasm** (`node js/bench/wasm.bench.ts <dir>`, datasets from
`wire.bench.ts`; ms). This is the measurement docs/wire.md deferred to djs-wasm.

Inbound: encode into linear memory and decode into a fresh VM, against `JSON.stringify` +
`encodeInto` + `vm_parse_json`:

| dataset | encode | stringify + encodeInto | decode | parse_json | wire total | JSON total |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 10k similar objects | 2.70 | 1.84 | 5.21 | 9.01 | **7.9** | 10.9 |
| one 1 MiB ASCII string | 0.02 | 0.14 | 0.08 | 0.75 | **0.10** | 0.89 |
| 1 MiB text, some non-ASCII | 0.20 | 1.13 | 0.67 | 2.05 | **0.87** | 3.18 |
| tool results | 0.67 | 0.66 | 1.14 | 2.10 | **1.81** | 2.76 |
| 500k numbers | 2.84 | 5.27 | 2.15 | 13.23 | **5.0** | 18.5 |

Outbound: `readValue` on the arena, against `vm_json_stringify` + reading the string +
`JSON.parse`:

| dataset | readValue | vm_json_stringify | read JSON + JSON.parse | JSON total |
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

- **Builtins and statements.** What JS runs depends on the compiler and the builtins on
  main. When this was written, `try`, functions and most builtins (`Array.prototype.push`,
  …) were not there yet. The tests stick to what works.
- **Async.** Concurrent host calls need `asyncFunctions` (sync `functions` still run one
  at a time). Until the compiler supports `async`/`await` (0037 defines the runtime), code
  chains with `.then`.

## pi codemode

pi's codemode (pi-private `packages/codemode`, compared at pi d65de9db942d) runs scripts on
quickjs-wasi. Everything engine-specific is in one module, `src/runtime/vm.ts`: `startVm`
drives one VM through one script and talks to the host only through the messages of
`protocol.ts`. The launchers (a worker thread, the calling thread, a remote isolate) and the
host side (`execution.ts`: tools, deadlines, output, store writes, deterministic runs) do not
touch QuickJS. `js/pi/vm.ts` implements the same `startVm` contract on djs, running pi's
unmodified `PRELUDE_SOURCE`. To try it, swap that one module (issue 0049).

```ts
import { loadModule } from "@djs/host";
import { startVm, resumeVm } from "@djs/host/pi";          // js/pi/vm.ts
const session = await startVm({ data, wasm: await loadModule(), prelude: PRELUDE_SOURCE, post, shouldInterrupt });
session.deliver({ type: "result", id, ok: true, payload: "42" });
const bytes = session.snapshot();                        // djs only: the whole session
const again = resumeVm(bytes, { wasm, post, shouldInterrupt });
```

`js/pi/engine.ts` is a handle-level client of the exports (the Sandbox runs whole bodies,
but pi keeps the prelude's `settle`/`run`/`stalled` alive across calls).
`tests/pi/prelude-source.ts` is a verbatim copy of pi's prelude for the tests here.

### How the contract maps

| pi on quickjs-wasi (vm.ts) | djs (js/pi/vm.ts) |
| --- | --- |
| `QuickJS.create({ memoryLimit, maxStackSize, interruptHandler, wasi })` | `vm_new(memoryLimit)` in a new instance of the djs-wasm module. Frames live in the heap, so there is no host-stack guard and `maxStackSize` is ignored. Deep recursion fails at about 131,000 frames, inside the heap limit, with `RangeError: Maximum call stack size exceeded`, as on quickjs-wasi (D013) |
| `evalCode(PRELUDE_SOURCE, "codemode-prelude.js")`, then calling it with `bridge` and JSON | the same, as `return <prelude>` compiled with `vm_compile_file` (an async body; djs Scripts have no completion value, D004) |
| `bridge`: a sync host function called with primitives | a sync host function (`vm_new_host_function`): the VM parks, the adapter reads the arguments with the reader, `post`s, and resolves the call with `undefined` |
| `(async (tools, console) => {code\n})` as `codemode.js` | the same text after `return `, so line numbers match. Compile errors become pi's script-error JSON with a `codemode.js:line:col` frame |
| `drain()`: run jobs one by one, poll `shouldInterrupt` every 50 jobs, then `stalled()` | `vm_run` runs the job queue to empty itself (D1370). `stalled()` is called after it |
| QuickJS polls `interruptHandler` while it runs; true throws an uncatchable error | `shouldInterrupt` is polled after every fuel slice (`fuelPerPoll`, default 10,000). True → `vm_abort` and a `crash`, which pi's launchers turn into `timeout` or `aborted` as before |
| deterministic: WASI clocks read virtual time, `random_get` a PRNG, UTC, no WeakRef | `vm_set_now(clock.now)` before every slice, time zone 0, `vm_set_random_seed`. djs reads no clock or entropy of its own (ARCH "Nondeterminism") and has no WeakRef at all |
| `Math.random` seeded by QuickJS from its first clock read (seed + 1), one draw used up while creating the context | djs's `Math.random` is the same xorshift64*, so the adapter seeds it one step after seed + 1. **The same seed gives the same numbers on both engines**; pi's pinned values pass |
| fingerprint `quickjs-wasi X, quickjs Y, prelude H, deterministic 1, stack S, heap M, budget B` | `djs <build hash>, prelude H, deterministic 1, fuel per poll F, heap M, budget B` (`djs_build_hash`) |
| quickjs-wasi's host globals `performance`, `atob`, `btoa` (`queueMicrotask` is native since 0059) | defined by the adapter in JS before the prelude, so the lockdown freezes them too. `performance.now()` counts from the run's start on the VM clock (deterministic: from `startedAt`, as on quickjs). Deterministic runs also lose `WeakRef` and `FinalizationRegistry` there, as on quickjs |
| no snapshots: durability by deterministic replay | `snapshot()` / `resumeVm()` in addition: the VM image plus the adapter's few numbers, taken while the script waits. A resumed session continues with no replay (tested against an uninterrupted run) |

**Prelude images.** Running pi's prelude is most of a start: its lockdown walks and freezes
about 700 objects, and the djs interpreter is still several times slower than QuickJS
(the board's perf track). The adapter keeps the VM image taken right after the prelude,
keyed by everything the prelude saw (prelude, tools, globals, store, heap limit, deterministic mode; at most
16 images per module, 450 KB each). A start with the same inputs restores the image
(0.6 ms) instead. Only `performance`'s origin and the seed are set per run, after the
restore. `reuseImages: false` turns it off. pi's worker launcher creates a worker per
execution, so it would need to keep the image in the parent (or keep workers warm) to
benefit. The in-thread and remote launchers benefit as they are.

### pi's own tests on djs

`node js/pi/pi-tests.ts` copies pi's package (sources and tests) into `target/pi-codemode`,
swaps `src/runtime/vm.ts` for a three-line wrapper around js/pi/vm.ts and `src/wasm.ts` for
the djs-wasm loader, and runs pi's installed vitest on the copy. node_modules are symlinks
and vite's cache stays in the copy, so nothing is written to pi-private. The run writes
`report.md` with pass/fail per test and the failure messages. At pi d65de9db942d: **215 of
226 pass** after 0059 (204 in 0049, 213 after 0054). 0054 fixed the stack overflow test,
0050's regexp work the two large-image tests, and 0051 plus the adapter's deterministic
mode the WeakRef test. Declarations, source, portable and worker-lifetime pass
completely; sandbox 144/153; deterministic 28/30.
Each failing test fails on every backend it runs on (worker, inline, remote):

| test | why |
| --- | --- |
| keeps eval and Function inside the VM | the constructors work (0059), but `eval("typeof process")` returns `undefined`: eval returns the Script's completion value, which arrives with 0058 |
| rejects dynamic import | `import()` is a compile-time SyntaxError, so the whole script fails instead of the `await import()` rejecting |
| ignores patches to built-ins and built-in globals | djs is always strict (ARCH). Assigning to a frozen built-in throws a TypeError where sloppy QuickJS ignores it. The protection itself works: the prelude's JSON and promises are intact |
| reports the fingerprint … | the test matches `/^quickjs-wasi /`. Inherent: a different engine must have a different fingerprint |

### What differs for scripts

Besides the failures above:
- **Strict mode.** pi evaluates scripts sloppy on QuickJS. On djs they are strict: an
  assignment to an undeclared variable is a ReferenceError instead of creating a global,
  and writes to read-only properties throw. Model-written code rarely relies on either.
- **Globals djs lacks** (quickjs-wasi has them; compared again after 0059): `DOMException`
  and `SharedArrayBuffer` (an ARCH non-goal). `Iterator` (D1510), the disposables (D1512),
  WeakRef and FinalizationRegistry (D1513), the URI functions and `escape`/`unescape`
  (0054), `eval`, the Function constructors and `queueMicrotask` (0059) are there now. The
  adapter supplies `performance`, `atob` and `btoa`.
- **Dynamic code.** `eval(x)` is always indirect eval (D014): it sees globals, not the
  caller's locals. Functions made by `Function(...)` are strict like everything else, so
  `Function("return this")()` is `undefined`, not the global object.
- **Interrupt budgets.** A poll is 10,000 fuel. On a tight loop that is about as many polls
  per second as QuickJS makes (34,000/s vs 32,000/s; a budget of 20,000 lasts 0.58 s vs
  0.62 s). Code that allocates or calls builtins polls far more often than on QuickJS,
  which does not count that work. A push loop with a budget of 20,000 lasts 1.2 s on djs
  and 10 s on QuickJS; string keys with JSON take 3.4 s and 55 s. pi's prelude itself takes
  about 120 polls on djs and 4 on QuickJS.
- **Memory limit.** djs caps its arena. Running out is a catchable
  `InternalError: out of memory`, as on QuickJS. With a limit of 256 KB or less, `startVm`
  rejects because the VM cannot be created; 512 KB is enough for pi's prelude.

### What pi would change to run on djs behind a flag

1. An engine option (`engine: "quickjs" | "djs"`) that picks `startVm` (vm.ts or this
   adapter) and the wasm loader (`loadQuickJSWasm` or djs's `loadModule`). The launchers,
   `execution.ts`, the protocol and the prelude stay as they are. Ship `djs_wasm.wasm` and the
   host files the adapter imports (js/src/{sandbox,wire,reader,layout.generated}.ts,
   js/pi/{vm,engine,protocol}.ts).
2. Replays must not mix engines. The fingerprints already differ. pi's fingerprint test would
   accept either prefix.
3. Optionally, durable runs through `snapshot()`/`resumeVm()` instead of replaying all
   results after a restart.
4. Optionally, for fast starts in the worker launcher: keep prelude images in the parent
   thread, or keep warm workers. Splitting the lockdown out of the argument-dependent part of
   the prelude would let one image serve every tool table and store.
5. Before the flag is the default, close these engine gaps: eval's completion value
   (0058) and `import()` as a runtime rejection. Stack overflow is a `RangeError` since
   0054 (D013).
   Interpreter speed matters for heavier scripts (see below).

### Numbers

`node js/bench/pi.bench.ts`: djs through js/pi/vm.ts and quickjs-wasi 3.6.2 through pi's vm.ts,
both on the calling thread with pi's prelude, tools answered in a microtask. Medians on
an Apple M-series machine, node 25, djs-wasm `wasm` profile (opt-level s), on 0033's
branch:

| case | quickjs-wasi | djs, prelude image reused | djs, prelude run each time |
| --- | ---: | ---: | ---: |
| script start (`return 1`) | 3.2 ms | 0.49 ms | 9.8 ms |
| tool-heavy script (61 calls, 120 KB of results) | 5.0 ms | 7.1 ms | 16.1 ms |

The tool-heavy script searches, reads 60 files in batches of 10 concurrent calls, splits
and filters the lines, prints a table and stores a summary. With an image, djs starts 6x
faster than quickjs-wasi, because restoring an image is a copy plus a check. The script
part is about 4x slower, all in the interpreter (`Exec::run` and builtins in a profile; the
adapter's own work does not show). opt-level 3 for djs-wasm brings that row to 6.6 ms and
the cold start to 8.6 ms. pi's whole test suite on djs takes 29 s (100 s without prelude
images).
