# Determinism

A codemode run can be stopped, saved, moved and resumed, and the host may want to run
the same script again: after a crash, to debug what a model's script did, or to check a
new engine build against an old one. All of that works only if a run is a function of
its inputs. This page explains which inputs those are, how djs keeps everything else
out of a run, how a host checks that two runs really were the same, and where
determinism ends.

> **In short.** The VM reads nothing from its environment: the clock, the time zone,
> the random seed and the behavior of weak references are inputs the host sets, and
> results of host calls take effect in the order they were delivered, never when they
> arrived. Given the same inputs in the same order, a run does the same thing and leaves
> byte-identical heap images, however it was sliced, snapshotted or compiled. The gate
> checks that, and a canonical hash and run journals let hosts check it too.

## Why it matters

The tour so far has made two promises. A run can stop between any two instructions and
continue from an image ([Suspending anywhere](suspension.md), [Snapshots](snapshots.md)),
and the host hands it inputs and reads results without the VM doing any I/O
([The host interface](host.md)). Determinism is what makes those promises useful:

- **Durability.** pi makes a run durable by recording the results of its tool calls and,
  after a restart, replaying them into a fresh run. That only works if the fresh run
  makes the same calls in the same order and reaches the same state.
- **Debugging and reproduction.** A model's script that misbehaved can be run again with
  the recorded tool results, and it does exactly what it did.
- **Testing.** The suspension tests compare a run in one go with the same run stopped
  and restored every 97 instructions. If anything could differ for other reasons, those
  tests could not demand identical results, and suspension bugs would hide in the
  noise ([How djs is tested](testing.md)).

## Every source of nondeterminism is a host input

An engine meets the outside world in a few places: the clock, the local time zone,
`Math.random`, and the garbage collector, whose timing weak references expose. In djs
each of them is a value the host sets, stored in the heap like any other state, so an
image carries it along (ARCH.md "Nondeterminism"):

| Source | In djs | Decision |
| --- | --- | --- |
| `Date.now()`, `new Date()` | Reads a time the host sets with `vm_set_now` (0 until then). A host that wants time to pass sets it again between fuel slices, or with each async result. | D1383 |
| The local time zone | A fixed offset the host sets (`vm_set_time_zone_offset`), UTC by default, no daylight-saving rules. | D1383 |
| `Math.random()` | xorshift64*, as in QuickJS, seeded by the host (`vm_set_random_seed`). The state is in the heap, so a restored VM continues the sequence. | D1314 |
| `WeakRef`, `FinalizationRegistry` | By default they work as specified, and collection timing is observable. The host can make them strong (nothing ever dies) or absent (the globals are deleted). | D1680 |
| Tool results | Applied in delivery order, as macrotasks (below). | D1670 |

The examples in this guide run with the clock at 0 and the default seed, so their
output is fixed and the gate can check it:

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

## Results in the order they were delivered

Async tool calls are where timing would leak in. A result can reach the host at any
moment, and if the VM applied it at once, the order in which promise callbacks run
would depend on the fuel slice that happened to be running. So results are macrotasks
(D1670): they wait in an inbox in the heap, and the VM applies the oldest one only when
its job queue is empty. A script's behavior depends on the order of the results, never
on when they arrived ([The host interface](host.md) has the details).

A sync host function cannot leak timing at all: it parks the VM at a fixed instruction,
and the script continues only with the value the host returns.

## The same program, the same heap

Determinism goes further than the script's output. Two runs with the same inputs leave
the same *heap*, byte for byte. That is a stronger property, and the design of several
parts of the engine exists to keep it:

- **Fuel** stops a run at the same instruction however the budget was sliced, because
  an overshoot is carried to the next run as a debt (D1740;
  [Suspending anywhere](suspension.md)).
- **Collections** happen at the same instructions under any slicing and restore. The
  threshold lives in the heap (D1601), and nothing about a stop allocates or collects
  (D1710; [The garbage collector](gc.md)).
- **The collector's layout** is a function of the object graph: it copies breadth-first
  from the roots in field order, so equal graphs give equal bytes (D1090).
- **Nothing depends on addresses or timing.** Map and Set rehash after every collection
  instead of hashing by a per-object id that would vary, and shape transitions are
  strong so that the shape tree does not depend on when collections ran (D1831;
  [Cells, strings and objects](heap.md)).
- **Out of memory** is a function of the program and the cap: running out of the heap
  cap is a JavaScript error, and the host's own refusal to provide memory is kept
  separate (D1690; [One arena](arena.md)).

Byte-identical images make the strongest possible check: if two runs left the same
image, they built the same graph, with no need to compare outputs.

> **Trade-off.** Byte-identical heaps over equal outputs only. Equal images make
> suspension bugs visible even when they do not change the output yet, and make
> "did these two runs agree?" one hash comparison. The cost: every design choice in the
> heap has to keep the layout independent of timing, which rules out some common
> techniques (weak shape transitions, address-based hashing, opportunistic collections
> at the start of a run) and makes others slower (rehashing after every collection).

## Deterministic mode

