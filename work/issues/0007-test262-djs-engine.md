# 0007: Run test262 on djs and put the ratchet in the gate

status: done
agent: infra
branch: issue/0007-test262-djs-engine
depends-on: 0002, 0022
touches: crates/djs-test262/, tests/test262/, scripts/gate.sh, crates/djs-core/src/vm/ (only additive hooks needed by $262, coordinated in the Log)

## Goal
Plug djs into the test262 runner as an in-process engine (Script mode, D004), check in
`tests/test262/expected-failures.txt` and make the ratchet part of the gate. First
milestone: "the harness loads" (TESTING.md). Like 0016, the deliverables are the
plumbing, the ratchet and a clustered failure report.

## Acceptance criteria
- [ ] A `djs` engine for djs-test262 behind the existing `Engine` trait. A fresh `Vm` per
      test. The harness files and the test compiled as Scripts (the harness can be
      compiled once per distinct include set and reused through the heap image, if that
      saves time: optional). Negative tests by phase: a parse failure must be a
      `CompileError` of the right type; resolution-phase tests are treated as parse;
      runtime negatives check the thrown error's constructor name. Async tests via
      `print` and the `doneprintHandle.js` protocol (`$DONE`): they will mostly fail
      until promises exist; that is fine.
- [ ] `$262`: `global`, `evalScript(src)` (compile and run a Script in the same realm,
      returning the completion value if the compiler provides it, a 0014 follow-up;
      otherwise undefined, logged), `detachArrayBuffer` (throws for now), `gc`
      (calls collect at the next safepoint, or a no-op), and `print`. Implemented as host
      functions; anything needing new VM hooks goes through the coordinator.
- [ ] A fuel limit and a memory limit per test (failures are `timeout` / `oom`).
- [ ] `tests/test262/expected-failures.txt` for the active group, and the ratchet step in
      `scripts/gate.sh`. Gate time: measure it. If the full active set (~20k tests) is
      too slow for the gate in debug, build the runner in release for the gate step, or
      run a fixed deterministic subset in the gate and the full set in a separate
      `scripts/test262.sh`. Record the decision.
- [ ] A Log entry with pass counts (active, later), per top-level directory, and the
      biggest failure clusters (first error line), like 0016. In particular: does the
      harness load (assert.js, sta.js, propertyHelper.js, compareArray.js)? If not, what
      blocks it?
- [ ] AGENTS.md workflow note: agents whose changes make tests pass run the ratchet
      updates (`djs-difftest --engine djs --update-ratchet`, the test262 `--update`) on
      their branch before merging. A stale ratchet hides regressions.

## Log

2026-11-01 (infra)
- djs engine (`crates/djs-test262/src/djs.rs`, D1070): fresh `Vm` per test; harness and
  test compiled as separate Scripts; `$262` = `global`, `evalScript` (host compile while
  parked plus a JS wrapper; var/let/syntax errors behave as in a real Script),
  `gc` (no-op), `detachArrayBuffer` (TypeError), and `print`. The thrown value's
  constructor name is read from data properties (host never runs JS). Fuel 5e9 per test,
  the runner's memory limit as the heap cap; timeouts say which limit hit, and
  `InternalError: out of memory` is `oom`. No new VM hooks were needed:
  `crates/djs-core/src/vm/` is untouched.
- Async tests: the `$DONE`/`print` protocol works (`$DONE()` is judged correctly, also
  "called twice"), but anything with promises fails (`Promise` is not defined).
- False passes (D1071): the compiler reports unimplemented syntax as a SyntaxError
  ("... not supported yet"), so negative parse tests passed for the wrong reason. The engine
  now reports those as `Unsupported`: active 3,945 → 2,687 passes, later 1,455 → 83.
- Tests: `tests/djs_engine.rs` on a fixture suite that only uses what djs runs
  (`tests/fixture/djs`, 7 pass / 13 fail cases with exact reasons). The qjs fixture suite
  still passes; the qjs engine no longer puts stderr into a timeout's reason.
- `--clusters` in djs-test262 (reasons with `«values»` and numbers normalized).
- Speed: the full active + later run takes 0.5 s in release; active alone 4.2 s in the
  debug gate build. Slowest test 0.84 s (debug). The gate runs all of active (D1072).
- **Pass counts** (test262 @7ab7faf): **active 2,687 / 20,124 (13.3%)**, later 83 /
  17,445 (0.5%). By directory, active: language 2,643 / 10,969 (24%), built-ins
  26 / 9,049 (0.3%), harness 18 / 106. Some finer dirs: language/literals 197/219,
  language/identifiers 208/268, language/expressions 1,090/5,416,
  language/statements 608/4,173, built-ins/Array 15/2,680, built-ins/Object 1/3,197,
  built-ins/String 0/1,040.
