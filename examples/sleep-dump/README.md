# sleep-dump: suspend a run while a host tool sleeps, continue it somewhere else

A host (node) gives a script one tool, `tools.sleep(ms)`, which waits on the host side. The
script calls it twice, 5000 ms each, from a helper inside a loop. While the first call is
open, the VM has nothing to do: it is **idle**, and the host suspends the run into a
**format 2 dump**, a JavaScript file that recreates the script's suspended state: not
only data, but the operations in the middle of running, and the open call with the
host's own state of it (here: when the sleep is due). The host then throws the sandbox
away (or exits), and resumes the dump in a new sandbox (or a new process), which sleeps
only what is left of the first five seconds. The script continues after its `await` as
if nothing had happened.

```sh
examples/sleep-dump/run.sh              # one process: suspend, drop the sandbox, resume
examples/sleep-dump/run.sh --restart    # two processes: the first writes out/sleep.dump.js and exits
examples/sleep-dump/run.sh 500          # a shorter sleep
```

`run.sh` builds djs-wasm with the dumper (cargo feature `dump`, which the shipped module
leaves out) and runs [sleep-dump.ts](sleep-dump.ts) with node (22.6 or later: it runs
TypeScript directly). `js/test/sleep-dump.test.ts` runs it in one process with a 50 ms
sleep.

## The script

```js
async function step(label, ms) {
  console.log(label, "sleeps", ms, "ms");
  const r = await tools.sleep(ms);
  return label + ": " + r;
}
const t0 = Date.now();
const results = [];
for (const label of ["first", "second"]) {
  results.push(await step(label, ms));
}
console.log("done after", Date.now() - t0 >= 2 * ms ? "at least" : "less than", 2 * ms, "ms:", results.join(", "));
return results;
```

When the host suspends it, during the first sleep, three things are in the middle of
running:

- `step("first", 5000)`, stopped at its `await tools.sleep(ms)`;
- the script itself, stopped at `await step(...)` halfway through the call
  `results.push(...)`: it has already looked up `results.push` and is waiting for the
  argument;
- the `for (const label of ...)` loop, whose iterator has handed out `"first"` and will
  hand out `"second"` next.

## The host

All of it, besides the script and the argument handling:

```ts
const sleep = {
  async call(ms) {
    const due = Date.now() + Number(ms);
    this.state = { due };                    // the call's state: a dump keeps it
    await until(due, this.signal);
    return `slept ${ms} ms`;
  },
  async resume({ due }, call) {              // after a load, with the state it was dumped with
    await until(due, call.signal);
    return `slept ${call.args[0]} ms`;
  },
} satisfies AsyncHostFunction<{ due: number }>;
const options = { asyncFunctions: { "tools.sleep": sleep } };

// Process 1: start the script, suspend it at its first idle point.
const sandbox = await Sandbox.create(options);
const dump = await sandbox.start(SCRIPT, { ms: 5000 }, { filename: "sleep.js" }).suspend();
sandbox.dispose();

// Process 2, any time later: continue it.
const task = await Sandbox.resume(dump, options);
await task.result;                           // ["first: slept 5000 ms", "second: slept 5000 ms"]
```

`until(due, signal)` is a `setTimeout` to the deadline that the signal cancels.

The sandbox does the bookkeeping: `start` runs the script and calls `tools.sleep` when
the script does; `suspend()` waits for the next idle point, asks every open call for its
state (the function's `save` hook, by default the call's `state`), writes the dump and
stops the run, aborting the calls' signals (so process 1 exits at once instead of waiting
for its timer). `Sandbox.resume` loads the dump into a new sandbox and continues each open
call with its function's `resume` hook and the saved state; the run goes on by itself, and
`task.result` is its outcome. Calls the script makes after the load (the second sleep) are
ordinary calls of `call`.

A host function without a `resume` hook (`{ call }`) is called again after a load, which
suits calls without side effects; a plain function, with no hooks at all, has its open
call rejected, since the sandbox cannot know whether calling it again is safe. A host that
wants to decide each call itself drives the run with the lower-level API instead
(`begin`, `settle`, `advance`, `Run.dump(states)`, `sandbox.loadRun(dump)`; see
docs/host.md).

## What the output looks like

In one process (`run.sh`); `[vm]` lines are the script's `console.log`:

```
  [vm] first sleeps 5000 ms
host: suspended the run into examples/sleep-dump/out/sleep.dump.js (1330 bytes)
  [vm] second sleeps 5000 ms
  [vm] done after at least 10000 ms: first: slept 5000 ms, second: slept 5000 ms
host: the run returned [ 'first: slept 5000 ms', 'second: slept 5000 ms' ]
```

The first sandbox printed only `first sleeps`; everything after `suspended` comes from the
second one, which never ran the start of the script. It takes ten seconds in all: the
resumed first sleep waits only for the rest of its deadline.

With `--restart` the halves are two node processes, and the dump travels as a file:

```
--- node sleep-dump.ts start (process 1)
  [vm] first sleeps 5000 ms
host: suspended the run into examples/sleep-dump/out/sleep.dump.js (1330 bytes)
--- node sleep-dump.ts resume (process 2)
  [vm] second sleeps 5000 ms
  [vm] done after at least 10000 ms: first: slept 5000 ms, second: slept 5000 ms
host: the run returned [ 'first: slept 5000 ms', 'second: slept 5000 ms' ]
```

