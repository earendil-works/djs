# Board

Owned by the coordinator. Status of all issues; details are in `issues/`.

## In progress

| issue | title | agent | worktree |
| --- | --- | --- | --- |
| 0071 | GC points and raw images independent of slice size | num | .worktrees/num |
| 0037 | P2 Promises, job queue, async host calls, suspendable frames | heap | .worktrees/0037-p2-promises-jobs |
| 0080 | for-of/iteration fast paths + natives wave 2 | infra | .worktrees/infra |
| 0053 | Dev-flow test fixes (no-stress, e2e parallel, DJS_JOBS, qjs cache) | devflow | .worktrees/devflow |
| 0079 | Integration and test efficiency (investigate first) | devflow, after 0053 | .worktrees/devflow |
| 0082 | The long tail: remaining test262/difftest failures | lexer | .worktrees/lexer |
| 0083 | Shapes + leaner object/image layout (design first) | heap | .worktrees/heap |
| 0074 | Carry fuel overshoot across runs | determinism | .worktrees/determinism |
| 0081 | Perf anatomy: disassembly-level investigation (report only) | perf | .worktrees/perf |

## Open (ready)

| issue | title | depends-on | touches |
| --- | --- | --- | --- |

## Open (blocked on dependencies)

| issue | title | depends-on |
| --- | --- | --- |
| 0078 | Leaf quick wins from 0077 (ASCII check, UTF-8 in transcode, short_decimal) | num, after 0075 |
| 0072 | Determinism checks in the gate, part 2 (part 1 done: 6ab3968) | 0071 |

## Done

