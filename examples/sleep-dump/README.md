# sleep-dump: suspend a run while a host tool sleeps, continue it somewhere else

A host (node) gives a script one tool, `tools.sleep(ms)`, which waits on the host side. The
script calls it twice, 5000 ms each, from a helper inside a loop. While the first call is
open, the VM has nothing to do: it is **idle**, and the host suspends the run into a
**plain dump**: an ordinary JavaScript Script that recreates the script's suspended
state, not only data but the functions stopped in the middle of running, each written as
the rest of its code from where it waits, and the open call with the host's own state of
it (here: when the sleep is due). The host then throws the sandbox away (or exits), and
resumes the dump in a new sandbox (or a new process), which sleeps only what is left of
the first five seconds. The script continues after its `await` as if nothing had
happened. Since the dump is plain JavaScript, node can continue it too, given the same
host functions.

```sh
examples/sleep-dump/run.sh              # one process: suspend, drop the sandbox, resume
examples/sleep-dump/run.sh --restart    # two processes: the first writes out/sleep.dump.js and exits
examples/sleep-dump/run.sh --node       # the same, the second process continuing it in node, no djs
examples/sleep-dump/run.sh 500          # a shorter sleep
```

`run.sh` builds djs-wasm with the dumper (cargo feature `dump`, which the shipped module
leaves out: it is needed to write a dump, not to load one) and runs [sleep-dump.ts](sleep-dump.ts) with node (22.6 or later: it runs
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

Loading a plain dump is running it as a Script with one extra global, `$djs`, that the
sandbox defines for the load (the dump runtime's helpers and the host's hooks): the dump
calls `$djs.seed(lo, hi, next)` first (the state of `Math.random` and the id of the VM's
next host call) and `$djs.resume(id, name, args, state)` for each open call, which the
sandbox answers by reopening call `id` under the same id and handing it to the function's
`resume` hook. The dump deletes `$djs` at its end.

A host function without a `resume` hook (`{ call }`) is called again after a load, which
suits calls without side effects; a plain function, with no hooks at all, has its open
call rejected, since the sandbox cannot know whether calling it again is safe. A host that
wants to decide each call itself drives the run with the lower-level API instead
(`begin`, `settle`, `advance`, `Run.dump(states)`, `sandbox.loadRun(dump)`; see
docs/host.md).

## What the output looks like

In one process (`run.sh`); `[vm]` lines are the script's `console.log`, and the `|` lines
are the dump, which the host prints as it writes it (explained [below](#the-dump)):

```
  [vm] first sleeps 5000 ms
host: suspended the run into examples/sleep-dump/out/sleep.dump.js (2285 bytes):
  | "djs-dump";
  | "use strict";
  | $djs.seed(1025895862, 1085256491, 3);
  | globalThis.$dump = {};
  | $dump.b0 = Object.getPrototypeOf([][Symbol.iterator]())["next"];
  | $dump.p1 = $djs.resume(2, "tools.sleep", [5000], {"due":1793823976747});
  | $dump.n2 = async function step(label, ms) {
  |   console.log(label, "sleeps", ms, "ms");
  |   const r = await tools.sleep(ms);
  |   return label + ": " + r;
  | };
  | $dump.Ka3 = async function () {
  | var label = "first";
  | var ms = 5000;
  | const r = (await $dump.p1);
  | return label + ": " + r;
  | };
  | Object.defineProperty($dump.Ka3, "$djs", { value: { f: $dump.n2, segs: [62,83,1,84,112,24], scaf: [["site",87,67]] } });
  | $dump.a3 = $dump.Ka3.call(undefined);
  | $dump.n4 = [];
  | $dump.i5 = $djs.advance(Array.prototype.values.call(["first", "second"]), 1);
  | $dump.a6_0 = Array.prototype.push;
  | $dump.Ka6 = async function () {
  | const $djs_R = $djs.R, $djs_L = $djs.L;
  | var ms = 5000;
  | var step = $dump.n2;
  | const t0 = 1793823971739;
  | const results = $dump.n4;
  | { let $djs_f1 = true; for (const $djs_v1 of $djs_L($dump.i5, $dump.b0)) { if ($djs_f1) { $djs_f1 = false; {
  | const label = "first";
  | {
  | 
  | const $djs_t0 = $dump.a6_0, $djs_t1 = $dump.n4;
  | $djs_R($djs_t0, $djs_t1, [(await $dump.a3)]);
  | }
  | } } else { const label = $djs_v1; {
  |   results.push(await step(label, ms));
  | } } } }
  | console.log("done after", Date.now() - t0 >= 2 * ms ? "at least" : "less than", 2 * ms, "ms:", results.join(", "));
  | return results;
  | };
  | $dump.Fa6 = async function (ms) {
  | 
  | async function step(label, ms) {
  |   console.log(label, "sleeps", ms, "ms");
  |   const r = await tools.sleep(ms);
  |   return label + ": " + r;
  | }
  | const t0 = Date.now();
  | const results = [];
  | for (const label of ["first", "second"]) {
  |   results.push(await step(label, ms));
  | }
  | console.log("done after", Date.now() - t0 >= 2 * ms ? "at least" : "less than", 2 * ms, "ms:", results.join(", "));
  | return results;
  | 
  | };
  | Object.defineProperty($dump.Ka6, "$djs", { value: { f: $dump.Fa6, segs: [389,210,11,412,246,42,461,289,115,577,405,15], scaf: [["for-of",205,170],["k",0,330],["k",0,337],["k",0,346],["site",263,357],["tmp",0,356,"call.arg@263","array.element"],["k",0,355]], cmap: [["call.callee@250",0,"$djs_t0"],["call.this@250",0,"$djs_t1"]] } });
  | $dump.a6 = $dump.Ka6.call(undefined);
  | try { ($dump.a6); } finally { delete globalThis.$dump; delete globalThis.$djs; }
  [vm] second sleeps 5000 ms
  [vm] done after at least 10000 ms: first: slept 5000 ms, second: slept 5000 ms
host: the run returned [ 'first: slept 5000 ms', 'second: slept 5000 ms' ]
```

The first sandbox printed only `first sleeps`; everything after `suspended` comes from the
second one, which never ran the start of the script. It takes ten seconds in all: the
resumed first sleep waits only for the rest of its deadline.

With `--restart` the halves are two node processes, and the dump travels as a file; with
`--node` the second process is node alone, which runs the dump as a Script with the
host's functions as plain globals (`resumeInNode` in [sleep-dump.ts](sleep-dump.ts): a
`$djs` with the dump runtime's helpers and a `resume` that calls the tool's `resume` hook,
and a `tools.sleep`):

```
--- node sleep-dump.ts start (process 1)
  [vm] first sleeps 5000 ms
host: suspended the run into examples/sleep-dump/out/sleep.dump.js (2285 bytes):
  | "djs-dump";
  | ... (the same dump as above)
--- node sleep-dump.ts node (process 2, no djs)
second sleeps 5000 ms
done after at least 10000 ms: first: slept 5000 ms, second: slept 5000 ms
node: the run returned [ 'first: slept 5000 ms', 'second: slept 5000 ms' ]
```

### The dump

Read it from the bottom. The completion value of the Script, `$dump.a6`, is the run's
result promise, which the host takes as the run. `a6` is what `$dump.Ka6()` returned:
`Ka6` is the **continuation** of the codemode body, the async function the body runs
as, written from its first `await` on. Its first lines declare what the body held there
(`ms`, `step`, `t0`, the `results` array `n4`), then comes the loop it stopped in: the
same `for...of` over the array iterator `i5` (made again and advanced past `"first"`
once, the call that put it in that state the first time), whose first step is a
placeholder (`$djs_f1`), so that the rest of the current iteration runs first; that rest
is the half-done `results.push(await step(label, ms))`, with the callee and `this` it
had already evaluated (`Array.prototype.push`, `results`) and the `await` on `a3`. Later
iterations run the original text. After the loop the rest of the body is copied verbatim.

`a3` is the continuation `Ka3` of `step("first", 5000)`, stopped at its own first
`await`: it declares `label` and `ms` and awaits `p1`, the open host call: call 2 (call
1 was the first `console.log`), `$djs.resume(2, "tools.sleep", [5000], {"due": ...})`, by
the host's name for the function, with its arguments and the state the call recorded,
JSON written as is. `$djs.seed` on the third line gives the sandbox `Math.random`'s
state and the next call id, 3, so the resumed run's calls get the ids the uninterrupted
run gives them.

The rest is plain JavaScript too: `$dump` is an ordinary object holding the values the
dump shares between statements, deleted again at the end. The small helpers the dump
calls (`$djs.advance`; `R` for a call with an evaluated callee and `L` for the loop's
placeholder step, which `Ka6` captures as `$djs_R` and `$djs_L`) are not in the dump:
they are the dump runtime, the same for every dump, which the host defines as members of
`$djs` next to its hooks ([js/src/dump-runtime.js](../../js/src/dump-runtime.js)). `Fa6`
is the body's original text: the continuation's origin, which the body's activation
alone held. Each continuation carries a property `$djs` naming its origin (the original
function, the site, and which stretches of its text came from where), so that a dump of a
resumed run can be written from the original code again instead of from the
continuation.

### What the dump holds, explained

`djs resume --explain` runs a dump to its first stop and describes it, answering nothing:
the open calls with their states, and each suspended activation with its source line
(positions are in the dump file) and its bindings, without the continuations' own
scaffolding (`$djs_*`):

```
$ cargo run -q -p djs-cli -- resume --explain examples/sleep-dump/out/sleep.dump.js
examples/sleep-dump/out/sleep.dump.js: format plain; 1 open call, 2 suspended activations; handles: run
#2 tools.sleep(5000), state {"due":1793823976747}
  async function <anonymous> at examples/sleep-dump/out/sleep.dump.js:15:12
      const r = (await $dump.p1);
                 ^
    label = 'first', ms = 5000, r = undefined
  async function <anonymous> at examples/sleep-dump/out/sleep.dump.js:34:28 (run)
      $djs_R($djs_t0, $djs_t1, [(await $dump.a3)]);
                                 ^
    ms = 5000, step = [AsyncFunction: step], t0 = 1793823971739, results = [], label = 'first'
```

## What the dump keeps, and what the host keeps

The dump holds the VM's side, and the host's state of each open call:

- the script's code: `step` verbatim, and each suspended function as its continuation,
  the rest of its text from the `await` it waits at, with the values it held there;
- the for-of's array iterator, made again and advanced past `"first"`;
- the open host call, `$djs.resume(2, "tools.sleep", [5000], {"due": ...})`: the id the VM
  gave it, the function by the host's name for it, its arguments, and the state the host
  gave it;
- `Math.random`'s state and the next call id, `$djs.seed(...)`;
- the run's result promise, as the Script's completion value.

It holds no reference to djs's compiled code: no sites, slot names or loader calls. Any
later djs runs it, and so does any engine that is given the same host functions.

The timer itself is not in the dump: it is the host's, and the `resume` hook starts a new
one from the saved deadline. The VM's clock is the host's too: `Date.now()` in the script
reads the time the host sets before every slice, so `Date.now() - t0` after the restart
counts the real time across it, including the time the run spent as a file.

A dump is code: load only dumps you wrote (or trust), into a fresh sandbox.
