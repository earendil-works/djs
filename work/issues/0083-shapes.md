# 0083: Shapes (hidden classes) and a leaner object/image layout

status: in-progress
agent: heap
branch: issue/0083-shapes
depends-on: 0062, 0063
touches: crates/djs-core/src/heap/ (object layout, property tables, shapes as cells), crates/djs-core/src/interp/ (the 0062 IC hit checks), crates/djs-core/src/heap/gc.rs + walk.rs, js/src/layout.generated.ts + js/src/reader.ts (the reader must read the new layout), docs/objects.md, docs/wire.md, ARCH.md "Heap" (objects paragraph)

## Goal
docs/objects.md part 2 plus 0052's S2 (work/issues/0052-bloat-investigation.md): shared
shapes / hidden classes, which mainly pay off in memory per object and base-image size (a
4-property object ~144 B → ~56 B; the 424 builtin `length`/`name` tables become one shared
shape; 16-byte dictionary entries), plus cheaper IC hits (a shape compare instead of a key
re-check).

## Acceptance criteria
- [ ] A design update in docs/objects.md first (shape cells, transitions and their cache,
      dictionary mode, prototypes and shapes, interaction with the copying GC and with
      deterministic images (D1090: shape trees must be deterministic), snapshots, the JS
      reader, the 0062 ICs). **Send it to the coordinator before implementing**; it's an
      ARCH change.
- [ ] Implementation in parts, with no semantic changes: ratchets, stress modes,
      determinism checks (0072), and the native-vs-wasm image check all stay identical or
      better.
- [ ] Numbers: memory per object (benchmarks with many small objects, a JSON parse of
      10k records), base image size (base.img, wasm data section), the engine track,
      the pi bench. Recorded only on a quiet machine.

## Notes
- infra (0080) works on native builtins and the iteration fast paths, which use
  `get_element_fast`/`array_length` (elements, not named properties). Keep those
  entry points stable, or coordinate through the coordinator.
- 0070 follow-up for later: native builtins have no collect-and-retry yet.

## Log

- Design (docs/objects.md §7) approved with conditions: ephemeron GC tests for internal-ref
  keys, determinism gate checks and reader cross tests green at every part, the 32/32
  limits measured on realistic data, numbers only on a quiet machine.
- Ids: CellKind 25 AccessorPair, 26 Shape, 27 PropValues; header flag OBJ_DICTIONARY 128.
- Part 1 (D1830): 16-byte PropEntry plus the AccessorPair cell. Sizes are byte counts, so
  they don't depend on machine load:
  - base image 416,120 -> 402,432 bytes (-13.7 KB, -3.3%);
  - memory per object (vm/tests.rs `memory_per_object_report`, 10k objects kept in an
    array, after a collect):

    | data | before | after |
    | --- | ---: | ---: |
    | `{a,b,c,d}` literal | 157.1 B | 125.1 B |
    | JSON.parse 5-key record | 259.7 B | 219.7 B |
    | class instance, 3 fields | 157.2 B | 125.2 B |
    | closure | 189.1 B | 157.1 B |
    | `{}` | 37.1 B | 37.1 B |

  No timing: the machine is at load ~10, and part 1 only changes the storage layout.

- Part 2 (D1831): shapes for ordinary objects.
  - Weak transitions broke D1730 (canonical hash independent of collection points), so
    transitions are strong, bounded by SHAPE_MAX 32 and a per-heap budget of 4096 shapes
    (coordinator agreed). There is no per-shape limit on children: with 32, the root's first
    32 keys would have been the only shapes ever. Children are sorted and binary searched.
  - Dictionaries: install targets (resolve_path), the global object, the lexical record,
    the constructor and symbol registries, attribute changes, index keys, more than 32
    keys, the budget.
  - Delete: the last key goes back to the parent; another key re-walks if at most one
    shape is new, else dictionary. `delete_own_property` returns a Result now.
  - Own-property ICs on shape-mode receivers compare the shape (pulled forward from
    part 3), because verified-index hits in shape mode made props.js +17%.
  - Tests: heap/shape_tests.rs (sharing, bulk creation, delete, attributes, index keys,
    limits, the budget's overflow, collections at any points give the same image, image
    round trip); vm/tests.rs `shapes_do_not_depend_on_collection_points` (canonical
    hash with no collections, collections every 997/5000 fuel, restores; budget overflow
    included).
  - Numbers in docs/objects.md 7.11: per object −26..−33% in the arena (wasm build); base
    image −8.9 KB; wasm code +12.2 KB; timing within ±3% natively and in wasm.
  - Checks green: lib, e2e ×3, filtered test262, difftest ratchet, stress difftest over
    object/json/array/language/codemode, native-vs-wasm, node tests, wire and reader
    cross tests, clippy (native and wasm32 release).
  - Also merged separately: issue/0083-test-time (slow vm tests sampled, main 2b91a3f).

- Part 3 (D1832): addition caches for put_field/define_field (define_field got a
  Cache immediate), receiver-shape proofs for prototype reads, IC_WORDS 3 -> 5, and
  the hit checks shared as `cached_get`/`cached_put`.
  - The first fill hook (after `set_property`) was wrong: a setter frame could be pushed
    and its code could collect before the fill read `obj`. Now the slow path takes an
    explicit `plain_addition` route (CreateDataProperty when nothing on the chain has
    the key) and only that fills.
  - Tests: tests/corpus/language/shape-caches.js (qjs-expected, stress and determinism
    green), vm `warm_and_cold_addition_caches_behave_the_same`.
  - Numbers in objects.md 7.12. Literals and constructors are about 3x faster natively
    and in wasm, and props.js is 14–15% faster. Pure own writes in wasm are +7..13%
    while native is unchanged; it looks like V8 codegen in the bigger fast path, and
    0093's run_hot reworks that loop.

## Follow-ups

- The wasm own-write regression (+7..13% on a write-only microbenchmark): revisit with
  0093's run_hot.
- compact_for_image could shrink descriptor and transition arrays (about 1 KB of the
  base image).
- Linear memory in wasm is about 2.7× the arena for allocation-heavy runs (to-space plus
  arena doubling): that's 0087's area.

## Result