### The dump

`out/sleep.dump.js`, as written (only the deadline and `t0`, clock readings, and the
`Math.random` state, from the sandbox's seed, differ between runs):

```js
"djs-dump 2";
$dump.random(1735690049, -805565979);
$dump.p1 = $dump.pending(2, "tools.sleep", [5000], {"due":1793776092499});
$dump.n2 = async function step(label, ms) {
  console.log(label, "sleeps", ms, "ms");
  const r = await tools.sleep(ms);
  return label + ": " + r;
};
$dump.a3 = $dump.resume($dump.n2, { at: "await#1", awaiting: $dump.p1, args: ["first", 5000], bindings: { label: "first", ms: 5000, r: undefined } });
$dump.n4 = [];
$dump.i5 = $dump.advance(Array.prototype.values.call(["first", "second"]), 1);
$dump.a6 = $dump.resume(async function (ms) {

async function step(label, ms) {
  console.log(label, "sleeps", ms, "ms");
  const r = await tools.sleep(ms);
  return label + ": " + r;
}
const t0 = Date.now();
const results = [];
for (const label of ["first", "second"]) {
  results.push(await step(label, ms));
}
console.log("done after", Date.now() - t0 >= 2 * ms ? "at least" : "less than", 2 * ms, "ms:", results.join(", "));
return results;

}, { at: "await#1", awaiting: $dump.a3, args: [5000], bindings: { ms: 5000, step: $dump.n2, t0: 1793776087488, results: $dump.n4, label: "first" }, operands: [["call.callee", Array.prototype.push], ["call.this", $dump.n4]], loops: [{ kind: "for-of", iterator: $dump.i5, next: $dump.builtin("%ArrayIteratorPrototype%.next") }] });
$dump.handle("run", $dump.a6);
```

Read from the bottom: `$dump.handle("run", $dump.a6)` gives the host the run's result
promise. `a6` is the script's own activation, the codemode body written as the async
function it runs as, verbatim, resumed at its first `await` (`await#1`, the one in
`results.push(await step(label, ms))`). It awaits `a3`, the activation of `step("first",
5000)` at its own first `await`, which awaits `p1`, the open host call: call 2 (calls 1 and
3 are `console.log`s), `tools.sleep(5000)`, with the host's state `{"due": ...}`, the JSON
the call recorded. `n4` is the `results` array, shared by the binding and the half-done
call's `this`; `i5` is the loop's iterator, made again and advanced once. The first line
after the header sets `Math.random`'s state, so the resumed run continues its sequence.
Everything named `$dump.*` that is called is the loader's: `$dump.advance` steps the new
iterator and `$dump.builtin` finds a builtin by its registry path (here the array
iterator's `next`), so a dump defines no helpers of its own and holds only the run's data,
its functions and these calls.

### What the dump holds, explained

`djs resume --explain` loads a dump without running anything and describes it: the open
calls with their states, and each suspended activation with its source line (positions
are in the dump file) and its bindings:

```
$ cargo run -q -p djs-cli -- resume --explain examples/sleep-dump/out/sleep.dump.js
examples/sleep-dump/out/sleep.dump.js: format 2; 1 open call, 2 suspended activations; handles: run
#2 tools.sleep(5000), state {"due":1793776092499}
  async function step at examples/sleep-dump/out/sleep.dump.js:6:13
      const r = await tools.sleep(ms);
                ^
    label = 'first', ms = 5000, r = undefined
  async function <anonymous> at examples/sleep-dump/out/sleep.dump.js:22:16 (run)
      results.push(await step(label, ms));
                   ^
    ms = 5000, step = [AsyncFunction: step], t0 = 1793776087488, results = [], label = 'first'
```

## What the dump keeps, and what the host keeps

The dump holds the VM's side, and the host's state of each open call:

- the script's code, verbatim, as the async function the body runs as, and `step`'s;
- each suspended activation, as a `$dump.resume(fn, {...})` call: the function, the site
  it stopped at (`at: "await#1"`: the function's first `await` in its text), and what the
  activation holds there: `bindings` (`label`, `ms`, and `r`, not assigned yet),
  `awaiting` (what it waits on), `operands` (values computed before the `await`: for the
  script, `push` and `results`, the callee and `this` of the half-done call) and `loops`
  (the for-of's iterator);
- the for-of's array iterator, made again and advanced past `"first"` with one `next()`,
  the same call that put it in that state the first time;
- the open host call: `$dump.pending(2, "tools.sleep", [5000], {"due": ...})`, with the id
  the VM gave it, the function by the host's name for it, its arguments, and the state
  the host gave it, JSON written as is;
- the run's result promise, named `run` for the host (`$dump.handle("run", ...)`).

A loader that does not find the same site, bindings, operands and loops in this build's
compiled code refuses the dump with the reason, instead of resuming somewhere else.

The timer itself is not in the dump: it is the host's, and the `resume` hook starts a new
one from the saved deadline. The VM's clock is the host's too: `Date.now()` in the script
reads the time the host sets before every slice, so `Date.now() - t0` after the restart
counts the real time across it, including the time the run spent as a file.

A dump is code: load only dumps you wrote (or trust), into a fresh sandbox.
