# 0063: Sloppy `this` for Function-constructor functions (D015) + the bgen Rust backend v1

status: in-progress
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
- [ ] D015 with tests, and the test262 numbers before/after.
- [ ] A design check before building: the native-frame layout in the arena, how the
      interpreter's frame walk, the GC, snapshots, stack traces and the suspension stress
      modes see native frames. Send it to the coordinator, since heap (0062) is
      redesigning the property access paths in interp at the same time.
- [ ] The backend: `@native` builtins compile to checked-in Rust (`scripts/bgen.sh`
      regenerates it; `--check` in the gate), with both backends' behavior identical.
      Run the corpus and test262 with the bytecode backend and the native backend
      (`--backend bytecode|native`, TESTING.md), identical in the stress modes.
- [ ] Integer overflow checks for `+ - *` in the Rust backend (D1171).
- [ ] Numbers: the engine track, sort/map/forEach microbenchmarks, the pi bench, and wasm
      code size per @native function (ARCH: each one adds size; report it).

## Log

## Follow-ups

## Result
