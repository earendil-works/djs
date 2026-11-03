# sleep-dump: dump a run while a host tool sleeps, continue it somewhere else

A host (node) gives a script one tool, `tools.sleep(ms)`, which waits on the host side. The
script calls it twice, 5000 ms each, from a helper inside a loop. While the first call is
open, the VM has nothing to do: it is **idle**, and the host writes it as a **format 2
dump**, a JavaScript file that recreates the script's suspended state: not only data, but
the operations in the middle of running. The host then throws the sandbox away (or
exits), loads the dump into a new sandbox (or a new process), and delivers the sleep's
result when it is due. The script continues after its `await` as if nothing had happened.

```sh
examples/sleep-dump/run.sh              # one process: dump, drop the sandbox, resume
examples/sleep-dump/run.sh --restart    # two processes: the first writes out/sleep.dump.js and exits
examples/sleep-dump/run.sh --ms 500     # a shorter sleep
```

`run.sh` builds djs-wasm with the dumper (cargo feature `dump`, which the shipped module
leaves out) and runs [sleep-dump.ts](sleep-dump.ts) with node (22.6 or later: it runs
TypeScript directly). `js/test/sleep-dump.test.ts` runs the in-process mode with a 50 ms
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
console.log("done after at least", Date.now() - t0 >= 2 * ms ? 2 * ms : "(less than!)", "ms:", results.join(", "));
return results;
```

When the host dumps, during the first sleep, three things are in the middle of running:

- `step("first", 5000)`, stopped at its `await tools.sleep(ms)`;
- the script itself, stopped at `await step(...)` halfway through the call
  `results.push(...)`: it has already looked up `results.push` and is waiting for the
  argument;
- the `for (const label of ...)` loop, whose iterator has handed out `"first"` and will
  hand out `"second"` next.

## The host

The host drives the run itself with the Sandbox's host-driven run API:

```ts
const sb = await Sandbox.create({ asyncFunctions: { "tools.sleep": sleep } });
const run = sb.begin(SCRIPT, { ms: 5000 }, { filename: "sleep.js" }); // to the first idle point
run.calls;                                   // [{ id: 2, fn: "tools.sleep", args: [5000] }]
// ... the host starts its timer for call 2 ...
const dump = run.dump();                     // only while idle
sb.dispose();                                // the restart: the VM and the timer are gone

const { sandbox, run: again } = await Sandbox.resume(dump, { asyncFunctions: { "tools.sleep": sleep } });
again.calls;                                 // the same call, the same id
// ... the host re-arms its timer for call 2; when it fires:
again.settle(2, { ok: true, value: "slept 5000 ms" });
await again.advance();                       // the script goes on: "second sleeps", call 4 opens
```

A host-driven run never calls async host functions itself (`runAsync` does): it lists them
in `run.calls`, and the host answers with `settle`. That is what lets a call outlive the
process that made it.

## What the output looks like

In one process (`run.sh`); `[vm]` lines are the script's `console.log`:

```
  [vm] first sleeps 5000 ms
host: call #2 tools.sleep(5000): timer armed, 5000 ms left
host: the VM is idle, waiting on tools.sleep(5000); dumping it
host: the dump (1308 bytes):
  | ... (the dump below)
host: loaded the dump into a new sandbox; open calls: #2 tools.sleep(5000)
host: call #2 tools.sleep(5000): timer armed, 4958 ms left
  [vm] second sleeps 5000 ms
host: call #4 tools.sleep(5000): timer armed, 5000 ms left
  [vm] done after at least 10000 ms: first: slept 5000 ms, second: slept 5000 ms
host: the run returned ["first: slept 5000 ms","second: slept 5000 ms"]
```

The first sandbox printed only `first sleeps`; everything after `loaded the dump` comes
from the second one, which never ran the start of the script. Calls #1 and #3 are the
`console.log`s (sync host calls), so the second sleep is call #4.

With `--restart` the halves are two node processes, and the dump travels as a file:

```
--- node sleep-dump.ts start (process 1)
  [vm] first sleeps 5000 ms
host: call #2 tools.sleep(5000): timer armed, 5000 ms left
host: the VM is idle, waiting on tools.sleep(5000); dumping it
host: wrote examples/sleep-dump/out/sleep.dump.js (1308 bytes) and host.json; exiting
--- node sleep-dump.ts resume (process 2)
host: loaded the dump into a new sandbox; open calls: #2 tools.sleep(5000)
host: call #2 tools.sleep(5000): timer armed, 4792 ms left
  [vm] second sleeps 5000 ms
host: call #4 tools.sleep(5000): timer armed, 5000 ms left
  [vm] done after at least 10000 ms: first: slept 5000 ms, second: slept 5000 ms
