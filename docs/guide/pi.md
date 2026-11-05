# djs in pi

djs was built for pi, a coding agent, whose **codemode** lets the model write a small
JavaScript program that calls pi's tools instead of calling them one at a time. pi runs
those programs on QuickJS, compiled to wasm, through one module with a small contract.
This page shows how djs takes QuickJS's place behind that contract, what it adds, how
pi's own tests fare, and how another agent host could wire djs up the same way. It is
the one page of the guide about a particular host; everywhere else, hosts are described
in general.

> **In short.** An adapter, `js/pi/vm.ts`, implements pi's VM contract on djs and runs
> pi's unmodified prelude, so trying djs in pi means swapping one module. It keeps an
> image of the VM after the prelude and starts later scripts from it, and it polls pi's
> interrupt every 10,000 fuel. In its *next* mode, with a prelude reshaped for djs and
> frozen builtins, a waiting session is written as a continuation dump and resumed in a
> new process or a later djs.

## pi's VM contract

pi's codemode runs each script in a fresh VM, driven by one module with a small
contract:

- **start** a VM for one script, with pi's *prelude*: JavaScript that runs before the
  script and builds `tools`, the output helpers (`text`, `image`, `exit`, `console`), a
  small key-value store and pi's globals on top of one bridge function, then freezes the
  realm with a *lockdown*;
- **post**: the VM passes messages to the host (a tool call, output, the end), from
  inside its callbacks;
- **deliver**: the host hands the VM a tool result, and the script runs until it waits
  again;
- **should interrupt**: polled while the script runs; when it says so, the VM stops.

[`js/pi/vm.ts`](../../js/pi/vm.ts) implements that contract on djs. pi's launchers
(worker thread, in-thread, remote) and its host side stay as they are; only the module
that drives one VM changes. [`js/pi/engine.ts`](../../js/pi/engine.ts) is a client of the
wasm module's exports at the level of handles, because pi's prelude keeps its functions
alive across calls, which the `Sandbox` of [the JavaScript interface](js-api.md), made
for whole function bodies, does not do.

> **Trade-off.** Implementing pi's existing contract over designing a new one around
> djs. pi can adopt djs behind a flag by swapping one module, and its tests run
> unchanged. The cost: the adapter carries pi's protocol (a sync bridge function,
> polling between fuel slices instead of an interrupt handler), and what djs could offer
> beyond QuickJS, such as persisting waiting sessions, is something pi has to opt into.

## What the adapter does

**Prelude images.** Running pi's prelude, whose lockdown freezes about 700 objects, is
most of a start. The adapter collects the heap right after it, keeps that VM image, keyed
by everything the prelude saw (the tools, the globals, the store, the heap limit), and
starts the next script with the same inputs from a copy of it. A start then takes about
1 ms instead of 7 ms, against about 4 ms on QuickJS. The prelude's work is not counted
against the script's interrupt budget, so a start from an image and one that runs the
prelude behave the same. The image never leaves the process: it is a cache, not a
persistence format.