| issue | title | merged |
| --- | --- | --- |
| 0001 | Workspace skeleton and process | initial commit |
| 0003 | Lexer | e4a42c3 (agent lexer; parser follow-ups listed in the issue) |
| 0005 | Values, arena heap, strings, atoms, objects | dc751e2 (agent heap; follow-ups in the issue) |
| 0023 | djs-wasm + JS host package | d4fb4a4 (agent num; djs runs in node; vm_new 11 us; 132 KB gz at opt s) |
| 0028 | P1-2 runtime | 468f874 (agent heap; prelude at Vm::new: 92 us native / 146 us wasm; Error stack) |
| 0029 | P1-5 Loops and iteration (compiler + iter builtins) | 8cc5aa2 (agent lexer; +1,774 test262) |
| 0030 | P1-5 runtime + 0028 leftovers + GC audit | 9f35b6f (agent heap; 4 parts) |
| 0041 | Async/await, generators (compiler) | f93fa30 (agent lexer; codemode body is a real async function) |
| 0043 | wasm size audit (pass 1) | a35c5e8 (agent heap; stripped 1,200 -> 948 KB; base 502 -> 288 KB; second code pass deferred until wasm-opt numbers) |
| 0042 | Async generator runtime, function kinds, host rejections | 6e3c965 (agent heap) |
| 0037 | P2 Promises, jobs, async host calls, suspendable frames | bd6f974 (agent heap; 3 parts) |
| 0036 | P1-6 Destructuring + tagged templates | 234fd3c (agent lexer; +~1,940 test262) |
| 0052 | Bloat investigation | 4fe04f1 (agent bloat; report + 5 analysis scripts) |
| 0057 | Size quick wins Q1-Q6 | f26f265 (agent bloat; code -18%; main now 1,241 KB, 428 KB gz) |
| 0048 | P3 TypeScript | c3261f9 (agent lexer; strip-types 4,053/4,053 byte-identical vs ts-blank-space) |
| 0050 | RegExp completion | e214e54 (agent num; \p{} on by default +27 KB gz; RegExp promoted; pi image regex fixed) |
| 0046 | BigInt | 63d8468 (agent heap; BigInt 1135/1142; active 98.8%) |
| 0033 | JSON + RegExp builtins, regex literals | 8b5ccfb (agent num; active 32,303 / 32,718) |
| 0040 | P1-7 Classes | 2d08a6c (agent lexer; test262 active 97.1%) |
| 0065 | Decorators (TC39 + accessor + TS legacy) | f4894f5 (agent lexer; strip-types 4,316/4,802, 0 differ) |
| 0058 | Compiler: using, completion values + Goal::Eval, import() | 8bc39dd (agent lexer; strip-types 4,085/4,802, 0 differ) |
| 0075 | Build identity + Float32 NaN | df34400 (agent num; D1750; libm pinned =0.2.16) |
| 0076 | Unicode normalization | 9825c90 (agent num; +20.5 KB gz, feature on by default D1760; 400,680/400,680 conformance) |
| 0061 | String append + JSON source text | dbd2e8d (agent num; 10M appends 574 -> 150 MB RSS; JSON 142/142) |
| 0059 | Indirect eval, Function ctors, queueMicrotask | 4435b3b (agent infra; active 37,464 / 37,722 = 99.3%; pi 215/226) |
| 0064 | djs repl | 31797b6 (agent repl, archived; docs/cli.md; follow-ups: tab completion, interactive .step) |
| 0077 | deser deps investigation | report (agent deps, archived): no new deps; quick wins -> 0078 |
| 0073 | Replay journal + canonical hash | 37e6172 (agent determinism; D1730) |
| 0068 | Deterministic mode for VM and Sandbox | 971036e (agent determinism; D1680) |
| 0067 | Host inbox (async host results as macrotasks) | 7988b76 (agent determinism; D1670) |
| 0066 | pi adapter determinism fixes | 6c523da (agent determinism; equal polls cold/warm; pi suite 221/226) |
| 0055 | Determinism investigation | b30f154 (agent determinism; native/wasm images already identical; issues 0066-0075) |
| 0063 | D015 + bgen Rust backend v1 (35 natives) | 9f662b7 (agent infra; higher-order 6.8x -> 2.3x qjs; +80 KB code) |
| 0069 | Host OOM status | 893495f (agent heap; D1690) |
| 0070 | Collect-and-retry at a failing allocation | 42b7fc5 (agent heap; D1700; natives not yet) |
| 0062 | Property access ICs (part 1) | 5671e8c (agent heap; global read 10.4 -> 1.3 ns; props -14..-18%) |
| 0060 | Out-of-memory recovery | b7c83a8 (agent heap; D1600 reserve, D1601 GC threshold in image) |
| 0056 | String/regex hot paths | 6d24f26 (agent num; test262 active run 250 -> 90 CPU-s, 61 -> 17 s wall) |
| 0051 | Remaining builtins | df48233 (agent heap; all promoted features 100%) |
| 0054 | pi follow-ups | 69ee33c (agent infra; pi suite 213/226; OOM recovery bug -> heap) |
| 0049 | pi codemode adapter | e39a47e (agent infra; pi's suite 204/226 on djs; start 0.49 ms vs 3.2 ms) |
| 0047 | Dev-flow investigation | report in issue file (D012 adopted) |
| 0044 | djs runs the shared difftest prelude | a214890 (agent infra; difftest ratchet 52 -> 38) |
| 0045 | Proxy | 7427f4f (agent infra; Proxy 352/375; ~5,000 behavior cases vs qjs) |
| 0039 | Binary data | c8aefb6 (agent infra; DataView 445/449, TypedArray 841/849) |
| 0038 | Builtins wave 3 | e43d075 (agent infra; Map/Set/Weak*/Reflect/Date; test262 active 64.8%) |
| 0034 | Core builtins wave 2 | a05575b (agent infra; Array 2,461/2,680, String 688/1,040; gate back to 60 s) |
| 0035 | Interpreter call-path performance | 2779e53 (agent heap; calls 7.5x -> 3.35x qjs, loop 9.6x -> 3.8x) |
| 0031 | Core builtins wave 1 | 25fdfa7 (agent infra; test262 active 2,961 -> 7,687; leftovers to 0034) |
| 0032 | Base snapshot | 60a816e (agent num; vm_new 144 us -> 0.4 us in wasm) |
| 0026 | Leaf performance sweep | 04fc8fa (agent num; stringify -24..-54%, GC -11..-19%; opt-level stays s) |
| 0007 | test262 on djs + ratchet in gate | 2312924 (agent infra; 2,961/20,124 active; gate runs all active in 4.2 s) |
| 0016 | djs in difftest + gate | b01a03a (agent infra; 50/154 pass; djs 4-9x slower than qjs on calls/loops/props) |
| 0027 | P1-2 Objects/privileged mode (compiler) | 2e11de2 (agent lexer; test262 compiling 26.0k) |
| 0022 | E2E wiring | f235cdd (agent heap; djs run works; 29 e2e cases x 3 modes) |
| 0025 | P1-3 Exceptions + P1-4 leftovers | d45e837 (agent lexer; test262 compiling 24.7k) |
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
- Perf track: (shapes now 0083). A one-codegen-unit LTO
  profile for `bench.sh --record` (D1620: default release layouts vary +-15%).
- Perf track: the bgen Rust backend, benchmarks. **Interpreter call path is ~10x slower
  than qjs** (0034: 3.4M plain JS calls 0.6 s vs 0.06 s; sort with comparator 30x): the
  next perf lever, likely for heap after 0030.

## Scores

| date | difftest (djs) | test262 active |
| --- | --- | --- |
| 2026-11-01 | 50 / 154 (11 skipped) | 2,961 / 20,124 (14.7%) |
| 2026-11-01 (0031 part 1) | 55 / 168 | 5,108 / 20,124 (25.4%) |
| 2026-11-01 (0031 part 2) | 74 / 171 | 7,687 / 20,124 (38.2%) |
| 2026-11-01 (0029 done) | ~95 / 175 | 11,773 / 20,124 (58.5%) |
| 2026-11-01 (0038 p1: promotion to active, +2,018 tests) | | 12,354 / 22,142 (55.8%) |
| 2026-11-01 (0038 p2: Map/Set/Weak*) | | 13,049 / 22,142 (58.9%) |
| 2026-11-01 (0038 done + 0037 p1) | 96 / 182 | 14,367 / 22,142 (64.8%) |
| 2026-11-01 (0039 done; binary data promoted, +1,978 tests) | | 18,484 / 24,120 (76.6%) |
| 2026-11-01 (0041 done) | 62 expected failures | 18,833 / 24,120 (78.1%) |
| 2026-11-01 (D011: generators + async iteration promoted, +8,213 tests) | | 22,605 / 32,333 (69.9%) |
| 2026-11-01 (0040 classes done) | 52 expected failures | 31,791 / 32,718 (97.1%) |
| 2026-11-01 (0044 + 0033 done) | 38 expected failures (before 0033) | 32,303 / 32,718 (98.7%) |
| 2026-11-01 (0046 BigInt done + promoted) | | 33,450 / 33,854 (98.8%) |
| 2026-11-01 (0051 p1: iterator helpers promoted) | | 33,879 / 34,283 (98.8%) |
| 2026-11-01 (0050: RegExp promoted, +2,186 tests) | | 36,285 / 36,689 (98.9%) |
| 2026-11-01 (0051 done) | | 36,778 / 37,182 (98.9%) |
| 2026-11-01 (0059: indirect eval + Function, +540 tests) | | 37,464 / 37,722 (99.3%) |

## Id reservations (in flight; check main for the authoritative list)

| what | id | owner / issue |
| --- | --- | --- |
| ObjectClass RegExp, RegExpStringIterator | 25, 26 | num / 0033 |
| ObjectClass Proxy | 27 | infra / 0045 (merged) |
| ObjectClass BigInt wrapper | 30 | heap / 0046 (merged) |
| CellKind BigInt | 24 | heap / 0046 (merged) |
| ObjectClass IteratorHelper, WeakRef, FinalizationRegistry, DisposableStack | 28, 31, 32, 33 | heap / 0051 (merged) |
| ObjectClass RawJSON | 29 | num / 0061 (merged) |
| CellKind AccessorPair, Shape, PropValues | 25, 26, 27 | heap / 0083 |
| object header flag OBJ_DICTIONARY | 128 | heap / 0083 |
| next free ObjectClass | 34 | |
| func_flags PRIVILEGED | 1 << 8 | heap / 0043 (merged) |
| func_flags HOME | 1 << 6 (check) | lexer / 0040 |

Realm `Intrinsic` enum: append at the end; rebase conflicts there are expected and trivial.

## Parallelism notes

0002, 0003, 0004 and 0005 touch disjoint areas (the only shared file is
`djs-core/src/lib.rs`, mod lines only, which rebases trivially), and none depends on
another. So they run in parallel.
