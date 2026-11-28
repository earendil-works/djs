# Determinism

A codemode run can be stopped, saved, moved and continued, and sometimes the host has to
run the same script again: when a newer build of djs refuses the snapshot of a waiting
run, to debug what a model's script did, or to check one engine build against another.
All of that works only if a run is a function of its inputs, which is why
[determinism](goals.md#determinism) is a requirement under every goal of djs. This page
explains which inputs those are, how djs keeps everything else out of a run, how a host
runs a script again from its recorded results and checks that the two runs agree, and
where determinism ends.

> **In short.** The VM reads nothing from its environment: the clock, the time zone, the
> random seed and the behavior of weak references are inputs the host sets, and results
> of host calls take effect one at a time in the order they were settled, never when they
> arrived. Given the same inputs in the same order, a run does the same thing and leaves
> byte-identical heap images, however it was sliced, restored from images or compiled. A
> deterministic run begun again from its script with the results it had comes back to
> where it was, which is the host's way back when a newer build refuses a snapshot.

## Why it matters

A run can stop between any two instructions and continue, from an image of its memory or
from a [snapshot](persistence.md), and the host hands it inputs and reads results
without the VM doing any I/O ([the JavaScript interface](js-api.md)). Determinism is
what makes that useful:

- **Durability.** A snapshot loads only into a build of djs that compiles the run's
  waiting functions the same way. When a newer build refuses one, the host begins the
  run again from its script and gives it the results its calls had, in the same order: it
  makes the same calls and comes back to the state it was in, in the new build
  ([below](#when-a-snapshot-is-refused)).
- **Debugging and reproduction.** A model's script that misbehaved can be run again with
  the recorded tool results, and it does exactly what it did.
- **Testing.** The suspension tests compare a run in one go with the same run stopped
  and restored every 97 instructions. If anything could differ for other reasons, those
  tests could not demand identical results, and suspension bugs would hide in the noise
  ([How djs is tested](testing.md)).

## Every source of nondeterminism is a host input

An engine meets the outside world in a few places: the clock, the local time zone,
`Math.random`, and the garbage collector, whose timing weak references expose. In djs
each of them is a value the host sets, stored in the heap like any other state, so an
image carries it along:

| Source | In djs |
| --- | --- |
| `Date.now()`, `new Date()` | Reads a time the host sets with `vm_set_now` (0 until then). A host that wants time to pass sets it again between fuel slices, or with each async result. |
| The local time zone | A fixed offset the host sets (`vm_set_time_zone_offset`), UTC by default, no daylight-saving rules. |
| `Math.random()` | xorshift64*, as in QuickJS, seeded by the host (`vm_set_random_seed`). The state is in the heap, so a restored VM continues the sequence. |
| The string hash's key | Derived from the same seed (one-way), and kept in the image. It decides where the VM's own atoms and Map and Set keys sit in their tables, never what a script sees: iteration follows insertion order. |
| `WeakRef`, `FinalizationRegistry` | By default they work as specified, and collection timing is observable. Deterministic mode makes them strong (nothing ever dies). |
| Tool results | Applied one at a time in the order they were settled, as macrotasks (below). |

The examples in this guide run with the clock at 0 and the default seed, so their output
is fixed and the gate can check it:

```js
console.log(Date.now(), new Date().toISOString());
console.log(Math.random(), Math.random());
```
<!-- gen:run -->
```text
0 1970-01-01T00:00:00.000Z
0.05279087335850807 0.3311202810018534
```
<!-- /gen -->

## Results in the order they were settled

Async tool calls are where timing would leak in. A result can reach the host at any
moment, and if the VM applied it at once, the order in which promise callbacks run would
depend on the fuel slice that happened to be running. So results are macrotasks:
they wait in an inbox in the heap, and the VM applies the oldest one only when its job
queue is empty. A script's behavior depends on the order of the results, never on when
they arrived ([the wasm boundary](host.md) has the details). It does not depend on how
the host grouped them either: two results settled before one step are applied as if
each had come alone, the first run to its end before the second, so a run given the
same results one per step reaches the same states.

A sync host function cannot leak timing at all: it parks the VM at a fixed instruction,
and the script continues only with the value the host returns.

## The same program, the same heap

Determinism goes further than the script's output. Two runs with the same inputs leave
the same *heap*, byte for byte. That is a stronger property, and the design of several
parts of the engine exists to keep it:

- **Fuel** stops a run at the same instruction however the budget was sliced, because an
  overshoot is carried to the next run as a debt ([Suspending
  anywhere](suspension.md)). The work builtins charge is taken at safepoints, and a run
  stops only where none is owed, so where that happens does not depend on the slicing
  either; collections charge nothing ([fuel](limits.md#fuel)).
- **Collections** happen at the same instructions under any slicing and restore. The
  threshold lives in the heap, and nothing about a stop allocates or collects
  ([The garbage collector](gc.md)). So does the choice between a minor and a full
  collection: where the old region ends and how many minor collections ran since the last
  full one are in the heap too.
- **The collector's layout** is a function of what the program did. A collection slides
  the live cells down in their allocation order, so the same allocations give the same
  bytes, whenever the collections came. A minor collection keeps dead cells of the
  old region until the next full one, but a full collection drops them, and its result
  doesn't depend on whether minor ones ran before it. For a layout that depends on the
  object graph alone, the canonical hash collects a copy breadth-first. The copy
  also ends up with the whole heap as its old region, so the hash does not depend on
  where a VM's old region ended.
- **Builtins leave no trace.** Calling a builtin never writes into the builtin prefix,
  the part of the heap every VM of a build starts with: no cached string hash,
  no flag set on first use, no shape transition added to a builtin's shape. Two runs that
  call different builtins, in any order, leave the same prefix. Nor do they leave
  anything in the session's part of the heap. The shapes of the objects builtins create
  (iterator results, RegExp match results, property descriptors, functions and their
  prototypes, errors) are already in each VM's shape tree when it starts, and so are the
  engine's queues for promise jobs, host results and unhandled rejections. Two
  runs that use Map iteration and RegExp matching in opposite orders, and keep nothing,
  leave the same image. What stays the script's own is what its source and data name:
  its identifiers, its object literals' keys, results keyed by data such as
  `Object.groupBy`, and a property it adds to an object a builtin made.
- **Nothing depends on addresses or timing.** Map and Set rehash after every collection
  instead of hashing by a per-object id that would vary, and shape transitions are
  strong so that the shape tree does not depend on when collections ran ([Cells, strings and objects](heap.md)).
- **Out of memory** is a function of the program and the cap: running out of the heap
  cap is a JavaScript error, and the host's own refusal to provide memory is kept
  separate ([One arena](arena.md)).

Byte-identical images make the strongest possible check: if two runs left the same
raw image, they did the same allocations and built the same graph, with no need to
compare outputs.

> **Trade-off.** Byte-identical heaps over equal outputs only. Equal images make
> suspension bugs visible even when they do not change the output yet, and make "did
> these two runs agree?" one hash comparison. The cost: every design choice in the heap
> has to keep the layout independent of timing, which rules out some common techniques
> (weak shape transitions, address-based hashing, opportunistic collections at the start
> of a run) and makes others slower (rehashing after every collection).

## Deterministic mode

A host chooses how much of this it wants. By default the `Sandbox` behaves like an
ordinary engine: the clock follows the host's clock, `Math.random` gets a seed from the
host's entropy, and weak references are collected ([the JavaScript
interface](js-api.md#deterministic-runs) has the options). With
`deterministic: { startedAt, seed }`, it sets everything up for repeatable runs:

- `Math.random` is seeded from `seed`, and so is the key of the string hash, so the same
  seed lays the heap out the same way; the time zone is UTC;
- weak references are strong;
- the clock starts at `startedAt` and moves only when an async result is applied: to
  the host's clock when the result was settled, never backwards. Fuel slices and sync
  calls do not move it. The run records each result it applied with that clock, its
  **deliveries**.

So the same code, parameters, sync results and async results in the same order do the
same thing on every run, with any fuel slice, and across restores from images (the
clock is in the heap). A snapshot keeps the clock, `Math.random`'s state and the ids of
the run's host calls too, so a run loaded from a snapshot makes the calls an
uninterrupted run makes, under the same ids; it loads only into a sandbox that is
deterministic too.

## When a snapshot is refused

A host that keeps a deterministic run's snapshots keeps its deliveries beside them: per
result, the call (id, function, arguments), the outcome and the clock. When a newer
build refuses a snapshot, the host makes a sandbox with the same options and begins the
script again with those deliveries ([the JavaScript
interface](js-api.md#deterministic-runs)). The sandbox applies each one once its call is
open, checking that the call is the one recorded, and hands the run back idle where the
original was after its last result, waiting on the same calls; the host goes on from
there. If the new build or a changed script makes another call, or the same call with
other arguments, the re-run stops with a `diverged` error that names the first
difference, before anything of the host's is touched.

Running a script again should not do again what it did. At the engine's level that
means: no async host function runs during a re-run, since every result comes from the
deliveries; the sync host functions are called as the script calls them, and they are
the host's to stub: output the host has already shown, data it has already stored, a
lookup it answers from its record. A host that records everything instead (a journal,
below) can replay a run with no host function called at all.

A re-run costs the run's whole computation again, and the fuel it spends is counted
again in the new build, which may differ slightly where builtins changed. What has to be
the same is what the script does with the results: its calls, which the re-run checks.

> **Trade-off.** Re-running from the results over keeping the old build alive. The
> results are small and the host has them anyway; a re-run needs nothing of the old
> build and lands the run in the new one. The cost: the re-run executes everything again,
> and it works only for runs made deterministic from their start, since a run's clock,
> random numbers and weak references must be inputs from the first instruction.

## Canonical hashes and journals

To check that two runs agree, a host compares **canonical hashes**:
`vm_canonical_hash` is a 64-bit FNV-1a hash of the image a copying collection leaves,
computed on a copy so that the VM itself is not touched. The copying collector lays the
heap out breadth-first from the roots, so garbage, the order of allocation and the moment
of the last collection do not affect it: it identifies the object graph. The builtin
prefix is hashed as it is: the same bytes in every VM of the build, unless a script
changed a builtin. A re-run from its deliveries reaches the canonical hash, and the
snapshot bytes, of the original at every idle point.

On top of that, a deterministic `Sandbox` can record a run as a **journal**: the start
(engine build, code, parameters, hash), every sync call and its outcome, every async
result with the clock it carried, a hash at every point where the VM went idle, and the
end. `replay(journal)` runs the script again without calling any host function: it
answers sync calls from the journal, applies async results one per idle point in the
recorded order, and compares the hashes along the way. A difference is a `diverged`
error that says where. Because results are macrotasks, a replay passes through every
recorded idle state even if the recording got several results at once.

> **Trade-off.** Journals of inputs, checked by hashes, over replaying from saved states
> alone. A journal is small, independent of the image format, and says exactly where a
> replay went wrong. The cost: a replay re-executes everything, and it must run on the
> same engine build, because a different build may legitimately do the same thing in
> another number of steps (the journal records the build hash for that reason).

## What the gate compares

The determinism claims are checked on every merge ([How djs is
tested](testing.md)):

- **Across schedules.** The end-to-end tests and the difftest corpus run each case in
  one go and again in slices (1 and 997 fuel in the end-to-end tests, 97 and 1009 in the
  corpus), restoring a fresh VM from its image at every stop. The raw image at the end, and
  the canonical hash, must equal the plain run's.
- **Across builds.** The same programs run in a native build and in djs-wasm, driven
  through the same export calls. The build hash, the raw images at every slice, the
  images after a final collection, the canonical hash, the number of stops and the
  results must all match. This is why djs avoids
  float operations whose bits differ between platforms: wasm leaves the NaN bits of
  `f64` to `f32` conversion open, so djs canonicalizes them.

GC stress is the one mode excluded from the image comparisons: collecting at every
safepoint is another collection policy, and weak tables lose dead entries at other
times. Its output must still match.

## Where determinism ends

Some things are deliberately outside the guarantee:

- **The host's memory.** Running out of the heap cap is deterministic; the host failing
  to provide memory below the cap is not, since it depends on the machine. That stops
  the run with its own status, not a JavaScript error. A host that wants
  deterministic out-of-memory chooses a cap it can always back (in wasm32, about 1 GiB
  at most).
- **Weak references in the default mode.** When a `WeakRef`'s target dies depends on
  when collections run, and collections the host requests or a smaller heap cap change
  that. Deterministic mode makes them strong for this reason.
- **Fuel of builtins in Rust.** A builtin compiled to Rust charges fuel per loop
  iteration and call, and for the work of its heap operations, where bytecode charges
  per instruction.
  Which builtins run as Rust is part of the build, so a different build may spend
  different fuel on the same script.
- **Other builds.** Journals and canonical hashes are tied to the build hash, which
  covers the engine's sources (its math library's included) and features: a different
  build may run the same script in a different number of steps. Images load only into
  the build that made them ([images](snapshots.md#what-an-image-depends-on)); a run
  loaded from a [snapshot](persistence.md) into another build goes on with that build's
  code for every function it does not wait in, and a re-run in another build is checked
  by its calls, not by hashes.
- **The clock in the default mode.** Without `deterministic`, the Sandbox sets the clock
  before every fuel slice, so what `Date.now()` returns depends on timing, as it does in
  any engine.

> **Trade-off.** Determinism as an opt-in mode over always. Strong weak references and a
> virtual clock are not what a script expects from an ordinary engine, and an embedder
> that does not run scripts again gains nothing from them. The cost: a host must choose
> the mode up front, since a run that did not start in deterministic mode cannot be run
> again to the same state.

## Where to look

- [`js/src/sandbox.ts`](../../js/src/sandbox.ts): deliveries, re-runs, journals and
  replay; [`js/test/determinism.test.ts`](../../js/test/determinism.test.ts): re-runs of
  the harvested corpus checked at every idle point.
  [`crates/djs-core/src/vm/mod.rs`](../../crates/djs-core/src/vm/mod.rs):
  `canonical_hash`, `set_now`, `set_random_seed`, `set_weak_refs`.
- [`scripts/determinism/`](../../scripts/determinism/): the native-versus-wasm check.
