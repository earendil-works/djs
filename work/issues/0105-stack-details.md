# 0105: Stack trace details (Error.stackTraceLimit, call-site columns)

status: done
agent: infra
branch: issue/0105-stack-details
depends-on: 0102
touches: builtins/error.ts + crates/djs-core/src/interp/ (stack trace capture), crates/djs-core/src/compiler/ (line-table positions for `return f()` only), tests/corpus/

## Goal
0102's follow-ups: `Error.stackTraceLimit` (qjs and V8 have it; djs caps at 10 frames),
and call-site columns in `return f()` stacks: qjs points at the start of the `return`
statement (its tail call), djs at the call. Goal: match qjs. Small, user-visible polish
for error messages that models read. (Corrected by the coordinator: the first version
had the direction the wrong way round.)

## Acceptance criteria
- [x] Error.stackTraceLimit per qjs (getter/setter semantics, non-number values, 0,
      Infinity), applied to captureStackTrace and error construction.
- [x] `return f()` and similar tail positions record the `return` statement's position,
      matching qjs, and a difftest-like check that compares stack lines (the normal
      difftest drops them).
- [x] Ratchets and both backends unchanged.

## Log
- Probed qjs with ~60 call shapes (work/scratch notes in the corpus case). qjs's rule,
  from quickjs.c (resolve_labels: `OP_call`/`OP_call_method` followed by `OP_return`
  becomes a tail call that takes the return's `emit_source_pos`): a tail call reports
  the `return` keyword; every other call its `(`. Not tail calls: async/generator
  functions, derived constructors, inside try or catch, for-of, finally, `using`, spread,
  optional calls, `eval(...)`, concise arrow bodies. Base constructors, getters and
  for-in/switch/labels are tail calls. Separately, `new X(...)`/`new X` report the token
  after `X` (djs reported `new`). Implemented as line-table marks only (D2031); the
  compiler snapshots changed in exactly those positions.
- Error.stackTraceLimit per qjs (D2030): plain data property read without running code
  from the realm's original Error; type and range handling as `js_get_stack_trace_limit`.
- `difftest: stacks` (D2032) keeps normalized frame lines; the qjs driver loads the
  prelude with std.loadScript so harness frames are recognisable. Corpus: qjs passes all
  233, djs 227 with the 1 ratchet entry. New cases errors/stack-positions.js (55 shapes)
  and errors/stack-trace-limit.js; both stress-identical on both backends, and
  stack-positions fails on the old compiler.
- REPL tests: three expected positions moved to qjs's (`new TypeError(...)` etc.).
- Checked: djs-core lib, e2e plain, djs-cli, djs-difftest unit tests, wasm32 build,
  test262 Error/NativeErrors/new/return/call/tagged-template/try on both backends, full
  active test262 on the bytecode backend (ratchet OK, 40 expected).

## Follow-ups
- `Error.prepareStackTrace` (qjs has it, as a plain data property read without code).
- Error's own property order: qjs `length,name,isError,captureStackTrace,stackTraceLimit,
  prototype`, djs `length,name,prototype,captureStackTrace,stackTraceLimit,isError`.
- eval SyntaxErrors: qjs's stack starts with an `at <input>:line:col` frame, djs shows
  `at eval (native)` (djs's eval is a builtin); the same for the Function constructor.

## Result
`Error.stackTraceLimit` as in qjs (D2030). Call positions match qjs: tail calls report
the `return`, `new` the token after the constructor (D2031). Difftest cases marked
`// difftest: stacks` compare stack frames (D2032, TESTING.md); two new such cases.
