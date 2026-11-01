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
- direct eval (`direct-eval`, skipped, D014): `language/eval-code/direct` and every test
  whose source has an `eval(` call. djs treats `eval(x)` as indirect eval (0059). Indirect
  eval (`(0, eval)(`) and the Function constructors are active.

The initial `later` set covered everything in phases P3 to P5 (BigInt, Proxy, Reflect, typed
arrays, ArrayBuffer/DataView, generators, regexp features, WeakRef/FinalizationRegistry,
decorators, explicit-resource-management, Symbol-heavy features, …). Those phases are done
and their features are active. 0097 promoted the last 179 (immutable ArrayBuffers,
the error-stack accessor, Promise.allKeyed, Symbol.unscopables), so the later group is
empty: 38,109 active, 525 direct-eval and 14,963 skipped, of 53,597 tests
(`djs-test262 list`). Moving tests from `later` to
`active` is a coordinator decision; `tests/test262.toml` says why each rule exists.

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
  (`timeout` / `oom`), never a hang. For djs the fuel budget decides (`--fuel`), so a
  loaded machine cannot turn a pass into a timeout; the wall-clock `--timeout` is only a
  hang guard (D1790).

### Runner

`djs-test262` (a std crate). It has an engine abstraction so the same filters and harness
handling can run:

- `--engine qjs`: dquickts' `qjs` as a subprocess. This validates the runner and gives
  the oracle baseline before djs exists. Its results are cached in `vendor/qjs-cache/`
  next to the test262 checkout, keyed by the qjs binary, the test262 commit and the
  driver, so a repeated qjs run takes seconds; `--no-cache` runs everything (D1531).
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
- `--backend bytecode|native` (djs, 0063): run the `@native` builtins as their
  generated Rust state machines (the default) or as bytecode. Both backends must give
  the same results; see "Builtin backends" below.

The runner is parallel (one test per worker) and fast enough to run the full active set
in the gate.

### How to run

```sh
scripts/fetch-test262.sh                 # once: vendor/test262 at the pinned commit
cargo build --release -p djs-test262
T=target/release/djs-test262

$T list                                  # counts per group and top-level dir
$T list --depth 2 --features             # finer dirs, plus the per-feature table
$T list --paths --group direct-eval      # each test with the rule that grouped it

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

The summary is printed as Markdown tables. `--fuel N` (djs), `--timeout SECS` (20 for qjs;
for djs a hang guard of 120, D1790) and `--memory-limit N` (default 512M) bound each test;
`--slowest N` lists the slowest tests. `--qjs PATH` / `$DJS_QJS` select the qjs binary
and `--test262 DIR` / `$DJS_TEST262` the checkout. qjs results are cached (D1531), so a
repeated qjs run of active + later (about 38,100 tests) takes seconds; djs runs the active
group in about a minute of CPU (release, `-j` workers).

Status at 0097: djs passes 38,069 of the 38,109 active tests (99.9%), with both builtin
backends; the 40 failures are listed with reasons in `tests/test262/expected-failures.txt`.
qjs fails 4 active tests under our runner (`tests/test262/oracle-failures.txt`).

`cargo test -p djs-test262` includes a fixture suite run with qjs that checks failures
are detected (skipped if qjs is missing). `cargo test -p djs-test262 --test
frontmatter_all -- --ignored` parses the frontmatter of every test in the checkout.

### Milestones (reached)

The original order was: the harness, then `language/` and the negative syntax tests,
then `built-ins/` in the order codemode needs them. All three are done; what is left is
the ratchet's list.

## Differential tests (dquickts oracle)

`tests/corpus/**/*.{js,ts}`: each case is run by `qjs` and by djs in the codemode
execution model (async function body, strict). The expected output is checked in next to
the case (`case.expected`) and regenerated from qjs with a script. We compare the
output, the uncaught error (type + message) and the return value.

Sources (about 230 cases at 0095, in `builtins/`, `codemode/`, `console/`, `dquickts/`,
`errors/`, `language/`, `result/` and `ts/`):

1. Codemode-style scripts with fake tools (`codemode/`, `result/`, `console/`), and 300
   scripts models wrote in real pi sessions (`codemode-harvest/`, 0118, 0121, D2280,
   D2281, D2283, D2284), redacted: the harvester (`scripts/harvest/`, its README has the
   steps and the privacy rules) drops comments, replaces the text of tool arguments,
   shell commands and long literals, renames words that are not common English or
   programming terms, and gives each case synthetic tool results (the sessions do not
   record them): text for the string tools, values built from the session's declared
   result types for MCP tools, fakes for `models`, `image` and `load`.
   `codemode-harvest/REDACTION.md` lists what was redacted per case. Every case was
   reviewed by hand. pi's own codemode tests run on djs too (docs/host.md "pi's
   own tests on djs").
2. dquickts' `tests/test_language.js`, `test_builtin.js`, `test_closure.js`,
   `test_loop.js` and `test_typescript.ts`, split into one case per test function
   (D1102).
3. Hand-written cases for builtins, formatting, error messages and the engine's fast
   paths (each fast path's corpus case covers the cases where it must decline).

Error message texts: djs matches QuickJS's messages verbatim. A deliberate difference is
declared per case with `djs-expect` (below, D1540). Status at 0097: 224 of 230 pass, 1
known failure (`tests/corpus/expected-failures.txt`), 5 skipped as djs non-goals.

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

Stack frame lines (`    at ...`) are dropped on both sides before comparing, except in
cases marked `// difftest: stacks` (D2032): there a frame in the case reads
`name (case:line:col)` for both engines, `(native)` frames stay and the harness's own
frames (prelude, qjs driver) are dropped, so names and positions are compared
(tests/corpus/errors/stack-positions.js, stack-trace-limit.js). Line 1 of a case is a
directive comment, so the wrappers' different first-line prefixes do not show. Leading
comment lines can carry directives:
`// difftest: skip-djs <reason>` (a djs non-goal, such as direct eval), `// difftest: ts` (a
`.js` file parsed as TypeScript), `// difftest: experimental-decorators` and
`// difftest: emit-decorator-metadata` (TypeScript legacy decorators, D1651),
`// difftest: fuel <n>`, `// difftest: timeout <secs>`,
`// difftest: djs-expect <oracle line> => <djs line>` (a deliberate difference from qjs, such
as D013's RangeError for stack overflow: with djs, that `.expected` line must read the
second text; D1540), `// difftest: no-stress <reason>` (the stress variants skip a
case that cannot finish under them, D1530).

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
                                     # (reasons: an excerpt around the first difference, D2282)

