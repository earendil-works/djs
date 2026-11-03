# The host interface: djs-wasm and the Sandbox API

ARCH.md "Host Interface" and "Execution API". djs runs in a host (node, a browser, any wasm
runtime) as `djs_wasm.wasm`, a `wasm32-unknown-unknown` cdylib (`crates/djs-wasm`) with a
thin export layer over `djs_core::vm::Vm`. `js/src/sandbox.ts` (package `@djs/host`) is its
TypeScript client. Values go in through the wire format and come out through direct heap
reads (docs/wire.md, both halves).

```sh
scripts/build-wasm.sh              # target/wasm32-unknown-unknown/wasm/djs_wasm.wasm, stripped
scripts/build-wasm.sh --keep-names # with the name section (profiling, scripts/wasm-funcs.py)
scripts/build-wasm.sh --slim       # target/djs-slim.wasm: the slim build (D2411)
node --test js/test/*.test.ts    # sandbox.test.ts needs a build ($DJS_WASM overrides the path)
```

The gate builds a debug djs-wasm and runs the node tests against it.

The slim module (djs-core without its default `full` and `unicode` features, D2411) has
the same exports and ABI. It lacks typed arrays, BigInt, the Iterator helpers, weak
references, DisposableStack/`using` and decorators, so the wire format's bytes tag
(`Uint8Array`) fails to decode there, and its images (build hash, base prefix) load only
into a slim module. pi's adapter runs it with `frozenBuiltins` and pi's prelude without its
lockdown (js/pi/pi-tests.ts `--no-lockdown`); the Sandbox keeps the full module by default.

## The Sandbox API

```ts
import { Sandbox, loadModule } from "@djs/host";

const sb = await Sandbox.create({
  functions: { readFile: (p) => fs.readFileSync(p as string, "utf8") },
  asyncFunctions: { fetchJson: (url) => fetch(url as string).then((r) => r.json()) },
  console: myConsole,            // receives console.log/info/warn/error; default: console
  memoryLimit: 64 << 20,         // heap cap in bytes; default 4 GiB (see below)
  fuelSlice: 1_000_000,          // fuel per vm_run, i.e. how often the host gets control
  maxFuel: 1e9,                  // per run; default unlimited
  now: Date.now,                 // the VM's clock (ms), a function or a fixed time; default Date.now
  timeZoneOffset: 0,             // minutes east of UTC; default: the host's current offset
  unhandledRejections: "warn",   // or "throw", or "collect" (sb.takeUnhandledRejections())
  randomSeed: 42n,               // Math.random's 64-bit seed; default: from crypto.getRandomValues
  weakRefs: "collected",         // or "strong", or "absent" (D1680)
  frozenBuiltins: false,         // builtins behave as frozen, with the override fix (D2304)
  deterministic: { startedAt, seed }, // repeatable runs (D1680): see below
});
sb.run("return readFile(path).length", { path: "README.md" });   // codemode function body
await sb.runAsync("return await fetchJson(url)", { url });       // host functions may return promises
const snap = sb.bytes();                                         // compact snapshot (see "Snapshots")
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
  Results are macrotasks (D1670): the VM applies one only when its job queue is empty,
  oldest first, and drains the queue before the next, so the output does not depend on
  `fuelSlice` or on when a result came back.
- The VM reads no clock (D1383, ARCH "Nondeterminism"): before every fuel slice the
  Sandbox sets `Date.now()` from `opts.now` (default `Date.now`; a number fixes it, for
  replays) and the local time zone from `opts.timeZoneOffset` (minutes east of UTC, no
  DST; default the host's offset at that time). Without a host clock `Date.now()` is 0.
- Deterministic mode (`deterministic: { startedAt, seed }`, D1680): the same code,
  parameters and results in the same order do the same thing on every run. `Math.random`
  is seeded from `seed`, the time zone is UTC, weak references are strong (`weakRefs`
  overrides it), and the clock starts at `startedAt`. It moves only when an async
  function's result is delivered: to `opts.now` at that moment (never backwards), applied
  when the VM takes the result from its inbox (D1670). Fuel slices and sync host
  functions do not move it. With a fixed `now` the clock is fully reproducible; a
  snapshot keeps it. Without `deterministic`, `randomSeed` (default: entropy) seeds a new
  sandbox and `weakRefs` defaults to `"collected"`.
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
  add globals between runs.
- Errors are `SandboxError` with a `kind`:
  - `threw`: an uncaught exception in the VM. `message` is the display form (`TypeError: x`);
    `value` is the value as read.
  - `syntax`: a compile error. `memory`: the heap cap ran out outside a run (inside a run,
    out of memory is a thrown `InternalError`, so `threw`), or the host could not provide
    memory below the cap during a run (`out of host memory`, D1690): the wasm memory could
    not grow. That is not a script error, scripts cannot catch it, and the call is
    abandoned.
  - `fuel`: `maxFuel` ran out. `trap`: the module trapped (a djs bug), with the panic
    message. `misuse`: a bad snapshot, a missing host function, a disposed sandbox.

  After a `trap` the sandbox is unusable. Every other error leaves it usable. A failed
  run that left the VM mid-call (`fuel`, or a host-side error escaping the run loop) is
  abandoned with `vm_abort`, and globals set earlier stay intact.
- Snapshots: `bytes()` collects, then writes the container below ("Snapshots") with the
  host function names and the clock as its metadata, compressed against the module's base
  image. `fromBytes` binds the host function ids to the implementations by name. It refuses
  a snapshot whose functions are not all provided, one whose dependency descriptor or
  builtin prefix differs from the module's (D2265), and one whose reference it lacks. The image is the arena itself and is validated on load (`Vm::from_bytes`).
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

### Journals: record and replay (D1730)

A deterministic sandbox can record a run and replay it later, for durability (restart a
run after a crash), debugging, or to check a build against another.

```ts
const journal: JournalEntry[] = [];
const result = await sb.runAsync(code, params, { journal });   // or sb.run(...)
// later, in a sandbox in the same state (a fresh one made the same way, or one restored
// from a snapshot taken right before the run), with any fuel slice:
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
when they arrived. That is why a replay at another fuel slice, after a snapshot, or
delivering one result at a time where the recording got several at once, reaches the
same states. Raw value bits and offsets are never journaled: they depend on collection
points.

