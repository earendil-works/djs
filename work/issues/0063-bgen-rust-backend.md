# 0063: Sloppy `this` for Function-constructor functions (D015) + the bgen Rust backend v1

status: done
agent: infra
branch: issue/0063-bgen-rust-backend
depends-on: 0059
touches: builtins/function.ts + crates/djs-core/src/compiler/dynamic.rs (D015: a template flag only), crates/djs-core/src/interp/ (the this-binding for that flag, isolated), crates/djs-bgen/ (the Rust backend), crates/djs-core/src/builtins/gen/ (new: checked-in generated Rust), crates/djs-core/src/interp/ (native frames: coordinate with heap's 0062 via the coordinator), crates/djs-bench/, ARCH.md "Builtins" (corrections only)

## Goal
Part 1 (small): D015, sloppy `this` binding for Function-constructor functions without
"use strict" (a template flag set by compiler/dynamic.rs, honored at call time). Report
how many of the 56 sloppy test262 failures it fixes.

Part 2 (the main work): ARCH.md "The builtin language" → "Rust backend (opt-in,
`/** @native */`)" and "Generated state machines". Builtin-language code runs at
interpreter speed: sort with a comparator is ~10x qjs and arrays ~12.8x (0035), Promise
and iterator machinery all go through bytecode. bgen already has the typed AST, local
types, resume points and try/catch structure (0017 follow-ups). Build the Rust backend:
generate resumable state machines in native frames per ARCH (locals in arena frame slots,
resume points as states, no GC inside an arm, fuel per transition, fast paths inline),
for a first set of hot builtins chosen by profiling (for example
`Array.prototype.{map,forEach,filter,indexOf,join,sort}`, `%GetIterator`, the array
iterator's `next`, and the Promise reaction job).

## Acceptance criteria
- [x] D015 with tests, and the test262 numbers before/after.
- [x] A design check before building: the native-frame layout in the arena, how the
      interpreter's frame walk, the GC, snapshots, stack traces and the suspension stress
      modes see native frames. Send it to the coordinator, since heap (0062) is
      redesigning the property access paths in interp at the same time.
- [x] The backend: `@native` builtins compile to checked-in Rust (`scripts/bgen.sh`
      regenerates it; `--check` in the gate), with both backends' behavior identical.
      Run the corpus and test262 with the bytecode backend and the native backend
      (`--backend bytecode|native`, TESTING.md), identical in the stress modes.
- [x] Integer overflow checks for `+ - *` in the Rust backend (D1171).
- [x] Numbers: the engine track, sort/map/forEach microbenchmarks, the pi bench, and wasm
      code size per @native function (ARCH: each one adds size; report it).

## Log
- Part 1, D015 (D1630): `func_flags::SLOPPY_THIS` is set by compiler/dynamic.rs when the
  body has no "use strict" (`FuncState::use_strict`, recorded in the body's directive
  prologue). `Exec::call` binds `this` sloppily before the frame; `fast_call_at` declines
  such templates. test262 active 37,464 → 37,504 (40 stale entries removed, all sloppy
  `this`: Function 7, apply 13, call 13, function-code 4, Object.entries/values, harness
  fnGlobalObject). 16 of the 56 sloppy failures stay: duplicate parameters and
  `eval`/`arguments` as names. Unit test `vm::tests::sloppy_this_for_dynamic_functions`.
- Part 2, design check sent and approved (frame record + FLAG_NATIVE, call_intrinsic
  for every intrinsic, fuel per transition, backend in the identity, %MarkNative fails
  the base build, try/finally a bgen error); heap (0062) saw no conflict.
- Profile first (release cli, 200k elements): the cost was bytecode dispatch plus small JS
  helper frames (LengthOfArrayLike → ToLength → ToIntegerOrInfinity → ToNumber; push
  allocated a rest array and ran two of those per call), and heap's 0062 measurement
  showed push/pop and charCodeAt hot in pi-like scripts. First set (35 natives):
  Array.prototype.{map, forEach, filter, reduce, reduceRight, indexOf, includes, some,
  every, find, findIndex, join, push, pop, slice, sort, toSorted}, the helpers
  EveryOrSome, FindViaPredicate, Reduce, SortList, CollectPresent, %ArrayIteratorPrototype%
  .next, GetIterator, PromiseReactionJob, String.prototype.{charAt, charCodeAt,
  codePointAt, at, slice, substring, indexOf, startsWith}. New @intrinsic tags for
  callees: LengthOfArrayLike, ArraySpeciesCreate, ResolvePromise (plus the helpers).
- Runtime (D1631): interp/run/native.rs; run.rs touched only in call (native branch),
  fast_call_at (flag mask), load_frame/enter_template, step's cold branch, throw_value and
  the ThrowPending key. bgen (D1632): emit_rust.rs (lowering to blocks, spills, short
  circuits, inlining of small helpers, virtual rest parameters, jump threading, leaf
  builtins as Rust fns, handler tables). Fuel and identity (D1633), code size (D1634),
  tests (D1635).
- Tests: interp::run::native::tests (both backends agree in plain / suspension through
  images every 7 fuel / GC stress; a backend switch at every stop; fuel; Vm::backtrace
  through native frames; %MarkNative name check; checked arithmetic), bgen golden
  fixture native.ts → native.rs.txt, checker rejections. Corpus: native in all stress
  modes and bytecode plain, ratchet unchanged; test262 active 37,711/37,912 on both
  backends (after rebasing onto main).
- Rebased onto main (0062 ICs took FuncTemplate RESERVED; NATIVE sits at 68 next to IC;
  Fault became a boxed struct: Fault::throw/error).

## Follow-ups
- try/finally in @native builtins (completion records in slots), and @native
  constructors (new.target, FLAG_CONSTRUCT).
- More fast paths where profiles point: Array.prototype.pop's `Set(length, n - 1)` and
  DeletePropertyOrThrow (need a heap entry point for a dense truncate, 0062's area);
  generic `n_leaf` (push/dispatch/pop) for the hottest leaves (StrCharCodeAt,
  StrSubstring, CreateIterResultObject).
- for-of over arrays is now dominated by allocating iterator result objects and the
  collections they cause (10-15x qjs): an interpreter-level fast path for unmodified
  arrays (like `iteration_unmodified` for spread) would avoid the objects.
- Record the engine track on a quiet machine (`scripts/bench.sh --record engine`): the
  load during 0063 (load average 20-30) made the recorded run meaningless, so it was not
  kept.
- Candidates for the next natives by profile: JSON.stringify's slow path, Object.keys /
  entries, Array.from, String.prototype.split/replace callers, Map/Set iteration.

## Result
Part 1: D015 (D1630), test262 +40.

Part 2: the bgen Rust backend. `@native` builtins compile to resumable Rust state
machines in `crates/djs-core/src/builtins/gen/natives.rs` (checked in; `scripts/bgen.sh`
writes it and `--check` covers it) and run in native frames (ordinary frame records with
a header bit, D1631) through `interp/run/native.rs`. Both backends behave the same:
`--backend bytecode|native` in djs run, djs-difftest and djs-test262; the gate runs the
corpus natively in all stress modes and with the bytecode backend; test262 active
37,711/37,912 on both. Integer `+ - *` are checked (D1171). 35 natives (see Log).

Numbers (release, host; the machine was loaded, so interleaved runs; bytecode / native /
qjs user seconds for the operation, setup subtracted): push ×2M 2.59 / .38 / .06,
push+pop ×2M 3.99 / .82 / .13, map ×20 of 200k .79 / .40 / .20, forEach .46 / .23 / .11,
filter .84 / .39 / .17, indexOf ×50 .95 / .21 / .03, join ×10 .71 / .27 / .07, for-of
×20 3.38 / 1.20 / .08, charCodeAt 500k .19 / .10 / .02, String slice 500k .38 / .12 /
.02, sort 50k with comparator ×3 .66 / .21 / .06, reduce .45 / .25 / .11. Engine track
(first run, less load): bytecode÷native higher-order 5.3x, sort 4.5x, push-pop 3.9x,
map 3.4x, foreach 1.9x; djs÷qjs higher-order 6.8x → 2.3x, map 1.5x, sort 2.8x. pi bench
(wasm, tool-heavy script, 3 interleaved runs): djs with image reuse 11.4 ms native vs
13.8 ms bytecode (quickjs-wasi ~8 ms). Wasm size: the code section grows by 80 KB (+9%)
over the merge base, 71 KB generated + 7 KB runtime (D1634); per function
`scripts/native-sizes.sh` (SortList 7.9 KB, Array slice 4.3 KB, startsWith 4.0 KB, ...,
map 1.9 KB, push 1.8 KB, forEach 1.5 KB).

Known gaps: try/finally and constructors in @native builtins (bgen errors); an i32 -0 is
0 in Rust; fast paths only for Arrays' dense elements.
