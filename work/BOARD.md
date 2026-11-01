# Board

Owned by the coordinator. Status of all issues; details are in `issues/`.

## In progress

| issue | title | agent | worktree |
| --- | --- | --- | --- |
| 0026 | Leaf performance sweep | num | .worktrees/0026-leaf-perf-sweep |
| 0022 | E2E wiring: Vm::compile, djs run, missing P0 interp ops | heap | .worktrees/0022-e2e-wiring |
| 0025 | P1-3 Exceptions + P1-4 leftovers (compiler) | lexer | .worktrees/0025-p1-exceptions |

## Open (ready)

| issue | title | depends-on | touches |
| --- | --- | --- | --- |

## Open (blocked on dependencies)

| issue | title | depends-on |
| --- | --- | --- |
| 0016 | djs in difftest + gate | 0014, 0022 (infra, next) |
| 0007 | test262 on djs + ratchet in gate | 0014, 0015, P1-8 (script mode) |

## Done

| issue | title | merged |
| --- | --- | --- |
| 0001 | Workspace skeleton and process | initial commit |
| 0003 | Lexer | e4a42c3 (agent lexer; parser follow-ups listed in the issue) |
| 0005 | Values, arena heap, strings, atoms, objects | dc751e2 (agent heap; follow-ups in the issue) |
| 0023 | djs-wasm + JS host package | d4fb4a4 (agent num; djs runs in node; vm_new 11 us; 132 KB gz at opt s) |
| 0024 | P1-1 Functions and closures (compiler) | ad6c70f (agent lexer; test262 compiling 11.4k -> 23.5k) |
| 0014 | P0 compiler | 5c5ce95 (agent lexer; 31 e2e programs match qjs; test262 sweep no panics) |
| 0021 | Outbound direct heap reader | 91b5ae5 (agent num; 1 MiB string at memcpy speed; 1.9-5.9x faster than JSON path) |
| 0015 | P0 interpreter and embedding | 2373382 (agent heap; P0 + most builtin-free P1 ops; e2e left to 0022) |
| 0020 | Flaky difftest timeout | 6818440 (agent infra; D1200 timeout convention) |
| 0018 | Inbound wire format | 95afebe (agent num; beats JSON path on all datasets; node tests in gate) |
| 0019 | Benchmark harness | 0bb9d9b (agent infra; baseline in tests/scores/bench.tsv; wasm 181 KB stripped, 85 KB gz) |
| 0017 | djs-bgen v0 | 8496b74 (agent infra; Rust, no deps; D1170-D1173; --check in gate) |
| 0011 | Regex engine core | d57c4a9 (agent num; 13.6k qjs cases; \p{} deferred, D1112) |
| 0013 | Bytecode table and tools | 96c4a0a (agent lexer; 147 opcodes) |
| 0012 | Native JSON (leaf) | 9f437fa (agent heap; parse ~80% of node, stringify ~50%) |
| 0010 | Differential corpus and difftest runner | c5039b3 (agent infra; 165 cases, all pass on qjs) |
| 0008 | Compiler and bytecode design note | 977acdd (agent lexer; docs/compiler.md; D006 accepts it) |
| 0006 | P0 umbrella | split into 0013-0016 |
| 0009 | Copying GC and handle table | 6030aff (agent heap; ~15 ns/cell; LOS designed, not built) |
| 0004 | Number <-> string conversion | 9b0a654 (agent num; matches qjs on 2.1M cases; D1041 differs on >38-digit inputs) |
| 0002 | test262 infrastructure and oracle baseline | 5087390 (agent infra; qjs: 37,569 tests in ~10s, 4 oracle failures) |

## Backlog (not yet written up)

- P1 (split in docs/compiler.md section 10, P1-1..P1-10): functions + closures + the resolution pass, objects/arrays literals, exceptions,
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