`sb.canonicalHash()` (`vm_canonical_hash`, `Vm::canonical_hash`) is FNV-1a (64 bits) of
the image a collection would leave, computed on a copy: the VM itself is unchanged, and
the hash does not depend on fuel slices, snapshots or when the VM last collected.

### Snapshots (D2260)

`Sandbox.bytes()` and the pi adapter's `snapshot()` write one container; `Sandbox.fromBytes()`
and `resumeVm()` read it (js/src/snapshot.ts). The VM collects first, so the image holds
only live cells (D1890, D2263). The image is compressed with zstd against a *reference
image* that the writer and the reader can both produce without storing it: the module's
base snapshot (`djs_base_ptr`, `djs_base_len`), or for the pi adapter the prelude image
the session started from (D2262). Little-endian:

| offset | size | field |
| ---: | ---: | --- |
| 0 | 4 | magic `"DJSN"` (`0x4e534a44`) |
| 4 | 2 | format version, 3: any other version is refused (D024) |
| 6 | 1 | app, which says what the metadata is: 1 Sandbox, 2 pi session, 3 the djs REPL (`{}`) |
| 7 | 1 | compression: 0 none, 1 zstd |
| 8 | 24 | the image's dependency descriptor (D2265), six u32 from its roots cell: the cell layout hash (`Roots::VERSION`), the feature set (`Roots::FEATURES`), the compatibility version (`Roots::COMPAT`), the opcode table hash (`OPS`), the intrinsic table hash (`INTRINSICS`), the internal id tables hash (`RECORDS`) |
| 32 | 8 | FNV-1a 64 of the base image's builtin prefix (`base[0..BASE_END]`), low word first |
| 40 | 1 | the reference: 0 none, 1 the base image, 2 pi's prelude image |
| 41 | 1 | what the session image's prefix words are against: 0 the base image, 1 the reference |
| 42 | 2 | reserved, 0 |
| 44 | 8 | the reference's FNV-1a 64, low word first; 0 without one |
| 52 | 4 | the reference's length in bytes |
| 56 | 4 | the session image's length |
| 60 | 4 | the metadata's length |
| 64 | 4 | the payload's length |
| 68 | | the metadata (UTF-8 JSON), then the payload: the session image as is, or one zstd frame with a content checksum, compressed with the reference as a raw-content dictionary |

