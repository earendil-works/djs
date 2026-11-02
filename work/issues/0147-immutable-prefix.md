# 0147: Immutable builtin prefix: session-only images, hash, GC and determinism

status: done
agent: heap
branch: issue/0147-immutable-prefix
depends-on: 0144
guide: docs/guide/gc.md, snapshots.md, determinism.md, arena.md
touches: crates/djs-core/src/heap/ (roots, compaction, old region, canonical_hash, image write/read, verifier), crates/djs-core/src/vm/ (per-VM state moved out of the base region, prepare_base_image, seal), crates/djs-wasm/, js/src/snapshot.ts (images from the session region), crates/djs-cli/src/heap/ (analyzer), scripts/snapshot-research; perf (0146) is in interp/ and builtins/ write paths

## Goal
docs/frozen-builtins.md phase 2: the base image becomes an immutable prefix of every arena
(same offsets in every VM of a build). Everything written at run time lives above it:
- first, as an early small merge: a `Roots::BASE_END` field (the prefix boundary, distinct
  from OLD_END), set by prepare_base_image, so 0146 can build on it;
- move the per-VM state out of the prefix (0144's write audit: value stack, interp
  registers/roots/jobs/handles, the atom table's index and counters, lazily computed string
  hashes, prototype header flags, shape descriptor counts, the global object's property
  table, the realm slot): into the session region at vm_new, or computed eagerly at
  image build (string hashes, flags);
- a debug/test write barrier (Arena::write in debug and the gate's stress modes) that
  traps any write below BASE_END;
- images, snapshots (D2260 containers), canonical_hash, collections (never mark/move the
  prefix) and determinism defined over the session region only; restore pastes the
  session region onto the build's prefix; the format records the prefix's build hash;
- the analyzer (`djs heap`) understands the split and prints builtin references by
  registry id when 0145 lands.

## Acceptance criteria
- [x] Write barrier clean on the whole gate (e2e, difftest stress, test262 active, js tests,
      pi suite). Armed for the codemode corpora in every stress mode, and for frozen
      builtins; e2e and test262 change builtins on purpose and run unarmed (D2065).
- [x] Snapshot/image sizes, vm_new, GC time, wasm instructions and peak memory measured
      before/after (mem-perturb); no workload slower than +1% without a recorded trade.
- [~] Order-independence test: two runs touching different builtins in different orders
      give identical session images and canonical hashes. Holds for builtin calls whose
      results are primitives or plain arrays; shapes of objects builtins create are
      session state (coordinator: land as (B), shape work is 0148).
- [x] Guide pages updated (D2102 rules).

## Log
- Early merge (22a4247e, D2062): `Roots::BASE_END`, set by `prepare_base_image`. Full
  collections forward it like `OLD_END`, minor ones leave it alone, Cheney sets it to 0.
  Reported to the coordinator for 0146.
- Tail of per-VM cells (D2063), using 0144's write audit (re-run with
  `WATCH = BASE_END`):
  - first pass: header flags (IS_PROTO set by caches on prototypes not yet used as
    protos at build) and the host call table (Roots.HOST_CALLS) were still written;
  - then all builtin objects are marked at build, flags are written only on change, and
    host calls go in the tail;
  - result: 0 prefix writes over 300 harvested cases plus the codemode corpus (until
    the first collection).
- String hashes: precomputing them cost +1.6 KB gzip (incompressible), so prefix strings
  just don't cache them.
- Checked: djs-core, prelude and wasm tests, e2e plain, js tests, native-vs-wasm, difftest
  codemode/console/builtins with all stress modes, test262 Object/Array/Symbol.
- Instructions vs main: ±0.5%, props −1.2%; module +60 B gzip.
- Coordinator approved (a) armed barrier, (b) prefix diff in images, (c) fixed prefix.
- (c) D2064:
  - full collections = `mark_compact(BASE_END)`, the prefix walked as old;
  - Cheney copies the prefix verbatim and keeps BASE_END;
  - removed perf's `FROZEN_EXEMPT` and the `collect_canonical` fallback;
  - `Vm::new` + frozen runs `prepare_base_image`;
  - test `collections_never_move_the_prefix`;
  - pi-like −1.5% instructions.
