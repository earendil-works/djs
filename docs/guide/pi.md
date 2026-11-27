# djs in pi

djs was built for pi, a coding agent, whose **codemode** lets the model write a small
JavaScript program that calls pi's tools instead of calling them one at a time. pi runs
those programs on QuickJS, compiled to wasm. This page shows how pi's durable code mode
runs them on djs instead, how djs compares on pi's workloads, and how another agent host
could wire djs up the same way. It is the one page of the guide about a particular host;
everywhere else, hosts are described in general.

> **In short.** pi's durable code mode runs each script as a host-driven run of a plain
> `Sandbox`: no prelude, every tool and output helper a host function with a stable
> identity, and the latest snapshot of the idle run is the script's state.

## Durable code mode on host-driven runs

pi's durable code mode, being built on djs, drops the prelude altogether: a script runs as
a [host-driven run](js-api.md#host-driven-runs) of a plain `Sandbox`, and everything it
can reach outside the language is a host function named by a stable identity. No code
of pi's sits in the VM, so none is in a snapshot.

- **One module.** pi's code mode (the coding agent's, rollups) and its durable code mode
  load the same file, the package's one module: it has
  [snapshots](persistence.md), so the durable mode writes its snapshots in the VM the
  script runs in, and the other modes carry them unused.
- **Tools and globals are async host functions** with identities such as `tools.read`,
  `searchTools` or `@mcp/github#search_issues`, bound as the `tools` namespace, as
  globals, or as bindings of the body. The run never calls them: they show up in
  `run.calls`, and pi answers each one as a nested task of its own, keyed by the call's
  id, so that a call can outlive the process that issued it.
- **Output and the store are sync host functions** (`text`, `image`, `store`, `load`,
  `console.*`); their checks and limits live on pi's side. `exit()` throws `RunExit`, so
  the script cannot catch it.
- **MCP servers are host modules** (`modules`): a script imports their tools
  (`import { search_issues } from "@mcp/github"`) after reading the module's `.d.ts`,
  which pi writes from the same descriptor, and `tools.*` stays for pi's own tools.
- **`ALL_TOOLS` is host data** (`values`): a snapshot names it instead of carrying the tool
  list, and the resuming process gives it again.
- **The JSON boundary** makes what a script passes to tools, stores and returns what
  pi's prelude made of it: JSON, with a `TypeError` at the call for a cycle or a BigInt.
  The output and store functions are left out of it (`jsonBoundary: { except }`), so pi
  formats `NaN`, errors and maps itself, as the prelude did. Errors of tools carry their
  `structuredOutput` into the script's `catch`.
- **What the model sees** of a failure comes from the run as data: a thrown error's
  name, message and stack, and `console` arguments as host values. The `tools` namespace
  is frozen, lists its tools, reaches a tool by its own name too (`aliases`), and a
  misspelled tool throws pi's hint at the read (`missingMember`). A script ends when its
  body does: calls still running then are in `run.dropped`, and pi cancels them.
- **Saving.** Before a call leaves the VM, and before the script parks to wait for a
  person or a slow job, pi writes the idle run as a snapshot (`run.snapshot()`), with
  the output and store writes so far. A restart or a wake, maybe weeks later, loads it
  with `loadSnapshot` and the same scope, finds each open call's task by its id and goes
  on. Nothing is replayed, and no journal is kept: the latest snapshot is the script's
  state. The namespaces with their aliases and hints, `ALL_TOOLS`, the converting
  functions and the imported modules are named in the snapshot and are the resuming
  sandbox's own, and the fuel used travels with it. djs's tests move every harvested
  script of pi's through a snapshot at each of its idle points in this setup. [Later
  builds](persistence.md#later-builds) says what a snapshot relies on: an upgrade of djs
  that compiles a waiting function differently refuses the snapshots waiting in it, with
  the reason.
- **Steps.** pi runs code mode on its own thread, so each `advance` is bounded:
  `yieldMs` keeps pi responsive while a script computes, and pi's abort signal stops
  the step.

## pi's workloads on djs

Speed, measured in machine instructions for the same workloads on djs-wasm and on the
QuickJS build pi uses (lower is better; "a pi-like script" is a typical codemode
script):

<!-- gen:status speed -->
| workload | djs-wasm | quickjs-wasi | ratio |
| --- | ---: | ---: | ---: |
| tight loop | 3.9G | 15.3G | 0.26 |
| property access (prop1) | 1.5G | 5.4G | 0.28 |
| strings (split, slice, join, ...) | 1.6G | 5.8G | 0.28 |
| arrays | 5.2G | 16.3G | 0.32 |
| JSON round trips | 1.0G | 2.0G | 0.51 |
| JSON with Dates | 0.8G | 1.7G | 0.46 |
| for-of | 3.5G | 7.0G | 0.50 |
| calls | 2.1G | 3.9G | 0.53 |
| push/pop and string methods | 2.9G | 2.9G | 1.00 |
| a pi-like script | 5.3G | 6.9G | 0.78 |
| forEach | 3.6G | 5.8G | 0.62 |
| map/filter/reduce (higher-order) | 3.9G | 4.5G | 0.85 |

Machine instructions per workload (`scripts/perf/w/`), measured at edd129067f,
2026-11-21; lower is better.
<!-- /gen -->

The measurements behind this section, from `tests/scores/status.tsv`
(`scripts/status.py speed`):

<!-- gen:status recorded -->
| measurement | value | measured at |
| --- | ---: | --- |
| `speed/pi-like/djs-wasm` | 5341.6 | edd129067f, 2026-11-21 |
| `speed/pi-like/qjs-wasi` | 6875.1 | edd129067f, 2026-11-21 |
<!-- /gen -->

## Wiring up another agent host

An agent host that wants djs for its codemode builds on the `Sandbox`:

1. **Tools as async host functions.** Give each tool a name (`tools.search`) and make it
   an async host function, so a script can run several at once
   (`Promise.all(queries.map(tools.search))`). Bind them as a `tools` namespace, or pass
   them as parameters of the body ([host functions](js-api.md#host-functions)).
2. **Output as sync host functions.** Helpers such as `text(...)` that only record
   output are sync host functions: cheap, and they keep the order of output.
3. **Limits.** Choose a heap cap, a fuel budget and a slice ([limits](limits.md)); in a
   host-driven run, bound each step with a signal, a fuel budget and yields to your
   event loop ([bounded steps](js-api.md#host-driven-runs)).
4. **Frozen builtins** instead of a lockdown in a prelude: a script that writes a builtin
   fails at the write, and the run stays one a snapshot can hold.
5. **Persistence.** Start each script as a task; when the agent must wait long (a person,
   a slow job), suspend the task into a snapshot, store it with the open calls' state,
   and resume it later with `Sandbox.resume` ([snapshots](persistence.md)). Or, as
   pi's durable code mode does, drive the run yourself and keep its latest snapshot.
6. **Values at the boundary.** `jsonBoundary` gives tools JSON, as a prelude that
   stringifies arguments would, without a prelude ([the JSON
   boundary](js-api.md#the-json-boundary)).
