# How djs is tested

djs promises things that are hard to test: it behaves like QuickJS, down to the text of
error messages; it can stop after any instruction and continue in another process from
a copy of its memory or from a continuation dump; and it gives the same result whichever
way its builtins are compiled, on whichever platform. This page explains the test layers
that check these promises, why each exists, and how they come together in the gate every
change must pass.

> **In short.** test262 checks conformance against a checked-in list of known failures
> that may only shrink (the ratchet), and the difftest corpus runs codemode scripts in
> djs and in QuickJS and compares what a model would see. Stress modes run the same
> cases again while suspending, restoring, collecting and dumping all the time, with the
> other builtin backend and in wasm, and require identical results. One gate, cached by
> each check's inputs, runs all of it before anything reaches `main`.

## The oracle

djs replaces QuickJS, so QuickJS is the reference: `qjs` from dquickts, a QuickJS with
TypeScript support, at a fixed build. Wherever the specification leaves room (console
formatting, error messages, stack traces, property order), djs does what `qjs` does
([QuickJS's behavior](goals.md#quickjss-behavior)): models read error messages, and the
tests of agent hosts compare them.

The oracle is read-only for this project: it is never rebuilt or patched here, and its
results don't change, so they are recorded once and checked in. Where djs differs on
purpose (it is always in strict mode, it has no direct `eval`, a stack overflow is a
`RangeError`), the difference is declared where it is tested, not silently accepted.

> **Trade-off.** QuickJS as the oracle over the specification alone. Matching an
> existing engine catches what test262 does not check, and makes djs a drop-in
> replacement. The cost: QuickJS's own bugs and quirks become targets too, so each
> deliberate difference needs a reason and a declaration in the tests (`djs-expect`,
> and the reason next to every ratchet entry, below).

## test262

test262 is the conformance suite of ECMAScript. djs runs it at the commit dquickts uses,
<!--n test262/total-->53,597<!--/n--> tests, with its own runner (`djs-test262`, in-process, in parallel).
[`tests/test262.toml`](../../tests/test262.toml) puts every test into a group:

- **skip**: features djs will never have (sloppy-mode tests, `Intl`, Temporal,
  `SharedArrayBuffer`, direct `eval`, ...) and the module features it does not have yet
  (top-level `await`, deferred, text, bytes and source-phase imports). They are left out of the count, so the score stays honest. A rule matches by frontmatter flag, directory, feature tag, or a scan of
  the source for constructs no tag names, such as a direct `eval(` call.
- **active**: everything else, <!--n test262/active-->39,017<!--/n--> tests.

A test runs as a Script, strict, with the harness files it asks for, and a `$262` host
object (`evalScript`, `detachArrayBuffer`, `gc`). A test flagged `module` runs as the
root module of a graph instead, after the harness Scripts: its key is its path, and the
runner's module loader resolves each specifier against the importing module's directory
and reads the `_FIXTURE` files next to the tests. Compiling the test alone is the parse
phase and loading and linking its graph the resolution phase, so a negative test can
expect a missing export to fail at link time. `import()`, from modules and Scripts alike,
loads through the same loader. djs passes all but a few dozen active
tests ([status in numbers](status.md#conformance)). Those are listed with a reason in
[`tests/test262/expected-failures.txt`](../../tests/test262/expected-failures.txt), the
**ratchet**: a test that fails and is not in the file fails the gate, and a listed test
that starts passing is reported, so its entry gets removed and the list only shrinks.
Every remaining entry is a deliberate difference or waits for a feature:

- **Always strict.** djs has no sloppy mode, so tests that need it fail: `Function()`
  with duplicate parameter names or parameters named `eval` or `arguments`, sloppy code
  that assigns to `eval` or `arguments`, and indirect `eval` whose `var`s would become
  global properties (in djs, eval code is strict, so its declarations stay local). This
  is most of the list.
- **Annex B accessors.** `__lookupGetter__` and `__lookupSetter__` are not implemented;
  a few class tests use them without declaring the feature.
- **Top-level `await`.** Four module tests use it without declaring the feature (an
  `await using` at the top of a module, a fixture that awaits); djs does not have it
  yet.
- **Same as QuickJS.** One test expects an assignment to an undeclared global to throw
  because the reference is resolved before the right-hand side runs; djs resolves it at
  the store, as QuickJS does.

Every test runs with a fuel budget and a memory limit, and a test that exhausts either
fails as `timeout` or `oom`. The budget is fuel, not wall-clock time
([fuel](limits.md#fuel)): a loaded machine slows a test down but cannot turn a pass into
a failure. A generous wall-clock limit remains only as a guard against hangs outside the
engine's own work.

> **Trade-off.** A ratchet over requiring every test to pass. The suite runs in the gate
> whatever djs fails, and every change is checked for regressions anywhere. The cost: a
> stale entry hides a regression of that test, so the list must be updated whenever a
> fix makes tests pass, which the gate's report shows but does not enforce.

> **Trade-off.** Fuel budgets over wall-clock timeouts. Results do not depend on the
> machine's load, which matters when many builds and test runs share one machine. The
> cost: fuel counts only the engine's own work, so a wall-clock guard still exists.

## Difftest: what the model sees

test262 checks pass or fail. It does not check what `console.log` prints, the text of an
error, or how a script that reads tool results behaves. The difftest corpus
([`tests/corpus/`](../../tests/corpus/), several hundred cases) checks exactly that.

A case is a codemode script: the body of a strict async function, as an agent's codemode
runs it. Both engines run it in the same model: the same JavaScript prelude
([`tests/corpus/prelude.js`](../../tests/corpus/prelude.js)) defines `console`, fake
`tools` and the formatting of results, and the result is a *transcript*: every output
line, then how the script ended (`=== return` with the value, `=== completed`, or
`=== throw` with the error). A fake tool answers through an asynchronous host call that
the runner settles at the next idle point, so every tool call is an idle point, as with a
real tool (under QuickJS, an already settled promise). QuickJS's transcript is checked in as `<case>.expected`, and the runner
compares djs's with it.

The examples in this guide run the same way. This one could be a difftest case:

```js live
const r = await Promise.allSettled([Promise.resolve(1), Promise.reject(new TypeError("no"))]);
console.log(r.map((x) => x.status).join(", "));
try {
  null.x;
} catch (e) {
  console.log(e.name + ": " + e.message);
}
console.log({ ok: r[0].value, err: String(r[1].reason) });
```
<!-- gen:run -->
```text
fulfilled, rejected
TypeError: cannot read property 'x' of null
{"ok":1,"err":"TypeError: no"}
```
<!-- /gen -->

Stack frame lines are left out of the comparison, because the two harnesses differ below
the case. Cases marked `// difftest: stacks` compare them too, with the frames
normalized to the case's own. Other directives in a case's leading comments mark a
TypeScript case, a deliberate difference (`djs-expect`, with the line QuickJS prints and
the one djs prints instead), a djs non-goal that only QuickJS runs (`skip-djs`), or a
case that cannot finish under the stress modes (`no-stress`, with the measured reason).

The cases come from four sources:

- dquickts' own tests, split into one case per test function;
- codemode-style scripts with fake tools;
- <!--n corpus/harvest-->300<!--/n--> scripts that models actually wrote in agent sessions, redacted before they
  were committed;
- hand-written cases for builtins, formatting, errors and every fast path, including the
  inputs where a fast path must decline.

The harvested scripts are the most realistic input there is: they read files and run
commands in parallel, slice and join the results, and print summaries. The sessions do
not record what the tools returned, so each harvested case answers its tool calls with
synthetic results of the same kind: text for files and commands, and for tools with
structured results a value built from the result type the session declared for the
tool. The text of the calls' arguments, which is where the private paths and commands
were, is replaced, and so is any long string literal, such as a prompt or an embedded
program. The harvester is [`scripts/harvest/`](../../scripts/harvest/).

The corpus has its own ratchet,
[`tests/corpus/expected-failures.txt`](../../tests/corpus/expected-failures.txt). Its
entries are a dquickts test that needs `SharedArrayBuffer`, which djs leaves out, and a
few harvested scripts that spread a string into an object (`{...await tools.bash(...)}`):
the spec copies the string's characters as index properties, as djs and V8 do, and
QuickJS copies nothing.

> **Trade-off.** Comparing transcripts with QuickJS over writing expected output by
> hand. A new case costs only its source: `--update` records the expected output from
> `qjs`, never from djs. The cost: every case must run on QuickJS, so djs-only features
> and deliberate differences need directives, and a case can only check what shows up in
> the transcript.

## Stress modes: the same answer under any schedule

Suspension is djs's hardest promise ([suspend anywhere](goals.md#suspend-anywhere)). A
run may stop after any instruction (out of fuel, or waiting for a tool), be written to an
image and continue in a fresh VM restored from it. A bug there shows only when a stop
falls on the wrong instruction, which ordinary tests almost never hit. So the tests make
stops happen everywhere:

- **Suspension** (`--suspend-every 97`, and 1009): every case runs again in slices of 97
  fuel, and at every stop the VM continues in a new VM restored from its image.
- **GC stress** (`--gc-stress`): a collection at every safepoint, so every place that
  holds a heap value across an allocation is exercised.
- **Determinism** (`--determinism`): the suspended runs must leave exactly the same heap
  image as the plain run, byte for byte, and the same canonical hash (a hash of the
  object graph alone). Collections happen at the same instructions under any slicing,
  and the collector keeps cells in allocation order, so any difference means a stop or a
  restore changed something ([determinism](determinism.md)). The plain run also compiles
  each case through the compile cache, and the cached and the direct compile must leave
  the same heap.
- **Lazy and eager compilation.** The stressed corpus runs with functions compiled at
  their first call; test262 runs both lazily and eagerly, side by side.
- **Native and wasm**: [`scripts/determinism/native-vs-wasm.sh`](../../scripts/determinism/native-vs-wasm.sh)
  runs the same programs in a native build and in djs-wasm and compares their images and
  fuel stop for stop ([wasm32 first](goals.md#wasm32-first)).
- **Both builtin backends**: the corpus runs again with the builtins as bytecode instead
  of generated Rust and without the `@fast` entries ([builtins](builtins.md)), so both
  implementations of a builtin stay equal.
- **The unchecked heap**: the shipped modules' arena accessors skip their bounds checks,
  so that build runs the stressed corpus in release, after its accessors' assertions
  have run the end-to-end tests in debug, and a host's codemode tests run on the shipped
  modules themselves.

In every mode the transcript must equal the plain run's: a difference is a failure on
its own, separate from the comparison with QuickJS, and a stressed run must pass the
same number of idle points. The end-to-end tests of djs-core
([`tests/data/e2e/`](../../tests/data/e2e/), each case with QuickJS's output) run the
same ways at a finer grain: plainly, with one unit of fuel per slice and a restore at
every stop, and under GC stress.

> **Trade-off.** Re-running whole suites under stress over targeted unit tests of
> suspension. Every feature, including those added later, is tested for resumability
> without anyone writing a test for it. The cost: CPU time. The stressed corpus and the
> test262 run are the largest part of the gate, which is why a change is checked only
> against the cases it touches while it is being written.

## Dumps: continuing in another engine

A [continuation dump](dumps.md) must resume in a fresh VM, in a later build, and in
node, which has never seen djs. Three kinds of tests hold it to that:

- **A dump at every idle point.** The plain-dumps check dumps each codemode case of the
  corpus at every idle point, with frozen builtins, runs the dump in a fresh VM and goes
  on from there, in chains of up to 50 links. The transcript must equal the plain
  run's, and node must produce the same transcript from the same dumps. A dump the
  writer refuses ends the chain; which cases may refuse is a ratchet of its own,
  [`tests/plain-refusals.txt`](../../tests/plain-refusals.txt). The end-to-end tests add
  a dump mode: every case dumps its final state, the dump runs in a fresh VM and is
  dumped again, and a further round must give the same dump.
- **Golden dumps.** [`tests/dump/v2/golden/`](../../tests/dump/v2/golden/) holds small
  programs and the dumps written of them at every idle point, one program per construct
  the writer handles (operand positions, loop kinds, promise combinators, generators,
  classes). Every build must load each dump and finish the program with the output of
  the uninterrupted run, plainly and under GC stress. A golden dump is never rewritten to
  make a build pass: that a later build runs an earlier build's dumps is the point.
- **Size.** The dumps of the corpus, of the golden programs and of an example are
  measured, and [`tests/scores/dump-size.tsv`](../../tests/scores/dump-size.tsv) records
  each case's raw and gzip size; the gate fails when a dump grows past its score.

A fuzzer looks for what these miss: [`scripts/plain-fuzz/fuzz.mjs`](../../scripts/plain-fuzz/fuzz.mjs)
generates codemode-like programs from seeds (JSON data, Maps, Sets, Dates, regular
expressions, typed arrays, errors, symbol keys, classes with accessors, every third one
TypeScript), runs each with a chain of dumps from every idle point in djs and in node,
and reports each distinct failure with its smallest program. It is too slow for every
merge; it runs before changes to the dump writer, and each finding becomes a corpus case
under `tests/corpus/codemode/plain-fuzz/`, marked as a known failure until it is fixed.

## More checks against an independent reference

Several parts of djs have a second implementation or an external reference, and are
tested against it:

- **The builtins against QuickJS's natives.** djs-bgen's behavior test runs the
  generated prelude inside `qjs`, next to QuickJS's own builtins, and compares results,
  errors and the order of proxy traps.
- **The wire format and the reader across languages.** Values encoded in TypeScript are
  decoded in Rust, and heap images written by Rust are read in TypeScript
  ([the wasm boundary](host.md)).
- **The TypeScript type eraser against ts-blank-space.** `djs strip-types` prints a
  TypeScript file with its types blanked out and must produce exactly what
  ts-blank-space produces. The gate runs a curated set,
  [`tests/strip-types/`](../../tests/strip-types/);
  [`scripts/strip-types/oracle.sh`](../../scripts/strip-types/oracle.sh) compares
  TypeScript's own sources and conformance tests, thousands of files.
- **The image verifier against corrupt images.** A test flips, truncates and overwrites
  bytes of a heap image with most cell kinds in it, and loading must return an error or
  a valid heap, never panic or read outside the arena.
- **Image compatibility.** Tests check that each hash in an image's dependency
  descriptor changes when one entry of its table does, so a build refuses an image it
  would misread ([what an image depends on](snapshots.md#what-an-image-depends-on)).
- **The builtin prefix.** Debug builds can arm a write barrier that panics on any write
  to the base image's cells; it is armed for every VM with frozen builtins and for the
  codemode cases of the corpus, whose scripts never change a builtin.
- **An agent host's own tests.** The codemode test suite of [an agent
  host](pi.md) runs on the shipped modules through its adapter, and on the slim module
  with frozen builtins.

## The gate

[`scripts/gate.sh`](../../scripts/gate.sh) is the one check that guards `main`. It
consists of legs, each one check:

- formatting, and clippy with warnings as errors, natively, for wasm32 and on every
  feature combination that ships;
- all unit and integration tests, which include the freshness of the generated builtins
  and the golden dumps;
- the `no_std` wasm32 build of djs-core;
- the builtin registry's append-only check against `main`;
- `djs-docs check` for this guide, which also reruns its examples;
- the host package's tests on the djs-wasm modules, native-versus-wasm images, and the
  cross-language wire and reader checks;
- the difftest corpus with suspension, GC stress and determinism; the plain dumps in djs
  and node with their size; the corpus again with the bytecode backend and on the
  unchecked-heap build;
- the active test262 group against its ratchet;
- the slim build's tests, corpora and test262, against their own ratchets;
- the size budget of the shipped modules;
- an agent host's codemode tests on the shipped modules.

Nothing reaches `main` without passing it.
[`scripts/merge-to-main.sh`](../../scripts/merge-to-main.sh) rebases the branch onto
`main`, regenerates the generated files from the merged sources, runs the gate on
exactly that commit in a dedicated worktree, and fast-forwards `main` to it only if it
passes and `main` has not moved since; otherwise it rebases and gates again. The lock
that orders changes to `main` is held only for the fast-forward. So `main` only ever
points at commits that passed the gate, and the gate runs at full priority while
everything else on the machine runs niced.

Three things keep it fast without checking less:

- **Parallel builds.** Every build the legs need gets its own cargo target directory,
  so they all build at once, and each leg starts as soon as its builds are done.
- **A cache by inputs.** A leg's key hashes the files it can read and what it reads
  outside the repository (the toolchain, the oracle, the test262 checkout, the agent
  host's sources), and a leg that passed on exactly those inputs is not run again. Which
  files a leg can read is one of three classes: everything (the guide), everything but
  the documentation (the JavaScript legs), and that minus the JavaScript package (the
  Rust legs). An always-run check fails when Rust code starts naming a file outside its
  class. So a documentation change gates in seconds, a change to the host package reruns
  only the JavaScript legs, and a change to the engine reruns every Rust leg.
- **Slow checks apart.** `scripts/gate.sh --full` ignores the cache and adds checks too
  slow for every merge, such as the agent host's tests with a dump and a resume at every
  idle point. It runs daily and before releases.

> **Trade-off.** One gate on every merge over checks after the fact. `main` is always
> green, and a regression is caught in the change that caused it. The cost: merges queue
> behind each other. The cache makes that cost proportional to what a change can affect,
> at the price of trusting the input classes; the full run is the backstop for what a
> class misses, such as a flaky test or a path built at run time.

## Benchmarks

Speed and size are measured, not gated by timing, because a timed check on a shared
machine fails at random:

- **Instructions in wasm.** [`scripts/perf/wmeasure.py`](../../scripts/perf/wmeasure.py)
  runs codemode workloads ([`scripts/perf/w/`](../../scripts/perf/w/)) in djs-wasm and in
  quickjs-wasi under node and counts the machine instructions the process retires, minus
  an empty run. Instruction counts repeat to within half a percent on a loaded machine,
  where times do not. [Status in numbers](status.md#speed-in-wasm) shows the results.
- **Components and engine runs.** [`scripts/bench.sh`](../../scripts/bench.sh) runs
  [`crates/djs-bench/`](../../crates/djs-bench/) in release: the lexer, number
  conversion, regular expressions, JSON, the collector, and whole scripts from
  [`tests/bench/`](../../tests/bench/), each checked against its expected result. With
  `--record` it appends to [`tests/scores/bench.tsv`](../../tests/scores/bench.tsv), and
  `--compare` flags changes beyond the noise; numbers compare only within one machine,
  recorded when it is quiet. The gate runs the same scenarios once, untimed, so the
  harness keeps working.
- **Size.** The size-budget leg builds the shipped modules and compares their gzip size
  with a ceiling in [`tests/scores/size.tsv`](../../tests/scores/size.tsv); a change
  that would cross it has to make room elsewhere, or raise it as a deliberate trade.

## Where to look

- [`tests/test262.toml`](../../tests/test262.toml),
  [`tests/test262/expected-failures.txt`](../../tests/test262/expected-failures.txt):
  the groups and the ratchet; [`crates/djs-test262/`](../../crates/djs-test262/): the
  runner.
- [`tests/corpus/`](../../tests/corpus/): the difftest cases, their expected output and
  the shared prelude; [`crates/djs-difftest/`](../../crates/djs-difftest/): the runner
  and its stress modes.
- [`crates/djs-core/tests/e2e.rs`](../../crates/djs-core/tests/e2e.rs) and
  [`tests/data/e2e/`](../../tests/data/e2e/): the end-to-end tests.
- [`crates/djs-core/src/vm/dump_resume_tests.rs`](../../crates/djs-core/src/vm/dump_resume_tests.rs)
  and [`tests/dump/v2/`](../../tests/dump/v2/): the golden dumps;
  [`scripts/plain-node.mjs`](../../scripts/plain-node.mjs): dumps continued in node;
  [`scripts/plain-fuzz/`](../../scripts/plain-fuzz/): the dump fuzzer.
- [`crates/djs-core/tests/it/verify_fuzz.rs`](../../crates/djs-core/tests/it/verify_fuzz.rs):
  the verifier against corrupt images;
  [`crates/djs-core/src/vm/compat_tests.rs`](../../crates/djs-core/src/vm/compat_tests.rs):
  image compatibility.
- [`scripts/gate.py`](../../scripts/gate.py) (run by
  [`scripts/gate.sh`](../../scripts/gate.sh)) and
  [`scripts/merge-to-main.sh`](../../scripts/merge-to-main.sh): the gate's legs, their
  input classes and the cache, and the merge flow.
- [`scripts/bench.sh`](../../scripts/bench.sh), [`scripts/perf/`](../../scripts/perf/)
  and [`scripts/size/`](../../scripts/size/): benchmarks, instruction counts and size
  attribution.
