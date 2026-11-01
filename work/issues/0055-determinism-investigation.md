# 0055: Determinism investigation (report only; becomes longer-term issues)

status: done
agent: determinism
branch: issue/0055-determinism
depends-on: -
touches: work/issues/0055-determinism-investigation.md (the report), work/scratch/0055/ (experiments), scripts/ or tests/ (new experiment scripts only). No changes to engine code.

## Goal
Codemode needs a high level of determinism. External effects (tool results, their
contents) cannot be deterministic, but everything else should be: given the same script,
the same engine build, and the same sequence of external inputs (tool results and the
order in which they arrive, the clock values and seeds the host supplies), djs must
produce the same observable behavior, the same outputs, and ideally byte-identical heap
images/snapshots. That should hold across runs, across machines and native vs wasm, and
across snapshot/restore and suspension at any point.

Investigate what is still needed in the design and the implementation to make that fully
true, with particular attention to promises and the job queue: the order of reactions,
and how the settlement order of concurrent async host calls interacts with the
microtask queue (pi's deterministic mode delivers results in a fixed order; see below).

## Context
- ARCH "Nondeterminism": `Math.random` seed, `Date.now`, `performance.now` come from host
  imports or heap state. Done so far: `Vm::set_now`, `set_time_zone_offset` (D1383),
  `set_random_seed` (D1314, D1491), and no clock or entropy of djs's own.
- pi's codemode has a deterministic mode: `~/Development/pi-private/packages/codemode/src/
  runtime/vm.ts` (read-only), top comment and `deterministicWasi`, `VirtualClock`,
  `DETERMINISTIC_VERSION`, and how results are applied in order ("the next result to
  apply, and results that arrived before it"). infra's adapter (js/pi/, D1490–D1495,
  docs/host.md "pi codemode") maps it onto djs.
- The GC is deterministic (D1090: collecting twice gives identical bytes), and the base
  image is deterministic (0032). The `gc_pending` threshold depends on heap size.
- WeakRef/FinalizationRegistry are being added (heap, 0051): they make GC timing
  observable.

## What to deliver
- [x] An inventory of every source of nondeterminism or platform dependence, checked
      against the code, each classified (already deterministic / deterministic given host
      inputs / a gap). Candidates (not exhaustive):
  - the job queue and promise reaction order; async host call settlement order and
    timing relative to fuel slices (does resolving a call between two `run` slices give
    a different interleaving than resolving it inside the same slice?); `Idle`
    handling; host call ids;
  - fuel accounting: identical across native/wasm and opt-levels? Do fuel costs depend
    on anything but the bytecode (for example GC work, string flattening, rope depth,
    hash table probes)? Where the embedder suspends (the fuel slice size) must not
    change the observable behavior;
  - GC: when collections happen (thresholds based on heap size, the arena growth
    policy), and what is observable about them (WeakRef deref,
    FinalizationRegistry callbacks, memory-limit OOM points, performance);
  - heap images: byte-identical given the same history? Across native and wasm (0032
    showed the base matches)? Across a snapshot/restore in the middle of a run?
  - floating point: libm (pure Rust, so the same on every platform?), NaN
    canonicalization and NaN payloads observable through typed arrays / DataView, `-0`,
    fused multiply-add or x87-style differences in wasm vs native, `Math.*` results;
  - number/string conversions, `Array.prototype.sort` stability and comparator call
    order, `Object.keys` order, Map/Set order (spec-defined), hash seeds (are any hash
    functions seeded randomly? HashMap with RandomState anywhere in djs-core?), atom ids;
  - Date (time zone, DST: fixed offset), `toLocaleString`, `localeCompare`, `normalize`;
  - stack traces, error messages, `Function.prototype.toString`;
  - RegExp backtracking limits and fuel (input-size dependent but deterministic?);
  - memory limits: is the OOM point deterministic for a given input?
  - anything the host can observe differently (timing aside).
- [x] Experiments where cheap: run the same scripts natively and in wasm, with different
      fuel slice sizes and suspension points, with and without GC stress, and compare
      outputs and heap images byte for byte (the e2e harness and the difftest stress
      modes do part of this; extend them in scratch).
- [x] Design proposals for the gaps: for example a deterministic job/host-call delivery
      contract (results enqueued only at well-defined points, ordered by call id or by
      delivery order), deterministic GC triggering (allocation-count based instead of
      heap-size based?), a "deterministic mode" flag in the VM that turns off or pins
      anything else (WeakRef liveness only at defined points, and so on), a replay journal
      format, and a determinism test mode in the gate (run every corpus case twice with
      different slice sizes and compare the images).
- [x] **A list of proposed issues**, each with a title, goal, acceptance criteria and
      dependencies, ready for the coordinator to turn into work/issues files. Don't
      create issue files yourself.

## Notes
- Your own worktree for experiments (`.worktrees/determinism`).
- Read AGENTS.md "Testing while iterating" first. The machine is shared: use the 6-job
  cap and filtered runs.

## Log

- Read ARCH (Nondeterminism, Host Interface, Snapshots), compiler.md 7.5–7.8, host.md, the
  decisions listed in the assignment, pi's `runtime/vm.ts`, `inline.ts`, `protocol.ts`
  (read-only), js/pi/vm.ts, js/src/sandbox.ts, and the engine code for the job queue, host
  calls, GC, arena, fuel, typed arrays, hashing and the build hash.
- Wrote experiment tools, committed in `scripts/determinism/` (outside the workspace,
  not in the gate; README there): `detprobe` (one script under many embedder schedules:
  fuel slice, restore at every stop, GC stress, eager vs idle async delivery; compares
  outputs, raw images and collected images), `abidrive` (native) + `probe/abidrive.mjs`
  (wasm) making identical export calls, and node scripts for the Sandbox and pi adapter.
- Rebased onto main after 0060 (D1600 OOM reserve, D1601 GC threshold in the image) and
  0051 (D1513 WeakRef/FinalizationRegistry) and re-ran everything; results below are
  from after the rebase unless marked "before 0060".
- Told heap (during 0051) that weak-ref cleanup needs a host-controlled mode; D1513's
  `finalization_after_gc` flag is that.

## Findings

### Summary, ranked by importance for codemode

1. **pi adapter: cold and warm starts poll a different number of times.** pi's inline and
   remote launchers count `shouldInterrupt` polls over the whole execution, and the djs
   adapter also polls while it runs the prelude. A start that reuses the prelude image
   skips those polls: 678 polls cold vs 540 warm for the same deterministic run
   (`pi_polls.ts`). A budget between those numbers times out on one and not the other.
   A deterministic replay after a restart is a cold start, while the original run was
   probably warm. Trivial fix in js/pi.
2. **Async host call results are applied wherever the host happens to resolve them.**
   `resolve_host_call` queues the settle job at once, in any VM state (D1374). The
   Sandbox passes results back at the top of its loop: after OUT_OF_FUEL, after a sync
   host call, and at IDLE. So **the fuel slice size changes the script's output**:
   `sandbox_interleave.ts` prints `b1, b2, a 2` with `fuelSlice` 1e6 and `b1, a 2, b2`
   with 1000, when the async function returns a value synchronously. The same happens
   when an async result arrives while the Sandbox awaits a sync host function's promise.
   Reproduced in core with `detprobe js/async_interleave.js --async-policy eager`.
   pi on djs is *not* affected today: its bridge is a sync host function, and the prelude
   settles its own promises in `settle`, only at idle, one result per drain, in `seq`
   order. Any host that uses djs's async host functions (a native codemode integration,
   the Sandbox) is affected.
3. **WeakRef/FinalizationRegistry exist now (D1513), and pi's deterministic contract
   says they are absent.** quickjs-wasi's deterministic mode removes them
   (`Intrinsics.ALL & ~WEAK_REF`), and pi's test checks it. The adapter does not delete
   them yet, and its prelude-image cache key does not include the mode. In djs they are
   deterministic under slicing and restore (`js/weakref.js`: same output for every
   slice, every restore, and even gc-stress for that case). They are still observable
   through the memory limit (the threshold depends on the cap), through gc-stress, and
   through the host's `Vm::collect`, which collects and queues cleanup jobs.
4. **OOM: host-refused memory looks like the deterministic limit OOM.** D1600/D1601
   made the limit OOM deterministic (verified below). But when the host allocator
   refuses (arena growth `try_reserve_exact` fails, or the GC's to-space reservation
   fails) the result is the same catchable `InternalError: out of memory`, or a skipped
   collection (`safepoint_collect` ignores `collect()` errors), which moves every later
   GC and OOM point. On wasm32 the default cap is 4 GiB, while from-space plus to-space
   plus Vec doubling needs about 2–3x the cap of address space. The OOM point then
   depends on the host's memory. Also, a single large allocation still fails even when
   collecting would make room (no GC-and-retry at the failing allocation): `oom.js` at a
   2 MB cap prints `.EEEE…`, although the live data is under 1 MB. That is
   deterministic, but it is a spurious OOM.
5. **Clock, time zone and seed in the Sandbox.** The Sandbox sets `Date.now()` before
   every fuel slice (default `Date.now`), so `Date` depends on slice boundaries unless
   `now` is a fixed number. It has no seed option at all: every Sandbox starts from the
   same fixed `Math.random` seed (deterministic, but a footgun for non-deterministic
   use). It also has no deterministic mode that bundles these (pi's adapter has its own).
6. **Raw images still depend on the fuel slice size** (not visible to JS):
   - `Exec::run` collects at its start if `gc_pending` (`interp/run.rs`, `self.safepoint()`
     before the loop), so a pending collection runs a few instructions earlier when a
     slice ends between the allocation and the next natural safepoint
     (`alloc_only.js`: slice 101 gives a different raw image than max/100003/997);
   - suspended intrinsics allocate their progress records only when they suspend
     (regexp `save_exec_state`, D1332 `suspend_in`): e2e `regexp.js` has 4 different raw
     images over 5 schedules, and `regexp_props.js` and `iteration_runtime.js` have 2 each.
   The images after a collect are identical in every case. Today this changes only the
   bytes of an image, raw value bits seen by the host, and (in theory) where the GC runs
   relative to an OOM. A journal that hashes raw images would see it.
7. **Fuel is not additive across slices.** A slice ends with negative fuel after an
   expensive op, and the overshoot is forgiven in the next `run`. A resumed intrinsic is
   charged its op cost again. So the total fuel for a script depends on the slice size
   (floats_body: about 3.76 M at slice 997 vs 3.64 M at 13). Budgets are therefore only
   reproducible for a fixed slice size. pi includes `fuel per poll` in its fingerprint,
   and the adapter restarts the slice after every bridge call (deterministic). Not
   visible to JS.
8. **Build identity.** `BUILD_HASH` covers the djs-core sources and the prelude, but not
   cargo features (`regexp-unicode-props` changes behavior: `\p{..}` works or is a
   SyntaxError) or the `libm` version (Math results). The pi fingerprint does not hash
   the adapter's `HOST_GLOBALS`; it relies on a manual `DETERMINISTIC_VERSION` bump.
9. **Float32 NaN encoding** (`heap/typed.rs` `encode`: `n as f32`) relies on the
   hardware/engine NaN propagation of f64→f32 demotion. The wasm spec allows
   nondeterministic NaN bits there. It was not observed (arm64, x86-64 and V8 all give
   `0x7fc00000`), but an explicit `if n.is_nan()` costs nothing.
10. **Host-visible but not JS-visible:** raw value bits and offsets in `vm_host_call` /
    `vm_handle_bits` differ between schedules (abidrive's `logs` hash of raw argument
    bits differs between slice sizes); `vm_promise_state` marks a promise handled, so a
    host that inspects promises changes unhandled-rejection reports; unhandled rejections
    are judged whenever the host checks (the Sandbox checks at IDLE/DONE).

### Inventory

Legend: **D** already deterministic; **H** deterministic given host inputs; **G** a gap
(number = rank above).

| source | class | evidence |
| --- | --- | --- |
| Job queue, promise reaction order | D | FIFO `ValueArray` in the heap (D1370); reactions in registration order (D1371); e2e promise_* identical in all modes |
| Async host-call delivery position | **G2** | `resolve_host_call` queues at once in any state; Sandbox output depends on `fuelSlice` |
| Several results ready at once (batch vs one at a time) | **G2** | the Sandbox resolves all `finished` before one `vm_run`; pi applies one, drains, then the next: different interleavings |
| Host call ids | D | `Interp::NEXT_CALL_ID` in the heap; `vm_new_host_calls` oldest first; the table keeps order on removal |
| `Idle` / `Done` | D | Done only when queue empty and no async call pending (D1370) |
| Unhandled-rejection reporting | H / G10 | judged when the host takes them; deterministic if the host checks at the same points |
| Fuel per op | D | the `OPS` cost table plus length-based extras (strings, bigints, regexp steps, frame copies); nothing depends on layout, GC or hash probes |
| Fuel native vs wasm vs opt-level | D | identical stop counts for arm64 (opt 1), x86-64 under Rosetta (opt 1, before 0060) and wasm (opt s) at slices 1e6/997/13 |
| Fuel vs slice size | **G7** | overshoot forgiven; resumed intrinsics charged again |
| GC trigger / threshold on restore | D (since D1601) | before 0060: restoring at every slice ran `alloc_only.js` to 10.9 MB instead of 1.7 MB and changed OOM outcomes (`oom.js` at 2 MB: `....` vs `..E.E.` vs uncaught). After: identical raw images and outputs with and without restore |
| GC at the start of `run` | **G6** | raw image differs at slice 101 (`alloc_only.js`) |
| Suspension records | **G6** | regexp / intrinsic progress cells exist only when a slice ends inside them |
| GC layout | D | Cheney BFS: collected images identical across slices, restores and gc-stress for all 62 e2e cases and every probe workload |
| Map/Set hashing by offset | D | rehash after GC (gc.rs); iteration in insertion order; probe counts are not charged |
| Hash seeds | D | none: no `RandomState`/HashMap in non-test djs-core code; FNV and a fixed mix |
| Atoms | D | content-hashed heap table, never keyed by offsets |
| OOM at the limit | D (since D1600/D1601) | `the_out_of_memory_point_is_deterministic`; `oom.js` same at all slices/restores (1.5/1.7/2/2.5/3 MB) |
| OOM from the host allocator, GC to-space refusal | **G4** | same catchable error, or a silently skipped collection |
| Spurious OOM with collectable garbage | G4 (quality) | `oom.js` at 2 MB: `.EEEE…` |
| WeakRef / FinalizationRegistry | H / **G3** | D1513: after collections (or when the host schedules); a function of program + inputs + cap; `Vm::collect` by the host is an extra input; pi's contract wants them absent |
| Memory reserve state | D | `Roots::FLAGS` (D1600) |
| Floating point arithmetic | D | IEEE ops, no FMA contraction in Rust, no fast-math |
| libm | D | pure Rust; `floats_body.js` (24 Math functions × 18 inputs, pow/atan2/hypot/%) identical on arm64, x86-64 and wasm |
| NaN canonicalization | D | `Value::number` canonicalizes; NaN written through `Uint8Array` and re-stored comes out canonical in Float64/Float32/DataView |
| Float32 NaN demotion | G9 (hardening) | `n as f32` |
| Number ↔ string | D | own ryu/format/parse; `toFixed/toExponential/toPrecision/toString(radix)` identical across platforms |
| sort, `Object.keys`, Map/Set order | D | deterministic algorithm (sort.rs); spec orders |
| Date | H | `set_now`/`set_time_zone_offset` (D1383); fixed offset, no DST |
| Sandbox clock/tz per slice | **G5** | `setClock()` before every `vm_run` |
| `Math.random` | H | xorshift64* state in the heap (D1314); Sandbox has no seed option (G5) |
| `toLocaleString`, `localeCompare`, `normalize` | D | no Intl; fixed formats; `normalize` of non-trivial strings is a TypeError |
| Stack traces, messages, `Function.prototype.toString` | D | from heap data and host-given file names |
| RegExp limits and fuel | D | backtrack stack limit in words (`ExecLimits`), fuel per step, resumable |
| Compile / JSON / regexp nesting limits | D | counter-based; at each construct's maximum depth natively, the wasm build compiles the same source (`deep_search.py`); deep JSON/data/promise chains identical (`deep_body.js`) |
| Native vs wasm vs x86-64 images | D | byte-identical raw and collected images for `general_body.js` (with GC) and `floats_body.js` |
| Raw value bits seen by the host | G10 | differ between schedules |
| Build identity | **G8** | features and libm not in `BUILD_HASH` |
| pi: polls during the prelude | **G1** | 678 vs 540 |
| pi: snapshot/resume | D (since D1601) | `pi_resume.ts`: identical images at every call, uninterrupted vs `resumeVm` at every call (before 0060 they differed) |

### Design proposals

**P1. Host results are macrotasks (a host inbox in the VM).** The core contract becomes:
an async host call result is applied only at a *microtask checkpoint*, never in the
middle of synchronous code or of a job drain, and results are applied one at a time:
- `resolve_host_call` for an async call appends `{promise, ok, value, now?}` to a heap
  queue `Interp::INBOX` (in snapshots like everything else) instead of queuing the settle
  job. The order is delivery order; the host decides it and records it in its journal.
- When the microtask queue is empty, `run` takes the oldest inbox entry, sets the clock
  if the entry carries one, queues its `%HostCallSettle` job and drains the queue. Then
  it takes the next entry. `Idle` means: microtasks empty, inbox empty, async calls pending.
- **A rejection checkpoint after each macrotask:** promises still unhandled when a
  macrotask's drain ends move from `REJECTIONS` to a "reported" list that
  `take_unhandled_rejections` returns. So batching does not change the reports
  (HTML's "notify about rejected promises" at the checkpoint).
- Optional `resolve_host_call_at(id, result, now_ms)` (`vm_resolve_host_call_at`): a
  per-result clock, which is pi's virtual clock without the host stepping one result
  at a time.
This is what node/browsers do with I/O callbacks, and what pi's prelude does by hand.
Afterwards the script's behavior depends only on the *sequence* of results, not on
when they arrive relative to the VM's execution. No Sandbox change is needed for
correctness. Microtask starvation (`while (true) await null` never sees results) is the
same as in browsers.

**P2. A deterministic mode, as one option.** `VmOptions::deterministic` (plus
`Sandbox`/adapter options `deterministic: { startedAt, seed }`) pins everything that
is not an input:
- weak references: either absent (pi's contract: the adapter deletes the globals before
  the lockdown) or strong (`deref` always returns the target and cleanup never runs;
  the spec allows both). Either way `Vm::collect` by the host does not clear weak
  refs or queue cleanup. Default for pi: absent;
- `finalization_after_gc` off (D1513), with no host scheduling in deterministic mode;
- the clock changes only at macrotask boundaries (P1's per-result `now`) and at sync
  host call returns. The Sandbox stops calling `vm_set_now` per slice; time zone 0;
- the seed is required (no fixed default); the Sandbox gains `randomSeed` (default:
  entropy) for non-deterministic use;
- host allocator failure is not a catchable OOM (P3);
- an explicit memory cap no larger than what the host can always back (wasm32: at most
  about 1 GiB, so that cap + to-space + growth fits in the 4 GiB address space).

**P3. Limit OOM vs host OOM.** A new `HeapError::HostOutOfMemory` for `try_reserve`
failures and for a refused GC to-space reservation: the run ends with a new,
uncatchable status (as `vm_abort` after an interrupt; a `crash` in pi), instead of a
catchable InternalError or a silently skipped collection. Optionally reserve the
to-space up front in deterministic mode. Separately, as a quality issue: GC-and-retry at
the failing allocation where the op can restart (the interpreter's own allocations:
literals, arrays, string concatenation, `new Array(n)`), so `oom.js`-style spurious
OOMs go away. The retry happens at the failing allocation, which is a deterministic
point, so determinism is kept.

**P4. GC points independent of slicing.** Remove the collection at the start of `run`;
pending collections run at the next natural safepoint (backward jump, call, return,
D1600's handler). Host-side allocations (wire decode, `new_object`) set `gc_pending`
and wait for that safepoint like everything else. Make suspension records not count
toward the threshold (allocate them through a non-counting path, or keep one
preallocated progress cell per intrinsic call). Acceptance: raw images identical for
every slice size. Low priority: not visible to JS today. It matters for P6's image
hashes.

**P5. Fuel debt.** Keep the overshoot as debt in `Interp` (heap), subtract it from the
next `run`, and don't charge a resumed intrinsic's dispatch again. Then the total fuel
of a script is independent of slicing, and a budget in fuel units means the same at any
poll interval. Low priority.

**P6. Replay journal.** A small, engine-level format that hosts can adopt (pi keeps its
own seq/now protocol, which maps onto it 1:1):
```
header: {engine: BUILD_HASH + features, abi, options: {memoryLimit, maxStack,
         deterministic, weakRefs}, seed, startedAt, tz, script hash, params (wire)}
events, in order:
  {sync: call_id, fn_id, args_hash, result (wire) | error}       // position implied by order
  {async: call_id, result (wire) | error, now}                   // inbox delivery order
  {check: canonical_image_hash, issued_calls}                    // optional, at idle points
```
Replay = the same header, results fed in order. The checks compare the calls issued
(fn_id, args) and, optionally, `Vm::canonical_hash()`: a copy of the arena collected in
a scratch arena and hashed, which does not change the VM, so it never perturbs the run.
Raw value bits and offsets must never be journaled (G10).

**P7. A determinism mode in the gate.** Part 1 can land today (everything below already
holds):
- e2e: also compare the *collected* image hash of the final VM across plain, suspend and
  gc-stress (all 62 cases agree now);
- e2e/vm tests: an async host-call case resolved eagerly vs at idle and at several
  slice sizes (enable after P1), and a WeakRef case under restore;
- a native-vs-wasm image check: a Rust test writes final raw-image hashes for a few
  cases with the ABI driver to `target/`, and a node test runs `abidrive.mjs` against
  the gate's debug wasm and compares (the twin drivers in `scripts/determinism`);
- difftest `--determinism`: every case at two slice sizes with restore, comparing
  outputs and collected hashes (after P4, raw hashes).

**P8. pi adapter.** Don't count polls while the prelude runs (call the hooks' poller only
after `started`, or poll the prelude with a private no-op). In deterministic mode,
delete `WeakRef`/`FinalizationRegistry` in `HOST_GLOBALS` (before the lockdown) and add
the mode to the image-cache key. Hash `HOST_GLOBALS` into the fingerprint next to the
prelude hash. Update docs/host.md (the WeakRef rows are now stale).

**P9. Build identity.** Fold enabled cargo features and the `libm` version (from
`Cargo.lock`, read in build.rs) into `SOURCE_HASH`/`BUILD_HASH`, so a fingerprint or
image never claims the same engine for different behavior.

## Proposed issues

1. **pi adapter determinism fixes** (P8). Goal: deterministic pi runs behave the same on
   cold and warm starts and match pi's deterministic contract now that WeakRef exists.
   Acceptance: `scripts/determinism/pi_polls.ts` reports the same poll count cold and
   warm (as a js/test case); in deterministic mode `typeof WeakRef ===
   "undefined"` and the same for FinalizationRegistry, while non-deterministic runs have
   them; images are cached per mode; the fingerprint includes a hash of `HOST_GLOBALS`;
   pi's "runs in UTC … without WeakRef" test passes in `js/pi/pi-tests.ts`; docs/host.md
   updated. Touches js/pi, js/test, docs/host.md. Depends on: none.
2. **Async host results as macrotasks: the host inbox** (P1). Goal: an async host call's
   effect depends only on the order of results, not on when the host resolves them.
   Acceptance: `async_interleave.js` prints the same for eager and idle delivery at
   slices max/1000/97/1, with and without restore (vm tests), and
   `sandbox_interleave.ts` prints the same for every `fuelSlice` (node test); two results
   delivered in one batch behave like two separate deliveries, including
   unhandled-rejection reports (the checkpoint); an image with inbox entries restores and
   continues; optional `resolve_host_call_at` with a per-result clock; decision appended
   (amends D1374/D1370); docs/compiler.md 7.5 and host.md updated. Touches
   interp/run/promise.rs, interp/run.rs (next_job), vm/mod.rs, heap/cells.rs (Interp
   field), djs-wasm, js/src/sandbox.ts. Depends on: none.
3. **Deterministic mode for VM and Sandbox** (P2). Goal: one switch that pins
   everything that is not an input. Acceptance: `VmOptions::deterministic` (or a small
   options struct) selects absent or strong weak refs, no finalization scheduling, no
   weak clearing by `Vm::collect`; the Sandbox gets `deterministic: {startedAt, seed}`
   (clock only advances with results, tz 0) and `randomSeed` (default entropy; a test
   shows two default sandboxes now differ); a test runs a WeakRef + `Date` + `Math.random`
   script with explicit `Vm::collect`s at random points and gets identical output.
   Touches vm/, interp/realm.rs, builtins (weakref), djs-wasm, js/src. Depends on: 2.
4. **Host OOM is not a JS OOM** (P3, part 1). Goal: a catchable OOM always means "the
   cap was reached", a function of the program. Acceptance: `HeapError::HostOutOfMemory`
   from arena growth and the GC to-space; the run stops with a distinct status (wasm:
   a new status code; the Sandbox: `kind: "memory"`; pi: `crash`), never a skipped
   collection; unit tests with the existing `fail_next_gc_reserve` hook and a
   failing-reserve hook for growth; documented cap guidance for wasm32. Touches
   heap/arena.rs, heap/gc.rs, interp/run.rs, vm/, djs-wasm, js/src. Depends on: none.
5. **GC-and-retry at a failing allocation** (P3, part 2). Goal: no OOM while
   collecting would make room. Acceptance: `oom.js` at a 2 MB cap prints all `.`; the
   OOM-point determinism test of D1601 still passes; the e2e/difftest stress modes pass.
   Touches interp/run.rs and the allocating ops. Depends on: 4 (or with it).
6. **GC points and images independent of the fuel slice** (P4). Goal: byte-identical
   raw images for any slicing. Acceptance: no collection at the start of `run`;
   suspension records don't count toward the threshold or the arena limit (or are
   preallocated); detprobe-style vm test: raw image hashes identical at slices
   max/997/101/1 for `alloc_only.js`, `regexp.js` and `iteration_runtime.js`; the e2e
   suspend mode compares the final raw image with plain mode. Touches interp/run.rs,
   interp/run/json_regexp.rs, the suspend paths, heap/mod.rs. Depends on: none.
7. **Determinism gate checks** (P7). Goal: determinism is a tested property. Part 1
   now: collected-image equality across the e2e modes; the native-vs-wasm image check
   (the abidrive twins as a cargo test + node test in the gate); difftest
   `--determinism`. Part 2 after 2 and 6: async delivery cases and raw-image equality.
   Acceptance: the gate fails if a case's output or collected image differs between
   schedules or between native and wasm. Touches crates/djs-core/tests, js/test,
   crates/djs-difftest, scripts/gate.sh. Depends on: none (part 1); 2 and 6 (part 2).
8. **Replay journal and canonical image hash** (P6). Goal: hosts can record a run and
   replay it with verification. Acceptance: `Vm::canonical_hash()` (non-destructive,
   identical before and after an explicit collect); a journal format in
   docs/host.md; `djs replay` (cli) or a Sandbox `record`/`replay` that reproduces a
   run with async tools and verifies issued calls and hashes; a test that replays a
   recorded run after snapshot/restore at a different slice size. Touches vm/, djs-cli
   or js/src, docs. Depends on: 2 (and 6 for raw hashes; the canonical hash works
   without it).
9. **Fuel debt across runs** (P5). Goal: total fuel independent of slicing.
   Acceptance: the summed fuel of `floats_body.js` and `regexp.js` is identical at
   slices max/997/13 (a vm test using a fuel-used accessor); resumed intrinsics are not
   charged twice. Touches interp/run.rs, Interp cell. Depends on: none. Low priority.
10. **Build identity and float hardening** (P9, G9). Goal: the build hash changes
    whenever behavior can. Acceptance: features and the libm version are in
    `BUILD_HASH` (a test builds the hash input list); Float32 encode and `Math.fround`
    write the canonical f32 NaN explicitly (unit test with bit patterns). Touches
    crates/djs-core/build.rs, heap/typed.rs, interp/run/intrinsics.rs. Depends on: none.
    Small.

Order: 1 and 2 first (1 is small; 2 matters for any async-host-function codemode); 7
part 1, 4 and 10 in parallel; then 3, 5 and 6; 8 last; 9 whenever.

## Follow-ups

- docs/host.md "pi codemode" still says djs has no WeakRef (rows "deterministic" and
  "runs in UTC …"): stale since D1513. Proposed issue 1 covers it.
- ARCH "Nondeterminism" could name the inbox contract (P1) and the deterministic mode
  (P2) once they land.

## Result

Investigation done. The findings, the inventory, design proposals P1–P9 and ten
proposed issues are above. Experiment tools are in `scripts/determinism/` (README
there): they are not in the workspace or the gate, and no engine code was changed.
Headline: native arm64, x86-64 and wasm builds are already byte-identical in images
and fuel. Since 0060, snapshot/restore and slicing no longer change GC points or OOM
outcomes. The remaining JS-visible gaps are async host-result delivery timing (the
Sandbox's output depends on `fuelSlice`), the pi adapter's cold/warm poll difference,
WeakRef in pi's deterministic mode, and host-allocator OOMs that look like limit OOMs.