The engine build hash (`djs_build_hash`) is not in it (D026, D2265): a snapshot restores
into any module whose base image has the same descriptor and the same builtin prefix,
not only into the build that wrote it. What the components cover is in "What an image
depends on" below.

The stored image is a *session image* (D2066): what the base does not already hold of a VM
made from the module's base (its builtin prefix ends at `Roots::BASE_END`). A writer
refuses an image without that prefix. Little-endian:

| offset | size | field |
| ---: | ---: | --- |
| 0 | 4 | magic `"DJSs"` (`0x73534a44`) |
| 4 | 4 | format, 2 |
| 8 | 4 | the base's dependency descriptor, hashed (`ImageDeps::hash`: FNV-1a of the six u32) |
| 12 | 4 | `BASE_END`, the prefix's end |
| 16 | 8 | FNV-1a 64 of the base's bytes `0..BASE_END` |
| 24 | 4 | the roots cell's size |
| 28 | 4 | n, the changed prefix words |
| 32 | | n u32 offsets, ascending (padded to 8 bytes); n u64 values; the roots cell; the image from `BASE_END` on |

The prefix words are taken against the container's reference when it has the same prefix
(the base image, or pi's prelude image, which already holds the builtins pi's prelude
changed: as a diff against the base they would cost about 8 KB per container), else
against the base image. A VM that leaves its builtins alone has none. The reader puts the
image back together from the base or the reference (`expandSession`,
`vm::session::expand_session`) and checks that it is the one the image was made against.

A reader checks, in this order, and stops at the first problem with a clear message (a
`misuse` SandboxError; a `SnapshotError` from `resumeVm`):
1. the container: magic, format version, lengths ("truncated snapshot");
2. the app ("a pi session snapshot, not a sandbox snapshot");
3. the dependency descriptor and the builtin prefix, against the module's base image, in
   the order of the header; the first that differs is named: "the snapshot is from a djs
   with another opcode table (snapshot: 1a2b3c4d; this module: 5e6f7a8b): it restores
   only into a djs with the same opcode table". The feature set shows by name ("snapshot:
   full, regexp-unicode-props, unicode-normalization; this module:
   regexp-unicode-props, unicode-normalization" is a full build's snapshot in a slim
   module), the compatibility version in decimal, the prefix as "another base image". A
   host that must restore snapshots whose descriptor differs has to keep the builds that
   wrote them (an engine registry: not built);
