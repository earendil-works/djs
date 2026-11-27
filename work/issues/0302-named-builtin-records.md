# 0302: Named records for builtin closure kinds

status: done
agent: psnap
branch: issue/0302-named-builtin-records
depends-on: 0301
touches: builtins/*.ts (records), crates/djs-bgen, crates/djs-core/src/builtins, crates/djs-core/src/vm/snapshot/, docs/decisions.md
guide: -

## Goal
Builtin closures (Promise.all elements, finally data, async-from-sync iterators, ...) keep
their state in positional records that nothing names: a builtin change that reorders a
record's slots would make a snapshot continue silently wrong instead of refusing (the worst
failure mode, 0300's review). Give each record kind a schema (field names, in builtins/*.ts,
used by NewRecord/RecordGet by name through bgen) and write records by name in snapshots;
state constants a record or class holds (promise/generator states, flag bits) are named in
the same tables.

## Acceptance criteria
- [x] Every record kind builtins make has a named schema; positional access is gone.
- [x] Snapshots write and read builtin records and named constants by name; a missing or
      renamed field refuses with the record's kind and field.
- [x] A test perturbs a record's field order and a state constant's value, and snapshots
      taken before still continue the same (or refuse, never miscontinue).
- [x] No cost in the hot paths (records compile to the same slot accesses).

## Log

- Design (D3710): schemas are TS `enum`s with JSDoc tags. `@record` (fields = slots in
  order; `@class IteratorHelper`, `@rest name`, `@extends Base`), `@constants`/`@flags`
  (named values, bits), `@slots Class` (a view of class slots, checked at compile time
  against heap/object.rs's table). Member tags `@enum Set`/`@flags Set` type a field.
  bgen (new `records.rs`) collects and checks them, resolves `Name.member` to a number
  and a record enum named as a value to `ExprKind::Schema` (type `Schema`, its kind id),
  and writes `crates/djs-core/src/builtins/gen/records.rs`: a module per enum with its
  constants (`element_record::CALLED`, `RECORD_KIND`, `FIELD_COUNT`), `RECORDS`,
  `CONSTANT_SETS`, and const asserts for the `@slots` views.
- The record kind lives in six spare bits of the object header (`OBJ_RECORD_KIND`) of
  Record and IteratorHelper objects, not in a slot: no memory, and the slot layouts the
  dumper reads until 0304 mostly stay. 34 kinds of 63. `Heap::alloc_record(kind, proto,
  rest)`, `record_kind`, `as_record(v, kind)`.
- Converted: promise.ts (Capability, ResolvingFunctions, ElementRecord, RemainingCount,
  CombinatorRecord, FinallyData), asyncgen.ts (AsyncGeneratorRequest; the function of that
  name is now AsyncGeneratorEnqueueRequest), iterator.ts (HelperHeader, IteratorHelper,
  CallbackHelper, FlatMapHelper, LimitHelper, ChunksHelper, WindowsHelper, ConcatHelper,
  ZipHelper, ZipSettings; HelperKinds/HelperStates/ZipModes; CreateIteratorHelper(schema,
  kind)), disposable.ts (DisposableStackSlots, DisposableStates, ResourceKinds,
  DisposeErrors, AsyncDisposeState, UsingRecord: error fields first so that
  AsyncDisposeError takes either), decorators.ts (DecoratorState, DecoratorElement,
  DecoratorAdder, DecoratorParamData; compiler/decorators.rs reads the generated
  constants), fromasync.ts (FromAsyncState, FromAsyncPhases), module.ts (ModuleRecord,
  ModuleStatus, ModuleFlags, RequestedModules, SyntheticBody, ModuleRegistry,
  EvaluationState, ModuleReactions, ModuleReaction), proxy.ts (ForInState), error.ts
  (PendingStack), weakref.ts (FinalizationRegistrySlots), intrinsics.d.ts
  (AsyncFromSyncIterator, PromiseStates, PromiseFlags, GeneratorStates, GeneratorKinds,
  ResumeMode). Rust readers use the generated constants (promise.rs, suspend.rs, late.rs,
  proxy.rs, stack.rs, module.rs, intrinsics.rs, run.rs ForInNext, dump_flight's
  `using` record), and the class table types Promise.state/flags, Generator.state/kind,
  DisposableStack.state as `Enum`/`Bits` of the generated sets.
- Snapshots: a record is `Object.<Class>.<Kind>` with its schema's fields by name;
  constant fields are `enum:Set`/`bits:Set` (0 + a value for a non-int32, else member
  index or bits + 1), the used sets' member names in the sets table. Import maps fields
  and members by name, refuses a field either side lacks (naming kind and field), a
  member this build lacks, a field whose constant set differs, an int in a constant
  field not written as a member; check.rs checks every kinded object's kind and slot
  count, and that nothing else has a kind. That fixes the last fuzz panic: 100,000
  mutations, no panic site left (`all_element_job` also checks the element record's kind
  and that `values` is an Array).
- The pinned functions' bytecode hash changed with the builtins: `get_builtin` slots
  (known builtins after the prelude's functions) move with every builtin function added.
  The hash now names them (D3711); main with the same hash gives the identical pin file,
  so 0302 changes no waiting function's bytecode.
- Tests: `records_and_constants_travel_by_name` (every kind of record at idle points,
  continued from a snapshot written with fields and members reversed),
  `a_missing_or_renamed_field_refuses` (export skews: a field renamed or dropped, a
  member renamed); scripts/snapshot-perturb.sh (+ .py), full-only gate leg "snapshot
  perturbation": the corpus's 1,361 snapshots in a build with six records' fields
  reversed and the promise/generator/helper/disposable states and promise flags
  renumbered: 1,361 continue the same, 0 differ, 0 refused; that build passes the difftest
  corpus itself (D3712).
- JS: js/src/layout.generated.ts has the record schemas (`records`) and the kind bits;
  the heap reader finds a pending stack by its kind and field name.
- Size: 769,623 -> 775,252 B gz (+2.3 KB data: the tables; +3.4 KB code: snapshot
  records/constants, the hash's names); ceiling 776,276 (D3713). Codemode memory rows
  unchanged. Hot paths: RecordGet/RecordSet are the same intrinsic calls with the same
  constant indexes; NewRecord indexes the schema table for the count and ORs the kind
  into the header it writes anyway. Benchmarks not run (machine load 8-10).
- Checked: djs-core lib (624), workspace tests, DJS_SNAP=1 snapshot legs (1,164 + 197
  idle points, chains, nothing refused), test262 dirs (Promise, Iterator, async
  generators, disposable stacks, fromAsync, for-in, using, modules, ...: 4,546 pass),
  difftest + stress (no new failures), dumps (djs and node), js tests, reader cross,
  native vs wasm images, codemode workloads, clippy (all gate combinations), docs check.
  Guide pages: none affected (regenerated regions only).

## Follow-ups

- Class-held constants not named yet: ArrayIterator and CollectionIterator kinds,
  RegExpStringIterator and Proxy flags, TypedArray element kinds, RegExp flags, TextCodec
  flags and state. Each is used by literal in interp/heap code (iter.rs, collection.rs,
  regexp.rs, typed.rs) and builtins; naming them is the same work as here, outside this
  issue's touches.
- A suspended frame's header (and its flags) travels as raw values of the frame's
  ValueArray: the bytecode hash covers the function, not the header layout. A header
  change would need a format bump or a named header.
- Constants that travel as plain numbers: resource dispose kinds in a stack's resource
  Array, the module namespace traps' op numbers (builtin closure data).
- IteratorHelper slot 0 (`kind`) duplicates the header's record kind (the dumper reads
  slot 0); 0304 can drop the slot and dispatch on the kind.
- js/src/layout.generated.ts carries record kind numbers: a change of the record enums
  needs `DJS_UPDATE_LAYOUT=1 cargo test -p djs-core layout_generated` too.

## Result

Every internal record the builtins and the engine make has a named schema in
builtins/*.ts (`@record` enums; 34 kinds), and the state constants of records and of the
Promise, Generator and DisposableStack slots are named (`@constants`/`@flags`); bgen
compiles `Name.member` to the same slot indexes and generates
`crates/djs-core/src/builtins/gen/records.rs` for the Rust side (builtins/README.md
"Records and named constants"). Records carry their kind in the object header; logical
snapshots write them and constants by name and refuse a missing or renamed field or
member with the record kind and field (D3710). The bytecode hash names `get_builtin`
slots (D3711). Tested in-process with skewed exporters and across builds with
scripts/snapshot-perturb.sh (full-only gate leg, D3712). Size +5.6 KB gz, ceiling 776,276
(D3713). Gaps: the follow-ups above (other class constants, frame headers).
