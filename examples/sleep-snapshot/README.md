# sleep-snapshot: snapshot a run while a host tool sleeps, continue it somewhere else

A host (node) gives a script one tool, `tools.sleep(ms)`, which waits on the host side. The
script calls it twice, 1000 ms each, from a helper inside a loop. While the first call is
open, the VM has nothing to do: it is **idle**, and the host writes the run as a
**snapshot**: the run's state as data, the functions stopped in the middle of running
included, with the open call and the host's own state of it (here: when the sleep is due).
The host then throws the sandbox away (or exits), and loads the snapshot into a new
sandbox (or a new process), which settles the sleep when what is left of the first second
has passed. The script continues after its `await` as if nothing had happened.

```sh
examples/sleep-snapshot/run.sh              # one process: snapshot, drop the sandbox, load
examples/sleep-snapshot/run.sh --restart    # two processes: the first writes out/sleep.snapshot and exits
examples/sleep-snapshot/run.sh 300          # a shorter sleep
```

`run.sh` runs [sleep-snapshot.ts](sleep-snapshot.ts) with node (22.6 or later: it runs
TypeScript directly) on the release build of djs-wasm, the module djs ships
(`scripts/build-wasm.sh`: target/wasm32-unknown-unknown/wasm/djs_wasm.wasm). It builds
that module the first time, which takes a minute or two, and again only when a source of
it changed; after that a run takes no noticeable time besides the sleeps.
`js/test/sleep-snapshot.test.ts` runs it with sleeps of 30 to 50 ms.

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

When the host snapshots it, during the first sleep, three things are in the middle of
running:

- `step("first", 1000)`, stopped at its `await tools.sleep(ms)`;
- the script itself, stopped at `await step(...)` halfway through the call
  `results.push(...)`: it has already looked up `results.push` and is waiting for the
  argument;
- the `for (const label of ...)` loop, whose iterator has handed out `"first"` and will
  hand out `"second"` next.

## The host

All of it, besides the script and the argument handling:

```ts
// `tools.sleep` is an async host function the host settles itself: the run lists its calls.
const options = { asyncFunctions: { "tools.sleep": () => {} } };

// Process 1: start the script; at its first idle point the first sleep is open.
const sandbox = await Sandbox.create(options);
const run = sandbox.begin(SCRIPT, { ms: 1000 }, { filename: "sleep.js" });
const [call] = run.calls;                     // { id: 2, fn: "tools.sleep", args: [1000] }
// The host's state of the call, its deadline, goes into the snapshot with it.
const snapshot = run.snapshot({ states: { [call.id]: { due: Date.now() + 1000 } } });
sandbox.dispose();

// Process 2, any time later: load it and settle each sleep at its deadline.
const again = await Sandbox.create(options);
const loaded = again.loadSnapshot(snapshot);
while (loaded.state === "idle" && loaded.calls.length > 0) {
  const [c] = loaded.calls;                    // a loaded call has its state
  await until(c.state?.due ?? Date.now() + c.args[0]);
  loaded.settle(c.id, { ok: true, value: `slept ${c.args[0]} ms` });
  await loaded.advance();                     // runs to the next idle point or the end
}
loaded.value;                                 // ["first: slept 1000 ms", "second: slept 1000 ms"]
```

`until(due)` is a `setTimeout` to the deadline.

The run never calls `tools.sleep` itself: `begin` runs the script to its first idle point
and lists the open call in `run.calls`; the host answers it with `settle` whenever it
likes, and `advance` runs on to the next idle point. While the run is idle, `snapshot()`
writes it, with the host's states of its open calls. `loadSnapshot` continues it in a new
sandbox made with the same options: the run is idle on the same open call, with the same
id and the state the host gave it. Calls the script makes after the load (the second
sleep) are new calls, without a state.

## What the output looks like

In one process (`run.sh`); `[vm]` lines are the script's `console.log`, and the `|` lines
are what the host prints about the snapshot it wrote (`sandbox.describeSnapshot`):

