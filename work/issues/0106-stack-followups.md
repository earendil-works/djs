# 0106: Stack trace follow-ups: Error.prepareStackTrace, eval/Function SyntaxError frames

status: open
agent: -
branch: issue/0106-stack-followups
depends-on: 0105
touches: builtins/error.ts + crates/djs-core/src/builtins/, crates/djs-core/src/interp/ (stack capture), tests/corpus/

## Goal
0105's follow-ups, as qjs does them: `Error.prepareStackTrace` (with callsite objects, as
far as qjs implements it), the `<input>` frame for SyntaxErrors from indirect eval and
`new Function` (djs shows `eval (native)` / `Function (native)`), and the order of Error's
own properties.

## Acceptance criteria
- [ ] All three, matching qjs, with `// difftest: stacks` cases on both backends and in the
      stress modes.

## Log

## Follow-ups

## Result
