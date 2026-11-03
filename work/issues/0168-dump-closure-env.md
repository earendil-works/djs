# 0168: `.dump` loses a closure's captured environment

status: done
agent: portable
branch: issue/0168-dump-closure-env
depends-on: 0167
guide: -
touches: crates/djs-core/src/vm/dump*.rs, tests, js/proto (keep the TS dumper in step), docs/functions-as-code.md

## Owner's report
```
> let foo = [1,2,3,4];
> let x = this;
> function getStuff() { return [foo, x]; }
> function blah(a) { return () => a }
> const blah2 = blah(2);
> .dump /tmp/test.js
```
writes
```
let foo = [1, 2, 3, 4];
let x = globalThis;
const blah2 = () => a;
function getStuff() { return [foo, x]; }
function blah(a) { return () => a }
Object.defineProperty(blah2, "name", { value: "" });
```
`blah2` is emitted as `() => a` without its captured `a = 2`: loading the dump gives a
closure that throws ReferenceError (or reads a global `a`). The design (D2352/D2353) says
closures over non-global bindings are rebuilt by a factory per sharing group; that didn't
happen here. Silent wrong output is worse than a refusal.

## Acceptance criteria
- [x] Root cause found and fixed (parameter captures? REPL scripts compiled one line at a
      time? the factory path skipped when the closure's outer function is a global?).
- [x] blah2 dumps as a factory with `a = 2`; loading and calling it returns 2.
- [x] A **checker**: after building a dump, every free identifier of every emitted function
      body resolves to an emitted binding, a factory parameter, or a global/builtin; any
      other free name makes the dump refuse (no silent wrong output). Covered by tests.
- [x] Tests for closures from factories (params, locals, nested scopes, loops with `let`,
      closures sharing a binding, a closure capturing another closure), in a script run
      and in REPL line-by-line input; the TS dumper in js/proto agrees on these cases.
- [x] Minor: `.dump` into a missing directory gives a clear message (it does); fine.

## Log
- Root cause: `emit_root` wrote a root whose value was itself a closure of a scope group
  in place (`render`), bypassing `emit_group`; closures reached through objects (`c =
  makeCounter()`) went through `expr` and were fine. Not the parameter capture, not the
  REPL's line-by-line scripts, not the outer function being global: any root that is a
  capturing closure (`const f = mk()`, `const [inc, get] = pair()`) was affected. Fixed:
  such roots go through `expr`, which emits the factory (key: the root's name when the
  function has none). Same fix in js/proto/dump.ts. A guard refuses ("internal: a closure
  written outside its factory") if a grouped closure is ever rendered in place.
- Checker (D2355): compiles the dump, cut down to function-bearing lines and root names,
  into the dumper's private base heap, and compares each function's free names
  (captures; globals from get_global/put_global/typeof, nested functions included) with
  the original template's. Refuses names not in scope, globals hidden by factory
  variables, globals missing after load, a dump that does not compile. First version
  compiled the full dump with `Heap::new()` (+8.6 KB gzip, slice panics) and compared
  every node against every same-text entry (588 ms for 10k records + 1k counters in
  wasm); now dedupes entries by text+names and reuses the base heap: +3 KB gzip, 178 ms
  (99 without checker). Harvest coverage found one false refusal (a captured variable
  left to the root of the same name holding the same value); the rule allows it.
- Tests: dump_tests closure cases (params, locals, nested, block scope, `let` loops,
  shared bindings, closures capturing closures), each as one Script and line by line;
  `dump::check_tests` feed the checker bad texts; js/proto/compare.ts gained the same
  cases in both input styles and js/test/dumpproto.test.ts runs it in the gate (Rust and
  TS give identical text on all of them). REPL checked by hand: `.dump` then `.restore`,
  `blah2()` is 2.
- Coverage unchanged: harvest 300/300 equal; private 2,607 dump and load, 2,601 equal.
  The old equality compared functions as text, so it could not see this bug.
- Docs: docs/functions-as-code.md (the checker, sizes, timings, tests); D2355.

## Result
A root that is a closure over a function's variables (`const blah2 = blah(2)`) is now
made by its group's factory (`let a = 2; return { blah2: () => a }`) in the Rust and TS
dumpers. Every dump is checked by compiling it: a function whose captured or global names
would resolve differently after loading refuses the dump (D2355). Tests in both input
styles and a gate-run Rust/TS comparison. Module cost of the checker ~3 KB gzip
(dump feature 479,077 vs 443,468); dumping time +~75% on a closure-heavy 10k-record heap.
