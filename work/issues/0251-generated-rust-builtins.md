# 0251: Builtins generated as Rust from their TS sources; one implementation per builtin

status: done
agent: owner session
branch: issue/0251-generated-rust-builtins
depends-on: -
touches: crates/djs-bgen (emit_leaf.rs, emit_js.rs, check.rs, behavior test), builtins/*.ts (@rust tags, GetSite, StrPad), crates/djs-core (builtins/fast.rs, builtins/gen/leaves.rs, interp/run/fastpath.rs, run.rs, stack.rs, frame.rs, conv.rs, heap/string.rs), djs-cli/difftest/test262/bench (--backend removed), scripts/gate.py, tests/scores/size.tsv, tests/builtins/registry*
guide: builtins.md, interpreter.md, testing.md, cli.md, determinism.md

## Goal
Small builtins (`trim`, `toLowerCase`, `Math.floor`, `slice`, ...) were 1.5-3x QuickJS -O2:
a call ran a bytecode frame, the spec prologue and several intrinsic dispatches, or a
hand-written `@fast` twin that had to mirror the TS step by step. Generate Rust for the
hot builtins from the TS sources instead, drop the requirement that a bytecode twin exist
and agree, and keep the module within the D020 budget.

## Acceptance criteria
- [x] djs-bgen's leaf backend: `@rust` builtins compile to straight-line Rust; bgen refuses
  ineligible bodies.
- [x] User code only at conversion sites (ToPrimitive of an object, Get), replayed through
  the interpreter with shadow slots; errors, stack frames and suspension as for bytecode.
- [x] Collect-and-retry at the heap cap; test.
- [x] One implementation per builtin: no bytecode bodies for @native/@rust; the bytecode
  backend option and the gate leg removed.
- [x] COMPAT_VERSION bump; decisions; guide; size ceilings.

## Log
- Classified the installed builtins (experiment branch `experiment/tostring-inline`):
  ~119 are leaf-shaped on primitive arguments, ~104 when ToPrimitive of an argument may
  be replayed.
- Hand-written owned Rust first (experiment branch `experiment/rust-builtins`): it
  replaced arguments with their primitives and so got
  `trim.call({ toString() { return null; } })` wrong; the generated version with shadow
  slots does not.
- Generated code needs typed direct leaf calls (through `pure_leaf` it was 25-30% slower
  than the hand-written entries) and inlined helpers (out-of-line helper functions saved
  13 KB raw wasm but no gzip and cost 2-12%).
- Leaf fast paths found by profiling the generated builtins: one-byte trim/case/substring,
  radix-10 toString without the radix buffer, StrPad (one buffer, memset for one-char
  fillers).
- Conversion sites for Get (`%GetSite`) make startsWith/endsWith/includes generatable; an
  error after a replay is thrown at the original call's key (it was thrown at the
  returning frame's pc and escaped the handler: caught by the corpus case).
- The inlining budget of the bytecode prelude: 40 (smallest base image).
- Verified locally: fmt; clippy host, wasm32 release, feature combinations, the wasm
  modules; workspace tests; difftest full corpus lazy with --suspend-every 97, GC stress,
  determinism; unsafe-heap tests and release difftest; test262 active (lazy); slim tests,
  slim difftest, slim test262; js tests; codemode workloads and bridge; native vs wasm
  images; registry append-only; size budget.
- Guide pages updated: builtins.md (three backends, `@rust`, which builtins run as Rust,
  trade-offs), interpreter.md (the call table), testing.md, cli.md, determinism.md.

## Follow-ups
- Re-record the engine benchmarks (`scripts/status.py engines`) on a quiet machine and
  update the prose of docs/guide/benchmarks.md "Operation by operation", which still
  describes the slow small builtins.
- The module is 289 B under the D020 budget: the next size increase needs an offsetting
  saving.
- Remaining gaps to QuickJS -O2: `String(i)`, `toFixed`, `charCodeAt` loops (number to
  string allocation; leaving the tier-0 loop for every builtin call).
- Convert more `@fast` entries where they fit `@rust` (e.g. `Number`, `String` as
  functions need `new.target`/`%ArgumentCount` handled; `split` needs a loop).
- A getter with a computed symbol name prints `<anonymous>` in stack traces where qjs
  prints `get [Symbol.match]` (pre-existing).

## Result
Merged: djs-bgen's leaf backend (D3260) with conversion sites, 56 builtins generated
(22 functions), their `@fast` entries deleted; one implementation per builtin (D3261),
`--backend` and `ExecOptions::native_builtins` gone; inlining budget 40 (D3262);
COMPAT_VERSION 9; size ceilings +2.8 KB (D3263). Wasm instructions per call, main →
now: lastIndexOf -73%, startsWith -75%, toLowerCase -62%, trim -51%, toString -52%,
Math.floor -42%, padStart -31%, slice -18%; wall clock against QuickJS -O2:
toLowerCase 1.85x → 0.54x, trim 2.93x → 1.34x, Math.abs 1.86x → 1.10x.