4. the reference: kind, hash and length ("compressed against the base image X; this
   host's is Y");
5. zstd's checksum ("corrupt snapshot"); a session image's prefix hash ("made against
   another base image"); then the image's verification (`Vm::from_bytes`).

#### What an image depends on (D026, D2265)

An image holds numbers whose meaning is fixed by the engine: cell kinds and field offsets,
opcodes in every function's bytecode, intrinsic ids in the builtins' calls, realm slots,
object classes, native builtin indexes. The roots cell of every VM image carries a hash of
each of those tables, and loading checks them first (`Vm::from_bytes` refuses with
`LoadError::Incompatible`, naming the component):

| component | `Roots` field | what it covers |
| --- | --- | --- |
| cell layout | `VERSION` | every cell kind with its fields, records and tails, and the well-known atoms (`LAYOUT_HASH`) |
| feature set | `FEATURES` | the cargo features that change images, by bit: `full` (the builtin set), `regexp-unicode-props`, `unicode-normalization` (D2267) |
| compatibility version | `COMPAT` | `vm::COMPAT_VERSION`, bumped by hand for a change of meaning the hashes cannot see (TESTING.md "Snapshot compatibility") |
| opcode table | `OPS` | the instruction table: names, immediates, stack effects, costs, flags, numbering (`OPS_HASH`) |
| intrinsic table | `INTRINSICS` | intrinsic names, kinds, arities and numbering (`INTRINSICS_HASH`) |
| internal id tables | `RECORDS` | the id tables and slot conventions in source: object classes, realm slots, run states, header and template flags, value tags, frame continuations, the fast and native builtin tables, and the named slot numbers of internal records (`RECORDS_HASH`, D2266) |

A container adds the hash of its base's builtin prefix, since a session image is a diff
against it. Everything else in the engine can change between the build that wrote a
snapshot and the one that restores it: the restored VM simply runs on the new code. The
engine build hash (`djs_build_hash`) still identifies the exact build where that matters:
Sandbox journals replay only on it, and pi's fingerprint names it.

Compression runs in the host (D2261): `nodeZstd()` uses node's built-in zstd
(`node:zlib`, node 22.15 and 23.8 on), found through `process.getBuiltinModule`, so the
package still loads in a browser. Level 3 (`DEFAULT_ZSTD_LEVEL`), a window covering the
reference and the image. Without it (a browser, an older node) snapshots are written
uncompressed, and a compressed one cannot be read. `snapshotCodec` (SandboxOptions,
VmHooks) plugs in any zstd that takes raw-content dictionaries (a wasm build such as
zstd-rs interoperates with node's), `null` never compresses, and
`bytes({ compression: "none" })` skips it for one snapshot.

Measured on the `wasm` profile with node 25 (`scripts/snapshot-research/wasm.ts`: six pi
codemode cases, a snapshot and a resume at each of their 144 idle points; medians, max
where given; re-measured at 0133; times rough at a machine load of about 30):

| | image (collected) | container | `snapshot()` | `resumeVm()` | uncompressed resume |
| --- | ---: | ---: | ---: | ---: | ---: |
| pi session waiting on a tool (ask; agent-loop, sequential) | 489; 501–503 KB | 2.1; 7.0–7.4 KB | 2.8–3.1 ms | 1.0–1.3 ms | 0.9–1.0 ms |
| pi session holding 60 files (tool-heavy), median / max | 671 / 854 KB | 58 / 111 KB | 4.2 ms | 1.5 ms | 1.0 ms |
| pi session holding 1500 records / a 1.5 MB log | 1.7 / 2.2 MB | 335 / 404 KB | 9.4 / 9.8 ms | 4.1 / 3.6 ms | 2.2 / 1.2 ms |
| a new Sandbox / one holding 1000 records | 333 / 498 KB | 15.7 / 31.2 KB | 3.3 / 4.0 ms | 1.4 / 2.4 ms | 1.3 / 2.2 ms |

A waiting session compresses to what it changed because in-VM collections slide cells in
allocation order (D2040): the cells it inherited from the prelude image stay at their
offsets (99% of its words), and zstd encodes them as one long match. Against the base
image instead, the same session is 51 KB; without a dictionary 134 KB. A Sandbox's
`console` grows the global object's property table, exactly full in the base image; the
collection then slides everything after its old place down by 1.1 KB, so only the first
68 KB line up with the base, and the moved cells' references to other moved cells (43 KB)
are what the 15.7 KB encode. `djs heap` (docs/cli.md, D2264) shows all of this for any
snapshot: `djs heap --only diff session.djsn`.

Since 0117, images are about 57 KB smaller (the base image went from 376 to 314 KB:
internal names dropped and cells ordered by kind, D2251, D2253), and waiting sessions'
containers are unchanged. A new Sandbox's went 33 → 15.7 KB with the kind-ordered base:
the first of the cells its first definitions grow (the atom table's entries, the global
object's property table) used to sit 3 KB into the base; now it is the global object's
table, 68 KB in, so less of the image moves. tool-heavy and records changed with their corpus (this repository's sources
and docs, which grew).

## The ABI

Exports take and return only `i32`s, except the `f64` clock of `vm_set_now` and `vm_resolve_host_call_at`. There are no imports: the module needs nothing from
the host, so `WebAssembly.instantiate(module, {})` works anywhere. `djs_abi_version()`
returns `1` and is bumped on any incompatible change. The Sandbox checks it.

**Buffers.** Data crosses the boundary through two buffers in linear memory.
- `djs_staging(len) -> ptr` makes the *staging* buffer at least `len` bytes. The host writes
  into it, then calls the export that consumes it.
- `djs_image_staging(len, memoryLimit) -> ptr` is `djs_staging` for an image about to go to
  `vm_from_bytes`: the buffer gets the capacity the restored arena reserves, so handing it
  over grows nothing (D1942). Hosts check for it (modules before 0094 lack it).
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
| `vm_new_with(limit, live, flags) -> vm` | `vm_new` with creation flags; `live` = the heap size to reserve room for (0: none, as `vm_new_reserved`). Flags: 1 = frozen builtins (D2304); 0 with a message for an unknown flag |
| `vm_frozen_builtins(vm) -> 0/1` | whether the VM's builtins are frozen (part of the image) |
| `vm_new_compiled(limit) -> vm` | builds the realm and compiles the prelude instead (only with the `compiled-prelude` feature; tests) |
| `vm_from_bytes(len, limit) -> vm` | restore the image in staging (takes the staging buffer over) |
| `vm_free(vm)` | drop the VM |
| `vm_arena_ptr(vm)`, `vm_arena_len(vm)` | the arena (the image, and the reader's view) |
| `vm_collect(vm) -> 0/-1` | run the GC |
| `vm_compile(vm, goal, params_len, src_len) -> h` | compile staging: `params_len` bytes of `\n`-separated parameter names (goal 1, async body) then the source; goal 0 is a Script; `goal \| 2` parses TypeScript (0048), `\| 4` with legacy `experimentalDecorators`, `\| 8` also with `emitDecoratorMetadata` (D1651) |
| `vm_compile_file(vm, goal, name_len, params_len, src_len) -> h` | `vm_compile` with the file name stack traces show (staging: name, parameters, source; 0049) |
| `vm_global(vm) -> h`, `vm_new_object(vm) -> h` | objects |
| `vm_new_host_function(vm, fn_id, name_len) -> h` | a function that parks the VM with `fn_id` |
| `vm_new_async_host_function(vm, fn_id, name_len) -> h` | a function that returns a promise and adds a pending call (D1374) |
| `vm_define(vm, obj, name_len, value) -> 0/-1` | define a non-enumerable property (name in staging) |
| `vm_decode_wire(vm, len) -> h` | decode wire data from staging (docs/wire.md) |
| `vm_string_reserve(vm, len) -> h` | a new one-byte string of `len` zero units, whose `len` data bytes (at `vm_arena_ptr` + the value's offset from `vm_handle_bits` + 16) the host fills, one Latin-1 unit per byte (ASCII UTF-8 as is), before anything allocates (D2551) |
| `vm_parse_json(vm, len) -> h` | parse UTF-8 JSON text from staging (`JSON.parse`) |
| `vm_json_stringify(vm, h) -> h` | `JSON.stringify` of plain data as a heap string |
| `vm_new_error(vm, kind, msg_len) -> h` | kind 0 Error, 1 TypeError, 2 RangeError, 3 ReferenceError, 4 SyntaxError, 5 InternalError |
| `vm_handle_bits(vm, h)` | out = the value's 64 bits as u32 lo, hi (for `readValue`) |
| `vm_handle_free(vm, h)` | release a handle |
| `vm_display(vm, h) -> len` | out = the value as `djs run`'s console.log shows it |
| `vm_call(vm, f, this, argc) -> 0/-1` | start a call; argument handles as u32 LE in staging |
| `vm_run(vm, fuel) -> status` | 0 DONE, 1 THREW, 2 OUT_OF_FUEL, 3 HOST_CALL, 4 IDLE, 5 HOST_OUT_OF_MEMORY (the host could not back the heap below its cap; the call is abandoned and the VM is idle, D1690) |
| `vm_set_now(vm, ms: f64)` | the clock `Date` reads, ms since the epoch (D1383); set it before every `vm_run` |
| `vm_set_time_zone_offset(vm, minutes)` | the local time zone, minutes east of UTC (default 0) |
| `vm_canonical_hash(vm) -> 0/-1` | out = the canonical hash (u32 lo, hi) of the VM's state, computed on a collected copy (D1730) |
| `vm_set_weak_refs(vm, mode) -> 0/-1` | WeakRef and FinalizationRegistry (D1680): 0 collected (weak), 1 strong (no collection is observable), 2 absent (the globals are deleted); set it on a new VM |
| `vm_set_random_seed(vm, lo, hi)` | reseed `Math.random` (xorshift64*, as QuickJS) with `hi << 32 \| lo`; a new VM has a fixed default seed (0049) |
| `vm_abort(vm)` | abandon the current call (out of fuel or parked): the VM is idle and usable again |
| `vm_result(vm) -> h` | the DONE/THREW value, once |
| `vm_host_call(vm) -> argc` | out = `[call_id, fn_id, this lo, hi, (arg lo, hi)*]` |
| `vm_host_call_info(vm, call_id) -> argc` | `vm_host_call` for any pending call (async ones) |
| `vm_new_host_calls(vm) -> n` | out = the ids (u32) of the async calls made since the last check |
| `vm_resolve_host_call(vm, call_id, ok, value) -> 0/-1` | return (`ok` 1) or throw (`ok` 0) `value`; for an async call, fulfill or reject its promise: the result goes into the VM's inbox and `vm_run` applies it when the job queue is empty (D1670) |
| `vm_resolve_host_call_at(vm, call_id, ok, value, now: f64) -> 0/-1` | the same, and `Date` reads `now` (ms) from when the result is applied: a virtual clock per result (D1670) |
| `vm_promise_state(vm, h) -> state` | 0 not a promise, 1 pending, 2 fulfilled, 3 rejected; settled: out = a handle (u32) to the result, and the promise counts as handled |
| `vm_take_unhandled_rejections(vm) -> n` | out = handles (u32) to the reasons of promises rejected with no handler, oldest first (D1423) |
| `djs_build_hash() -> u32` | the engine build hash: identifies the engine build (0049), including its sources, cargo features and libm version (D1750). Images and snapshots are keyed on their dependency descriptor instead (D2265), which the base image's roots cell holds |
| `djs_base_ptr()`, `djs_base_len()` | the base snapshot in the module's memory (read-only): the reference image of compact snapshots (D2260). The module ships it coded (D2252): the first call (or `vm_new`) decodes it, unless the host injected it. The pointer is valid until the next `vm_new`: the instance's first VM takes the bytes as its arena (D2060), and a later call decodes them again. 0 if there is nothing to decode (no `djs_base_coded_ptr` bytes) |
| `djs_base_coded_ptr(len) -> ptr` | room for the coded base, the module's custom section `djs_base` (D2541: not in linear memory); the host writes the section there when it has no decoded base to inject (`provideCodedBase`, js/src/snapshot.ts). Without either, there is no base and `vm_new` fails. 0 if the instance has its base or coded base already, or `len` is not the section's length |
| `djs_base_inject_ptr(len) -> ptr` | room for the base snapshot that another instance of the same module decoded (its `djs_base_ptr` bytes), so that this one need not decode (D2252); 0 if the instance has its base already or `len` is not the base's length |
| `djs_base_inject() -> 1/0` | takes the bytes written at `djs_base_inject_ptr` as the base if their length and hash match the coded base's; 0: refused, the instance decodes its own |
| `djs_panic_ptr()`, `djs_panic_len()` | the last panic message, after a trap |
| `djs_debug_panic()` | trap on purpose (tests) |

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
host delivered them, the fuel slice, or snapshots in between (as I/O callbacks in an
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
once per compiled module instead: `js/src/snapshot.ts`'s `provideBase` (right after
instantiating) copies the base an earlier instance of the module decoded into the new
one, which checks its length and hash, and `rememberBase` keeps it, per
`WebAssembly.Module`. Both the Sandbox and pi's engine do this. A later instance then costs
about 0.15 ms more than with the uncoded base (the copy and its check), and a host that
never injects still works (each instance decodes its own).

**The base becomes the arena (D2060).** The first `vm_new` of an instance takes the
decoded or injected base as its arena and grows it in place (it ends where memory ends,
D2053); the instance keeps no copy. Hosts therefore call `rememberBase` before `vm_new`
(a host that reads `djs_base_ptr` after it makes the instance decode again), and inject
only into instances that create a VM, not into ones that restore an image. An instance
that creates a second VM decodes the base once more and keeps it for its further VMs.
Measured: every sandbox and pi session uses 0.31 MiB less wasm memory (an empty Sandbox
1.50 → 1.19 MiB, pi warm `return 1` 1.81 → 1.50 MiB), and later `Sandbox.create` is
about 20 µs faster (no copy).
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

- **Host refs** (ARCH "Data passing") are not built: pass-through data such as images is
  copied in and out.
- **Snapshots** load only into the build that wrote them (D2260). Restoring old snapshots
  after an upgrade needs the old builds (an engine registry, not built); durability across
  engine versions (journal replay) is undecided.
- **Sync host functions** run one at a time (the VM parks); use `asyncFunctions` for
  concurrent calls.

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
const again = resumeVm(bytes, { wasm, post, shouldInterrupt, prelude: PRELUDE_SOURCE, data });
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
| QuickJS polls `interruptHandler` while it runs; true throws an uncatchable error | `shouldInterrupt` is polled after every fuel slice (`fuelPerPoll`, default 10,000) of the script's work, not while the host globals and the prelude run (D1660: a start that reuses a prelude image skips that work, so a budget means the same on cold and warm starts). True → `vm_abort` and a `crash`, which pi's launchers turn into `timeout` or `aborted` as before |
| deterministic: WASI clocks read virtual time, `random_get` a PRNG, UTC, no WeakRef | `vm_set_now(clock.now)` before every slice, time zone 0, `vm_set_random_seed`. djs reads no clock or entropy of its own (ARCH "Nondeterminism"). In deterministic runs the VM deletes `WeakRef` and `FinalizationRegistry` (`vm_set_weak_refs`, D1680) before the prelude's lockdown |
| `Math.random` seeded by QuickJS from its first clock read (seed + 1), one draw used up while creating the context | djs's `Math.random` is the same xorshift64*, so the adapter seeds it one step after seed + 1. **The same seed gives the same numbers on both engines**; pi's pinned values pass |
| fingerprint `quickjs-wasi X, quickjs Y, prelude H, deterministic 1, stack S, heap M, budget B` | `djs <build hash>, prelude H, host G, deterministic 2, fuel per poll F, heap M, budget B` (`djs_build_hash`; G hashes the adapter's own globals, D1660; the adapter's version is 2 since it collects after the prelude, D1890) |
| quickjs-wasi's host global `performance` (`queueMicrotask` is native since 0059, `atob`, `btoa` and `DOMException` since 0084) | defined by the adapter in JS before the prelude, so the lockdown freezes it too. `performance.now()` counts from the run's start on the VM clock (deterministic: from `startedAt`, as on quickjs) |
| no snapshots: durability by deterministic replay | `snapshot()` / `resumeVm()` in addition: the session's state and its VM image in the snapshot container ("Snapshots"), taken while the script waits. The session collects first (D1890), and the image is compressed against the prelude image the session started from (D2262): 2–9 KB for a session waiting on a tool (489–503 KB as an image). `resumeVm` finds that prelude image in the process, or recomputes it from `prelude` and `data` (the session's start data); a session started with `reuseImages: false` is compressed against the base image. A resumed session continues with no replay (tested against an uninterrupted run, with the same messages and interrupt polls) |

**Prelude images.** Running pi's prelude is most of a start: its lockdown walks and freezes
about 700 objects, and builtin-heavy code like it still runs 1.5-4x more instructions on
djs than on quickjs-wasi (README.md "Status"). The adapter keeps the VM image taken right after the prelude,
keyed by everything the prelude saw (prelude, tools, globals, store, heap limit, deterministic mode; at most
16 images per module, 540 KB each at 0117: every cold start collects after the prelude,
D1890; they were 933 KB of mostly garbage before 0089). A start with the same inputs
restores the image (0.6 ms) instead. The prelude runs with the clock at 0 in UTC, so its
image depends only on those inputs and the build, and another process can recompute it to
restore a snapshot compressed against it (D2262). Only `performance`'s origin and the seed are set per run, after the
restore. `reuseImages: false` turns it off. pi's worker launcher creates a worker per
execution, so it would need to keep the image in the parent (or keep workers warm) to
benefit. The in-thread and remote launchers benefit as they are.

### pi's own tests on djs

`node js/pi/pi-tests.ts` copies pi's package (sources and tests) into `target/pi-codemode`,
swaps `src/runtime/vm.ts` for a three-line wrapper around js/pi/vm.ts and `src/wasm.ts` for
the djs-wasm loader, and runs pi's installed vitest on the copy. node_modules are symlinks
and vite's cache stays in the copy, so nothing is written to pi-private. The run writes
`report.md` with pass/fail per test and the failure messages. Rerun at 0102 (main after
0093 and 0097, pi c1b68f09da5b): **221 of 226 pass**, the same tests as at pi d65de9db942d
after 0058 and 0066 (204 in 0049, 213 after 0054, 215 after 0059), so no regressions. 0054 fixed the
stack overflow test, 0050's regexp work the two large-image tests, 0051 plus the adapter's
deterministic mode the WeakRef test, and 0058 the eval (completion value) and dynamic
import (a runtime rejection) tests. Declarations, source, portable and worker-lifetime
pass completely; sandbox 150/153; deterministic 28/30.
Each failing test fails on every backend it runs on (worker, inline, remote):

| test | why |
| --- | --- |
| ignores patches to built-ins and built-in globals | djs is always strict (ARCH). Assigning to a frozen built-in throws a TypeError where sloppy QuickJS ignores it. The protection itself works: the prelude's JSON and promises are intact |
| reports the fingerprint … | the test matches `/^quickjs-wasi /`. Inherent: a different engine must have a different fingerprint |

**Since 0116 (re-run at 0127): 215 of 226.** The six worker-lifetime "process lifetime"
tests fail too (Node and Bun × abort, timeout, close), because the fast search finished
their workload. Their fixture (`test/fixtures/worker-process-exit.ts`) cancels a script
that is meant to run long, `'a'.repeat(300000).indexOf('a'.repeat(10000) + 'b')`.
Byte-at-a-time search made that quadratic, and seconds long. The one-byte search tests
the needle's first and last byte together (D2250), so it rejects every position at once
and the script completes in microseconds, before the cancel: "The script completed
instead of being cancelled". Cancellation itself works. The same fixture with a busy loop
(`let x = 0; for (;;) { x++; }`) is aborted, timed out and closed in 0.2–0.3 s on Node
and on Bun, with the process exiting right after, as the tests require. These six are
known and explained, not regressions; they pass again if pi's fixture uses a workload
that is slow on every engine.

### What differs for scripts

Besides the failures above:
- **Strict mode.** pi evaluates scripts sloppy on QuickJS. On djs they are strict: an
  assignment to an undeclared variable is a ReferenceError instead of creating a global,
  and writes to read-only properties throw. Model-written code rarely relies on either.
- **Globals djs lacks** (quickjs-wasi has them; compared again after 0059):
  `SharedArrayBuffer` (an ARCH non-goal). `Iterator` (D1510), the disposables (D1512),
  WeakRef and FinalizationRegistry (D1513), the URI functions and `escape`/`unescape`
  (0054), `eval`, the Function constructors and `queueMicrotask` (0059), `atob`, `btoa`
  and `DOMException` (0084, D1840) are there now. The adapter supplies `performance`.
- **Dynamic code.** `eval(x)` is always indirect eval (D014): it sees globals, not the
  caller's locals. Functions made by `Function(...)` are strict like everything else, so
  `Function("return this")()` is `undefined`, not the global object.
- **Interrupt budgets.** A poll is 10,000 fuel. On a tight loop that is about as many polls
  per second as QuickJS makes (34,000/s vs 32,000/s; a budget of 20,000 lasts 0.58 s vs
  0.62 s). Code that allocates or calls builtins polls far more often than on QuickJS,
  which does not count that work. A push loop with a budget of 20,000 lasts 1.2 s on djs
  and 10 s on QuickJS; string keys with JSON take 3.4 s and 55 s. pi's prelude uses no polls
  on djs (about 4 on QuickJS): it is not polled, so cold and warm starts count the same
  (D1660).
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
5. The engine gaps found earlier are closed: eval's completion value and `import()` as a
   runtime rejection (0058), stack overflow as a `RangeError` (0054, D013). Interpreter
   speed still matters for builtin-heavy scripts (see below).

### Numbers

`node js/bench/pi.bench.ts`: djs through js/pi/vm.ts and quickjs-wasi 3.6.2 through pi's vm.ts,
both on the calling thread with pi's prelude, tools answered in a microtask. Medians on
an Apple M-series machine, node 25, djs-wasm `wasm` profile (opt-level s). At 0095
(main after 0091, load about 7, so ±20%):

| case | quickjs-wasi | djs, prelude image reused | djs, prelude run each time |
| --- | ---: | ---: | ---: |
| script start (`return 1`) | 3.9 ms | 1.0 ms | 7.0 ms |
| tool-heavy script (61 calls, 120 KB of results) | 5.3 ms | 4.2 ms | 9.1 ms |

On 0033's branch the tool-heavy row was 5.0 / 7.1 / 16.1 ms and the start 3.2 / 0.49 /
9.8 ms; the interpreter and builtin work since (0063, 0080-0091) made the script part
faster than quickjs-wasi.

The tool-heavy script searches, reads 60 files in batches of 10 concurrent calls, splits
and filters the lines, prints a table and stores a summary. With an image, djs starts
several times faster than quickjs-wasi, because restoring an image is a copy plus a check.
(At 0033: the script part was about 4x slower, opt-level 3 brought that row to 6.6 ms,
and pi's whole test suite on djs took 29 s, 100 s without prelude images.)
