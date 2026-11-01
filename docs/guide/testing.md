# How djs is tested

djs promises three things that are hard to test: it behaves like QuickJS, down to the
text of error messages; it can stop after any instruction and continue in another
process from a copy of its memory; and it gives the same result whichever way its
builtins are compiled. This page explains the test layers that check these promises, why
each exists, and how they come together in the gate every change must pass.

> **In short.** test262 checks conformance, against a checked-in list of known failures
> that may only shrink (the ratchet). The difftest corpus runs codemode scripts in djs
> and in QuickJS and compares what a model would see. Stress modes run both again while
> suspending, restoring and collecting all the time, and with the other builtin backend,
> and require identical results; a single gate runs all of it before anything reaches
> `main`.

## The oracle

djs replaces QuickJS, so QuickJS is the reference: `qjs` from dquickts, a QuickJS with
TypeScript support, at a fixed build. Wherever the specification leaves room (console
formatting, error messages, stack traces, property order), djs does what `qjs` does.
Models read error messages, and pi's tests expect them.

The oracle is read-only for this project: it is never rebuilt or patched here. Where djs
differs on purpose (it is always in strict mode, it has no direct `eval`, a stack
overflow is a `RangeError`), the difference is declared where it is tested, not silently
accepted.

> **Trade-off.** QuickJS as the oracle over the specification alone. Matching an
> existing engine catches what test262 does not check, and makes djs a drop-in
> replacement. The cost: QuickJS's own bugs and quirks become targets too, so each
> deliberate difference needs a decision and a declaration in the tests (`djs-expect`,
> D1540, and the reason next to every ratchet entry, below).

## test262

test262 is the conformance suite of ECMAScript, more than 50,000 tests. djs runs it at
the commit dquickts uses, with its own runner (`djs-test262`, in-process, in parallel).
Every test is assigned to a group in `tests/test262.toml` (D1022):

- **skip**: features djs will never have (sloppy-mode tests, modules, `Intl`, Temporal,
  `SharedArrayBuffer`, direct `eval`, ...). They are left out of the count, so the score
  stays honest.
- **active**: everything else, about 38,000 tests. (A *later* group held features on the
  roadmap; it is empty now.)

djs passes all but a few dozen active tests. Those are listed with a reason in
`tests/test262/expected-failures.txt`, the **ratchet** (D1023): a test that fails and is
not in the file fails the gate, and a listed test that starts passing is reported, so
the entry gets removed and the list can only shrink. Every remaining entry is a
deliberate difference:

- **Always strict.** djs has no sloppy mode, so tests that need it fail: `Function()`
  with duplicate parameter names or parameters named `eval` or `arguments`, sloppy code
  that assigns to `eval` or `arguments`, and indirect `eval` whose `var`s would become
  global properties (in djs, eval code is strict, so its declarations stay local). This
  is most of the list.
- **Annex B accessors.** `__lookupGetter__` and `__lookupSetter__` are not implemented;
  a few class tests use them without declaring the feature.
- **Same as QuickJS.** One test expects an assignment to an undeclared global to throw
  because the reference is resolved before the right-hand side runs; djs resolves it at
  the store, as QuickJS does.

The corpus's ratchet has one case, a dquickts test that needs `SharedArrayBuffer`, which
djs leaves out on purpose.

Every test runs with a fuel budget and a memory limit, and a test that exhausts either
fails as `timeout` or `oom`. The budget is fuel, not wall-clock time (D1790): a loaded
machine slows a test down but cannot turn a pass into a failure.

> **Trade-off.** A ratchet over requiring every test to pass. The suite can be in the
> gate from the first day, while most tests still fail, and every change is checked for
> regressions anywhere. The cost: a stale entry hides a regression of that test, so the
> list must be updated whenever a fix makes tests pass, which the gate's report makes
> visible but does not enforce.

> **Trade-off.** Fuel budgets over wall-clock timeouts. Results do not depend on the
> machine's load, which matters when many agents share one machine. The cost: fuel only
> counts the engine's own work, so a wall-clock guard still exists for hangs outside it.

## Difftest: what the model sees

test262 checks pass or fail. It does not check what `console.log` prints, the text of an
error, or how a script that reads tool results behaves. The difftest corpus
(`tests/corpus/`, a few hundred cases) checks exactly that.

A case is a codemode script: the body of a strict async function, as in pi (D008). Both
engines run it in the same model: the same JavaScript prelude
(`tests/corpus/prelude.js`, D1440) defines `console`, fake `tools` and pi's formatting,
and the result is a *transcript*: every output line, then how the script ended
(`=== return` with the value, `=== completed`, or `=== throw` with the error). QuickJS's
transcript is checked in as `<case>.expected` and the runner compares djs's with it
(D1100).

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
normalized to the case's own (D2032). Other directives in a case's leading comments mark
a TypeScript case, a deliberate difference (`djs-expect`), or a case that cannot run
under the stress modes (`no-stress`, with the measured reason, D1530).

The cases come from three sources: dquickts' own tests, split into one case per test
function (D1102); codemode-style scripts with fake tools; and hand-written cases for
builtins, formatting, errors and every fast path, including the inputs where a fast path
must decline.

> **Trade-off.** Comparing transcripts with QuickJS over writing expected output by
> hand. A new case costs only its source: `--update` records the expected output from
> `qjs`, never from djs. The cost: every case must run on QuickJS, so djs-only features
> and deliberate differences need directives, and a case can only check what shows up in
> the transcript.

## Stress modes: the same answer under any schedule

Suspension is djs's hardest promise. A run may stop after any instruction (out of fuel,
or waiting for a tool), be written to an image and continue in a fresh VM restored from
it. A bug there shows only when a stop falls on the wrong instruction, which ordinary
tests almost never hit. So the tests make stops happen everywhere:

- **Suspension** (`--suspend-every 97`, and 1009): every case runs again in slices of 97
  fuel, and at every stop the VM continues in a new VM restored from its image.
- **GC stress** (`--gc-stress`): a collection at every safepoint, so every place that
  holds a heap value across an allocation is exercised.
- **Determinism** (`--determinism`, D1720, D1725): the suspended runs must leave exactly
  the same heap image as the plain run. Collections happen at the same instructions
  under any slicing, and the copying collector lays the heap out as a function of the
  object graph, so any difference means a stop or a restore changed something.
- **Native and wasm**: `scripts/determinism/native-vs-wasm.sh` runs the same programs in
  a native build and in djs-wasm and compares their images stop for stop.
- **Both builtin backends** (D1635): the corpus runs again with the builtins as bytecode
  instead of generated Rust and without the `@fast` entries ([Builtins](builtins.md)),
  so both implementations of a builtin stay equal.

In every mode the transcript must equal the plain run's: a difference is a failure on
its own, separate from the comparison with QuickJS (D1161). The end-to-end tests of
djs-core (`tests/data/e2e/`) run the same three ways at a finer grain, with one unit of
fuel per slice.

> **Trade-off.** Re-running whole suites under stress over targeted unit tests of
> suspension. Every feature, including those added later, is tested for resumability
> without anyone writing a test for it. The cost: CPU time. The stressed corpus and the
> full test262 run are the largest part of the gate, which is why agents run only the
> cases their change touches while they work (D012).

## More checks against an independent reference

Several parts of djs have a second implementation or an external reference, and are
tested against it:

- **The builtins against QuickJS's natives.** djs-bgen's behavior test runs the
  generated prelude inside `qjs`, next to QuickJS's own builtins, and compares results,
  errors and the order of proxy traps.
- **The wire format and the reader across languages.** Values encoded in TypeScript are
  decoded in Rust, and heap images written by Rust are read in TypeScript, in the gate.
- **The TypeScript type eraser against ts-blank-space**, byte for byte on TypeScript's
  own sources: `djs strip-types` prints a TypeScript file with its types blanked out,
  and must produce exactly what ts-blank-space produces.
- **pi's own codemode tests**, through the pi adapter ([The host interface](host.md)).

## The gate

`scripts/gate.sh` is the one check that guards `main`. In order: formatting, clippy
(also for wasm32), all unit and integration tests, the `no_std` wasm32 build, the
freshness of the generated builtins, `djs-docs check` for this guide, the djs-wasm build
and the host package's tests, native-versus-wasm images, the cross-language wire and
reader checks, the difftest corpus with suspension, GC stress and determinism, the
corpus again with the bytecode backend, and the active test262 group against its
ratchet.

Nothing reaches `main` without passing it. `scripts/merge-to-main.sh` takes a lock,
rebases the branch onto `main`, regenerates the generated files from the merged sources
(D1791), runs the gate on exactly that commit in a dedicated worktree (D1792), and
fast-forwards `main` only if it passes. Merges are serial, so `main` always passes the
gate, and the gate runs at full priority while everything else on the machine runs niced
(D1793).

> **Trade-off.** One serial gate on every merge over checks after the fact. `main` is
> always green, and a regression is caught in the change that caused it, by the agent
> that wrote it. The cost: merges queue behind each other, and the gate's run time
> (minutes, dominated by test262 and the stressed corpus) is paid on every merge; so the
> gate is kept fast enough to run in full every time rather than sampled.

## Where to look

- [`tests/test262.toml`](../../tests/test262.toml),
  [`tests/test262/expected-failures.txt`](../../tests/test262/expected-failures.txt):
  the groups and the ratchet. [`crates/djs-test262/`](../../crates/djs-test262/): the
  runner.
- [`tests/corpus/`](../../tests/corpus/): the difftest cases and the shared prelude.
  [`crates/djs-difftest/`](../../crates/djs-difftest/): the runner.
- [`crates/djs-core/tests/e2e.rs`](../../crates/djs-core/tests/e2e.rs) and
  [`tests/data/e2e/`](../../tests/data/e2e/): the end-to-end tests.
- [`scripts/gate.sh`](../../scripts/gate.sh) and
  [`scripts/merge-to-main.sh`](../../scripts/merge-to-main.sh): the gate and the merge
  flow.

Decisions on this page:

<!-- gen:decisions D008 D012 D1022 D1023 D1100 D1102 D1161 D1440 D1530 D1540 D1635 D1720 D1725 D1790 D1791 D1792 D1793 D2032 -->
- D008: Codemode top level is a function body
- D012: Development flow: per-agent job caps, iteration habits, worktree slots
- D1022: test262 grouping rules
- D1023: Ratchet file semantics
- D1100: Difftest compares what pi's codemode shows the model
- D1102: Importing dquickts' tests
- D1161: Stress variants in difftest and the gate
- D1440: djs-difftest runs the shared prelude (supersedes D1160)
- D1530: difftest `no-stress`; a stressed timeout is a failure again
- D1540: D013 in practice: which overflows are RangeErrors; `djs-expect` in difftest
- D1635: Both backends are tested
- D1720: Determinism checks in the gate: heap images across schedules and builds
- D1725: The determinism checks compare raw images too
- D1790: The gate's runners decide timeouts by fuel, not by the wall clock
- D1791: Generated files are regenerated after a rebase, not merged; line lists union
- D1792: The gate runs in one persistent gate worktree
- D1793: Agents' runs are niced; the gate is not
- D2032: `difftest: stacks` compares stack frames
<!-- /gen -->
