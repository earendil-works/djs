# 0108: Function-name inference bug, syntax messages, static property order

status: done
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
- [x] 1 with tests covering every naming position (const/let/var, assignment, default
      params, object literal values, class fields, destructuring defaults) with and
      without parentheses and with calls/members following. Check test262's
      fn-name-cover tests still pass.
- [x] 2 and 3 matching qjs, with difftest cases.

## Log
- Name inference (D2070):
  - The bug was wider than parentheses. Any function at the initializer's start took the
    name, also `function () {}.bind(x)`, `class {}.name` and `(f) || x`.
  - Fix: `Parser::parse_named` wraps every hint site and takes the name back unless the
    function spans the whole initializer (`last_anon_fn`).
  - qjs names through TypeScript assertions (`as`, `satisfies`, `!`, `<T>`). djs did not
    for `<T>x` even before; all four work now.
  - Tests: corpus language/function-name-inference.js covers const/let/var, assignment,
    logical assignment, default parameters, object and array destructuring, object
    literal values and class fields (public, static, private), each with and without
    parentheses and with members, calls and operators following, plus IIFE stack
    frames. ts/function-name-assertions.ts covers the TypeScript assertions.
  - test262 language/ (all fn-name-* tests included) shows no new failures on both
    backends.
- Messages (D2071):
  - End of input reads `''` now.
  - Statement lists at the end of input give qjs's message; a class body gives `invalid
    property name`.
  - The Function constructors' messages for parameters or a body that close early now
    follow what qjs's single parse of the wrapped source gives. djs still rejects the
    injection cases that qjs accepts (spec).
  - Updated: compiler tests, vm::tests::dynamic_code and the REPL test.
  - New corpus case errors/syntax-messages-eof.js (stacks).
- Statics order (D2072):
  - A qjs order table in djs-bgen, applied in place to the installs onto Array, BigInt,
    Iterator, Number, Object, Promise, Symbol, Uint8Array, Math and JSON.
  - Every constructor and namespace now matches qjs, except Set.groupBy (a qjs
    extension djs does not have), SharedArrayBuffer and Atomics (non-goals).
  - builtins/constructor-property-order.js extended.
- Checked:
  - djs-core lib, e2e, djs-cli and djs-bgen tests;
  - the whole corpus (ratchet OK);
  - the new cases under the stress modes on both backends;
  - the full active test262 run on the bytecode backend;
  - djs-docs check. Guide: no pages affected.

## Follow-ups
- Prototype objects' property order differs from qjs: qjs puts `constructor` last and
  orders methods by its own lists. The same table approach would apply.
- `let` alone as an indirect-eval program: qjs evaluates it as a sloppy identifier
  (ReferenceError); djs's eval code is strict, so it is a SyntaxError. That is a
  strictness difference, not a message one.

## Result
- Anonymous functions are named only when they are the whole initializer (D2070).
- qjs's syntax messages at the end of input and for Function constructor bodies or
  parameters that close early (D2071).
- Constructor statics and Math/JSON members are in qjs's order (D2072).
- Four new corpus cases and one extended.
