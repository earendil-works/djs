# 0036: P1-6 Destructuring, tagged templates, remaining spread/rest

status: done
agent: lexer
branch: issue/0036-p1-destructuring-templates
depends-on: 0029
touches: crates/djs-core/src/compiler/, crates/djs-core/src/bytecode/ (additive), builtins/ (only if a helper is needed, e.g. %CopyDataProperties for object rest), docs/compiler.md

## Goal
docs/compiler.md section 10, row P1-6: binding and assignment patterns everywhere (`let
{a, b: [c, d = 1], ...rest} = x`, parameters, `catch (e)` patterns, for-in/of heads, and
assignment `[a, b] = [b, a]` with the 3.3 memoized pattern scan replacing the minimal
check), defaults, holes and elisions, object rest (copy with excluded keys), array rest,
and iterator closing on abrupt completion during array destructuring (the same unwind
mechanism as for-of). Tagged templates (cooked/raw, invalid escapes → undefined cooked,
the per-site template object cache, frozen arrays with `raw`). Codemode scripts use
destructuring constantly, and test262 has ~1,500 tests behind it (0007 clusters).

## Acceptance criteria
- [x] All pattern positions above, with spec evaluation order (property keys, defaults,
      and iterator steps interleaved correctly), and early errors (invalid targets,
      rest not last, rest with an initializer, duplicate `__proto__` only outside
      patterns, which is already done).
- [x] Interpreter needs (for example a copy-with-excluded-keys op or intrinsic, or the
      template object cache in the realm) are sent to the coordinator early as one
      list. heap is on 0035 (perf) in interp/. If an item is tiny and additive, you may
      do it yourself and log it.
- [x] e2e tests checked against qjs, a disassembly snapshot, the test262 compile sweep
      before/after, and the ratchets updated.

## Log

- test262 compile sweep before (release): 27,531 ok / 22,670 not supported yet / 1,810
  negative / 607 other errors.
- Interp needs sent to the coordinator as one list; it OK'd doing them myself:
  `pick`/`poke` stack ops (+ the `push M + imm K` stack-effect form in `ops!`, decoder,
  `OPS_HASH`, the interpreter's debug stack-effect check now reads U8 immediates too),
  `get_template_object` (new arm + `template_object` helper in interp/run.rs), and the
  `copy_data_props` fallback to `%CopyDataProperties` for getters/strings (like
  `array_spread`). `require_object_coercible`'s message is now qjs's "cannot convert to
  object" (the op had no other user). All in interp/run.rs as new arms/helpers; expect
  a rebase against heap's 0037 there.
- Builtins: `%IteratorRest` (iter.ts), `%CopyDataProperties` and
  `%CopyDataPropertiesExcluding` (spread.ts), `@intrinsic` on ToPropertyKey; the
  intrinsic id table gained them in prelude order.
- Compiler: new `compiler/pattern.rs`. Patterns before their value are skipped and
  parsed after it by rewinding the lexer (D1360). Array patterns keep `next it` and
  reach under target references with pick/poke; the close handler's depth is relative
  to a label and computed at resolution with the new `bytecode::stack_depths` (D1361).
  The 3.3 memo (`scan::PatternScan`) replaces `closes_before_assign` and records what
  follows every bracket (`=`, element end, `of`/`in`).
- Sites: let/const/var declarators, parameters (pattern parameters take an argument
  slot without a binding; their names are `Param`s, pushed to `f.params` for the
  prologue), catch (binds like `let`), for-in/of declaration and expression heads,
  assignment expressions.
- Tagged templates in call and `new` member position (D1362); NamedEvaluation through
  parentheses (`x = (function () {})`), which the dstr `fn-name-cover` tests need.
- Error messages and positions follow qjs (strict) for the early errors in
  `tests.rs::errors`. Remaining differences: qjs reports a duplicate pattern parameter
  at the end of the function, and `[a] += 1` one token later.
- Sweep after: 30,606 ok / 19,361 not supported yet / 2,045 negative / 594 other errors
  (no new "other errors"). test262 active: 1,940 more passing (6,365 expected failures
  left); the remaining dstr failures are all classes. Corpus: 3 more passing
  (test_function_length, test_template, test_template_skip) plus the new
  `language/destructuring.js` (clean under --suspend-every 97 and --gc-stress).
- Fix after the first merge (1447e1c): removing the CR test case from the corpus file
  left half a line behind, so `destructuring.expected` held qjs's SyntaxError and the
  case tested nothing. Fixed; it now has 107 lines of results.

## Follow-ups

- Array destructuring fast path for arrays with unmodified iteration (as D1292 for
  for-of): `const [a, b] = pair` runs the full protocol now.
- `(fn) = function () {}` must not name the function (parenthesized LHS is not an
  IdentifierRef; test262 `fn-name-lhs-cover.js`); djs names it. Pre-existing.
- `({}).__proto__` is undefined in djs (no `Object.prototype.__proto__` accessor), so
  `const {__proto__: p} = {}` differs from qjs.
- `construct` on a non-constructor says "not a function"; qjs says "not a constructor"
  (`new id`x``).
- Generators (P5): a `yield` inside a pattern default is a return completion when
  the generator is closed; the array pattern's iterator must then be closed too
  (an `Iter`-like unwind entry for patterns).

## Result

Destructuring everywhere: binding patterns in `let`/`const`/`var`, parameters (also
arrows, methods, setters, rest `...[a, b]`), `catch`, for-in/of heads, and assignment
patterns (`[a, b] = [b, a]`, `({x: o.y = 1} = v)`, nested, in for heads). Defaults,
holes, elisions, object rest (`%CopyDataPropertiesExcluding`), array rest
(`%IteratorRest`), computed keys converted once, spec evaluation order, iterator close
on normal and abrupt completion (not on iterator errors). Tagged templates with cooked
(`undefined` for invalid escapes) and raw strings and one frozen template object per
site. Object spread with getters and strings. Design: docs/compiler.md 3.3 and 3.10,
D1360 to D1363. Tests: `tests.rs` (`end_to_end_destructuring`,
`end_to_end_destructuring_iteration`, errors), the `destructuring` disassembly
snapshot, `tests/corpus/language/destructuring.js`, scan unit tests.

Known gaps: see Follow-ups (array fast path, parenthesized-LHS naming, generators).