**Interrupts.** pi's contract polls `shouldInterrupt`; the adapter runs the script in
slices of 10,000 fuel and polls between them. A tight loop then polls about as often per
second as pi's QuickJS VM does, about 33,000 times, so pi's interrupt budgets mean about
the same CPU time ([limits](limits.md#fuel)). Code that allocates or calls builtins
polls more often than on QuickJS, since djs charges fuel for that work too. When the
poll says stop, the adapter abandons the run, which the script cannot catch.

**Globals.** The adapter defines `performance.now()` before the prelude, which pi's
QuickJS host layer provides, so the lockdown freezes it too; `atob`, `btoa` and
`queueMicrotask` are djs builtins.

**Deterministic runs.** In pi's deterministic mode the adapter makes the VM
deterministic as [determinism](determinism.md) describes: the clock reads the start
time, then the time of the result being processed; `Math.random` is seeded from pi's
seed; the time zone is UTC; and `WeakRef` and `FinalizationRegistry` are absent, as in
pi's QuickJS VM.

## The next mode: sessions as dumps

pi's prelude keeps its own in-flight state: a map of pending tool calls, the store's
writes, reactions on the script's promise. A dump would have to write all of it as
prelude internals, and the prelude's lockdown changes builtins in ways a dump cannot
recreate. So the adapter has a second mode, `shape: "next"`, with a variant of the
prelude in the shape pi would adopt on djs,
[`tests/pi/prelude-next.ts`](../../tests/pi/prelude-next.ts), whose every difference from
pi's prelude is listed at its top:

- tools are the engine's async host calls, so an open tool call is an open host call
  that a dump writes;
- the script's promise, the output counters and the store belong to the adapter;
- frozen builtins replace the lockdown.

A waiting session is then written by `dumpSession()` as a [continuation dump](dumps.md)
plus the adapter's own state, JSON that the host stores. `resumeFromDump` continues it
in a new instance, another process or a later djs: it runs the prelude again, loads the
dump, and the script goes on where it waited, without replaying anything. `openCalls()`
lists the calls the session waits for, which the host answers with a recorded result,
issues again or rejects. Each open call carries its id in pi's protocol as its state, so
the resumed session posts results under the ids pi knows.

pi makes runs durable today by recording tool results and replaying them into a fresh
run. Dumps are the alternative it would adopt: a resumed session does not run the script
again.

## pi's tests on djs

pi's own codemode test suite runs against the adapter (`node js/pi/pi-tests.ts`), in
the default mode, in the next mode, and with a dump and a resume at every idle point:

<!-- gen:status conformance -->
- test262 (at dquickts' pinned commit): djs passes **38,069 of the 38,109** tests in the
  active group (99.9%; 53,597 tests in all, the rest skipped by design). The 40 known
  failures are listed with reasons in `tests/test262/expected-failures.txt`; dquickts'
  qjs fails 4 active tests under the same runner.
- The differential corpus (codemode scripts, console and result formatting, error
  messages, TypeScript, dquickts' own tests): **620 of 631** cases match qjs; 6 known
  failures (`tests/corpus/expected-failures.txt`) and 5 skipped as non-goals.
- pi's codemode test suite on djs: 221 of 226 tests pass (measured at 2026-11-01).
<!-- /gen -->

The tests that fail come from djs being always strict, from a test that expects
QuickJS's fingerprint, and from a workload pi's tests use to cancel scripts, which
finishes on djs before the cancel.

Speed, measured in machine instructions for the same workloads on djs-wasm and on the
QuickJS build pi uses (lower is better; "a pi-like script" is a typical codemode
script):

<!-- gen:status speed -->
| workload | djs-wasm | quickjs-wasi | ratio |
| --- | ---: | ---: | ---: |
| tight loop | 6.6G | 15.7G | 0.42 |
| property access (prop1) | 2.7G | 5.5G | 0.49 |
| strings (split, slice, join, ...) | 3.1G | 6.0G | 0.51 |
| arrays | 10.8G | 16.8G | 0.64 |
| JSON round trips | 1.3G | 2.2G | 0.57 |
| JSON with Dates | 0.9G | 1.9G | 0.48 |
| for-of | 5.8G | 7.2G | 0.81 |
| calls | 3.3G | 4.0G | 0.81 |
| push/pop and string methods | 4.3G | 3.1G | 1.40 |
| a pi-like script | 10.0G | 7.3G | 1.38 |
| forEach | 9.0G | 6.1G | 1.49 |
| map/filter/reduce (higher-order) | 13.9G | 4.8G | 2.92 |

Machine instructions per workload (`scripts/perf/w/`), measured at d780463b0d,
2026-11-01; lower is better.
<!-- /gen -->

## Wiring up another agent host

An agent host that wants djs for its codemode needs little of what the adapter does,
since the `Sandbox` covers it:

1. **Tools as async host functions.** Give each tool a name (`tools.search`) and make it
   an async host function, so a script can run several at once
   (`Promise.all(queries.map(tools.search))`). Bind them as a `tools` namespace, or pass
   them as parameters of the body ([host functions](js-api.md#host-functions)).
2. **Output as sync host functions.** Helpers such as `text(...)` that only record
   output are sync host functions: cheap, and they keep the order of output.
3. **Limits.** Choose a heap cap, a fuel budget and a slice ([limits](limits.md)); poll
   your own deadline between slices if you need wall-clock limits.
4. **Frozen builtins** instead of a lockdown in a prelude, which also makes dumps
   possible.
5. **Persistence.** Start each script as a task; when the agent must wait long (a person,
   a slow job), suspend the task into a dump, store it compressed with the open calls'
   state, and resume it later with `Sandbox.resume` ([dumps](dumps.md)).

## Where to look

- [`js/pi/vm.ts`](../../js/pi/vm.ts): the adapter, its prelude images and the next
  mode; [`js/pi/engine.ts`](../../js/pi/engine.ts): its client of the exports.
- [`tests/pi/`](../../tests/pi/): pi's prelude, as copied from pi, and the next mode's
  variant.
- [`js/pi/pi-tests.ts`](../../js/pi/pi-tests.ts): runs pi's test suite against the
  adapter.
