# 0209: Plain dump data bugs: accessors of factory closures, static private members, runtime-made objects

status: in-progress
agent: pjd
branch: issue/0209-plain-data-bugs
depends-on: -
guide: -
touches: vm/dump_flight.rs, vm/dump_plain.rs, vm/dump.rs (data/heap side only), tests/corpus, js/test/dumpload.test.ts

## Goal
Fix plain-dump bugs on the data side (found by 0208's fuzzer and by pjf in 0205):

1. An object-literal accessor that captures a factory variable is written as `get: ,`
   (the dump doesn't compile): `let n = 0; const o = { get a() { n++; return n; } };
   await tools.echo(1); return o.a;` (~60 of 300 fuzz programs).
2. Any class with a static private member is refused as "an instance of class … with
   private name #o": `class B { static #o = 0; static o() { return B.#o; } }
   globalThis.B = B;`. D2878 says static private members are unaffected, and the message
   mislabels the class as an instance.
3. 0207 follow-up: objects the dump runtime made (`$djs.fromSync`'s wrapper,
   `closeThrow`'s handler) are written as the runtime's closures (~1.6 KB per re-dump of a
   `for await` over a sync iterator). Write them as their helper call again.

4. (E, pjz batch 3) An integer-like key equal to its value, `const o = { 5: 5, x: 1 }`, is
   written as the shorthand `{ 5, x: 1 }` (key text == value text): the dump does not
   compile, or a chain crashes at load. Shorthand only for identifier keys that are valid
   binding names.
5. (F, pjz batch 3) An empty reference, a cousin of `get: ,`: an instance of a class that
   lives in K's block, captured by an async function that is not running, gets
   `Object.setPrototypeOf($dump.n10, );`. Find the common cause with bug 1 (a value whose
   expression is known only once K has run, written as empty text) and fix it once.
   ```js
   class C { m() { return tools; } } const o = new C();
   async function f() { for (let i = 0; i < 1; i++) { let [] = [await tools.echo(1), o]; } }
   return await tools.echo(1);
   ```

Each fix gets a corpus case (or a dumpload case), and any plain-xfail marker 0208 added
for it is removed.

## Constraints
pjr is changing classes in dump.rs/dump_plain.rs (class self-reference cycles), then
dump_origin.rs; pjs is changing dump_cont.rs; pjf is deleting format 2. Keep merges small
and rebase often. Decision IDs D2950–D2959.

## Log
- Bug 1 (`get: ,`): the accessor's getter is a closure a plain dump makes in the
  continuation (`plain_patched`), so `expr` returns empty and the descriptor patch had a
  hole. `Dumper::descriptor` now reads every value with `expr_or_loc` (descriptors are
  only written in patches, after the values exist). Corpus case
  codemode/plain-accessors.js (djs and node, 2 links).
- Item 2 (static private members) dropped by the coordinator: pjr fixed it in 657c5a5e
  (D2883), with self-naming classes. Remaining: bug 1, then bug 3 (runtime-made objects).
- First merge attempt: the gate's build:core-tests failed to link (undefined symbols in
  compiler/resolve and regexp, untouched here): a stale incremental cache, which
  gate.py's retry did not recognize (it matched only `anon.*.llvm.*`). Widened its regex
  to any `.llvm.<n>` symbol (coordinator's OK). Next attempt: the new corpus case needed
  its pinned suspension map (tests/smap).
- Coordinator added E (integer-like shorthand keys) and F (empty prototype reference).

## Follow-ups

## Result
