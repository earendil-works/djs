# 0097: Finish the later group: immutable ArrayBuffer and the rest

status: open
agent: -
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

## Follow-ups

## Result
