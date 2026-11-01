# 0029: P1-5 Loops and iteration (compiler side + iterator builtins)

status: open
agent: -
branch: issue/0029-p1-iteration-compiler
depends-on: 0027
touches: crates/djs-core/src/compiler/, crates/djs-core/src/bytecode/ (additive), builtins/iter*.ts (new files only) + builtins/gen (regenerated), docs/compiler.md (corrections)

## Goal
docs/compiler.md section 10, row P1-5: `for-in`, `for-of` with the full iterator
protocol, including closing the iterator on `break`/`return`/`throw` (the 0025
follow-up: reuse the BlockEnv/stmt_depth mechanism that runs finally blocks), labeled
break/continue across them, and the array fast path from the note. The iterator
protocol's multi-step logic (GetIterator, IteratorStep, IteratorClose) lives in
privileged JS per D1085, written in the builtin language in new `builtins/iter*.ts`
files: `%GetIterator`, `%IteratorClose`, `Array.prototype[Symbol.iterator]` /
`values`, `%ArrayIteratorPrototype%.next`, and `String.prototype[Symbol.iterator]` if
cheap. Interpreter changes (for-in enumeration ops, symbol-keyed installation, and the
like) are routed through the coordinator to the heap agent (0028 in flight).

## Acceptance criteria
- [ ] `for (x of y)`, `for (const/let/var x of y)` with per-iteration bindings,
      `for-in` (spec order of keys from 0005's own-keys, prototype chain, deleted
      during enumeration, shadowing), all with labels.
- [ ] Iterator close on abrupt completion, in every exit path, plus the exception
      precedence rules (a throw from the body wins over a throw from `return()`).
- [ ] The array fast path (if the note specifies one): it must be unobservable, i.e.
      only taken when `Array.prototype[Symbol.iterator]` and
      `%ArrayIteratorPrototype%.next` are unmodified.
- [ ] Iterator builtins in the builtin language, checked by bgen, generated, compiling
      in privileged mode. bgen may need installation forms for symbol keys (the 0017
      follow-up); add them to bgen in a minimal way (log it, since that is infra's code)
      or ask.
- [ ] e2e programs checked against qjs. Cases that need interpreter work go into an
      ignored `_0028`/`_rt` test, with the list of interp needs sent to the coordinator
      early.
- [ ] The test262 compile sweep before/after.

## Log

## Follow-ups

## Result
