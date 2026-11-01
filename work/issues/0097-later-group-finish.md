# 0097: Finish the later group: immutable ArrayBuffer and the rest

status: done
agent: infra
branch: issue/0097-later-group-finish
depends-on: 0095
touches: builtins/ (arraybuffer.ts, typedarray.ts, …), crates/djs-core/src/builtins/, crates/djs-core/src/heap/typed.rs (immutable flag, small), tests/test262.toml (promotion), ratchets

## Goal
Only 179 tests remain in test262's `later` group, and the corpus has two failures left
(dquickts test_immutable_array_buffer.js, test_typed_array.js, both "not a function": the
immutable ArrayBuffer proposal: `transferToImmutable`, `sliceToImmutable`, `immutable`,
which qjs implements). List what the 179 are, implement what's reasonable (the
immutable-arraybuffer feature first), promote it, and record the rest with reasons (for
example `error-stack-accessor` or anything proposal-stage we choose not to do).

## Acceptance criteria
- [x] The later group inventory with a decision per feature.
- [x] Immutable ArrayBuffer per the proposal at the test262 commit (writes through typed
      arrays/DataView throw or are ignored per spec, detach/resize refused, structured
      checks in every TA/DataView setter), matching qjs on the two corpus cases.
- [x] Promotions with numbers, ratchets updated, both backends.

## Log
- Inventory (djs-test262 list/run --group later, at 8287580d): 179 tests, qjs passes all
  179, djs 12.
  | feature | tests | djs passes | what |
  | --- | ---: | ---: | --- |
  | await-dictionary | 89 | 0 | `Promise.allKeyed` / `allSettledKeyed` (proposal) |
  | immutable-arraybuffer | ~49 | 2 | `transferToImmutable`, `sliceToImmutable`, `immutable` and the checks in every TA/DataView writer |
  | error-stack-accessor | 33 | 0 | `Error.prototype.stack` as an accessor; instances get no own `stack` |
  | Symbol.unscopables | 8 | 8 | already passes (the `with` tests are skipped anyway) |
- Decision per feature (coordinator agreed): implement all three that qjs has
  (immutable-arraybuffer D1970, error-stack-accessor D1971, await-dictionary D1972) and
  promote Symbol.unscopables as is. All 179 pass with both backends; the later group is
  empty; active 38,109, djs passes 38,069 (99.9%), the ratchet unchanged at 40.
- Immutable ArrayBuffers: the flag is slot 1 = true (no layout change, heap/typed.rs);
  checks in every element-write path (heap [[Set]]/[[DefineOwnProperty]]/
  [[GetOwnProperty]], set_property, ordinary_set, set_element_fast, put_elem's
  conversion, Reflect.set, Object.defineProperty), the writing methods, DataView setters,
  TypedArrayCreateFromConstructor. Corpus: dquickts' test_immutable_array_buffer.js stays in
  the ratchet (needs SharedArrayBuffer/Atomics, coordinator: option b, listed in TESTING.md
  "Known differences"); tests/corpus/builtins/immutable-arraybuffer.js is it without
  those lines (passes); test_typed_array.js passes now.
- Error stack accessor: the trace in an Error slot (heap/object.rs: Error has 1 slot,
  DOMException 3; told the coordinator), getter/setter on Error.prototype. Checked: pi
  adapter tests (js/test/pi-vm.test.ts stack assertions) and sandbox tests pass, djs run's
  `Uncaught` stack and `Vm::error_stack` (own `stack` first, else the slot), the difftest
  corpus (stack lines normalized as before), JSON.stringify of errors (`{}` as before),
  the reader (Opaque, unchanged). Not run: pi's own vitest suite (needs pi-private).
- Corpus case builtins/later-features.js (stack accessor and keyed combinators against
  qjs, stress-identical on both backends).

## Follow-ups
- `Error.captureStackTrace` (qjs has it; djs does not): not part of any test262 feature
  here; a corpus difference only if a script uses it.

## Result
test262's later group is empty: immutable ArrayBuffers (D1970), Error.prototype.stack as
an accessor (D1971) and Promise.allKeyed/allSettledKeyed (D1972) are implemented as qjs
has them, and Symbol.unscopables is promoted. Active group 38,109, djs passes 38,069 on
both backends. Corpus 224/230: the one failure needs SharedArrayBuffer/Atomics (a
non-goal). TESTING.md and README.md updated.