A host chooses how much of this it wants. By default the `Sandbox` behaves like an
ordinary engine: the clock follows the host's clock, `Math.random` gets a seed from the
host's entropy, and weak references are collected. With `deterministic: { startedAt,
seed }` (D1680), it sets everything up for repeatable runs:

- `Math.random` is seeded from `seed`, and the time zone is UTC;
- weak references are strong, unless the host asks for them to be absent (pi's
  deterministic contract, matching quickjs-wasi);
- the clock starts at `startedAt` and moves only when an async result is delivered: to
  the host's clock at that moment, never backwards. Fuel slices and sync calls do not
  move it.

With a fixed clock as well, the same code, parameters and results in the same order do
the same thing on every run, with any fuel slice, and across snapshots (the clock is in
the snapshot).

## Canonical hashes and journals

To check that two runs agree, a host compares **canonical hashes** (D1730):
`vm_canonical_hash` is a 64-bit FNV-1a hash of the image a collection would leave,
computed on a copy so that the VM itself is not touched. Garbage and the moment of the
last collection do not affect it, so it identifies the object graph.

On top of that, a deterministic `Sandbox` can record a run as a **journal**: the start
(engine build, code, parameters, hash), every sync call and its outcome, every async
result with the clock it carried, a hash at every point where the VM went idle, and the
end. `replay(journal)` runs the script again without calling any host function: it
answers sync calls from the journal, delivers async results one per idle point in the
recorded order, and compares the hashes along the way. A difference is a `diverged`
error that says where. Because results are macrotasks, a replay passes through every
recorded idle state even if the recording got several results at once. docs/host.md
has the journal format.

> **Trade-off.** Journals of inputs, checked by hashes, over replaying from snapshots
> alone. A journal is small, independent of the image format, and says exactly where a
> replay went wrong. The cost: a replay re-executes everything, and it must run on the
> same engine build, because a different build may legitimately do the same thing in
> another number of steps (the journal records the build hash for that reason).

## What the gate compares

The determinism claims are checked on every merge (D1720, D1725;
[How djs is tested](testing.md)):

- **Across schedules.** The end-to-end tests and the difftest corpus run each case in
  one go and again in slices (1 and 997 fuel in the end-to-end tests, 97 and 1009 in
  the corpus), restoring a fresh VM from its image at every stop. The raw image at the end, and the image after a final collection, must
  equal the plain run's.
- **Across builds.** The same programs run in a native build and in djs-wasm, driven
  through the same export calls. The build hash, the raw and collected images at every
  slice, the number of stops and the results must all match. This is why djs avoids
  float operations whose bits differ between platforms: wasm leaves the NaN bits of
  `f64` to `f32` conversion open, so djs canonicalizes them (D1750).

GC stress is the one mode excluded from the image comparisons: collecting at every
safepoint is another collection policy, and weak tables lose dead entries at other
times. Its output must still match.

## Where determinism ends

Some things are deliberately outside the guarantee:

- **The host's memory.** Running out of the heap cap is deterministic; the host failing
  to provide memory below the cap is not, since it depends on the machine. That stops
  the run with its own status, not a JavaScript error (D1690). A host that wants
  deterministic out-of-memory chooses a cap it can always back (docs/host.md).
- **Weak references in the default mode.** When a `WeakRef`'s target dies depends on
  when collections run, and collections the host requests or a smaller heap cap change
  that. Deterministic mode makes them strong or absent for this reason.
- **Fuel across backends.** A builtin compiled to Rust charges fuel per loop iteration
  and call, and the same builtin as bytecode charges per instruction. Results are the
  same, fuel counts are not, so the backend is part of the engine's identity (D1633).
  djs-wasm always uses the Rust backend.
- **Other builds.** Images and journals are tied to the build hash, which covers the
  engine's sources, features and the `libm` version (D1750). A different build may run
  the same script in a different number of steps.
- **The clock in the default mode.** Without `deterministic`, the Sandbox sets the clock
  before every fuel slice, so what `Date.now()` returns depends on timing, as it does
  in any engine.

> **Trade-off.** Determinism as an opt-in mode over always. Strong weak references and
> a virtual clock are not what a script expects from an ordinary engine, and an
> embedder that does not replay runs gains nothing from them. The cost: a host must
> choose the mode up front, since a run that was not recorded in deterministic mode
> cannot be replayed.

## Where to look

- [docs/host.md](../host.md): the Sandbox options, deterministic mode and the journal
  format.
- `js/src/sandbox.ts`: journals and replay. `crates/djs-core/src/vm/mod.rs`:
  `canonical_hash`, `set_now`, `set_random_seed`, `set_weak_refs`.
- `scripts/determinism/`: the native-versus-wasm check.
- ARCH.md, "Host Interface" ("Nondeterminism") and "Snapshots (G4)".

Decisions on this page:

<!-- gen:decisions D1314 D1383 D1670 D1680 D1690 D1090 D1601 D1710 D1740 D1831 D1730 D1720 D1725 D1750 D1633 -->
- D1314: Math.random
- D1383: Date reads the host's clock and a fixed local offset
- D1670: Async host results are macrotasks: the host inbox
- D1680: Deterministic mode: weak references collected, strong or absent; the Sandbox's deterministic option
- D1690: Running out of host memory is not a JS out-of-memory error
- D1090: Copying collector
- D1601: The GC threshold is part of the heap image
- D1710: GC points and raw images do not depend on the fuel slice
- D1740: Fuel overshoot carries to the next run; a suspended intrinsic is charged once
- D1831: Shapes with strong transitions and a per-heap shape budget
- D1730: Canonical hash and run journals
- D1720: Determinism checks in the gate: heap images across schedules and builds
- D1725: The determinism checks compare raw images too
- D1750: The build hash covers features and libm; canonical f32/f16 NaN
- D1633: Fuel per transition; the backend is part of the engine's identity
<!-- /gen -->