scripts/difftest-import-dquickts.py  # re-split the dquickts tests (then --update dquickts)

$D --engine djs --clusters           # djs in-process; failures grouped by error
$D --engine djs --ratchet --suspend-every 97 --gc-stress --determinism   # what the gate runs
                                     # (--suspend-every 97 also runs 1009)
$D --engine djs --backend bytecode --ratchet               # the gate, too (0063)
$D --engine djs --update-ratchet     # after fixing cases: rewrite expected-failures.txt
```

With `--engine djs` the case runs exactly as with qjs: djs evaluates the shared prelude
and the case Script in-process, and only `host.output`/`host.done` are host functions
(D1440). `--suspend-every N` and `--gc-stress` run
every case a second (third) time; a transcript that differs from the plain djs run is a
hard failure, separate from the oracle comparison and the ratchet (D1161). Stressed runs
get three times the case's time limit, and a stressed run that still times out is a
failure too: mark such a case `no-stress` with the measured reason (D1530).
`--determinism` adds a second suspension slice (1009) and requires the suspended runs to
leave the same heap image as the plain run, both the raw image and its canonical hash
(the graph alone, computed by a copying collection of a copy; D1720, D1725, D2040). The e2e
test compares its plain and suspension modes the same way, and
`scripts/determinism/native-vs-wasm.sh` compares native and wasm builds image for image
and stop for stop (both in the gate).

`--update` refuses to record timeouts and crashes. Every case must pass with qjs: it is
the oracle, so a failure there means the case or the runner is wrong. The importer
writes `tests/corpus/dquickts/<file>/<test>.{js,ts}`, leaves out tests that cannot work
in the codemode model (listed with reasons in the script), turns each one-line
`assertSyntaxError` of `test_typescript.ts` into its own parse-error case, and marks
the cases that need direct eval (a hand-kept list in the script) with `skip-djs`. The full corpus runs in well under a
second with qjs. `cargo test -p djs-difftest` checks the runner itself on a scratch
corpus.

## Builtin backends (0063)

Builtins in the builtin language compile to bytecode (the prelude) and, if tagged
`@native`, also to Rust state machines (`crates/djs-core/src/builtins/gen/natives.rs`,
ARCH "Generated state machines"). `ExecOptions::native_builtins` picks the backend per
call (default on); `--backend bytecode|native` selects it in `djs run`, djs-difftest
and djs-test262, and the engine track runs both (`engine/djs` and
`engine/djs-bytecode`, with their ratio `engine/bytecode÷native/*`). The two must
behave the same in every mode; only the fuel spent differs (D1633).

- The gate runs the corpus with the native backend in all stress modes (suspension
  through heap images, GC stress) and once more, plainly, with the bytecode backend.
- The test262 active group with the bytecode backend is a periodic check, not in the
  gate: `djs-test262 run --engine djs --backend bytecode --group active --expected
  tests/test262/expected-failures.txt` (run it when changing a `@native` builtin, bgen's
  Rust backend or `interp/run/native.rs`). At 0063 both backends passed 37,504/37,722;
  at 0091 the bytecode backend passes the active group with the same ratchet (40).
- `@fast` entries (D1850, D1910) run only with the native backend, so the gate's bytecode
  run keeps testing the TS paths they bypass.
- Unit tests: `interp::run::native::tests` (both backends in plain, suspension and GC
  stress modes, a backend switch between runs, fuel; `backends_agree` samples the modes
  in about a second, the exhaustive `backends_agree_in_every_mode` is `#[ignore]`d:
  run it with `cargo test -p djs-core --lib backends_agree -- --ignored` before merging
  backend changes), bgen's golden fixture `crates/djs-bgen/tests/golden/native.ts` →
  `native.rs.txt`, and `scripts/native-sizes.sh` for the wasm code size per native
  function.

## TypeScript

- Runtime TS behavior (enums, namespaces, parameter properties) is covered by
  differential cases.
- **The type skipper** has a parse-only oracle. djs can dump its erased source with type
  ranges replaced by whitespace (`djs strip-types`), which must match `ts-blank-space` byte
  for byte on a large TS corpus (for example TypeScript's own sources). This tests erasure
  without any TS runtime test suite.

### How to run the strip-types oracle

```sh
djs strip-types file.ts               # the source with its types blanked
scripts/strip-types/oracle.sh         # the full comparison (fetches on first use)
scripts/strip-types/update-cases.sh   # regenerate tests/strip-types/*.js
```

`scripts/strip-types/fetch.sh` installs ts-blank-space 0.9.0 with TypeScript 5.9.3 (npm)
and extracts TypeScript 5.9.3's `src/` and `tests/cases/conformance/` into
`vendor/strip-types` of the primary checkout. The oracle runs ts-blank-space once over
every `.ts` file (not `.d.ts`), skipping files TypeScript reports syntax errors for and
files ts-blank-space cannot erase (enums, namespaces with values, parameter properties,
`<T>x`), then `djs strip-types --compare` over the rest. Every file djs accepts must
match byte for byte; the files djs rejects are listed by message (decorators, `using`,
`accessor`, and the conformance tests' deliberate errors that TypeScript only reports in
its checker, such as `with` or duplicate parameters). In strip mode the parser also
accepts module syntax (`import`/`export`), since the corpus is modules. The gate runs
the small curated set in `tests/strip-types/` (compiler test `ts_strip_corpus`).

At 0048: 4053/4802 compared files match, 0 differ, 749 are djs syntax errors.

## Known differences (0082)

What is left in the ratchets (`tests/test262/expected-failures.txt`, 40 active tests at
0095; `tests/corpus/expected-failures.txt`, 1 case at 0097) and why. Everything else in the active
group passes. Update this list when an entry goes away or a new one is accepted.

**By design** (no fix planned; 40 test262 tests):

- *Always strict* (ARCH "Semantics stance", D014, D015). djs has no sloppy mode, so
  code that needs sloppy semantics fails:
  - `Function()` with duplicate or `eval`/`arguments` parameter names:
    built-ins/Function/15.3.2.1-11-* (9) and built-ins/Function/length/S15.3.5.1_A*_T3
    (4).
  - Sloppy code from `Function()` or indirect eval that assigns to `eval` or
    `arguments` or declares them: language/statements/variable/12.2.1-* (8) and
    language/statements/function/13.0-12-s, 13.0_4-17gs (2).
  - Indirect eval is strict eval code, whose declarations stay local to the eval
    (D1581). The spec's eval code is sloppy, so its `var`s and functions would become
    global properties with CanDeclareGlobal* checks: language/eval-code/indirect (12).
- *Annex B `__lookupGetter__` / `__lookupSetter__`* are not implemented. The
  `__getter__`/`__setter__` features are skipped, but these 4 tests use them without
  the tag: {expressions,statements}/class/elements/private-{getter,setter}-is-not-a-own-property.
- *Same as qjs*: language/identifier-resolution/assign-to-global-undefined.js expects
  `undeclared = (this.undeclared = 5)` to throw, because the reference is resolved
  before the right side runs. djs resolves it at the store, as qjs does (also in
  tests/test262/oracle-failures.txt).

Fixed in 0090: the 3 typed-array tests on resizable buffers (Array.prototype.fill with an
object value, Object.freeze of a non-fixed-length view, subarray of an out-of-bounds view).

**Non-goal** (1 difftest case): dquickts/builtin/test_immutable_array_buffer.js needs
SharedArrayBuffer and Atomics (ARCH non-goals) and sloppy `Function` code. The immutable
ArrayBuffer proposal itself is implemented (0097, D1970): the same test without those
lines is tests/corpus/builtins/immutable-arraybuffer.js, and test_typed_array.js passes.

**Oracle bug** (5 difftest cases): codemode-harvest/h075, h082, h084, h124 and h150
spread a tool's string result into an object (`{query, ...await tools.bash(...)}`). The
spec's CopyDataProperties copies a string primitive's characters as index properties,
as djs and V8 do; qjs copies nothing (`{...'ab'}` is `{}`). No fix in djs (0118, D2280).

Fixed since 0082 (0084, D1840): Function.prototype is a function (9 tests), [[Set]] of
an array's `length` converts non-Number values (4), and `atob`/`btoa` and
`DOMException` are native globals as in qjs (2 difftest cases).

## The gate

`scripts/gate.sh` must pass before anything merges into `main` (`scripts/merge-to-main.sh`
runs it in the shared gate worktree after rebasing, D1792). Its steps: fmt; clippy
(workspace, and wasm32 release for djs-core and djs-wasm); the workspace's unit and
integration tests; the no_std wasm32 build of djs-core; `scripts/bgen.sh --check`; the
djs-wasm build and the JS host package's tests (`node --test js/test/*.test.ts`); native
versus wasm heap images (`scripts/determinism/native-vs-wasm.sh`); the wire and reader
cross-language checks; difftest on djs with the ratchet, suspension, GC stress and
heap-image determinism; difftest with the bytecode backend; and the test262 active group
against its ratchet; and the size budget (below). It prints the slowest unit and test262
tests. Issues that add a test
suite also add it to the gate; agents do not run the gate's steps by hand (AGENTS.md).

### Size budget (D020)

The shipped module (`scripts/build-wasm.sh`: profile `wasm`, stripped) must stay within
1.5x quickjs-wasi's size, gzip -9. The gate builds it beside its other steps (in
`target/size`) and its last step runs `scripts/size/budget.py` on it, which compares its
gzip with `tests/scores/size.tsv`:

- `quickjs_wasi_gzip`: quickjs-wasi's module (3.6.2, pi's engine), recorded so that the
  gate needs no copy of it. Re-record it when pi moves to a new version.
- `ceiling_gzip`: a ratchet while the module is above the budget. The gate fails when the
  module's gzip exceeds it. Once the module is within 1.5x, the budget itself is the
  limit as well.

`budget.py` also fails when the module contains a build-machine path (`/Users/`,
`/home/`, `.cargo/registry`, `.rustup/`): panic locations carry source file paths, and
every build of the shipped module takes its build environment from `scripts/wasm-build-env.sh`,
which remaps them (`--remap-path-prefix`, D2331).

An issue that makes the module smaller lowers the ceiling on its branch
(`scripts/size/budget.py MODULE --update`: this gzip plus 1 KB of slack, never higher).
Raising it is a trade, recorded as a decision with the size and what it buys. The
attribution tools are in `scripts/size/` (README there).

