# sleep-dump: dump a run while a host tool sleeps, continue it somewhere else

A host (node) gives a script one tool, `sleep(ms)`, which waits on the host side. The
script calls it with 5000. While that call is open, the VM has nothing to do: it is
**idle**, and the host writes it as a **format 2 dump**, a JavaScript file that recreates
the script's suspended state. The host then throws the sandbox away (or exits), loads the
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
console.log("before");
const t0 = Date.now();
const r = await sleep(ms);
console.log("after", r, "(the script waited", Date.now() - t0 >= ms ? "at least" : "less than", ms, "ms)");
return r;
```

## The host

The host drives the run itself with the Sandbox's host-driven run API:

```ts
const sb = await Sandbox.create({ asyncFunctions: { sleep } });
const run = sb.begin(SCRIPT, { ms: 5000 });  // runs to the first idle point
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

- the script's code (the body as compiled, verbatim) and its suspended activation: where
  it stopped (`await#1`, the first `await` in the text), its bindings (`ms`, `t0`; `r` is
  still in its temporal dead zone) and what it awaits;
- the open host call: `$dump.pending(id, "sleep", [5000])`, with the id the VM gave it, the
  function by the host's name for it, and its arguments;
- the run's result promise, named `run` for the host (`$dump.handle("run", ...)`).

The timer is not in the VM: it is host state. A new sandbox that loads the dump knows that
call 2 to `sleep(5000)` is open, nothing more. What happens to it is the host's decision:

- **re-arm it** (this example): the host keeps when each call is due (`out/host.json` in
  the two-process mode, a plain object in one process) and re-arms the timer for the time
  left, from the open-call list `Sandbox.resume` returns (`run.calls`). Without that record
  it could start a fresh 5 s from the arguments;
- **deliver a recorded result**, if the call finished while the VM was gone;
- **issue it again**, for a call without side effects;
- **reject it** (`settle(id, { ok: false, value: new Error("interrupted") })`): the
  script sees the `await` throw.

The VM's clock is the host's too: `Date.now()` in the script reads the time the host sets
before every slice, so `Date.now() - t0` after the restart counts the real time across
it, including the time the run spent as a file.

A dump is code: load only dumps you wrote (or trust), into a fresh sandbox.
