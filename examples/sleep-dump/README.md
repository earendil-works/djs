# sleep-dump: dump a run while a host tool sleeps, continue it somewhere else

A host (node) gives a script one tool, `sleep(ms)`, which waits on the host side. The
script calls it twice, 5000 ms each, from a helper inside a loop. While the first call is
open, the VM has nothing to do: it is **idle**, and the host writes it as a **format 2
dump**, a JavaScript file that recreates the script's suspended state: not only data, but
the operations in the middle of running, each shown with the line of code it stopped at. The host then throws the sandbox away (or exits), loads the
dump into a new sandbox (or a new process), and delivers the sleep's result when it is
due. The script continues after its `await` as if nothing had happened.

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
  const r = await sleep(ms);
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

- `step("first", 5000)`, stopped at its `await sleep(ms)`;
- the script itself, stopped at `await step(...)` halfway through the call
  `results.push(...)`: it has already looked up `results.push` and is waiting for the
  argument;
- the `for (const label of ...)` loop, whose iterator has handed out `"first"` and will
  hand out `"second"` next.

The host calls the script `sleep.js` (`begin`'s `filename`), which the dump's comments use
to point at the code.

## The host

The host drives the run itself with the Sandbox's host-driven run API:

```ts
const sb = await Sandbox.create({ asyncFunctions: { sleep } });
const run = sb.begin(SCRIPT, { ms: 5000 }, { filename: "sleep.js" }); // to the first idle point
run.calls;                                   // [{ id: 2, fn: "sleep", args: [5000] }]
// ... the host starts its timer for call 2 ...
const dump = run.dump();                     // only while idle
sb.dispose();                                // the restart: the VM and the timer are gone

const { sandbox, run: again } = await Sandbox.resume(dump, { asyncFunctions: { sleep } });
again.calls;                                 // the same call, the same id
// ... the host re-arms its timer for call 2; when it fires:
again.settle(2, { ok: true, value: "slept 5000 ms" });
await again.advance();                       // the script prints "after ..."; again.value
```

A host-driven run never calls async host functions itself (`runAsync` does): it lists them
in `run.calls`, and the host answers with `settle`. That is what lets a call outlive the
process that made it.

## What the output looks like

TODO: the transcript and the dump, once the exporter writes it.

## What the dump keeps, and what the host keeps

The dump holds the VM's side, and only that:

- the script's code, verbatim, and `step`'s, as functions the dump defines again;
- each suspended activation, as a `$dump.resume(fn, {...})` call. A comment above it shows
  the function, the site (`await#1`: its first `await` in the text) and the source line
  with a caret under the `await` it stopped at. Its options say what the activation
  holds there: `bindings` (`label`, `ms`; `r` still in its temporal dead zone, `$dump.TDZ`),
  `awaiting` (what it waits on), `operands` (values computed before the `await`: for the
  script, `push` and `results`, the callee and `this` of the half-done call) and `loops`
  (the for-of's iterator);
- the for-of's array iterator, made again and advanced past `"first"` with `next()`, the
  same calls that put it in that state the first time;
- the open host call: `$dump.pending(id, "sleep", [5000])`, with the id the VM gave it, the
  function by the host's name for it, and its arguments. `step("first")` awaits it, and
  the script awaits `step("first")`'s promise;
- the run's result promise, named `run` for the host (`$dump.handle("run", ...)`).

A loader that does not find the same site, bindings, operands and loops in this build's
compiled code refuses the dump with the reason, instead of resuming somewhere else.

The timer is not in the VM: it is host state. A new sandbox that loads the dump knows that
call 2 to `sleep(5000)` is open, nothing more. (The second sleep is not issued yet: the
restarted sandbox makes that call itself, and the host arms a fresh timer for it.) What happens to it is the host's decision:

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
