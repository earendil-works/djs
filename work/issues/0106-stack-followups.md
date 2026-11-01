# 0106: Stack trace follow-ups: Error.prepareStackTrace, eval/Function SyntaxError frames

status: done
agent: infra
branch: issue/0106-stack-followups
depends-on: 0105
touches: builtins/error.ts + crates/djs-core/src/builtins/, crates/djs-core/src/interp/ (stack capture), tests/corpus/

## Goal
0105's follow-ups, as qjs does them: `Error.prepareStackTrace` (with callsite objects, as
far as qjs implements it), the `<input>` frame for SyntaxErrors from indirect eval and
`new Function` (djs shows `eval (native)` / `Function (native)`), and the order of Error's
own properties.

## Acceptance criteria
- [x] All three, matching qjs, with `// difftest: stacks` cases on both backends and in the
      stress modes.

## Log
- Read qjs's implementation in quickjs.c: build_stack_trace, js_get_prepare_stack_trace,
  get_backtrace and js_new_callsite, the CallSite prototype functions, and
  js_function_constructor. Then probed qjs with about 40 cases.
- Error.prepareStackTrace (D2050):
  - CallSite objects are the new heap class `ObjectClass::CallSite = 34`, cleared with
    the coordinator. It is an additive edit to heap/object.rs (enum, from_u32, slots) and
    goes with the layout regen. Note for heap: no GC or walk changes, since the slots are
    traced generically.
  - The new realm intrinsic is %CallSitePrototype%.
  - Stack capture moved to the new interp/run/stack.rs. One frame walker builds both the
    default string and the callsites.
  - Pending stacks are finished in the prelude. The VM's own errors are finished at
    throw: `Exec::throw_fault` pushes `%ThrowWithStack` as if called from the faulting
    instruction (Retry continuation at the faulting pc). It is used by step_once and
    native_step.
- `<input>` frame (D2051): eval.rs returns the SyntaxError position, and intrinsics.rs
  records it as the first frame. JSON.parse uses the same path now.
- Constructor property order (D2052):
  - bgen emits `%InstallConstructor`, then the constructor's statics, then
    `%InstallPrototype`, in one straight pass as the coordinator asked.
  - Error's statics were reordered in error.ts.
  - The bgen goldens, checker test and behavior shims were updated.
  - The test262 directories with property-order tests (Object, Reflect, Function, Error,
    NativeErrors, TypedArrayConstructors, Proxy) have no new failures. The difftest
    corpus ratchet is unchanged.
- New corpus cases: errors/prepare-stack-trace.js and
  errors/dynamic-syntax-error-stack.js (both `difftest: stacks`), and
  builtins/constructor-property-order.js. Their output is identical on both backends and
  under --suspend-every 97 and --gc-stress.
- Guide: no prose pages affected; ref-intrinsics.md was regenerated (new leaves).

## Follow-ups
- Name inference sees through parentheses that are followed by a call or member access.
  `const q = (() => ...)()` names the arrow `q` (in stacks and `.name`), and
  `(function(){}).name` in an initializer gives the binding's name instead of "".
  qjs gives `<anonymous>` / "". This is a compiler bug (expr.rs name_hint through
  parentheses).
- qjs's message for an unexpected end of input is now `unexpected token in expression:
  ''`; djs says `'end of input'` (compiler/parser.rs tok_text; the compiler tests cite
  the old qjs).
- Function constructor messages: for a body that closes the function early
  (`Function("x", "}")`), qjs parses the whole wrapped source and says `expecting ')'`;
  djs says `unexpected token after the function body`.
- Static property order of other constructors (Array, Number, Object, Promise, Symbol,
  ...) still differs from qjs. Only `prototype` is now last everywhere.
- Inside a direct-looking `eval(...)` djs shows an `at eval (native)` frame where qjs
  (direct eval) shows none (D014: djs's eval is always indirect).

## Result
- `Error.prepareStackTrace` works as in qjs, with CallSite objects (D2050).
- SyntaxErrors from indirect eval, the Function constructors and JSON.parse start their
  stack at `<input>:line:col` (D2051).
- Every constructor's `prototype` is installed last, and Error's statics are in qjs's
  order (D2052).
- Covered by three new difftest cases, two of them comparing stack lines.
