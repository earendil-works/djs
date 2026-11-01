# 0097: Finish the later group: immutable ArrayBuffer and the rest

status: in-progress
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
- [ ] The later group inventory with a decision per feature.
- [ ] Immutable ArrayBuffer per the proposal at the test262 commit (writes through typed
      arrays/DataView throw or are ignored per spec, detach/resize refused, structured
      checks in every TA/DataView setter), matching qjs on the two corpus cases.
- [ ] Promotions with numbers, ratchets updated, both backends.

## Log
- Inventory (djs-test262 list/run --group later, at 8287580d): 179 tests, qjs passes all
  179, djs 12.
  | feature | tests | djs passes | what |
  | --- | ---: | ---: | --- |
  | await-dictionary | 89 | 0 | `Promise.allKeyed` / `allSettledKeyed` (proposal) |
  | immutable-arraybuffer | ~49 | 2 | `transferToImmutable`, `sliceToImmutable`, `immutable` and the checks in every TA/DataView writer |
  | error-stack-accessor | 33 | 0 | `Error.prototype.stack` as an accessor; instances get no own `stack` |
  | Symbol.unscopables | 8 | 8 | already passes (the `with` tests are skipped anyway) |

## Follow-ups

## Result
