# 0168: `.dump` loses a closure's captured environment

status: open
agent: -
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
- [ ] Root cause found and fixed (parameter captures? REPL scripts compiled one line at a
      time? the factory path skipped when the closure's outer function is a global?).
- [ ] blah2 dumps as a factory with `a = 2`; loading and calling it returns 2.
- [ ] A **checker**: after building a dump, every free identifier of every emitted function
      body resolves to an emitted binding, a factory parameter, or a global/builtin; any
      other free name makes the dump refuse (no silent wrong output). Covered by tests.
- [ ] Tests for closures from factories (params, locals, nested scopes, loops with `let`,
      closures sharing a binding, a closure capturing another closure), in a script run
      and in REPL line-by-line input; the TS dumper in js/proto agrees on these cases.
- [ ] Minor: `.dump` into a missing directory gives a clear message (it does); fine.

## Log

## Result