- **Does the harness load?**
  - `assert.js`: yes. It compiles and runs, and the success paths (`assert`,
    `sameValue`, `notSameValue`) work. Failing assertions and `assert.throws` also need
    the global error constructors (`TypeError`, `ReferenceError`, `SyntaxError`,
    `RangeError`, `Error`) and errors whose prototype chain has `constructor`.
    `assert.throws` reads `thrown.constructor.name`, which gives "cannot read
    property 'name' of undefined" today (96 tests). Formatting messages needs `String()`.
  - `sta.js`: yes.
  - `compareArray.js`: yes, it loads. Its failure messages need `String()` and
    `Array.prototype.map/join`.
  - `propertyHelper.js`: **no** (2,441 active tests). Blocked first by one `for (var x in
    obj)` (line 153; 0029). After that, at load time it needs `Array.isArray`,
    `Object.defineProperty`, `Object.getOwnPropertyDescriptor`,
    `Object.getOwnPropertyNames`, `Function.prototype.call` and `.bind`,
    `Array.prototype.join` and `.push`, `Object.prototype.hasOwnProperty` and
    `.propertyIsEnumerable`, and `Math.pow`. `verifyProperty` then needs `String()`.
  - Other includes that fail: `testTypedArray.js` (Float64Array; later), `regExpUtils.js`
    (for-of), `nativeFunctionMatcher.js`, `deepEqual.js` and `temporalHelpers.js` (regex
    literals), `nativeErrors.js` (`Error`), `nans.js` (`Number`).
- **Biggest active failure clusters** (tests | reason):
  | 2597 | ReferenceError: 'Object' is not defined |
  | 2537 | parse: Unsupported: classes are not supported yet |
  | 2441 | harness propertyHelper.js: Unsupported: for-in and for-of loops are not supported yet |
  | 1036 | ReferenceError: 'Array' is not defined |
  | 961 | TypeError: not a function |
  | 786 | expected parse SyntaxError, got parse Unsupported: classes are not supported yet |
  | 743 | ReferenceError: 'String' is not defined |
  | 644 | parse: Unsupported: destructuring is not supported yet |
  | 589 | parse: Unsupported: for-in and for-of loops are not supported yet |
  | 583 | parse: Unsupported: destructuring parameters are not supported yet |
  | 492 | ReferenceError: 'Number' is not defined |
  | 475 | ReferenceError: 'TypeError' is not defined |
  | 287 | ReferenceError: 'Promise' is not defined |
  | 264 | ReferenceError: 'Boolean' is not defined |
  | 198 | ReferenceError: 'Math' is not defined |
  | 179 | ReferenceError: 'ReferenceError' is not defined |
  | 174 | parse: Unsupported: destructuring assignment is not supported yet |
  | 167 | ReferenceError: 'Symbol' is not defined |
  | 131 | InternalError: not supported yet: define_method |
  | 121 | parse: Unsupported: async functions are not supported yet |
  | 114 | parse: Unsupported: regular expressions are not supported yet |
  | 110 | expected parse SyntaxError, got parse Unsupported: destructuring parameters are not supported yet |
- What blocks most, in order of reach:
  1. The global objects: `Object` (2,597 tests stop there first), `Array` (1,036),
     `String` (743), `Number` (492), the error constructors (`TypeError` 475,
     `ReferenceError` 179, plus `SyntaxError`, `RangeError`, `URIError`, `Error`),
     `Boolean`, `Math`, `Function`, `JSON`, `isNaN`, `parseInt`/`parseFloat`. As far as
     its description goes, 0028 (call_intrinsic, prelude installation, ToPrimitive,
     Error stack) does not cover these globals. They are the largest unassigned block.
  2. Classes (2,537 + 786 negative-syntax tests), destructuring (644 + 583 + 174 + 110),
     async functions, methods/getters (`define_method` 131, `define_getter` 59), regex
     literals, generators: compiler P1 work.
  3. for-in/of (589 tests, plus all of propertyHelper.js): 0029.
  4. Missing methods on existing objects (`TypeError: not a function`, 961): Array
     methods (concat, copyWithin, ...) beyond the five bgen examples.
  5. ToPrimitive of objects (95, `not supported yet: conversion of an object to a
     primitive`) and Error objects with constructors: 0028.

## Follow-ups
- Coordinator: assign the global objects and constructors (Object first, with the
  propertyHelper.js load-time functions; Array, String, Number, the Error family). They
  are the largest unassigned blocker.
- `$262.evalScript`: the top-level `this` of the evaluated Script is undefined (the
  wrapper calls it plainly); use `Function.prototype.call` once it exists, or a VM hook to
  call a function with an explicit `this` from a host call.
- `$262.gc`: collect for real (needs the VM to allow a collection at a host call), and
  `$262.detachArrayBuffer` once ArrayBuffer exists.
- Async tests: need promises and the job queue (the `$DONE` protocol already works).
- Gate time: switch the test262 step to a release build above about 30 s (D1072).
- `tests/test262/expected-failures.txt` is 2 MB (17,437 lines), and `--update` churns it
  as reasons change. That is acceptable for now; reasons could be dropped if diffs get
  noisy.

## Result
- `djs-test262 run --engine djs` (in-process; `$262`, `print`, fuel and memory limits,
  `Unsupported` parse errors) and `--clusters`.
- `tests/test262/expected-failures.txt` for the active group (17,437 entries); 2,687 of
  20,124 active tests pass.
- The gate runs the active group against it (4.2 s).
- AGENTS.md: update both ratchets when a change makes tests pass.
- Decisions D1070 to D1072; usage lines in TESTING.md.
