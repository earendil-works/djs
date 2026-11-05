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
// Plain dumps need frozen builtins (pi's sessions run with them).
const options = { frozenBuiltins: true, asyncFunctions: { "tools.sleep": sleep } };

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
`resume` hook. Once the Script has run, the sandbox deletes `$djs` and the dump's `$dump`.

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
host: suspended the run into examples/sleep-dump/out/sleep.dump.js (750 bytes):
  | "djs-dump";
  | "use strict";
  | $djs.seed(-1705729789, -1615204441, 3);
  | $djs.k(async function (ms, step, t0, results, $djs_i1) {
  | results.push(await $djs.k(async function (label) {
  | const r = await $djs.resume(2, "tools.sleep", [5000], {"due":1793856436710});
  | return label + ": " + r;
  | }, step)("first"));
  | for (const label of $djs_i1) {
  |   results.push(await step(label, ms));
  | }
  | console.log("done after", Date.now() - t0 >= 2 * ms ? "at least" : "less than", 2 * ms, "ms:", results.join(", "));
  | return results;
  | }, $djs.source("sleep.js", "b48bd687f864a8d6", "ms"), 1, 118)(5000, async function step(label, ms) {
  |   console.log(label, "sleeps", ms, "ms");
  |   const r = await tools.sleep(ms);
  |   return label + ": " + r;
  | }, 1793856431700, [], ["second"].values());
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
host: suspended the run into examples/sleep-dump/out/sleep.dump.js (750 bytes):
  | "djs-dump";
  | ... (the same dump as above)
--- node sleep-dump.ts node (process 2, no djs)
second sleeps 5000 ms
done after at least 10000 ms: first: slept 5000 ms, second: slept 5000 ms
node: the run returned [ 'first: slept 5000 ms', 'second: slept 5000 ms' ]
```

### The dump

Read it from the bottom. The Script's last statement, whose value is the run's result
promise (which the host takes as the run), calls the **continuation** of the codemode
body: an async function of its own, written from the body's first `await` on, given to
`$djs.k` and called at once. Its parameters are what the body held there (`ms`, `step`,
`t0`, the `results` array and the loop's iterator), passed as the call's arguments: values
the dump uses once are written where they are used (the function `step`, `[]`,
`["second"].values()`, the array iterator made again over what was left of its array,
which nothing else holds). Its first
statement is the rest of the loop iteration it stopped in: the half-done
`results.push(await step(label, ms))`, whose callee and `this` (`results.push`) it reads
again, which gives what the run had read (the builtins are frozen); `label` is not
declared, since nothing in the rest of the iteration reads it any more. What it awaits is
the continuation of `step("first", 5000)`, written in place: it runs as the outer one's
first statement does, at the same moment of the load as on a line of its own. It takes
`label` (`ms` is dead there) and awaits the open host call: call 2 (call 1 was the first
`console.log`), `$djs.resume(2, "tools.sleep", [5000], {"due": ...})`, by the host's name
for the function, with its arguments and the state the call recorded, JSON written as is.
Then the loop goes on as written, over the iterator itself, and the rest of the body
follows verbatim. `$djs.seed` on the third line gives the sandbox `Math.random`'s state and
the next call id, 3, so the resumed run's calls get the ids the uninterrupted run gives
them.

The rest is plain JavaScript too: a dump that uses a value more than once keeps it on
`$dump`, an ordinary object the host deletes once the Script has run (this one needs
none: the inner continuation names `step` by the outer one's parameter). The small helpers the dump calls (here only `$djs.k`) are not in the dump: they are
the dump runtime, the same for every dump, which the host defines as members of `$djs`
next to its hooks ([js/src/dump-runtime.js](../../js/src/dump-runtime.js)). The arguments
of `$djs.k` after a continuation are its origin record: the original function (`step`
for the inner one; for the body, the script by its name and hash,
`$djs.source("sleep.js", …)`, which the host gives back from the script it ran), the await
it stands for (`1`, the first), the decisions the writer took on the run's values that
shaped the continuation (whether it reads `results.push` or the loop's `next` again
instead of keeping what the run had looked up, …), as bits (`118`), and where its text
holds values of the dump that reading it cannot find (none here: the inner continuation
written in place is read as the expression it is). A dump of a
resumed run writes the continuation again from those, which tells it which stretches of
the continuation's text came from where, so that it can be written from the original code
again instead of from the continuation.

### What the dump holds, explained

`djs resume --explain` runs a dump to its first stop and describes it, answering nothing:
the open calls with their states, and each suspended activation with its source line
(positions are in the dump file) and its bindings, without the continuations' own
scaffolding (`$djs_*`). It reads the script the dump continues from the path the dump
names, so run it beside `sleep.js`, which `sleep-dump.ts start` writes next to the dump:

```
$ cd examples/sleep-dump/out && djs resume --explain sleep.dump.js
sleep.dump.js: 1 open call, 2 suspended activations; handles: run
#2 tools.sleep(5000), state {"due":1793856436710}
  async function <anonymous> at sleep.dump.js:6:11
      const r = await $djs.resume(2, "tools.sleep", [5000], {"due":1793856436710});
                ^
    label = 'first', r = undefined
  async function <anonymous> at sleep.dump.js:5:14 (run)
      results.push(await $djs.k(async function (label) {
                   ^
    ms = 5000, step = [AsyncFunction: step], t0 = 1793856431700, results = [], label = undefined
```

(`label = undefined`: the continuation left the dead binding out.)

## What the dump keeps, and what the host keeps

The dump holds the VM's side, and the host's state of each open call:

- the script's code: `step` verbatim, and each suspended function as its continuation,
  the rest of its text from the `await` it waits at, with the values it held there;
- the for-of's array iterator, made again over what was left of its array, `"second"`;
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
