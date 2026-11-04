# 0209: Plain dump data bugs: accessors of factory closures, static private members, runtime-made objects

status: done
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

- Bug 3 (runtime-made objects, D2951): dump_flight_runtime.rs recognizes `fromSync`'s
  iterator, `closeThrow`'s handler and a `fromSync` step in flight by their closures'
  text (the runtime's, checked by a test) and captures. The iterator counts as an
  async-from-sync iterator (`is_from_sync`, written `$djs.fromSync(it, next)`); the
  closures become `Clo`s of the engine's kinds (`FROM_SYNC_UNWRAP`/`FROM_SYNC_CLOSE`);
  the step's reaction gets the step's promise as its derived promise (the runtime's own
  derived one is dropped) and no reject handler where the step passes rejections on.
  Case codemode/plain-for-await-sync.js (10 links, djs and node, none holds runtime
  code); a re-dump of such a loop is −1.7 KB.

- Item 7 (static code runs again, D2950, approach agreed with the coordinator): the
  compiler records the spans of the outermost class's static block bodies and static
  field initializers (`compiler::class_spans`, a `class_spans` recorder in parser.rs and
  class.rs: outside my touches, approved); dump_statics.rs blanks them in the text a
  plain dump writes for a class (`class_texts`, which `fn_source` returns, so the writer
  and the checker see the same text). A literal static private initializer stays; any
  other is refused. Computed keys turned out to be refused already unless stable
  (`computed_keys`/`stable_key`: literals and well-known symbols), so no new rule. Cases
  codemode/plain-class-statics.js (2 links, djs and node) and
  codemode/plain-class-static-private.js (a refusal). Docs: plain-js-dumps.md §10.

- Rebased onto 0205 (format 2 gone). Restored the tests 0205 changed to avoid my bugs:
  dump_tests' static block in `classes_with_statics_accessors_and_computed_keys`, static
  private fields/methods/accessors (a case of their own: instances with private names
  stay refused, D2878), a parameter-property case in
  `typescript_dumps_say_so_in_the_directive`; dump_resume_tests' TDZ closure made in a
  callback.
- Item 8 (0205's list: "a closure over a TDZ binding made in a callback is refused"):
  cheap. dump_plain.rs `emit_list` refused a global `let`/class declaration that a
  callback must write (its value is made in K). Like `var`, it is now declared at the top
  and assigned in the callback (`let` and class bindings are mutable, and nothing reads
  them before the callback runs); a global `const` there stays refused, with a clearer
  message.
- pjz's (0211) loop case: codemode/plain-class-static-loop.js (2 links, djs and node).

- js tests found that a class's internal functions (`<fields 0>`) share the class's text:
  `class_texts` is now keyed by the original text, so every function of that text reads
  the blanked one (dump_tests' Point got an instance field to cover it).
- Pre-merge checks for batch 2: fmt; clippy (djs-core, djs-cli, djs-difftest, slim lib);
  djs-core lib and djs-cli tests; slim difftest; plain-dumps leg (1297 links, 0 differ,
  refusals ratchet +plain-class-static-private); dump-size per case updated (new cases,
  classes-self +19 B gzip from blanks, harvest noise ±5 B); size budget (442,468 /
  378,032, under the ceilings); e2e plain and dump; js tests on the dev wasm (129 pass).

## Follow-ups
- dump_resume_tests.rs `plain_dump_chains_continue_like_the_run` still discounts lines
  that copy the runtime's code (`runtime_bytes`); after D2951 the allowance should be
  unneeded (left alone: 0205 was deleting/moving that file).
- Computed class keys (coordinator): they also run at class evaluation; the exporter
  already refuses any but literals and well-known symbols (`computed_keys`/`stable_key`).
  Writing other computed keys (e.g. as their evaluated key) is open.
- A global `const` that a callback must write is still refused ("a global const
  declaration inside a callback"); no case reaches it so far.
- Dead format-2 branches in the exporter (0205's follow-up) are still there.
- The decisions index needs D2950/D2951 (coordinator's file).

## Result
Merged in two batches (main a4bd72fb, 8bfa2779):
- `get: ,` and empty prototype references (bug 1, F): one cause, values whose location
  is known only once a continuation runs. `expr_or_loc` writes a deferred reference that
  `finish` resolves (or refuses as internal). E: shorthand keys only for identifiers.
- Static private members (item 2): fixed by pjr (D2883).
- Runtime-made objects (bug 3, D2951): `fromSync`'s iterator, `closeThrow`'s handler
  and a step in flight are read back by their text and written as the helper calls
  again (dump_flight_runtime.rs); −1.7 KB per re-dump of such a loop.
- TS parameter properties (item 6): not erasable; the dump stays TypeScript. Also fixes
  the REPL's top-level-await path for non-erasable TypeScript.
- Static code (item 7, D2950): class text written with static blocks and static field
  initializers blanked (compiler::class_spans, dump_statics.rs); literal static private
  initializers stay, others are refused. toString shows the blanks after a load.
- A global `let`/class assigned in a callback (item 8): declared at the top.
- gate.py: link failures on any `.llvm.<n>` symbol count as incremental corruption.
Cases: codemode/plain-accessors, plain-for-await-sync, plain-class-statics,
plain-class-static-private (refusal), plain-class-static-loop, ts/plain-parameter-properties;
pjz's fuzz cases for E/F/bug 1 now pass; dump_tests/dump_resume_tests cases 0205 avoided
are restored. Guide: no page affected.
