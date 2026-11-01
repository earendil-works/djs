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

### How to run

```sh
scripts/fetch-test262.sh                 # once: vendor/test262 at the pinned commit
cargo build --release -p djs-test262
T=target/release/djs-test262

$T list                                  # counts per group and top-level dir
$T list --depth 2 --features             # finer dirs, plus the per-feature table
$T list --paths --group scan-eval        # each test with the rule that grouped it

$T run                                   # active + later with qjs (the default engine)
$T run built-ins/Array --failures        # a path prefix, with one-line failure reasons
$T run --engine djs --clusters           # djs in-process; failures grouped by reason
$T run --engine djs --group active --expected tests/test262/expected-failures.txt  # the gate
$T run 'language/**/S11*.js' -j 4        # globs: * within a segment, ** across
$T run --group skip language/statements/with  # run a skipped group explicitly

# the ratchet: exit 1 on failures not in the file; --update rewrites the entries of
# the tests that ran (entries outside the selection are kept)
$T run --expected tests/test262/oracle-failures.txt
$T run --expected tests/test262/oracle-failures.txt --update
```

The summary is printed as Markdown tables. `--timeout SECS` (default 20) and
`--memory-limit N` (default 512M) bound each test; `--slowest N` lists the slowest
tests. `--qjs PATH` / `$DJS_QJS` select the qjs binary and `--test262 DIR` /
`$DJS_TEST262` the checkout. With qjs, the full active + later run (about 37,600
tests) takes about 10 s on an 18-core machine.

`cargo test -p djs-test262` includes a fixture suite run with qjs that checks failures
are detected (skipped if qjs is missing). `cargo test -p djs-test262 --test
frontmatter_all -- --ignored` parses the frontmatter of every test in the checkout.

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
the case (`case.expected`) and regenerated from qjs with a script. We compare the
output, the uncaught error (type + message) and the return value.

Sources, in order of importance:

1. Harvested pi codemode scripts with recorded tool results, replayed through a fake
   `tools` object.
2. dquickts' `tests/test_language.js`, `test_builtin.js`, `test_closure.js`,
   `test_loop.js` and `test_typescript.ts`, split into one case per test function.
3. Hand-written cases for formatting and error messages.

Error message texts: we match QuickJS messages verbatim where that is cheap, and
otherwise normalize them in the comparison (see `docs/decisions.md`).

### The codemode model (D1100)

A case is what the model sees from a pi codemode script, so every engine runs the same
JS prelude (`tests/corpus/prelude.js`) first. It defines `console.*`, `text()`, `exit()`,
`store`/`load` and fake `tools` (`echo`, `fail`) with pi's formatting. The case source is
the body of a strict `async (tools, console) => {...}` evaluated in a global Script, as
in pi (D008): top-level `await` and `return` work, `this` is the global object, there
is no `arguments`. The
transcript in `<name>.expected` is:

- one entry per output item in order: `console.*` arguments joined by spaces, strings
  as they are, errors as `Name: message` plus stack, everything else
  `JSON.stringify` (falling back to `String()`); `text()` gives primitives as `String()`
  and objects as JSON;
- a status line, then its text: `=== return` and the value (a string as it is,
  anything else compact JSON); `=== completed` if there is no value to show (undefined,
  functions, `exit()`); or `=== throw` and the error (`Name: message` for errors,
  `Error: <console form>` for other thrown values). A value JSON cannot serialize fails
  the script with that TypeError, and waiting on a promise nothing can settle fails
  with pi's "can never settle" error.

Stack frame lines (`    at ...`) are dropped on both sides before comparing; stack
formats are a separate topic. Leading comment lines can carry directives:
`// difftest: skip-djs <reason>` (a djs non-goal, such as eval), `// difftest: ts` (a
`.js` file parsed as TypeScript), `// difftest: fuel <n>`, `// difftest: timeout <secs>`,
`// difftest: djs-expect <oracle line> => <djs line>` (a deliberate difference from qjs, such
as D013's RangeError for stack overflow: with djs, that `.expected` line must read the
second text; D1540).

### How to run difftest

```sh
cargo build --release -p djs-difftest
D=target/release/djs-difftest

$D                                   # every case with qjs (the default engine)
$D console errors/type-errors.js     # path prefixes relative to tests/corpus
$D 'dquickts/*/test_try*'            # globs
$D --no-diff                         # failures as one line each instead of diffs
$D --update [filter...]              # record .expected from qjs (never from djs)
$D --ratchet                         # exit 1 on failures not in expected-failures.txt
$D --update-ratchet                  # rewrite tests/corpus/expected-failures.txt

scripts/difftest-import-dquickts.py  # re-split the dquickts tests (then --update dquickts)

$D --engine djs --clusters           # djs in-process; failures grouped by error
$D --engine djs --ratchet --suspend-every 97 --gc-stress   # what the gate runs
$D --engine djs --update-ratchet     # after fixing cases: rewrite expected-failures.txt
```

With `--engine djs` the case runs exactly as with qjs: djs evaluates the shared prelude
and the case Script in-process, and only `host.output`/`host.done` are host functions
(D1440). `--suspend-every N` and `--gc-stress` run
every case a second (third) time; a transcript that differs from the plain djs run is a
hard failure, separate from the oracle comparison and the ratchet (D1161).

`--update` refuses to record timeouts and crashes. Every case must pass with qjs: it is
the oracle, so a failure there means the case or the runner is wrong. The importer
writes `tests/corpus/dquickts/<file>/<test>.{js,ts}`, leaves out tests that cannot work
in the codemode model (listed with reasons in the script), turns each one-line
`assertSyntaxError` of `test_typescript.ts` into its own parse-error case, and marks
cases that use eval or Function with `skip-djs`. The full corpus runs in well under a
second with qjs. `cargo test -p djs-difftest` checks the runner itself on a scratch
corpus.

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
