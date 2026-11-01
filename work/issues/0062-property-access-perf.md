# 0062: Property access performance: shapes / inline caches (design first)

status: done
agent: heap
branch: issue/0062-property-access-perf
depends-on: 0060
touches: docs/ (a design note first), then crates/djs-core/src/heap/ (object layout), crates/djs-core/src/interp/ (caches in the dispatch loop), crates/djs-core/src/bytecode/ (cache slots per instruction, if needed), js/src/layout.generated.ts + js/src/reader.ts (if the object layout changes), crates/djs-bench/

## Goal
After 0035, property access is the main structural interpreter cost: a hash or linear
probe per access (0035 Result: "props" at 2.5x qjs; builtin-language code is all Get/Set
intrinsics). pi codemode scripts are 4x slower than quickjs-wasi in script work (0049),
"all in the interpreter". Find the right design for djs's constraints (a
position-independent arena, a copying GC, deterministic snapshots, the JS-side reader)
and implement it.

## Acceptance criteria
- [ ] Measurements first: where property access time goes now (lookup, the Get/Set
      intrinsic call overhead from the builtin language, prototype-chain walks for
      methods), on the engine track, a pi-like codemode script and test262's slow tests.
- [ ] A design note (docs/objects.md or a section in ARCH): options compared with
      estimated gains. For example: hidden classes/shapes as heap cells shared between
      objects (transition trees, a shape cache invalidated on GC moves?), monomorphic
      inline caches per instruction (cache = (shape, slot)), a cheaper per-instruction
      cache keyed by property-table identity, prototype method caches with a validity
      cell, and what each does to snapshots, the GC (are shapes rooted, collected,
      deterministic?), the reader (0021) and memory per object. **Send it to the
      coordinator before implementing**: this is an ARCH-level change.
- [ ] Then the implementation, in parts, with before/after numbers (engine track, pi bench,
      test262 CPU time), no semantic changes (ratchets, stress modes identical), and
      determinism kept (D1090, D1601).

## Log

- Measurements and design note: docs/objects.md (tools in work/scratch/0062: micro.py,
  classify.py, leaf.py, opcount.patch). Property access costs 10-30 ns vs ~1 ns in qjs,
  globals up to ~100 ns; but in a pi-like codemode script 97% of executed bytecodes are
  the builtin language and property lookup is 3.5% of the time. Proposal: cheaper lookups
  plus verified-index inline caches (globals, own properties, prototype methods with an
  epoch) now; shapes later as their own issue merged with 0052 S2 (memory/image), after
  0063. Sent to the coordinator for approval before implementing. Approved (globals
  first; caches in the heap; warm = cold restore test; stress modes identical).
- Part 1a, global caches (D1620): the `Cache` immediate on the six property/global ops
  (bytecode format, so OPS_HASH and the build hash change), FuncTemplate IC_COUNT/IC,
  interp/run/ic.rs, Heap::own_entry_index/data_at/set_data_at, realm GlobalLexEpoch.
  Tests: e2e global_caches.js (one qjs bug avoided: qjs keeps throwing after a
  read-only global becomes writable), vm/tests.rs global_caches_follow_every_change and
  warm_and_cold_inline_caches_behave_the_same. Measurements (LTO, cgu=1, interleaved,
  scratch ab.py/micro_ab.py): global read 10.4 -> 1.3 ns; test262 decodeURI test -13%;
  engine track within noise (float -4%: its time is the prelude Math functions). Found
  that default release builds vary ±15% between builds from layout (noted in D1620).
  Merged (main 09e5bc2).
- Part 1b, own-property caches: get_field/get_method/put_field hit an own data
  property through the site's slot in `fast_step` (`fast_field`); the slow path fills it
  (`lookup_ic`, PutField after `set_own_data_value`). Only non-index keys are cached, so
  an own property is always a table entry. Tests: e2e field_caches.js (14 receiver
  kinds incl. proxies, typed arrays, wrappers, frozen, getters; changes under the
  cache), warm/cold restore test extended. The machine is at load 25-30, so timings are
  CPU-time min-of-7 of interleaved LTO builds: engine/props -14..-18% (stable); own read
  ~6.5 -> ~1.7 ns, 4th of 4 14.7 -> 5.8, own write 15.7 -> 10.2, m.PI 9.4 -> 2.5; other
  engine benches within the noise (±10%). Merged (main 6d50b12).
- Part 1c, prototype caches (D1621): get_field/get_method on primitives and objects hit
  data properties on the prototype chain (receiver prototype offset + PROTO_EPOCH, chain
  marked OBJ_IS_PROTO, GC empties offset slots). Tests: e2e proto_caches.js (shadowing at
  every level, setPrototypeOf, getters, classes, primitives and patched builtin
  prototypes, typed array with a canonical numeric key, proxies), warm/cold restore with
  prototype slots, collect-twice identical images. Measurements (LTO cgu=1 builds,
  CPU-time min-of-7, load ~8-20): prototype read depth 1 13.7 -> 2.8 ns, depth 3 16.3 ->
  2.6 ns, method call p.f() 28.6 -> 20.1 ns; engine arrays -22%, higher-order -14%,
  calls -9%, loop -21% (the last two do not use properties: layout noise), props/float/
  strings/pi-like/decodeURI within ±3%. Merged (main 5c21213).

## Follow-ups

- Shapes + 0052 S2 (coordinator writes the issue from docs/objects.md), after 0063.
- `%Get`/`%Set` intrinsic calls from the builtin language have no caches (call-site
  caches would work the same way) and pay the full lookup; 0063 replaces the hot ones.
- Caching property additions (`this.x = v` in constructors) needs shapes.
- Getters on prototypes (`map.size`, `Symbol.toStringTag` accessors) take the slow path.
- Release-build layout noise of ±15% (D1620 note): benchmark recording might want an
  LTO/one-unit profile.

## Result

Landed in three merges (09e5bc2, 6d50b12, 5c21213): verified-index inline caches for
global reads and writes, own-property reads/writes/method loads, and prototype data
properties (D1620, D1621), with the `Cache` immediate on six ops (bytecode format
change), the per-template IC array, the realm's GlobalLexEpoch and Roots::PROTO_EPOCH.
Caches never change semantics (every hit re-checks the key), live in the heap, and a VM
restored with warm caches behaves like a cold one (tested). Collections empty the only
slots with offsets, so images stay byte-identical after collecting twice. Numbers:
docs/objects.md §6 (property accesses 2-10x faster; engine/props -14..-18%,
arrays/higher-order -14..-22%, Script-mode test262 -13%; codemode-like scripts within
noise, as the measurements predicted). Design and measurements for the next step
(shapes) are in docs/objects.md.