```
host: starting sleep.js with ms = 1000
  [vm] first sleeps 1000 ms
host: snapshotted the run into examples/sleep-snapshot/out/sleep.snapshot (2884 bytes) and dropped the sandbox; it holds:
  | types          1813 bytes
  | strings         291 bytes
  | builtins          9 bytes
  | intrinsics        1 bytes
  | host              7 bytes
  | scripts         406 bytes
  | functions        29 bytes
  | records         249 bytes
  | vm               72 bytes
  | 18 strings (272 bytes of text), 8 builtins, 0 intrinsics, 1 scripts, 2 functions, 21 records
  | builtins: Object.prototype Promise.prototype %AsyncFunctionPrototype% %ArrayIteratorPrototype%.next Array.prototype.push Function.prototype Array.prototype %ArrayIteratorPrototype%
  | flags and constants: Object BigInt Symbol PromiseStates PromiseFlags GeneratorStates GeneratorKinds
  | script 0: sleep.js, parameters ms (399 bytes of source)
  | host values: console tools tools.sleep() (and 0 members)
  | host data: {"states":[[2,{"due":1795813233385}]],"fuelUsed":110}
  | function 0:1..139 (bytecode)
  | function 0:0..399 (bytecode)
  |        1 HostCall
  |        2 Object.Array
  |        1 Object.ArrayIterator
  |        2 Object.Function
  |        2 Object.Generator
  |        3 Object.Promise
  |        1 Patch.Global
  |        1 Patch.GlobalLex
  |        1 Patch.SymbolRegistry
  |        7 ValueArray
host: resuming examples/sleep-snapshot/out/sleep.snapshot in a new sandbox
  [vm] second sleeps 1000 ms
  [vm] done after at least 2000 ms: first: slept 1000 ms, second: slept 1000 ms
host: the run returned [ 'first: slept 1000 ms', 'second: slept 1000 ms' ]
```

The first sandbox printed only `first sleeps`; the `[vm]` lines after `resuming` come from
the second one, which never ran the start of the script. It takes two seconds in all: the
loaded first sleep waits only for the rest of its deadline. With `--restart` the halves
are two node processes, and the snapshot travels as a file.

## What the snapshot holds

Read the description from the middle. The run's state is 21 **records**, one per object
or internal cell the run can reach beyond what a fresh sandbox has, each written by its
kind's name with its fields by name (the `types` section at the top lists the kinds and
their fields once, which is most of the bytes of so small a snapshot):

- two **Generator** records, the suspended activations of `step("first", 1000)` and of the
  script's body, each with its frame (the stopped function's registers and operand stack,
  `results.push` and `results` among them) and the instruction it stopped at;
- the functions they stopped in, `function 0:1..139` (`step`) and `function 0:0..399` (the
  body), named by their place in the script and checked by their bytecode: the loading
  sandbox compiles the script again, finds each function by its place, and refuses the
  snapshot if its bytecode differs, since a frame's registers and its instruction only
  mean the same in the same code;
- the for-of loop's **ArrayIterator** over `["first", "second"]`, at the second element;
- the **HostCall**: the open call 2 (call 1 was the first `console.log`), its function by
  the host's name, `tools.sleep`, its arguments, and the promise the script awaits;
- the **Patch** records: what the global object, the global lexical scope and the
  `Symbol.for` registry have beyond a fresh sandbox's (here nothing);
- the builtins the records refer to by their paths (`Array.prototype.push`), never their
  contents.

The snapshot also holds the **script**'s text (the `scripts` section), which the loading
sandbox compiles again. What it does not hold is as important:

- the **host's values**: `console`, the namespace `tools` and the function `tools.sleep`
  are named, and the loading sandbox's own objects of those names take their place, so a
  snapshot never carries a host's data (pi's tool list, say) and loads only where the host
  gives those names again;
- the **timer**: the host's, which the second host starts again from the saved
  deadline. The host's data in the snapshot (`host data`) is the open call's state and the
  fuel the run used so far, which the loaded run goes on counting from.

The VM's clock is the host's too: `Date.now()` in the script reads the time the host sets
before every slice, so `Date.now() - t0` after the restart counts the real time across it,
including the time the run spent as a file.

A snapshot loads only into the build of djs that can compile its waiting functions to the
same bytecode: a later djs reads it as long as those did not change, and refuses it with
the reason when they did. Load only snapshots you wrote (or trust).
