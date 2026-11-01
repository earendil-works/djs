# Board

Owned by the coordinator. Status of all issues; details are in `issues/`.

## In progress

| issue | title | agent | worktree |
| --- | --- | --- | --- |

## Open (ready)

| issue | title | depends-on | touches |
| --- | --- | --- | --- |
| 0002 | test262 infrastructure and oracle baseline | 0001 | crates/djs-test262, tests/test262* |
| 0003 | Lexer | 0001 | djs-core/src/lexer, unicode |
| 0004 | Number <-> string conversion | 0001 | djs-core/src/num |
| 0005 | Values, arena heap, strings, atoms, objects | 0001 | djs-core/src/{value,heap} |

## Open (blocked on dependencies)

| issue | title | depends-on |
| --- | --- | --- |
| 0006 | P0 compiler + interpreter | 0003, 0004, 0005 |
| 0007 | test262 on djs + ratchet in gate | 0002, 0006 |

## Done

| issue | title | merged |
| --- | --- | --- |
| 0001 | Workspace skeleton and process | initial commit |

## Backlog (not yet written up)

- P1: functions + closures + the resolution pass, objects/arrays literals, exceptions,
  let/const/TDZ, template literals, destructuring, spread, arrows, classes. The Cheney GC.
- P2: promises, async/await, job queue, async host calls, djs-bgen (checker + JS emitter),
  builtins in the builtin language, base snapshot, JSON, core builtins, djs-wasm, JS host
  package.
- P3: TypeScript type skipper (+ the `strip-types` oracle vs ts-blank-space), enums,
  namespaces, parameter properties.
- P4: snapshot/restore API, deltas, old generation.
- P5: regex engine, generators, Map/Set/WeakMap, Symbol, iterators, typed arrays, …
- Perf track: the bgen Rust backend, benchmarks.

## Parallelism notes

0002, 0003, 0004 and 0005 touch disjoint areas (the only shared file is
`djs-core/src/lib.rs`, mod lines only, which rebases trivially), and none depends on
another. So they run in parallel.
