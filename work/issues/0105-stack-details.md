# 0105: Stack trace details (Error.stackTraceLimit, call-site columns)

status: in-progress
agent: infra
branch: issue/0105-stack-details
depends-on: 0102
touches: builtins/error.ts + crates/djs-core/src/interp/ (stack trace capture), crates/djs-core/src/compiler/ (line-table positions for `return f()` only), tests/corpus/

## Goal
0102's follow-ups: `Error.stackTraceLimit` (qjs and V8 have it; djs caps at 10 frames),
and call-site columns in `return f()` stacks: qjs points at the start of the `return`
statement (its tail call), djs at the call. Goal: match qjs. Small, user-visible polish
for error messages that models read. (Corrected by the coordinator: the first version
had the direction the wrong way round.)

## Acceptance criteria
- [ ] Error.stackTraceLimit per qjs (getter/setter semantics, non-number values, 0,
      Infinity), applied to captureStackTrace and error construction.
- [ ] `return f()` and similar tail positions record the `return` statement's position,
      matching qjs, and a difftest-like check that compares stack lines (the normal
      difftest drops them).
- [ ] Ratchets and both backends unchanged.

## Log

## Follow-ups

## Result
