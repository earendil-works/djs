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

6. (pjf) TypeScript parameter properties are lost: `class P { constructor(public x:
   number) {} }` is blanked to `constructor(       x        ) {}` under the plain
   `"djs-dump";` header (D2879 treats `public` as a type), so `new P(1).x` is undefined
   after a load. Keep such texts TypeScript, insert the assignments, or refuse clearly;
   never change behaviour silently.
7. (pjf) Static blocks and static field initializers with side effects run again when a
   plain dump evaluates the class text (format 2 skipped them with jump_if_loading, which
   0205 deletes): `const log = []; class P { static { log.push('init') } }` gives "log is
   not initialized", or would push twice. Their effects are in the dumped heap already.
   Approach to be agreed with the coordinator first (class text, readability,
   Function.prototype.toString).

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
- E: `literal` wrote shorthand whenever the key text equalled the value text; now only
  for keys that are names (`is_ident`, which excludes reserved words). Case
  codemode/plain-literal-keys.js.
- F and bug 1 share a cause: a patch that names a value whose location is known only
  later (the class's prototype is located when the continuation's factory record is
  finished; a closure made in K) got empty text from `expr`/`expr_or_loc`. `expr_or_loc`
  now writes a reference (`\u{1}<node>\u{1}`) for a node without a location, and
  `finish` resolves them in every line (and in the plain writer's output) once all
  locations are known; one that never gets a location is refused ("internal: a value
  the dump refers to but never makes"). Case codemode/plain-class-instances.js.
- Owner's rule (batch merges, check locally first): before merging ran fmt, clippy
  -p djs-core, vm::dump tests, e2e dump mode, the plain-dumps leg locally (difftest
  --plain-out over the corpus, plain-node: 1228 links, 0 differ, 0 refused) and the
  dump-size score: corpus_raw +21 KB from the three new cases' six links, so the score
  was raised with `--update`. Pinned the new cases' suspension maps.
- Rebased onto 0208's fuzz cases, which cover E (integer-key-equal-value), bug 1
  (getter-capturing-frame-variable) and F (class-instance-in-idle-function, plain-xfail):
  dropped my duplicates plain-literal-keys/plain-class-instances (kept plain-accessors:
  setter and non-enumerable accessor), removed F's plain-xfail marker, dropped the two
  stale refusals, raised dump-size for the links that now dump (+17 KB raw).

- Item 6 (TS parameter properties): D2879 already meant such texts to stay TypeScript,
  but the blanking erased `public`/`readonly` and the result compiled. `strip_types` now
  keeps the modifiers of constructor parameters (not erasable, like an enum), so the
  dump's JavaScript check fails and the dump keeps `"djs-dump typescript";`. Outside my
  touches (minimal): compiler/ts.rs (the one erase), its ts_strip test, and the REPL's
  top-level-await path (djs-cli repl/mod.rs), which used the stripped text whenever
  strip_types succeeded: it now checks that the text compiles as JavaScript, else rewrites
  the TypeScript (that also fixes `enum F {...}; await 1; F.B`, which failed before).
  Cases: ts/plain-parameter-properties.ts, a repl test.

## Follow-ups

## Result