host: the run returned ["first: slept 5000 ms","second: slept 5000 ms"]
```

### The dump

`out/sleep.dump.js`, as written (only `t0`, a clock reading, and the `Math.random` state,
from the sandbox's seed, differ between runs):

```js
"djs-dump 2";
$dump.random(-1641497997, 2047772101);
$dump.p1 = $dump.pending(2, "tools.sleep", [5000]);
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
console.log("done after at least", Date.now() - t0 >= 2 * ms ? 2 * ms : "(less than!)", "ms:", results.join(", "));
return results;

}, { at: "await#1", awaiting: $dump.a3, args: [5000], bindings: { ms: 5000, step: $dump.n2, t0: 1793745323813, results: $dump.n4, label: "first" }, operands: [["call.callee", Array.prototype.push], ["call.this", $dump.n4]], loops: [{ kind: "for-of", iterator: $dump.i5, next: $dump.builtin("%ArrayIteratorPrototype%.next") }] });
$dump.handle("run", $dump.a6);
```

Read from the bottom: `$dump.handle("run", $dump.a6)` gives the host the run's result
promise. `a6` is the script's own activation, the codemode body written as the async
function it runs as, verbatim, resumed at its first `await` (`await#1`, the one in
`results.push(await step(label, ms))`). It awaits `a3`, the activation of `step("first",
5000)` at its own first `await`, which awaits `p1`, the open host call. `n4` is the
`results` array, shared by the binding and the half-done call's `this`; `i5` is the
loop's iterator, made again and advanced once. The first line after the header sets
`Math.random`'s state, so the resumed run continues its sequence. Everything named
`$dump.*` that is called is the loader's: `$dump.advance` steps the new iterator and
`$dump.builtin` finds a builtin by its registry path (here the array iterator's `next`),
so a dump defines no helpers of its own and holds only the run's data, its functions and
these calls.

### What the dump holds, explained

`djs resume --explain` loads a dump without running anything and describes it: the open
calls, and each suspended activation with its source line (positions are in the dump
file) and its bindings:

```
$ cargo run -q -p djs-cli -- resume --explain examples/sleep-dump/out/sleep.dump.js
examples/sleep-dump/out/sleep.dump.js: format 2; 1 open call, 2 suspended activations; handles: run
#2 tools.sleep(5000)
  async function step at examples/sleep-dump/out/sleep.dump.js:6:13
      const r = await tools.sleep(ms);
                ^
    label = 'first', ms = 5000, r = undefined
  async function <anonymous> at examples/sleep-dump/out/sleep.dump.js:22:16 (run)
      results.push(await step(label, ms));
                   ^
    ms = 5000, step = [AsyncFunction: step], t0 = 1793738494362, results = [], label = 'first'
```

## What the dump keeps, and what the host keeps

The dump holds the VM's side, and only that:

- the script's code, verbatim, as the async function the body runs as, and `step`'s;
- each suspended activation, as a `$dump.resume(fn, {...})` call: the function, the site
  it stopped at (`at: "await#1"`: the function's first `await` in its text), and what the
  activation holds there: `bindings` (`label`, `ms`, and `r`, not assigned yet),
  `awaiting` (what it waits on), `operands` (values computed before the `await`: for the
  script, `push` and `results`, the callee and `this` of the half-done call) and `loops`
  (the for-of's iterator);
- the for-of's array iterator, made again and advanced past `"first"` with one `next()`,
  the same call that put it in that state the first time;
- the open host call: `$dump.pending(2, "tools.sleep", [5000])`, with the id the VM gave
  it, the function by the host's name for it, and its arguments. `step("first")` awaits it, and
  the script awaits `step("first")`'s promise;
- the run's result promise, named `run` for the host (`$dump.handle("run", ...)`).

A loader that does not find the same site, bindings, operands and loops in this build's
compiled code refuses the dump with the reason, instead of resuming somewhere else.

The timer is not in the VM: it is host state. A new sandbox that loads the dump knows that
call 2 to `tools.sleep(5000)` is open, nothing more. (The second sleep is not issued yet: the
restarted sandbox makes that call itself, as #4, and the host arms a fresh timer for it.)
What happens to the open call is the host's decision:

- **re-arm it** (this example): the host keeps when each call is due (`out/host.json` in
  the two-process mode, a plain object in one process) and re-arms the timer for the time
  left, from the open-call list `Sandbox.resume` returns (`run.calls`: the calls the
  loader recreated, which `Vm::load_dump` reports as `Loaded.calls`, with the function
  named by the host and the arguments read back from the VM). Without that record it
  could start a fresh 5 s from the arguments;
- **deliver a recorded result**, if the call finished while the VM was gone;
- **issue it again**, for a call without side effects;
- **reject it** (`settle(id, { ok: false, value: new Error("interrupted") })`): the
  script sees the `await` throw.

The VM's clock is the host's too: `Date.now()` in the script reads the time the host sets
before every slice, so `Date.now() - t0` after the restart counts the real time across
it, including the time the run spent as a file.

A dump is code: load only dumps you wrote (or trust), into a fresh sandbox.
