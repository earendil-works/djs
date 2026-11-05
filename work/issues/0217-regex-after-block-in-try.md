# 0217: Parser: a regex literal after a block statement inside `try`

status: done
agent: pjp
branch: issue/0217-regex-after-block-in-try
depends-on: -
guide: -
touches: crates/djs-core/src/lexer/, crates/djs-core/src/compiler/ (parser), tests

## Goal
`try { if (1) {} /x./.test("xy"); } finally {}` fails with SyntaxError "expecting catch or
finally"; node accepts it. A regex literal of two or more characters right after a block
statement, inside a `try` block, fails to parse (`/a/` passes, and so does the same code outside
`try`). Found by 0211's fuzzer: plain dumps hit it because a continuation wraps user code in
try/finally (`}\n/./s.source;`), so the dump fails to load. Corpus case
language/regex-after-block-in-try.js (pjz adds it to the ratchet). Find the root cause in the
regex-vs-division decision after `}` (statement vs expression context, probably a lookahead or
re-lex path tied to the try parser), check related shapes (after `if`/`for`/`while` blocks,
labeled blocks, function declarations, class declarations, inside catch/finally, nested try,
`/=` and `/` division after an expression-closing `}`), add tests, merge.

## Log
- Root cause: not the parser but its token-level lookahead. `parse_try` asks
  `scan::try_has_finally` before parsing the block (D1080), and the scanner decides regex vs
  division from the previous token only (QuickJS's `is_regexp_allowed`): after `}` it is
  division. `/x./` then lexes as `/`, `x`, `.`, and the second `/` follows a `.`, so it starts
  a regex that runs to the end of the line, the scan fails and the try counts as having no
  finally. `/a/` happened to stay balanced as two divisions; outside `try` nothing scans.
- Fix (compiler/scan.rs, the shared `Scanner`): the bracket stack tells a block `{` from an
  object `{` and an `if`/`for`/`while`/`with` head `(` from an expression `(`; a `/` after
  one of those closes starts a regex. A `{` is a block after `)`, `=>`, `;`, `{`, `}`,
  `else`/`try`/`catch`/`finally`/`do`, after `:` inside a block (label, `case`), and as a
  class body. The try lookahead starts with a block. Still a guess: a function or class
  *expression* followed by `/` is taken for a declaration (division there means NaN).
  Benefits the other scans too (arrow heads, patterns, for heads) when they contain
  function bodies.
- Shapes checked (unit test `finally_lookahead`, corpus case): after if/else/for/while blocks,
  bare block, labeled block, `case` block, function and class declarations (also `extends
  f({})`), arrow body + newline, `if (a) /re/`, nested try, inside catch (with and without a
  parameter) and finally, the fuzzer's `}\n/./s.source`, `/=` as a regex start; division
  after an object literal, a call, a template, `/=`.
- Tests: compiler::scan unit tests; tests/corpus/language/regex-after-block-in-try.js
  (.expected from qjs; no ratchet line, it passes). Local: fmt, clippy, size ceilings main
  442,508 / 442,831 and slim 378,092 / 378,201, test262 language (ratchet OK), difftest
  language/dquickts/ts incl. stress for the new case.
- Guide: no pages affected.

## Follow-ups

## Result
`try { if (1) {} /x./.test("xy"); } finally {}` and the related shapes parse: the scan
lookahead (compiler/scan.rs) tracks block braces and statement-head parens and allows a
regex after them. Known gap: a `/` after a function/class expression's `}` is still lexed as
a regex start by the scans (the parser itself always knows).
