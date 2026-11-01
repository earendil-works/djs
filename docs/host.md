# The host interface: djs-wasm and the Sandbox API

ARCH.md "Host Interface" and "Execution API". djs runs in a host (node, a browser, any wasm
runtime) as `djs_wasm.wasm`, a `wasm32-unknown-unknown` cdylib (`crates/djs-wasm`) with a
thin export layer over `djs_core::vm::Vm`. `js/src/sandbox.ts` (package `@djs/host`) is its
TypeScript client. Values go in through the wire format and come out through direct heap
reads (docs/wire.md, both halves).

```sh
scripts/build-wasm.sh            # target/wasm32-unknown-unknown/wasm/djs_wasm.wasm
scripts/build-wasm.sh --strip    # also drop the custom sections (names) for shipping
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

Exports take and return only `i32`s. There are no imports: the module needs nothing from
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
| `vm_compile(vm, goal, params_len, src_len) -> h` | compile staging: `params_len` bytes of `\n`-separated parameter names (goal 1, async body) then the source; goal 0 is a Script |
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
| `vm_abort(vm)` | abandon the current call (out of fuel or parked): the VM is idle and usable again |
| `vm_result(vm) -> h` | the DONE/THREW value, once |
| `vm_host_call(vm) -> argc` | out = `[call_id, fn_id, this lo, hi, (arg lo, hi)*]` |
| `vm_host_call_info(vm, call_id) -> argc` | `vm_host_call` for any pending call (async ones) |
| `vm_new_host_calls(vm) -> n` | out = the ids (u32) of the async calls made since the last check |
| `vm_resolve_host_call(vm, call_id, ok, value) -> 0/-1` | return (`ok` 1) or throw (`ok` 0) `value`; for an async call, fulfill or reject its promise (in a job) |
| `vm_promise_state(vm, h) -> state` | 0 not a promise, 1 pending, 2 fulfilled, 3 rejected; settled: out = a handle (u32) to the result |
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
pending. An uncaught exception is THREW at once. `Vm::take_unhandled_rejections` (not yet
exported) lists rejected promises nobody handled, as qjs reports them.

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

**Size.** 564 KB as built, 450 KB without the name section, 158 KB gzip -9 stripped, including
the 36 KB base snapshot (0032; the module no longer needs the code that builds the realm). The
size track (`scripts/bench.sh size`) records `size/djs-wasm ...` next to the old probe.
The ARCH budget (400 KB) is meant after `wasm-opt`, which is not installed here yet.

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
  chains with `.then`. Unhandled rejections are not reported to the host yet.
