# 0029: P1-5 Loops and iteration (compiler side + iterator builtins)

status: done
agent: lexer
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
- [x] `for (x of y)`, `for (const/let/var x of y)` with per-iteration bindings,
      `for-in` (spec order of keys from 0005's own-keys, prototype chain, deleted
      during enumeration, shadowing), all with labels.
- [x] Iterator close on abrupt completion, in every exit path, plus the exception
      precedence rules (a throw from the body wins over a throw from `return()`).
- [x] The array fast path (if the note specifies one): it must be unobservable, i.e.
      only taken when `Array.prototype[Symbol.iterator]` and
      `%ArrayIteratorPrototype%.next` are unmodified.
- [x] Iterator builtins in the builtin language, checked by bgen, generated, compiling
      in privileged mode. bgen may need installation forms for symbol keys (the 0017
      follow-up); add them to bgen in a minimal way (log it, since that is infra's code)
      or ask.
- [x] e2e programs checked against qjs. Cases that need interpreter work go into an
      ignored `_0028`/`_rt` test, with the list of interp needs sent to the coordinator
      early.
- [x] The test262 compile sweep before/after.

## Log

- Part 1 (builtins, before the compiler side, so 0030 and 0034 can use the names):
  - `builtins/iter.ts`: `%GetIterator`, `%IteratorClose`, `%IteratorCloseThrow`
    (`@intrinsic`); Array.prototype.values (also `[Symbol.iterator]`), keys, entries;
    `%ArrayIteratorPrototype%.next`; `%IteratorPrototype%[Symbol.iterator]`. D1290.
  - `builtins/intrinsics.d.ts`: leaf intrinsics CreateArrayIterator, ArrayIteratorTarget /
    Index / SetIndex / Finish / Kind, CreateIterResultObject, CreateArrayFromPair. Their
    interpreter side is 0030's (until then they fault with "not supported yet").
  - bgen (rebased onto infra's 0031 symbol keys / repeatable `@builtin`; my duplicate
    version of that was dropped): `%Name%` as the first segment of an install target
    (D1291). behavior.rs rewrites `%` to `I.` only outside strings; behavior.js has
    stand-ins for the iterator leaves and compares values/keys/entries/@@iterator/next
    against qjs, capped so a huge length cannot hang (an earlier uncapped version hung
    qjs for ~1h45m). qjs reads Array Iterator lengths with ToUint32 (`{length: -5}`
    iterates 2^32 - 5 elements); that subject is skipped, we follow the spec.
  - `INTRINSICS` table regenerated (ids appended after InstanceofOperator).
  - String.prototype[Symbol.iterator] is not in part 1 (StringCodePoints exists; it needs a
    String Iterator object, left to a follow-up or part 2).
  - Merged as beb2e53. Coordinator: String.prototype[Symbol.iterator] goes to infra (0034).
- Part 2 (compiler):
  - `compiler/for_in_of.rs`: for-in/of heads (`var`/`let`/`const` binding, expression
    targets: name, field, element), QuickJS's source-order layout; declaration heads
    store next to the body (one jump per iteration). Head scans in scan.rs
    (`for_head_is_c_style`: `;` directly inside the parens; `for_lhs_ends_with_of`).
    D1292, docs/compiler.md 3.8 / 4.5 / 5.9 / 7.4 updated.
  - Iterator close: handler around store + body (`%IteratorCloseThrow`, then rethrow);
    own `break` closes inline after the loop; leaving jumps `gosub` a close subroutine
    (`BlockKind::Iter`), unwound with the finally blocks in one innermost-first pass
    (`emit_unwind`, replaces `emit_finally_for_return`). First attempt had the
    subroutine consume its operands: the stack checker models `gosub` as depth-neutral,
    so it now leaves `v it undefined` for the caller to drop.
  - qjs messages/positions for head errors ("expected 'of' or 'in' in for control
    expression", "a declaration in the head of a for-in/of loop can't have an
    initializer", "invalid for in/of left hand-side", "'for of' expression cannot start
    with 'async'"). Differences: `for (f() in o)` is a SyntaxError here (spec early
    error; qjs, sloppy, throws at run time).
  - Tests: `end_to_end_iteration` (61 cases vs qjs: protocol, close on every exit,
    precedence, TDZ, per-iteration closures, for-in order / prototype chain / shadowing /
    non-enumerable / deletion), errors, snapshot `iteration.js`, scan unit tests,
    corpus `language/iteration.js` (also under --suspend-every/--gc-stress).
  - Ignored: `end_to_end_iteration_rt` (arrays, needs 0030's Array Iterator leaves) and
    `end_to_end_iteration_messages` (symbol keys in "cannot read property" messages).
  - Bug found by the sweep: a body error inside a nested function popped the wrong
    BlockEnv; envs are now popped only on success.
  - test262 compile sweep (`compile_test262`): before 25,999 ok / 24,330 not supported
    yet / 1,742 negative / 607 other; after 27,531 ok / 22,670 / 1,810 / 607 (3 new
    "other": `for (using x of ...)` heads, explicit resource management). Missed early
    errors unchanged (8).
  - Ratchets (after the rebase onto 0030 part 4): test262 active 10,131 -> 8,365
    expected failures (1,766 pass now, no new failures); difftest 80 -> 78
    (test_for_in2, test_try_catch8).
  - After the rebase onto 0030 part 4 (Array Iterator leaves, qjs symbol-key messages):
    `end_to_end_iteration_rt` and `end_to_end_iteration_messages` pass and are no
    longer ignored; the corpus case covers arrays too.
    `%ArrayIteratorPrototype%[Symbol.toStringTag]` ("Array Iterator", configurable
    only) added to iter.ts as asked by the coordinator.

## Follow-ups
- Array fast path for for-of (docs/compiler.md 4.5 calls it a later optimization): a
  guard that `Array.prototype[Symbol.iterator]` and `%ArrayIteratorPrototype%.next` are
  the originals, then index directly.
- Destructuring heads (`for (const [k, v] of m)`), with destructuring in general.
- `for await` (P2).
- Empty handler regions (an expression-head for-of with an empty body) are harmless
  but could be dropped in resolve.

## Result
- Part 1 (beb2e53): `builtins/iter.ts` (`%GetIterator`, `%IteratorClose`,
  `%IteratorCloseThrow`, Array.prototype values/keys/entries/@@iterator,
  `%ArrayIteratorPrototype%.next`, `%IteratorPrototype%[Symbol.iterator]`), the Array
  Iterator leaf declarations (implemented by 0030), `%Name%` install targets in bgen.
- Part 2: for-of and for-in in the compiler with per-iteration bindings, labels and the
  full close protocol (D1292), over any iterable including arrays (with 0030).
  `%ArrayIteratorPrototype%[Symbol.toStringTag]`. No ignored tests left from 0029.
- Known gaps: destructuring heads, `for await`, no array fast path.
