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

In one process (`run.sh`); `[vm]` lines are the script's `console.log`:

```
host: starting sleep.js with ms = 1000
  [vm] first sleeps 1000 ms
host: snapshotted the run into examples/sleep-snapshot/out/sleep.snapshot (2882 bytes) and dropped the sandbox
host: resuming examples/sleep-snapshot/out/sleep.snapshot in a new sandbox
  [vm] second sleeps 1000 ms
  [vm] done after at least 2000 ms: first: slept 1000 ms, second: slept 1000 ms
host: the run returned [ 'first: slept 1000 ms', 'second: slept 1000 ms' ]
```

The first sandbox printed only `first sleeps`; the `[vm]` lines after `resuming` come from
the second one, which never ran the start of the script. It takes two seconds in all: the
loaded first sleep waits only for the rest of its deadline. With `--restart` the halves
are two node processes, and the snapshot travels as a file.

`djs describe` (the command line) says what the snapshot holds, here after `run.sh
--restart`'s first half (`node examples/sleep-snapshot/sleep-snapshot.ts start`), which
leaves the file:

```
$ djs describe examples/sleep-snapshot/out/sleep.snapshot
sleep.js, waiting for the host to answer 1 call (2.8 KB, 21 records)

tools.sleep(1000) (open call 2): the run waits for the host's answer
  · open call 2 is record #6 (HostCall), its promise #8
  step(label = "first", ms = 1000) is waiting for tools.sleep(1000), at `await
    tools.sleep(ms)` (line 4)
       4 |   const r = await tools.sleep(ms);
         |             ^
    r = undefined
    · activation #10 (Object.Generator, awaiting) continues at pc 46; its frame #11
    · its frame holds callee, this, 2 arguments, 2 locals, nothing on the operand stack
    · its promise #12
  the body is waiting for step, at `await step(label, ms)` (line 10)
      10 |   results.push(await step(label, ms));
         |                ^
    ms = 1000, step = function step, t0 = 1795872865118, results = [], label = "first"
    inside a for…of loop over ["first", "second"], at item 1 of 2, halfway through
      results.push(…)
    · activation #15 (Object.Generator, awaiting) continues at pc 90; its frame #16
    · its frame holds callee, this, 1 argument, 9 locals, operand stack #19,
        %ArrayIteratorPrototype%.next, Array.prototype.push, #18
    · its promise #3
  the run's result is a promise that is still pending
    · promise #3 (Object.Promise), pending

on the global object: console, tools
the host's own data of the run: {"states":[[2,{"due":1795872866121}]],"fuelUsed":110}
named, not stored (a loading djs must have them):
  the code of step and the body: the same bytecode, since they are stopped halfway
    (checked by hash)
  builtins every djs has: Object.prototype, Promise.prototype, %AsyncFunctionPrototype%,
    %ArrayIteratorPrototype%.next, Array.prototype.push, Function.prototype,
    Array.prototype, %ArrayIteratorPrototype%
  from the host: console, tools, tools.sleep()
  · step is sleep.js 1..139, bytecode hash e74cc8f7dd6ffe63
  · the body is sleep.js 0..399, bytecode hash 9437879384c355d6

2.8 KB: 1.8 KB name the 11 kinds of data and their fields, so a later djs can read it;
  406 bytes is the script's text, compiled again when the snapshot loads; 598 bytes is
  the run's own data: 21 records and 16 strings; 53 bytes name what it refers to without
  storing it: builtins, host values, functions.

how djs stores it: 21 records, each a kind with named fields
  #0 Patch.Global (17 B) the global object: props {console = console, tools = tools}
  #1 Patch.GlobalLex (6 B) the global scope's let and const: (empty)
  #2 Patch.SymbolRegistry (6 B) the Symbol.for registry: (empty)
  #3 Object.Promise (10 B) the run's result: state pending
  #4 ValueArray (2 B) rejections of the VM: (empty)
  #5 ValueArray (4 B) hostCalls of the VM: VALUES [#6]
  #6 HostCall (10 B) open call 2: ID 2, FN tools.sleep, ARGS #7, THIS tools, PROMISE #8, FLAGS 1
  #7 ValueArray (5 B) ARGS of #6: VALUES [1000]
  #8 Object.Promise (11 B) the promise of tools.sleep(1000): state pending, reactions #9
  #9 ValueArray (7 B) reactions of #8: VALUES [undefined, #10, #10]
  #10 Object.Generator (16 B) the activation of step(label = "first", ms = 1000): state awaiting, fram…
  #11 ValueArray (25 B) the frame of step(label = "first", ms = 1000): VALUES [#13, undefined, "first"…
  #12 Object.Promise (11 B) the promise of step(label = "first", ms = 1000): state pending, reactions …
  #13 Object.Function (8 B) function step: template code of step
  #14 ValueArray (7 B) reactions of #12: VALUES [undefined, #15, #15]
  #15 Object.Generator (17 B) the activation of the body: state awaiting, frame #16, pc 90, hp 3, prom…
  #16 ValueArray (46 B) the frame of the body: VALUES [#17, undefined, 1000, 0, 0, 0, 1, 3, undefined,…
  #17 Object.Function (8 B) the body: template code of the body
  #18 Object.Array (8 B) results: length 0
  #19 Object.ArrayIterator (12 B) an element of #16: target #20, index 1, kind 1
  #20 Object.Array (12 B) target of #19: elements ["first", "second"], length 2
  bytes per section: types 1813, strings 277, builtins 9, intrinsics 1, host 7,
    scripts 406, functions 29, records 249, vm 72
```

