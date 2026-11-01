# 0111: Prototype objects' property order (qjs)

status: open
agent: -
branch: issue/0111-prototype-order
depends-on: 0108
guide: -
touches: crates/djs-bgen/src/qjs_order.rs, builtins/ (install order), tests/corpus/

## Goal
0108's follow-up: prototype objects' own property order differs from qjs (qjs puts
`constructor` last and uses its own method order). Extend D2072's qjs-order table to
prototypes (Array.prototype, String.prototype, Object.prototype, Promise.prototype,
Map/Set, RegExp, Date, TypedArray, Error, …).

## Acceptance criteria
- [ ] Prototype property order matches qjs for every builtin prototype (a difftest case
      listing getOwnPropertyNames/Symbols of each), test262 unchanged, both backends.

## Log

## Follow-ups

## Result
