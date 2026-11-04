# djs: Testing

There are three layers of tests:

1. **test262**, which checks conformance (pass/fail per test). It is the backbone of the
   hill-climb.
2. **Differential tests against dquickts**, which compare behavior that test262 does not
   check: `console.log` formatting, error messages, stack traces, TypeScript, and codemode
   scripts.
3. **Stress modes**, which re-run layers 1 and 2 under conditions that must not change the
   outcome: suspend and restore from the VM's image every N instructions, dump and load
   at every idle point, and the bytecode versus native builtin backend.

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
`store`/`load` and fake `tools` (`echo`, `fail`) with pi's formatting. The fake tools
(and the harvested cases' own fakes) answer through `host.later(value, rejected)` (D2590):
in djs an async host call that the runner settles at the next idle point, one per idle
point in call order, so every tool call is an idle point as with a real tool; under qjs
`Promise.resolve`/`Promise.reject`. The case source is
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
$D --engine djs --ratchet --suspend-every 97 --gc-stress --determinism
                                     # what the gate runs (--suspend-every 97 also runs 1009)
$D --engine djs --backend bytecode --ratchet               # the gate, too (0063)
$D --engine djs --update-ratchet     # after fixing cases: rewrite expected-failures.txt
$D --engine djs --idle-points codemode   # the idle points per case (the total always prints)
```

With `--engine djs` the case runs exactly as with qjs: djs evaluates the shared prelude
and the case Script in-process, and only `host.output`/`host.done` are host functions,
plus the async `host.later` (D1440, D2590). A case waits on `later` only where it awaits a
tool, so djs and qjs differ only for a case whose output depends on whether a tool's
answer comes before or after other promise jobs (none in the corpus at 0173; the glue
test's `later.js` shows the difference). A stressed run must also pass the same number
of idle points as the plain run. `--suspend-every N` and `--gc-stress` run
every case a second (third) time; a transcript that differs from the plain djs run is a
hard failure, separate from the oracle comparison and the ratchet (D1161). Stressed runs
get three times the case's time limit, and a stressed run that still times out is a
failure too: mark such a case `no-stress` with the measured reason (D1530).
`--plain-dumps` (0172, 0201, D2830) dumps the VM at every idle point (a plain Script: the
session's globals, the prelude's state, open calls, pending promises, suspended
activations as continuations, the prelude's `api` as the completion value), runs the dump
in a fresh VM with the host's `$djs.seed`/`$djs.resume` and continues there, at most 50
links per case; the transcript must equal the plain run's. The VMs have frozen builtins
(D2836), so only cases with idle points are compared (only `codemode` and
`codemode-harvest` have them). A refused dump ends the case's dumps and the run goes on.
`--plain-out DIR` also writes every link's dump, the output before it and the plain run's
transcript, and `scripts/plain-node.mjs DIR` runs each dump in node and requires the same
transcript (node's clock pinned to djs's, its wording of the common TypeErrors mapped to
djs's). Its `--refusals tests/plain-refusals.txt` is the ratchet of cases that may refuse
(`--update` rewrites it). The gate leg "plain dumps (djs and node)" runs both.
`--plain-from K` starts the chain at idle point K. A case marked `// difftest:
plain-xfail <reason>` reproduces a known plain-dump bug: its differences (djs or node) are
counted, not failures, and the mark is reported as stale once both continue identically
(D2941).
The plain-dump fuzzer (0208, D2940) finds such bugs: `node scripts/plain-fuzz/fuzz.mjs
--seed S --cases N --time SECS` generates codemode-like programs from consecutive seeds
(scripts/plain-fuzz/gen.mjs, D2942: JSON data, Maps, Sets, Dates, RegExps, typed arrays,
Errors, symbol keys, classes with accessors and static members; every third seed a
TypeScript body; `fuzz.mjs gen S` prints one) and runs each with a chain of
plain dumps starting at every idle point, in djs and node, then lists the distinct
failure signatures with the smallest program of each. `fuzz.mjs check FILE...` runs given
programs (case bodies) the same way, and `fuzz.mjs reduce FILE --sig TEXT` shrinks one
while its failure signature contains TEXT. It runs at most 4 niced processes (`--jobs`) and
starts none while the load is above 30 (`--max-load`). It is not a gate leg; a finding becomes a
corpus case (tests/corpus/codemode/plain-fuzz/), marked `plain-xfail` or listed in
tests/plain-refusals.txt until it is fixed.
`--determinism` adds a second suspension slice (1009) and requires the suspended runs to
leave the same heap image as the plain run, both the raw image and its canonical hash
(the graph alone, computed by a copying collection of a copy; D1720, D1725, D2040). The e2e
test compares its plain and suspension modes the same way (its fourth mode, dump, runs
every case with frozen builtins, dumps its final state, runs the dump in a fresh VM and
dumps again: the second round must reproduce itself; `DJS_E2E_MODE=dump`; its output is
not compared, refusals are counted, not failures), and
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

## Instruction ratios to quickjs-wasi (0129)

`scripts/perf/wmeasure.py` reports wasm instructions for djs-wasm and quickjs-wasi in
node, minus an empty run of each. That subtraction removes startup but not V8's tier-up.
Once a script runs long enough, V8 recompiles the hot functions of the module with
TurboFan, and those instructions count too.

A nearly trivial script (one `Array.from` of 120 template strings) measures about 525 M
for djs and 322 M for quickjs-wasi. Nearly all of that is compilation, and every longer
workload carries it.

So when reading a ratio:
- for small workloads it is mostly the two modules' compile cost;
- for the larger ones (pi-like ~7 G) the engine-only ratio is a little better for djs
  than the printed one (0129: pi-like 0.94x printed, ≈0.91x without the compile).

Compare djs builds with each other (same baseline) for decisions. Use the quickjs-wasi
ratio as the user-visible figure, a process that runs one script.

## The prefix write barrier (D2065)

The builtin prefix (the base image's cells below `Roots::BASE_END`, D022) must not change
in a VM with frozen builtins, and in no VM unless a script changes a builtin itself. Debug
builds check that with a barrier: armed, any write below `BASE_END` (past the roots cell)
panics, in `Arena::write` and its siblings, and a comparison of the whole prefix after
every `Vm::run` and collection catches the writes through raw slices (the interpreter's
hot loop). Release builds have no barrier.

One switch arms it, `VmOptions::prefix_barrier` (`Heap::force_prefix_barrier`), and frozen
builtins (D2304) arm it always. Armed in the gate:
- every VM with frozen builtins (`vm/frozen_tests.rs`, and any suite that turns the mode
  on);
- difftest's `codemode/` and `codemode-harvest/` areas (`ARMED_AREAS` in
  crates/djs-difftest/src/djs.rs), in every mode the gate runs them (plain, suspension,
  GC stress, determinism).

Not armed: the rest of the corpus, e2e and test262, whose cases change builtins on purpose
(`defineProperty` on prototypes, Proxy and BigInt tests).

## Image compatibility (D026, D2265)

Images do not persist runs (dumps do, below; D2560, D2700), but they still cross from one
build's module to another inside a process: the shipped module and the dump module
(D2704), and the base image every VM is copied from. A VM image restores into any build
whose dependency descriptor matches the one in its roots cell (`vm::ImageDeps`,
crates/djs-core/src/vm/compat.rs): the layout hash, the image feature set,
`COMPAT_VERSION`, the opcode and intrinsic table hashes and `RECORDS_HASH` (the id tables
and slot constants that crates/djs-core/compat_inputs.rs lists, read from the sources).
The engine build hash is not part of it. `vm::compat_tests` (run by the
gate) checks that each hash changes when one entry of its table does, that every region
of compat_inputs.rs is found, and that loading names the component that differs.

The hashes see numbering, not meaning. **Bump `vm::COMPAT_VERSION`** in the change itself
when an image written before it would still load (every hash equal) but be read
differently after it:
- an existing slot, field or record element changes what it holds or how it is encoded
  (the `Value` stored in a slot of a Map table, a promise record, a generator's saved
  frame, a bound function, an iterator), or a convention of the frame and value stack
  of a suspended function changes;
- the builtins' JS reads an internal record differently (`%RecordGet`/`%RecordSet`
  indexes, the meaning of an internal slot of a builtin object) while the prelude's
  numbering stays;
- an opcode, an intrinsic or a native builtin keeps its entry but its operands or
  arguments mean something else (an immediate's encoding, an argument order);
- an invariant of stored data changes: string hashes and atom table hashing, rope and
  append-buffer rules, property table and shape hashing, inline cache words,
  dictionary-mode or dense-element conventions, RegExp program encoding beyond its
  opcode constants;
- ids from outside the tables start to be stored in images.

No bump for what only changes behavior (a fixed builtin, a faster path, a new
optimization of the compiler or the collector), for additions images of older builds
cannot contain (a new cell kind, opcode or intrinsic changes its hash anyway), or for a
change a hash already sees. When unsure, bump: a needless bump only refuses images, a
missed one lets an old image restore into an engine that misreads it. A new cargo
feature of djs-core must be classified in compat_inputs.rs (D2267); the build fails
until it is.

## Dumps across builds (0178, D2560, D2619, D2920)

Images are tied to a build's tables; continuation dumps are not: a dump is a plain
Script (D2830) that calls only the host's `$djs` (the dump runtime's helpers and the
hooks `seed` and `resume`, D2890, D2903), so every later build, and node, runs it. The
golden dumps hold that promise (every gate run,
`vm::dump_resume_tests::golden_dumps_load_and_finish`): tests/dump/v2/golden/ has small
programs (`NAME.src.js` Scripts, `NAME.body.js` codemode bodies) and the dumps the
exporter wrote of them at every idle point, `NAME.K.dump.js` (K in the name: dumps have
no comments). Each dump runs in the test host (`later`, `print`, `tools`, `console`), the
`later` results are delivered, and the program's output up to idle point K plus the
dump's run must equal the uninterrupted run's, plainly and under GC stress in slices of
97 fuel. One program per operand role sequence (call, spread call, new, member, super
member, object, array, binary, assignment, template, nested), loop kind (for-of over
array/Map/generator, for-await over a sync iterable and an async generator, nested
loops), re-enactment (then/catch/finally, held resolving functions, the combinators,
iterators, generators, activations: a returned promise, `arguments`, shared cells), class
case (fields, static block, accessor; derived with `super`; a class from a factory) and
the codemode body; a program whose idle points are all refused (instances with private
names, D2878) is listed in tests/dump/v2/plain-refusals.txt. Since the dumps are
JavaScript, they hold the runtime and the engine to the format, not the exporter: a
change to the exporter leaves them alone. **A golden dump is never rewritten to make a
build pass** (D2619): changing or deleting one needs a decision that says why the old
dump may stop running (the runtime changed incompatibly, which D024 allows while djs is
unreleased). New programs get their dumps with `DJS_GOLDEN_WRITE=1 cargo test -p
djs-core --lib write_golden_dumps -- --ignored`, which writes only programs that have
none. (The goldens were rewritten as plain dumps when format 2 was retired, D2920; the
two-commit check of format 2, scripts/dumps-across-builds.sh, went with it.)

## Peak memory claims (D2048)

Peak wasm memory (`wasm memory` in `scripts/perf/wasm-run.ts`) is deterministic for one
build and input, but sensitive to where the allocator places blocks. Before djs-wasm's
page allocator (D2053) a shift of a few KB in the live heap could move a workload's peak
by 3–6 MiB, in either direction; it is less sensitive now, but not insensitive. A single run per build shows the placement, not the policy. Any claim
that a change makes peak memory better or worse is backed by
`scripts/perf/mem-perturb.sh`: 24 runs per workload, run k with a kept string of
k × 1000 bytes allocated first. Compare the distributions (mean and max) against main,
with `--root` pointing at a main checkout for builds whose heap layout differs. A workload
is worse than main if its distribution is, not if one run is.

## The gate

`scripts/gate.sh` must pass before anything merges into `main` (`scripts/merge-to-main.sh`
runs it in the shared gate worktree after rebasing, D1792, and fast-forwards `main` only
to a commit that passed it, rebasing and gating again when `main` moved meanwhile,
D2666). It runs `scripts/gate.py`: a
set of **legs**, the checks, and the **builds** they need (D2660). `scripts/gate.sh
--list` prints them. The legs: fmt; clippy (workspace, and wasm32 release for djs-core
and djs-wasm), and on the other feature combinations that ship (D2668: djs-core slim,
slim with `dump`, full without `dump`; the wasm modules in release: main, slim, dump, slim
with dump); the workspace's unit and integration tests (djs-docs's apart; djs-bgen's
`builtins_gen_is_fresh` is what `scripts/bgen.sh --check` checks); the no_std wasm32 build
of djs-core; the builtin registry's append-only check (`scripts/registry.py check main`); the guide (`djs-docs
check`); the JS host package's tests on the dev modules; native versus wasm heap images;
the wire and reader cross-language checks; difftest on djs with the ratchet, suspension,
GC stress, heap-image determinism and a dump at every idle point; difftest with the
bytecode backend; the unsafe-heap build (verifier and e2e in debug, the stressed corpus
in release); the test262 active group against its ratchet; the slim build's legs (below);
the size budget (below); pi's tests on the slim module and in the adapter's next mode.
The `test` and `test262` legs print the slowest tests. The always-run `targets` leg
keeps what the manifests leave out of `cargo test` empty (D2790): no library has doctests
(`doctest = false` everywhere; a doctest fails the leg, so whoever adds one turns its
crate's doctests back on and gives the gate a doctest leg), and the programs with
`test = false` have no unit tests. Each package's integration tests are one binary,
`tests/it/main.rs` with a module per former file (djs-core's e2e stays its own:
`cargo test -p djs-core --test e2e`; the others are `--test it`, filtered by module, e.g.
`--test it slices::`), D2791. Issues that add a test suite add a
leg (`jobs()` in scripts/gate.py: its command, the builds it needs, its input class);
agents do not run the gate's steps by hand (AGENTS.md).

### How it runs (D2661)

Every cargo configuration the gate builds (a package and feature set, a profile, a
target) has its own target directory (`target/` for the workspace, `target/gate-core`,
`gate-difftest`, `gate-test262`, `gate-docs`, `gate-unsafe`, `slim`, `clippy`,
`wasm-dev`, `wasm-nodump`, `size`, `size-slim`, `clippy-combos`, one `clippy-wasm-*` per
module; the release unsafe-heap build in
`target/` under its own profile), because cargo locks a target directory while it builds:
so they all build at once. The commands are the ones the serial gate ran (the same
packages, features and profiles), split into a build and a run: legs run the built
binaries (hard links in `target/gate/bin/`) instead of `cargo run`, and the test binaries
of a `cargo test --no-run` in parallel, each the way `cargo test` runs it (in its
package's directory, through scripts/test-runner.sh). A leg starts when its builds are
done. The first failure stops the gate and prints that leg's output; every leg's output
is in `target/gate/log/`, the step times in `target/gate-steps.tsv`. One failure is not
the commit's: rustc's incremental cache going bad, so that a link fails on undefined
`_anon.*.llvm.*` symbols. A job that fails that way gets its target directory's
`incremental/` caches cleared and runs once more (D2896); the gate prints that, and the
failed run's log stays as `<leg>.log.corrupt`.

macOS scans every new executable on its first run (syspolicyd and XProtect, one at a time,
~0.1–2 s per binary); a gate after a Rust change links ~60 of them. The gate runs each new
test binary once (`--list`) as soon as cargo reports it, so the scans overlap the rest of
the build (D2665); a build ends when cargo does, and the runs still waiting go on beside
the legs (D2792). Fewer new executables is the other lever: no doctest binaries, no empty
test binaries, one integration-test binary per package (D2790, D2791). Builds that are not on the longest path
(everything but the workspace's test build) run at `nice 4`.

### The cache (D2662, D2663)

A leg that passed is not run again on the same inputs. Its key hashes the leg's name,
the committed tree restricted to the leg's **input class**, and what it reads outside the
repository. The classes, defined by what they leave out, so a new top-level directory is in
all of them:

| class | leaves out | legs |
|---|---|---|
| `all` | nothing | the guide, djs-docs's tests |
| `nodocs` | `docs/`, `work/`, `.github/`, the top-level `*.md` | js tests, wire/reader cross-language, native vs wasm, pi's tests |
| `rust` | `nodocs`'s, and `js/` (but `js/src/layout.generated.ts`) and `examples/` | everything else |

`scripts/` is in every class, so a change to the gate reruns every leg. Outside the
repository, every leg's key has the toolchain (`rustc -vV`, `cargo -V`, node, python),
the qjs binary (size and mtime; `$DJS_QJS`), vendor/test262's commit and local changes
(`$DJS_TEST262`) and vendor/unicode's NormalizationTest.txt; pi's legs also have
pi-private's commit, its local changes and its installed packages
(`$PI_PRIVATE`, node_modules/.package-lock.json). A leg that reads something new outside
the repository adds it to `EXTERNALS` in scripts/gate.py.

Why nothing the uncached gate caught can pass: a cached leg passed on a tree whose files
in its class are byte-identical, with the same tools and outside inputs, and a leg reads
nothing else, so it would pass again. The classes hold because only djs-docs reads
`docs/` and the top-level Markdown (its legs are `all`), nothing reads `work/` or
`.github/`, and of `js/` and `examples/` Rust reads only the layout file; the always-run
`paths` leg fails when Rust code outside djs-docs names a path outside the `rust` class
(`PATHS_ALLOWED` lists the exceptions with their reasons). Legs that depend on more than
the tree run every time: `fmt`, `registry` (it compares with main) and `paths`. Builds are
not cached (cargo's own incremental builds are); a build runs when a leg that needs it
runs.

The cache is `work/scratch/gate-cache/` of the primary checkout, one file per passed key
(`$DJS_GATE_CACHE` moves it). Entries are written only from a clean worktree (the keys
are the commit's), read entries that are missing, unreadable or corrupt mean "run the
leg", and entries unused for 14 days are pruned on every run. `scripts/gate.sh
--clear-cache` empties it; `--no-cache` runs every leg without reading it.

### Full runs (D2664)

`scripts/gate.sh --full` reads no cache and also runs the **full-only** legs, checks too
slow for every merge: pi's tests in the next mode with a dump and a resume at every idle
point (`scripts/pi-dump-every-idle.sh`, its own LTO build). It must run on main at least
daily (the coordinator) and before a release, in a worktree of its own: merge-to-main
fast-forwards main in the primary checkout, which would move the tree under the run, so
`scripts/gate.sh --full` started there runs in `.worktrees/full` (created on first use,
checked out at main; D2669); passes are recorded in
`work/scratch/gate-cache/full-passes.tsv`, and every other gate run reminds when the last
one is older than 24 hours. A full run also catches what the cache cannot see: a flaky test,
and a leg that reads a path outside its class without the `paths` leg noticing (a path
built at run time).

### Builtin registry (D2371)

tests/builtins/registry.tsv gives every builtin object, function and symbol and every
intrinsic a stable id, keyed by a spec-style path. Two checks keep it honest:
- `cargo test -p djs-prelude` (registry test): every builtin and intrinsic of this build
  has a live entry, and every live entry still resolves. When it fails after adding or
  removing builtins, run `scripts/registry-update.sh [reason]` and commit the file (new
  builtins are appended; removed ones become tombstones with the reason).
- The gate step `scripts/registry.py check main`: the file is append-only against main.
  No line of main's table may be renumbered, dropped or rewritten, except that a live
  entry may gain a tombstone.

The file merges with git's union driver; scripts/regen.sh (run by merge-to-main after the
rebase) renumbers a merged tail after main's entries and runs the update, so two branches
that both appended need no hand resolution.

### The slim build (D2411, D2413)

djs-core's default features `full` and `unicode` make the default build; without them it
is the slim build (`scripts/build-wasm.sh --slim`: target/djs-slim.wasm). Everything above
runs the full build. The slim build, natively in `target/slim`
(`--no-default-features`), gets:
- `cargo test -p djs-core --no-default-features --lib -- vm::slim_tests vm::frozen_tests
  vm::registry`, and djs-prelude's tests with `--features raw,registry` (the registry
  against tests/builtins/registry-slim.txt). `vm::slim_tests` runs in both builds and
  checks both sides of the boundary;
- difftest's `codemode` and `codemode-harvest` with `--ratchet --suspend-every 97
  --gc-stress --determinism`;
- test262's active group: a runner built without `full` skips tests/test262-slim.toml's
  groups and compares with tests/test262/expected-failures-slim.txt (update it with the
  same command and `--update`);
- pi's codemode tests on djs-slim.wasm, with pi's prelude minus its lockdown on frozen
  builtins: `DJS_WASM=target/djs-slim.wasm node js/pi/pi-tests.ts --no-lockdown --expected
  tests/pi/slim-expected-failures.txt` (only new failures fail);
- its size ratchet, `slim_ceiling_gzip` (`scripts/size/budget.py MODULE --slim`).

A change that adds or removes a builtin in an omitted family updates
tests/builtins/registry-slim.txt with `scripts/registry-update.sh` (merge-to-main runs it).
A new builtin file or builtin that belongs to the slim cut gets `// @slim-omit` (file) or
`@slim-omit` (builtin), and its globals go into `realm::ABSENT_GLOBALS`.

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

