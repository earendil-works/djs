# A slim djs for codemode: which globals, and a flag or a build?

Issue 0149, measured at main 0de6c26a (after 0148). Research only: nothing changes on
main. The prototypes are patches in work/scratch/0149 (see "Reproduce"). Sizes are bytes
of the shipped wasm module, stripped; "gz" is gzip -9. "A", "B" and "full" name the
prototype builds of section 5.

The owner asked:

> Investigate what a build of this engine looks like that is trimmed down to the minimum
> set of things to make it work for the current uses of codemode in ~/.pi/agent/sessions
> and ~/.pi/p2 sessions. [...] Come up with a reduced set of globals, a minimal version
> that works well enough for codemode without pulling everything in. It might even be
> acceptable to remove functions in that mode. Also investigate whether this is a
> separate build of djs or just a flag, and the impact on the wasm binding.

## In short

- **Data.** Every codemode execution in both session stores:
  - 2,782 executions, 2,687 distinct scripts, 363 sessions, 2026-09-25 to 2026-11-02;
  - ~/.pi/agent/sessions: 2,757 executions in 357 sessions;
  - ~/.pi/p2/sessions: 25 executions in 6 sessions. The p2 app keeps sessions in SQLite
    (pi-durable's `entries` table), and extract.py now reads them.

  Each script was analyzed statically (djs's own compiler, 0130's analyzer) and joined
  with the builtin registry by global and member (new `scripts/harvest/members.py`). The
  300 harvested corpus cases and the 10 codemode cases also ran on a build that records
  every builtin they touch, named by registry path.
- **Scripts use a small, stable core**, and p2's scripts are a subset of the agent
  store's:
  - `Promise` (769 scripts), JSON (358), `String` (120), `Object` (99), `Error` (77),
    `Date` (43), `Math` (32), `Set` (17), `Number`, `Array`, `Boolean`, `Map`, `RegExp`;
  - array and string methods;
  - the iterator and promise protocols behind for-of, destructuring and `await`.

  The 310 corpus runs together touch 135 builtin objects and functions out of ~960.
- **Never used in 2,687 scripts:** typed arrays/ArrayBuffer/DataView, `Iterator` and its
  helpers, `BigInt`, `WeakRef`/`FinalizationRegistry`, `DisposableStack`/`using`,
  decorators, `Proxy`, `Reflect`, `Symbol`, `WeakMap`/`WeakSet`, `eval`/`Function`, the
  URI functions, `atob`/`btoa`. No script names any global of the proposed drop set.
- **pi's prelude decides the floor.** Without its lockdown, which frozen builtins
  (D2304) replace, the prelude needs `Proxy` and `Reflect.get` (the `tools` object), `Map`,
  `Promise`, JSON, a few `Object` statics and the error constructors. Its lockdown,
  however, needs `Int8Array`, generator and async-generator prototypes, `Symbol` and
  `Iterator`. **A slim module requires pi to drop its lockdown** and use frozen builtins.
  Checked: pi's lockdown-free prelude runs unchanged on prototype A; with its lockdown it
  fails at `Int8Array`.
- **Proposed set (section 4):**
  - **Drop:** typed arrays, ArrayBuffer and DataView; Iterator helpers and the
    `Iterator` global; BigInt (global and literals); WeakRef and FinalizationRegistry;
    DisposableStack, AsyncDisposableStack, SuppressedError and `using`; decorators; the
    two Unicode cargo features.
  - **Optional:** the newest proposals and some Annex B leftovers (`Promise.allKeyed`,
    `Map.prototype.getOrInsert`, `Math.sumPrecise`, `RegExp.escape`, `JSON.rawJSON`,
    `Error.isError`, `Array.fromAsync`, `escape`, `substr`, ...).
  - **Keep** everything else: the ES2015–2023 library models write, `Proxy`/`Reflect`
    for pi, `Symbol`, `WeakMap`, generators and classes. Proxy and Reflect can go only
    if pi's `tools` stops being a Proxy.
- **What it buys, measured with prototype builds:**

  | build | module gz | vs main | × quickjs-wasi | base image | cells |
  | --- | ---: | ---: | ---: | ---: | ---: |
  | main | 458,952 | | 1.60 | 325,008 | 5,852 |
  | Unicode features off | 424,423 | −34.5 KB | 1.48 | 325,008 | 5,852 |
  | **slim A, full** (drop set + Unicode off + syntax cut) | **399,605** | **−59.3 KB (−12.9%)** | **1.39** | **238,712** | **4,309** |
  | slim B, full (A + Proxy, Reflect; needs a pi change) | 395,626 | −63.3 KB | 1.38 | 219,640 | 3,987 |

  - Of A's 59.3 KB: Unicode tables 34.5, the base image 14.8 (data), the builtins'
    Rust leaves 7.9, decorator and `using` syntax 1.8.
  - The engine-deep parts (BigInt arithmetic, typed-array element paths, weak-reference
    and Proxy handling in the heap) are still in the prototype. Real cargo features
    would remove an estimated 8–10 KB gz more: ~390 KB, 1.36×.
- **Flag or build: a build.** Code goes away only at compile time.
  - **A flag can't remove code.** It could only delete globals from the per-VM global
    object (as `WeakRefs::Absent` does today). That changes no size and no memory: the
    builtin prefix is the same shared base.
  - **Two bases in one module** cost the module 42.9 KB gz for the second coded image
    (the slim base coded, measured), and still no code goes away.
  - **Deriving a slim base at instantiation** needs the prelude compiler path and
    ~10 ms per module (native). Pruning a VM made from the base frees nothing, because
    the prefix never moves (D2064).

  So: a cargo feature `slim` on djs-core/djs-prelude/djs-wasm, and a second module,
  `djs-slim.wasm`, for pi.
- **Binding impact is small.**
  - The ABI stays the same. Snapshots are bound to the module by its build hash and
    prefix hash, so slim and full images refuse each other with a clear error, which
    D2066 and D2400 already give.
  - The registry keeps one table: the slim base's offset table marks absent ids, and
    exports from slim VMs import into full ones.
  - The wire format loses its bytes type (`Uint8Array`) in the slim module. pi uses JSON,
    so pi is unaffected; Sandbox hosts that pass bytes must use the full module.
- **Recommendation (section 9):**
  1. Now: ship pi a module without the two Unicode features (−34.5 KB gz, a build flag
     that exists today). Ask pi to drop its lockdown and use frozen builtins (pi-private).
  2. Then: a `slim` cargo feature for set A, built in phases. Image and leaves first
     (~1.5 weeks, −24 KB gz), then BigInt and typed arrays out of the engine core
     (~2–3 weeks, est. −8–10 KB gz).
  3. Gate it with the codemode corpora, pi's tests on the slim module, and test262
     filtered by feature tag, with its own size budget.
  4. Keep Proxy and Reflect unless pi rewrites `tools`.

## 1. Data and method

- **Sessions.**
  - The agent store is pi's JSONL sessions, read by 0118's extract.py. It also holds 607
    durable SQLite session files; one of them had a codemode run.
  - The p2 store is `~/.pi/p2/sessions/<id>.sqlite` (19 databases), pi-durable's schema.
    Each row of the `entries` table holds a JSON record whose `model` field carries the
    pi messages (assistant tool calls, tool results with `details.calls`), the same
    shapes as a JSONL session.
  - extract.py reads both (D2410). It opens copies of the databases (with their WAL files) in
    work/scratch read-only and touches nothing else under ~/.pi/p2.
  - Raw extracts stay in work/scratch/0149; this report has names, counts and dates
    only.
- **Static usage.**
  - features.py (0130) compiles each distinct script as pi runs it and reads globals,
    `Global.member` statics, and member reads and calls off the bytecode.
  - members.py joins that with tests/builtins/registry.tsv (D2371): for each builtin
    function or accessor that a global reaches, the scripts that name it.
  - Statics match exactly; prototype members match by name only (`.slice` counts for
    `Array`, `String` and `ArrayBuffer`), so their counts are upper bounds.
  - 13 scripts don't compile, all non-JavaScript or broken (as in 0130). One calls qjs's
    `print`.
- **Dynamic usage.**
  - 0143's touch instrumentation (scripts/size/patches/touch.py) marks every base cell
    that a property read or a function entry touches.
  - The 300 harvested codemode cases and the 10 codemode cases ran one per process,
    after the difftest codemode prelude.
  - A scratch example names the touched cells by registry path, with a function's code
    attributed to the function.
  - Limits: the corpus prelude is not pi's (no tools Proxy, no lockdown), and the tools
    are fake.
- **pi's prelude.** PRELUDE_SOURCE from tests/pi (identical to pi-private's today), with
  and without its `lockdown()` function. It was analyzed statically and run natively
  with a fake bridge on main and on the prototypes.