- (a) D2065: debug barrier (per-write in Arena + whole-prefix compare after runs and
  collections). Armed by ROOT_FROZEN_BUILTINS or `VmOptions::prefix_barrier`; difftest
  codemode areas armed (`ARMED_AREAS`), all stress modes clean; perf's frozen tests run
  armed. e2e can't be armed: 3 cases change builtins on purpose (BigInt, iterator,
  Proxy). TESTING.md section.
- (b) D2066, session images:
  - vm/session.rs: header + offset-sorted prefix diff + roots cell + `[BASE_END..]`.
    `Vm::session_bytes`, `Vm::from_session`, tests `session_images_round_trip` and
    `touching_builtins_in_any_order_leaves_the_same_session`.
  - Order independence per call, alone after a bare run (session image 28,960 B):
    `'abc'.toUpperCase().split('')`, `JSON.parse(JSON.stringify([1,[2]]))`,
    `Math.max`/`parseFloat`, `Object.keys([1]).concat`, `Array.from('ab').reverse()`,
    `Promise.resolve(1)`, `String(Symbol.iterator)`: +0. `new Map([[1,2]]).get(1)` and
    `new Set([1]).has(1)`: +168 (iterator-result shapes), `[3,1,2].map(..).sort().join()`
    +192, `/a+/.exec('caab')` +288 (match result shape). The prefix diff is empty in every
    case. With `{x:1}` / `JSON.stringify({a:..})` user shapes also differ by order (expected).
  - Containers: DJSN format 2, byte 13 = image kind (0 raw, 1 session vs base, 2 session vs
    reference). First version diffed always against the base: pi containers grew ~8 KB
    (lockdown's 1,749 prefix words out of context). Now against the reference when it has
    the same prefix. Format 1 still loads (js + Rust tests).
  - Measured (wasm release, scripts/snapshot-research/wasm.ts, vs main.wasm): pi containers
    agent-loop 6.2/6.6 KB (main 6.2/6.5), ask 2.1 (2.1), sequential 6.4/7.8 (6.5/7.8),
    tool-heavy 55.3/115.0 (54.7/113.1; the branch's images are 16 KB larger, main.wasm
    predates the tail), records 337.8 (340.2), big-log 404.0 (404.0). Sandbox new: 1,034 B
    zstd (raw image 1,041), 32 KB uncompressed instead of 330 KB. JS encode +0.05–0.1 ms,
    decode −0.02 ms. Module 458,304 B gzip (ceiling 458,444; session.rs is not in wasm).
  - Peak memory, mem-perturb (24 runs, all 23 workloads), main before D2063 (09dfb743)
    vs the branch: means equal within 0.1 MiB (props 2.13 → 2.20, strings 11.82 → 11.78,
    json-replacer 4.69 → 4.49, pi-like 2.66 → 2.66).
  - canonical_hash unchanged (raw canonical copy; equivalent given the base), recorded.
  - `djs heap`: `prefix` section (changed words by cell, registry paths, capped at --top);
    containers read/write session images (djs-cli container.rs); REPL `.snapshot`/
    `.restore` checked by hand.
  - Checked: djs-core lib, e2e plain, djs-cli, js tests (60), native-vs-wasm, difftest
    codemode + harvest with --determinism --gc-stress --suspend-every 97 (armed).
  - docs/host.md container table and session image layout updated (outside touches: the
    container's reference doc). Guide: arena.md, gc.md, determinism.md, snapshots.md.

## Follow-ups
- 0148 (coordinator: created, mine): shapes of builtin-created objects (iterator results,
  match results, descriptors) as prefix state, so creation order of builtin-related state
  doesn't matter. Check first whether pre-creating them at base build is enough; measure
  how often harvested scripts add properties to builtin-created objects.

## Result
- The builtin prefix `[0, BASE_END)` is shared, immutable in use and never moved or freed:
  the base ends with a tail of per-VM cells (D2063), collections walk the prefix as roots
  (D2064), a debug barrier checks it (D2065).
- Snapshots store session images (D2066): `Vm::session_bytes`/`from_session`, JS
  `sessionImage`/`expandSession`, DJSN format 2 (format 1 loads). `djs heap --only prefix`
  lists the builtins a script changed by registry path.
- Known gaps: shapes of builtin-created objects are session state in creation order
  (0148); canonical_hash hashes the raw canonical image; dead builtin storage after
  reshaping in default mode stays in the prefix (16.5 KB for pi's lockdown, D2064).