## What the snapshot holds

Read the description from the top: it tells the run in JavaScript terms first and puts
how djs stores each part on the indented `·` lines and in the record list at the end.
The run's state is 21 **records**, one per object or internal cell the run can reach
beyond what a fresh sandbox has, each written by its kind's name with its fields by name
(the snapshot lists the kinds and their fields once, which is most of the bytes of so
small a snapshot):

- the **open call** 2 (call 1 was the first `console.log`), `tools.sleep(1000)`, its
  function by the host's name, its arguments, and the promise the script awaits;
- the two suspended activations (`Object.Generator`), `step("first", 1000)` waiting for
  that call and the body waiting for `step`, each with its frame: its arguments, its
  variables, and its operand stack, where the body keeps what it had evaluated of
  `results.push(...)` and the for-of loop's iterator over `["first", "second"]`;
- the functions they stopped in, `step` (`sleep.js 1..139`) and the body (`0..399`), named
  by their place in the script and checked by their bytecode: the loading sandbox compiles
  the script again, finds each function by its place, and refuses the snapshot if its
  bytecode differs, since a frame's registers and its instruction only mean the same in
  the same code;
- the **Patch** records: what the global object, the global lexical scope and the
  `Symbol.for` registry have beyond a fresh sandbox's;
- the builtins the records refer to by their paths (`Array.prototype.push`), never their
  contents.

The snapshot also holds the **script**'s text (the `scripts` section), which the loading
sandbox compiles again. What it does not hold is as important:

- the **host's values**: `console`, the namespace `tools` and the function `tools.sleep`
  are named, and the loading sandbox's own objects of those names take their place, so a
  snapshot never carries a host's data (pi's tool list, say) and loads only where the host
  gives those names again;
- the **timer**: the host's, which the second host starts again from the saved
  deadline. The host's data in the snapshot ("the host's own data of the run") is the open call's state and the
  fuel the run used so far, which the loaded run goes on counting from.

The VM's clock is the host's too: `Date.now()` in the script reads the time the host sets
before every slice, so `Date.now() - t0` after the restart counts the real time across it,
including the time the run spent as a file.

A snapshot loads only into the build of djs that can compile its waiting functions to the
same bytecode: a later djs reads it as long as those did not change, and refuses it with
the reason when they did. Load only snapshots you wrote (or trust).