- **Size prototypes** (work/scratch/0149/slim_patch.py, applied by
  scripts/size/variant.sh to a scratch copy):
  - Before the base is prepared, the build deletes the dropped families' globals,
    constructors-table entries, realm intrinsic slots and installed intrinsics, and
    every property of a kept object whose function comes from a dropped builtins/*.ts
    file (Iterator.prototype's helpers, for example). The canonical collection then
    leaves out what nothing reaches.
  - **Arms:** the dispatch arms of the 40 Rust leaf intrinsics that only dropped files
    call are made unreachable, so rustc drops them and the linker drops their leaves.
    Their id constants alias the first arm's. This is a measurement build, not a
    runnable one. An early `return` in `call_intrinsic` did not let LLVM drop the arms
    (−0.6 KB raw only).
  - **Syntax cut:** the parser's entry points for decorators and `using` return a
    SyntaxError or don't match.
  - The image-only prototype A runs natively: all 310 codemode cases give the same
    results as main (the same 5 ratcheted differences).
  - Estimates for code the prototypes keep come from a DWARF line-table build
    (scripts/wasm-dwarf-lines.py), by source file.

## 2. Usage by global and member

Distinct scripts (of 2,687) that name the global; members are the registry's statics,
prototype methods and accessors with string keys. "Named": at least one script names
it (prototype members by name only). "Touched": at least one of the 310 corpus runs
touched it.

| global | scripts | members | named | touched |
| --- | ---: | ---: | ---: | ---: |
| Promise | 769 | 14 | 4 | 7 |
| JSON | 358 | 4 | 2 | 2 |
| String | 120 | 39 | 21 | 14 |
| Object | 99 | 32 | 5 | 17 |
| Error | 77 | 6 | 3 | 4 |
| Date | 43 | 48 | 5 | 5 |
| Math | 32 | 37 | 7 | 3 |
| Boolean | 19 | 3 | 1 | 0 |
| Set | 17 | 16 | 5 | 3 |
| Number | 14 | 11 | 2 | 2 |
| Array | 10 | 43 | 27 | 15 |
| parseInt, isFinite | 4, 1 | | | |
| Map, RegExp | 3, 3 | 14, 15 | 7, 4 | 4, 12 |
| Function (`.call`, `.bind` through the prelude) | 0 | 6 | 2 | 3 |
| Reflect | 0 | 13 | 0 | 2 (codemode cases) |
| Proxy | 0 | 1 | 0 | 0 (the tools-proxy case constructs one) |
| Symbol | 0 | 22 | 2 (`.description`, a tool field) | 0 |
| WeakMap, WeakSet | 0 | 7, 4 | by name only | 0 |
| Iterator | 0 | 22 | by name only (`.find`, `.map`, ...) | 1 (`Iterator.prototype[Symbol.iterator]`) |
| typed arrays, ArrayBuffer, DataView | 0 | 57 | by name only (`.slice`) | 0 |
| BigInt | 0 | 6 | by name only (`.toString`) | 0 |
| DisposableStack, AsyncDisposableStack, SuppressedError | 0 | 15 | 0 | 0 |
| WeakRef, FinalizationRegistry | 0 | 5 | 0 | 0 |
| AggregateError, EvalError, ReferenceError, SyntaxError, URIError, InternalError | 0 | | | (thrown by the engine) |
| DOMException, atob, btoa, eval, queueMicrotask, URI functions, escape, unescape, parseFloat, isNaN, globalThis | 0 | | | 0 |

What the runs touch beyond what scripts name: every run touches the array iterator
(`%ArrayIteratorPrototype%.next`, `Array.prototype.values`), `Promise.prototype.then`,
`Promise.resolve` and `Promise[Symbol.species]` through `await`, and the JSON functions.
The corpus prelude adds `Map`, `Object.create`/`defineProperty`/`freeze`/`keys`,
`Function.prototype.call`/`bind`, `Error` and `TypeError`. Regex methods reach
`RegExp.prototype.exec`, the `Symbol.match`/`replace` methods and all flag getters.
Generators, `Proxy` and `Reflect` appear only in the hand-written codemode cases that
test them.

**Members never named and never touched** in kept families (by name, so a strict
list):
- Array: `copyWithin`, `every`, `findLast`, `keys`, `reduceRight`, `reverse`, `unshift`,
  `with`, `toReversed`, `toSorted`, `toSpliced`, `of`, `fromAsync`.
- String: `charAt`, `charCodeAt`, `codePointAt`, `substring`, `toUpperCase`, `search`,
  `fromCharCode`, `fromCodePoint`, `raw`, `normalize`, `isWellFormed`, `substr`.
- Object: `assign`, `getPrototypeOf`, `getOwnPropertyDescriptor(s)`, `setPrototypeOf`,
  `hasOwn`, `is`, `groupBy`, `seal`, ...
- Date: every `get*`/`set*` method except `getUTCDay`, and `UTC`, `toDateString`,
  `toLocale*String`, `toTimeString`.
- Math: all but `max`, `min`, `floor`, `round`, `abs`, `ceil`, `random`.
- Promise: `any`, `reject`, `try`, `withResolvers`, `finally`, `allKeyed`.

Most of these are everyday JavaScript that models write all the time in longer programs
(`charCodeAt`, `toUpperCase`, `Object.assign`, Date getters, `Math.sqrt`). Codemode
scripts are short glue between tool calls, which is why they don't show up here. They
are not cut candidates (section 4).

## 3. What pi's prelude needs

| | with lockdown (today) | without lockdown, frozen builtins (D2304) |
| --- | --- | --- |
| `tools` object | `Proxy` with a `get` trap, `Reflect.get` | same |
| collections, promises | `Map`, `Set`, `Promise`, `new Promise` | `Map`, `Promise` |
| `Object` | `defineProperty`, `freeze`, `keys`, `create`, `entries`, `getOwnPropertyDescriptor(s)`, `getPrototypeOf` | `defineProperty`, `freeze`, `keys`, `create`, `entries` |
| reflection | `Reflect.ownKeys` | – |
| hidden prototypes | `function*`, `async function*`, `Int8Array`, the array/Map/Set/string/regexp-string iterators via `Symbol.iterator`/`Symbol.matchAll`, `Iterator` (guarded) | – |
| other | JSON, `Error`/`TypeError`/`RangeError`, `instanceof`, string and array methods, `Function.prototype.call` | same |

- **Slim A runs pi's lockdown-free prelude.** On the A prototype, it starts, calls two
  tools in parallel, prints, gives the misspelled-tool hint and returns. It fails with
  the lockdown (`ReferenceError: 'Int8Array' is not defined`).
- **Slim B fails** at `new Proxy` in either form.
- **p2** runs the same prelude through pi-codemode (`CodemodeSandbox`). Its extra host
  globals (the `models` namespace, discovery) are host functions, not builtins.
- So the slim module presumes the pi change 0144/0146 already propose: frozen builtins
  on, lockdown gone. With lockdown gone, pi's prelude needs nothing that set A drops.

## 4. The proposed global set

Tiers: **required** (scripts, pi's prelude or the engine itself need it), **keep**
(cheap, or models expect it in general even though this sample doesn't show it), and
**drop**. "Risk" is how likely LLM-written codemode scripts in general are to reach for
the family, judged from what models write in tool-glue code, not only this sample.

| tier | family | why | image (bytes) |
| --- | --- | --- | ---: |
| required | Object, Function, Array, String, Number, Boolean, Math, JSON, Promise, Error and the native errors, Map, Set, Date, RegExp, `globalThis`, `parseInt`/`parseFloat`/`isNaN`/`isFinite`, the iterator and array/string/Map/Set/regexp-string iterator prototypes, async functions | scripts, the protocols behind for-of/`await`/templates, pi's prelude, errors the engine throws | |
| required for pi today | Proxy, Reflect | the `tools` Proxy (92% of scripts read `tools.x` through its trap) | 21,144 |
| keep | Symbol (well-known symbols are engine-internal anyway), WeakMap/WeakSet, generators and async generators, classes, AggregateError (`Promise.any`), URI functions, `atob`/`btoa` and DOMException, `queueMicrotask`, `eval`/`Function` | cheap; core syntax or library models do write; generators share the suspension machinery with async functions; `atob`/`btoa` for base64 payloads | 3–8 KB each |
| **drop** | typed arrays, ArrayBuffer, DataView (incl. Float16Array, Uint8Array base64/hex) | never used; no `TextEncoder`/`Buffer`/`crypto` in codemode to pair them with; tool data arrives as JSON | 44,048 |
| **drop** | Iterator helpers and the `Iterator` global (`Iterator.from`, `.map`/`.filter`/`.take`/..., `concat`, `zip`) | ES2025; pi's prelude already guards it; array methods cover the use | 15,040 |
| **drop** | DisposableStack, AsyncDisposableStack, SuppressedError, `using` syntax | ES2026; never used | 10,080 |
| **drop** | decorators (TC39 and TypeScript legacy) | scripts are JavaScript; never used | 7,184 |
| **drop** | BigInt (global, literals) | never used; tool results are JSON (no BigInt) | 2,440 |
| **drop** | WeakRef, FinalizationRegistry | never used; pi's deterministic mode already deletes them (D1680) | 2,056 |
| **drop** | `\p{…}` in regexps, full normalization (cargo features) | never used; `normalize` keeps NFC quick checks, `localeCompare` compares code points | (code and tables) |
| optional | `Promise.allKeyed`/`allSettledKeyed`, `Map`/`WeakMap.prototype.getOrInsert(Computed)`, `Math.sumPrecise`/`f16round`, `RegExp.escape`, `JSON.rawJSON`/`isRawJSON`, `Error.isError`, `Array.fromAsync`; Annex B `escape`, `unescape`, `substr`, `getYear` | proposals newer than models' training data, and legacy | 9,392 together (measured) |

What a script sees for a dropped family:
- **Globals:** `ReferenceError: 'Uint8Array' is not defined`; `typeof Uint8Array` is
  `"undefined"`, so feature tests adapt.
- **Methods on kept objects:** `[].values().map(...)` throws `TypeError: not a
  function`.
- **Syntax:** `10n`, `using x = ...` and `@dec` would be a SyntaxError of the whole
  script that names the feature, for example "BigInt literals are not supported in
  codemode" (the prototype keeps BigInt literals working).
- **Messages:** a slim build should say "not available in codemode" instead of the
  generic messages, so the model's next turn doesn't retry. That needs a small table of
  dropped names in the engine's ReferenceError path (est. <1 KB).

Risk, from most to least likely to be missed:
- **Typed arrays.** Base64 or binary decoding, checksums, image bytes; models reach for
  `Uint8Array` with `atob` in those tasks. Codemode has no `TextEncoder` or `Buffer`, so
  such scripts already need workarounds today.
- **BigInt.** 64-bit ids and large counters, rare in tool glue.
- **Iterator helpers.** Newest, rare in model output.
- **`using`, decorators, weak references.** Practically never in short scripts.

Every dropped family costs at most one failed turn with a clear message. No script in
the sample would have failed.

## 5. What dropping buys

### Module size

| build | module | gz | Δ gz | code gz | data gz |
| --- | ---: | ---: | ---: | ---: | ---: |
| main | 1,064,203 | 458,952 | | 341,266 | 115,015 |
| Unicode features off (`regexp-unicode-props`, `unicode-normalization`) | 997,690 | 424,423 | −34,529 | 334,982 | 86,452 |
| syntax cut (decorators, `using`) | 1,059,278 | 457,165 | −1,787 | 339,487 | 114,917 |
| A image (set A pruned from the base) | 1,049,387 | 444,190 | −14,762 | 341,293 | 100,126 |
| A image + leaf arms | 1,023,913 | 436,274 | −22,678 | 333,264 | 100,021 |
| **A full** (+ Unicode off + syntax cut) | **952,369** | **399,605** | **−59,347** | 325,171 | 71,575 |
| B full (A + Proxy, Reflect) | 945,775 | 395,626 | −63,326 | 324,107 | 67,798 |

The Unicode saving is now 34.5 KB, against 0130's 40.7 KB: the tables have been packed
since (D2334, D2337).

**Not removed by the prototypes:** code that the remaining engine still reaches. These
are line-table bytes of the files' own code (raw); inlined generic code comes on top:

| what | own code (raw) | where | est. gz once cfg'd |
| --- | ---: | --- | ---: |
| BigInt arithmetic, conversion, the interpreter's BigInt branches | 12,141 | bigint/, interp/run/bigint.rs, heap/bigint.rs | 4–5 KB |
| typed-array storage and element access | 5,486 | heap/typed.rs (and element paths in property code) | ~2 KB |
| decorators and TypeScript metadata left after the syntax cut | ~4,000 | compiler/ts_meta.rs, class.rs paths | ~1 KB |
| weak references in the heap and collectors | 1,774 | heap/weakref.rs | ~0.7 KB |
| Proxy (set B only) | 2,718 | interp/run/proxy.rs, heap/proxy.rs | ~1 KB |

Set A with real cargo features would therefore land at about 390 KB gz, 1.36× quickjs-wasi.

### Heap and VM

| | main | A | A + optional | B |
| --- | ---: | ---: | ---: | ---: |
| base image (every VM's prefix) | 325,008 B | 238,712 B (−26.6%) | 229,320 B | 219,640 B |
| cells | 5,852 | 4,309 | 4,140 | 3,987 |
| builtin functions dropped | | 298 | 326 | 359 |

- **`vm_new`** copies the base, so it scales with it: ~3.9 µs to ~2.9 µs. That is noise
  for codemode.
- **Per-VM memory:** −86 KB of arena per VM. A pi session on frozen builtins without
  lockdown would start at est. ~290 KB instead of ~375 KB (D2304's 375.6 KB minus the
  prefix difference).
- **Snapshots:**
  - Session images and containers leave the prefix out (D2066, D2400), so stored
    snapshots barely change: 2–8 KB for waiting pi sessions either way.
  - Raw images (`Vm::bytes`, dumps) lose 86 KB.
- **Pre-created result shapes** (D2067): none of the chains in
  `BUILTIN_RESULT_SHAPES` belongs to set A. B drops `Proxy.revocable`'s `{proxy, revoke}`
  chain (about 100 bytes, still counted in B's numbers above).
- **Collector:** the prefix is never marked or moved (D2064), so a smaller prefix saves
  only the old-region walk over its slots (~1% of instructions after D2055), in
  proportion.
- **Interpreter speed:** unchanged. Dropped families are not on any hot path the kept
  workloads use.

## 6. Flag or build

| mechanism | removes code | smaller base and prefix | memory per VM | cost |
| --- | --- | --- | --- | --- |
| VM option that deletes globals (like `WeakRefs::Absent`) | no | no (the prefix is the shared base; the global object is per-VM) | no | ~0; semantics only |
| two coded bases in one module, option picks one | no | yes | yes | +42.9 KB gz data: the slim base, coded (D2252), measured; the full one is 57.7 KB |
| slim base derived at module instantiation (prune, then prepare) | no | yes | yes | needs `Vm::new`'s prelude compile path in the module (the `compiled-prelude` feature: the prelude source ships too) and ~10 ms per instantiation (native: Vm::new 6.5 ms + prune and prepare 3.3 ms). Pruning a VM made *from* the base frees nothing, because collections never move or free the prefix (D2064) |
| **cargo feature `slim`, second module** | **yes** | **yes** | **yes** | a second build and gate leg |

Only a build gets the code and the data. The useful parts of a flag are already there or
cheap:
- **frozen builtins** (D2304) stays a VM option in either module. The slim module is
  meant to run with it on.
- **deleting globals at run time** (`vm_set_weak_refs(absent)`) remains the way to hide a
  family per VM in the full module, for hosts that want the semantics without a second
  module.

Interactions:
- **Frozen builtins and the immutable prefix:** unaffected. The slim base is a prefix
  like any other: BASE_END, the tail, and the write barrier work the same.
- **Registry (D2371):** one append-only table. The slim build's offset table
  (build.rs) maps the dropped ids to "absent", and the registry test checks only the
  families the build has. Exports name builtins by id, so an export from a slim VM
  imports into a full build (its ids are a subset). The reverse fails on the first absent
  id, with that id's path in the message.
- **Snapshots:** a slim VM's image never restores into a full module or vice versa. The
  build hash differs and the session image's prefix hash differs (D2066), and the
  container check gives "built by a different module" today without any change.
- **Determinism:** each module is deterministic on its own. Nothing about laziness or
  order is introduced.
- **Intrinsic ids:** keep the table identical in both builds (ids are part of the
  bytecode format). The slim build's arms for absent leaves fail with an internal error,
  and no slim bytecode can reach them.

## 7. Impact on the wasm binding

- **djs-wasm:**
  - Same exports and ABI. A `djs_features()` export (or a bit in the existing build info)
    tells a host which module it loaded. Optional: the build hash already distinguishes
    the modules.
  - `vm_new_with`'s frozen flag stays. A slim module could default it to on, but explicit
    is simpler.
- **Wire format** (docs/wire.md): the `0x30` bytes tag decodes to a `Uint8Array`, which
  the slim module doesn't have.
  - The slim encoder never produces it; decoding it fails with a clear error.
  - js/src's heap reader reads typed arrays only if the heap has them.
- **js/src Sandbox:** `loadModule(location)` already takes a module path. Ship
  `djs-slim.wasm` next to `djs.wasm`.
  - The Sandbox should keep the full module by default: its users pass bytes through
    host functions.
  - Snapshots (`bytes()`/`fromBytes()`) are bound to the module that made them, as
    today.
- **js/pi adapter:** loads `djs-slim.wasm` with `frozenBuiltins: true` and pi's prelude
  without `lockdown()`. The prelude image key already includes the build and the frozen
  flag.
  - pi's own tests run against it (`node js/pi/pi-tests.ts` with a module option).
- **pi-private (the owner's call):** drop `lockdown()` from PRELUDE_SOURCE when running
  on djs, or guard its `Int8Array` and `Iterator` uses with `typeof`. Set B would also
  need `tools` as a plain object; it would lose the misspelled-tool hint unless the
  engine grows a hook for it.

## 8. Testing a slim build

test262 can't run unfiltered on it: the dropped families take ~5,900 test files with
them (0130: typed arrays ~3,000, BigInt 1,506, Iterator helpers 654,
`using`/DisposableStack ~400, weak references ~300). What would test it:
- **test262 filtered by feature tag** (`TypedArray`, `ArrayBuffer`, `DataView`, `BigInt`,
  `iterator-helpers`, `explicit-resource-management`, `decorators`, `WeakRef`,
  `FinalizationRegistry`, `Float16Array`, `uint8array-base64`, `Array.fromAsync`, ...),
  with the full build's expected-failure list for the rest. Untagged tests that use a
  dropped builtin go on a short slim-only list.
- **difftest:** `codemode/` and `codemode-harvest/` in every mode. The A prototype already
  passes all 310 identically. The rest of the corpus gets a `// slim: skip` directive
  where it uses a dropped family.
- **pi's tests** through js/pi on the slim module, with frozen builtins and the
  lockdown-free prelude.
- **A surface test:** the slim global object's keys equal the tier list, and each dropped
  name gives the intended error.
- **A size budget of its own** in tests/scores/size.tsv (for example a ceiling of
  ~400 KB gz).

The default module keeps the whole gate unchanged.

## 9. Recommendation

1. **Now, no engine work:** build pi's module with `--no-default-features` on djs-core
   (−34.5 KB gz, 1.48× quickjs-wasi, under the D020 budget). Propose the pi-private
   change: frozen builtins on, lockdown off. That change is also what 0146 measured as a
   6–7× faster pi start.
2. **`slim` cargo feature, phase 1 (~1.5 weeks):**
   - bgen/djs-prelude leave the set-A builtins/*.ts files out of the slim prelude;
   - the realm skips their prototypes;
   - the Rust leaves of those families are cfg'd out;
   - the decorator and `using` syntax gives a SyntaxError that names it;
   - clear "not available in codemode" messages;
   - a second wasm build, the surface test, the codemode corpora and pi's tests on it.

   Gives ~−24 KB gz beyond step 1 (prototype: 399.6 KB) and an 86 KB smaller prefix per
   VM.
3. **Phase 2 (~2–3 weeks):**
   - BigInt out of the lexer, the numeric operators, JSON and the wire;
   - typed arrays out of the heap and the property paths;
   - test262 filtered by feature for the slim module.

   Est. −8–10 KB gz more, ~390 KB, 1.36×.
4. **Not recommended:**
   - a run-time "slim" flag (saves nothing but globals);
   - trimming everyday members of kept families (the sample is short glue code; models
     use them in general);
   - dropping Proxy/Reflect (−4 KB gz, −19 KB prefix) unless pi rewrites `tools`
     anyway.
5. **Optional, cheap:** the newest-proposal and Annex B members in section 4 (−9.4 KB of
   prefix, ~1–2 KB gz). Decide together with what djs's test262 profile promises.

Risks:
- **The sample.** One owner, five weeks; p2 contributes only 25 runs. Data-heavy users
  would miss typed arrays first. Re-run the analyzer on other stores before freezing the
  list (section 10).
- **Two modules to keep building and testing.** That adds a gate leg of about a fifth of
  the current one: a wasm build, the corpora, pi's tests and a filtered test262.
- **pi depends on a pi-private change** (lockdown off) before it can use the slim module.

## 10. Reproduce

```sh
# sessions: agent store, plus a copy of the p2 databases (never the rest of ~/.pi/p2)
mkdir -p work/scratch/0149/p2db && cp ~/.pi/p2/sessions/*.sqlite* work/scratch/0149/p2db/
scripts/harvest/extract.py work/scratch/0149/extract-agent.jsonl
scripts/harvest/extract.py work/scratch/0149/p2db work/scratch/0149/extract-p2.jsonl
cat work/scratch/0149/extract-{agent,p2}.jsonl > work/scratch/0149/extract.jsonl
cargo build -p djs-cli
python3 scripts/harvest/features.py work/scratch/0149/extract.jsonl work/scratch/0149/features.jsonl
python3 scripts/harvest/features.py --summary work/scratch/0149/features.jsonl
# by member, with the touched-builtin counts of the instrumented corpus runs
python3 scripts/harvest/members.py work/scratch/0149/features.jsonl \
    --touched work/scratch/0149/touched-names.tsv [--families]
# size prototypes (work/scratch/0149: slim_patch.py, slim-env.sh, p-*.py)
OUT=work/scratch/0149/variants scripts/size/variant.sh a-full work/scratch/0149/p-a-arms.py
cargo run --release -p djs-prelude --example base-families   # in a patched scratch copy
```

`touched-names.tsv` comes from a scratch copy with touch.py applied and a small
`touched-names` example that maps each touched base offset to
`djs_prelude::registry::id_of_base_offset` (functions' templates to their function).
