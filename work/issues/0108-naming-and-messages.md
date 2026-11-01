# 0108: Function-name inference bug, syntax messages, static property order

status: in-progress
agent: infra
branch: issue/0108-naming-and-messages
depends-on: 0106
guide: -
touches: crates/djs-core/src/compiler/ (name inference, error messages), crates/djs-bgen/ + builtins/ (static property order), tests/corpus/

## Goal
0106's follow-ups:
1. **Bug:** `const q = (() => …)()` names the arrow `q`, and `(function(){}).name` in an
   initializer gives the binding name. The anonymous-function naming (D... in 0036/0024)
   sees through parentheses even when a call or member access follows. Per spec, only a
   parenthesized function expression directly as the initializer gets the name.
2. qjs's newer message at end of input (`unexpected token in expression: ''`) vs djs's
   'end of input', and the Function constructor's message when the body closes the
   function early.
3. Other constructors' static property order vs qjs.

## Acceptance criteria
- [ ] 1 with tests covering every naming position (const/let/var, assignment, default
      params, object literal values, class fields, destructuring defaults) with and
      without parentheses and with calls/members following. Check test262's
      fn-name-cover tests still pass.
- [ ] 2 and 3 matching qjs, with difftest cases.

## Log

## Follow-ups

## Result
