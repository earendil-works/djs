# djs: Testing

There are three layers of tests:

1. **test262**, which checks conformance (pass/fail per test). It is the backbone of the
   hill-climb.
2. **Differential tests against dquickts**, which compare behavior that test262 does not
   check: `console.log` formatting, error messages, stack traces, TypeScript, and codemode
   scripts.
3. **Stress modes**, which re-run layers 1 and 2 under conditions that must not change the
   outcome: suspend/snapshot/restore every N instructions, and the bytecode versus native
   builtin backend.

Plus ordinary Rust unit tests for internals (lexer, number conversion, heap, …).

## Oracle

`~/Development/dquickts` (read-only). `qjs` is the reference for differential tests.
`run-test262` is dquickts' own test262 runner. dquickts fails only about 41 tests at
the pinned test262 commit, mostly in `intl402` and `staging/sm`. So against the same
commit and filters, "dquickts passes it" is nearly "everything not skipped".

## test262

### Checkout

- Pinned to the commit dquickts uses: `7ab7fafa0003f73fc85c1b95d88094d33f7eb8bd`.
- `scripts/fetch-test262.sh` clones it into `vendor/test262` **of the primary checkout**
  (gitignored). It is idempotent and verifies the commit. Worktrees share that one copy:
  tools locate it from `git rev-parse --git-common-dir` (its parent directory is the
  primary checkout) or from `$DJS_TEST262`.
- We do not apply dquickts' `tests/test262.patch` unless a decision says otherwise (it
  only tweaks the Atomics timeouts and `regExpUtils.js`).

### Groups: skip / later / active

`tests/test262.toml` puts every test into exactly one group:

- **skip:** we will never implement this. It is excluded from the denominator, so the
  score stays honest.
- **later:** on the roadmap but outside the current milestone. Failures count, but they
  are reported separately so the headline number tracks the current phase.
- **active:** everything else.

Tests can be matched by `flags`, by directory prefix, by `features` (globs allowed), and by
a **source scan** for constructs that have no feature tag (direct `eval(`, the
`Function(` constructor). The scan is a cheap regex and is allowed to be conservative.

Initial skip set (from ARCH non-goals):

- flags: `noStrict` (djs is always strict), `module` (modules are not planned for now).
- dirs: `intl402`, `annexB`, `staging`, `built-ins/Temporal`,
  `built-ins/SharedArrayBuffer`, `built-ins/Atomics`, `built-ins/ShadowRealm`,
  `language/module-code`, `language/import`.
- features: `Intl-*`, `Intl.*`, `Temporal`, `SharedArrayBuffer`, `Atomics*`, `ShadowRealm`,
  `tail-call-optimization`, `IsHTMLDDA`, `cross-realm`, `caller`, `legacy-regexp`,
  `dynamic-import`, `import-*`, `json-modules`, `source-phase-imports*`, `top-level-await`,
  `host-gc-required`, `Atomics.waitAsync`, `import-defer`.
- scan: direct-eval and Function-constructor tests go into their own group (`scan-eval`),
  which is skipped for now. Its size tells us whether indirect eval and `Function` are
  worth adding.

The initial `later` set covers everything in phases P3 to P5 (BigInt, Proxy, Reflect, typed
arrays, ArrayBuffer/DataView, generators, regexp features, WeakRef/FinalizationRegistry,
decorators, explicit-resource-management, Symbol-heavy features, …). Moving a feature from
`later` to `active` is a coordinator decision, made when its phase starts.

### Execution model

- test262 tests are **Scripts**. djs has a Script compile mode next to the codemode
  "async function body" mode: top-level `var` and function declarations create global
  object properties, top-level `let`/`const`/`class` go into the global lexical scope, and
  top-level `this` is the global object.
- Strict mode always. Tests without flags run once, strict (as if `"use strict";` were
  prepended). `onlyStrict` tests run as they are. `raw` tests run as they are, with no
  harness.
- Harness: `assert.js` and `sta.js`, plus the test's `includes:`, put in front of the
  test (`doneprintHandle.js` for `async` tests).
- `$262` host object: `global`, `evalScript(src)` (compiles and runs a Script in the same
  realm), `detachArrayBuffer`, `gc`. No `createRealm` and no `agent`. `print` for async
  tests.
- Negative tests: `negative: {phase: parse|resolution|runtime, type: X}`. A parse-phase
  negative must fail at compile time with the right error type, without running
  anything.
- Async tests pass when `Test262:AsyncTestComplete` is printed and fail on
  `Test262:AsyncTestFailure`.
- Every test runs with a fuel limit and a memory limit. Hitting either one is a failure
  (`timeout` / `oom`), never a hang.

### Runner

`djs-test262` (a std crate). It has an engine abstraction so the same filters and harness
handling can run:

- `--engine qjs`: dquickts' `qjs` as a subprocess. This validates the runner and gives
  the oracle baseline before djs exists.
- `--engine djs`: djs in-process. Its own `$262` implementation is provided by the
  runner.

Outputs:

- A summary table: pass/fail/total for active and later, per top-level directory and per
  feature.
- `tests/test262/expected-failures.txt`: the ratchet, checked in. One line per active
  test that is known to fail (`path: short reason`). `--update` rewrites it. The gate
  fails if an active test fails that is not in the file (a regression), and it reports
  tests that now pass but are still listed (a stale entry; run `--update`).
- `tests/test262/oracle-failures.txt`: tests that dquickts fails under our runner. These
  are reviewed by hand and are not automatic targets.
- `--suspend-every N` and `--backend bytecode|native` for the stress modes, once those
  exist.

The runner is parallel (one test per worker) and fast enough to run the full active set
in the gate.

### Milestones

1. **The harness loads.** `assert.js`, `sta.js`, `compareArray.js` and
   `propertyHelper.js` run. `test/harness/*` is the first directory to go green.
2. `language/` (expressions, statements, the generated `dstr` destructuring cases), and
   the negative syntax tests (early errors).
3. `built-ins/` in the order codemode needs them: Object, Function, Array, String, JSON,
   Promise, Math, Number, Boolean, Error, Map/Set, Symbol, iterators.

## Differential tests (dquickts oracle)

`tests/corpus/**/*.{js,ts}`: each case is run by `qjs` and by djs in the codemode
execution model (async function body, strict). The expected output is checked in next to
the case (`case.js.expected`) and regenerated from qjs with a script. We compare stdout,
the uncaught error (type + message) and the return value.

Sources, in order of importance:

1. Harvested pi codemode scripts with recorded tool results, replayed through a fake
   `tools` object.
2. dquickts' `tests/test_language.js`, `test_builtin.js`, `test_closure.js`,
   `test_loop.js` and `test_typescript.ts`, split into one case per test function.
3. Hand-written cases for formatting and error messages.

Error message texts: we match QuickJS messages verbatim where that is cheap, and
otherwise normalize them in the comparison (see `docs/decisions.md`).

## TypeScript

- Runtime TS behavior (enums, namespaces, parameter properties) is covered by
  differential cases.
- **The type skipper** has a parse-only oracle. djs can dump its erased source with type
  ranges replaced by whitespace (`djs strip-types`), which must match `ts-blank-space` byte
  for byte on a large TS corpus (for example TypeScript's own sources). This tests erasure
  without any TS runtime test suite.

## The gate

`scripts/gate.sh` must pass before anything merges into `main`: fmt, clippy, unit tests,
the no_std wasm build of djs-core, and, once they exist, the test262 ratchet and the
differential corpus. Issues that add a test suite also add it to the gate.
