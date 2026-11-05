# 0217: Parser: a regex literal after a block statement inside `try`

status: open
agent: -
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

## Follow-ups

## Result
