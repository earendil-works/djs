# Benchmarks

How fast does djs start, run and dump the scripts an agent writes, and what does it cost
in download and memory, next to the QuickJS builds a host would otherwise embed? This
page compares them on workloads modeled on real agent codemode sessions, all in
WebAssembly, the way a host runs them, and says how to re-run every number.

> **In short.** Seven engines (djs, four QuickJS builds of both QuickJS lines, and monty,
> a Python subset) run nine codemode workloads in wasm in one node process, each run in a
> fresh VM, and must all return the same result. djs starts a VM fastest and has the
> lowest run time over the set; it is ahead where scripts move JSON, strings and tool
> results, and behind on the smallest bodies and on some single string builtins. Its
> module is the largest of the JavaScript engines and it takes more memory than the
> QuickJS releases built here, and its dumps of a paused run are kilobytes, written or
> loaded in under a millisecond for most workloads. `scripts/bench-engines.sh` fetches,
> builds and runs everything; the tables are generated from the recorded results.

## What is measured, and why this way

An agent's codemode body is short and makes a few tool calls: it reads files, runs
commands, reshapes their output and prints a summary. The engine's own work is small, so
what a host pays is mostly starting a VM, passing tool results in and out, parsing the
body and assembling strings. The comparison measures exactly that, as an embedder sees it:

- **Everything in wasm, in one host.** Every engine is a wasm module run by node: the
  build a host would ship, not a native binary. Each module is compiled once per process,
  as a host keeps a compiled module.
- **A fresh VM per run.** Every run gets a new instance and VM, isolated from the others,
  as a host gives each tool call its own sandbox. *New VM* is the time to get one ready;
  *run* is the time from handing it the script to having the result string.
- **A warm process.** The times are medians of 25 runs after 3 warm-up runs, so node has
  optimized each engine's code, as in a host that has been running for a while.
- **The same tool bridge.** Tool answers are recorded once and handed to every engine as
  one JSON string; inside the engine, each tool call turns its arguments into JSON to find
  its answer and parses the answer's JSON text. That is the work a host bridge does, and it
  is the same code in every engine, so the comparison measures the engines and not their
  host interfaces.
- **The same answer.** Every engine's result must be the string node computes for the same
  script, or the run fails.
- **Memory** is the instance's linear memory after the run. Wasm memory never shrinks, so
  that is the run's high-water mark, allocator overhead included.

Times depend on the machine and its load; the tables say when and at what load they were
taken. Instruction counts would not compare across engines with different hosts, so the
page gives wall-clock medians.

## The engines

| engine | what it is | how it is built here |
| --- | --- | --- |
| djs | this engine, the shipped module | [`scripts/build-wasm.sh`](../../scripts/build-wasm.sh) |
| quickjs-wasi | QuickJS-NG for WASI with snapshot support, version 3.6.2 | the npm package's `quickjs.wasm` |
| QuickJS | Bellard's QuickJS 2026-06-04, the latest release | wasi-sdk 34, `-O2`, a minimal embedding |
| QuickJS-NG | QuickJS-NG 0.18.0, the latest release | the same: wasi-sdk 34, `-O2`, the same embedding |
| quickjs-emscripten | the popular emscripten build, version 0.32.0, of Bellard's QuickJS 2025-09-13 | the npm variant `@jitl/quickjs-wasmfile-release-sync` |
| quickjs-emscripten (NG) | the same package with QuickJS-NG 0.12.1 | the npm variant `@jitl/quickjs-ng-wasmfile-release-sync` |
| monty | pydantic's Python subset for code written by models, version 1.3.0 | the npm package's wasm component |

The two latest QuickJS releases are built by the script with one compiler, the same flags
and the same small embedding ([`js/bench/engines/driver.c`](../../js/bench/engines/driver.c)):
a runtime, a context, an evaluation that runs the job queue dry, and a result string.
monty runs the workloads translated to Python by hand; its component is instantiated on
the calling thread with no filesystem, environment or network, as its own worker does.

## The workloads

The workload set ([`tests/bench/codemode/`](../../tests/bench/codemode/)) was derived from
about three thousand codemode bodies of agent sessions: they were grouped into clusters by
what they do, and each cluster got one synthetic body of its typical size and operation
mix. The data the tools return is invented: source-like files, command output, search
results, query rows, an image.

| workload | share of bodies | what it does |
| --- | ---: | --- |
| `sequential` | 30% | a read, an edit batch with large literal arguments, two commands |
| `fanout` | 22% | twelve reads and three checks with `Promise.all`, joined and cut to an output budget |
| `media` | 13% | render a view, read the image, hand a 200 KB data URL to the host |
| `inspect` | 9% | no tools: group and search the tool catalog |
| `loop` | 8% | eight searches one after another, each result formatted with a regular expression |
| `text` | 8% | parse grep-like output, group by file, scan files for markers and definitions |
| `json` | 7% | group 600 query rows, rewrite a package manifest |
| `ts` | 2% | the `json` shape in TypeScript (djs runs it as is; the other engines get it with the types stripped by the host) |
| `heavy` | the largest 1% | 97 calls: 96 file reads in batches with per-file regular expression work |

A typical body is about 500 characters and makes three tool calls; the workloads sit at or
a little above their cluster's median, so that the engines' work shows.

## Download size

What a host ships: the wasm module, and the JavaScript some builds need around it.

<!-- gen:status engines-size -->
![Module size per engine, raw and with gzip, in KB on a logarithmic scale](gen/chart-engines-size.svg)

| engine | wasm | gzip | brotli | JS glue, gzip |
| --- | ---: | ---: | ---: | ---: |
| djs | 980 KB | 427 KB | 360 KB | none |
| quickjs-wasi | 637 KB | 287 KB | 247 KB | 28 KB |
| QuickJS | 966 KB | 332 KB | 262 KB | none |
| QuickJS-NG | 1,087 KB | 371 KB | 295 KB | none |
| quickjs-emscripten | 503 KB | 231 KB | 199 KB | 8 KB |
| quickjs-emscripten (NG) | 529 KB | 247 KB | 212 KB | 8 KB |
| monty | 24,593 KB | 7,628 KB | 4,804 KB | 55 KB |

Module bytes as shipped; gzip -9, brotli quality 11; measured at d01ce5dc8f, 2026-11-06,
load 1.7.
<!-- /gen -->

djs's module is the largest of the JavaScript engines: it carries the TypeScript front
end and the base image of an initialized realm (every builtin object, coded to
<!--n wasm/base-coded kb-->46 KB<!--/n-->), which spares each new VM from building them.
quickjs-wasi and the emscripten builds are optimized for size; the two releases built
here use `-O2`, as djs's module is optimized for speed. monty's module is a different
order of size altogether: a Python interpreter with its standard library subset and a
type checker.

## Starting a VM

A host that gives every call a fresh VM pays this on every call.

<!-- gen:status engines-startup -->
![Compiling the module, starting a VM and an idle VM's memory, per engine](gen/chart-engines-startup.svg)

| engine | compile, ms | new VM, ms | idle VM memory, MiB |
| --- | ---: | ---: | ---: |
| djs | 1.14 | 0.13 | 0.6 |
| quickjs-wasi | 4.80 | 0.25 | 1.3 |
| QuickJS | 3.08 | 0.20 | 0.3 |
| QuickJS-NG | 2.02 | 0.26 | 0.4 |
| quickjs-emscripten | 1.05 | 0.23 | 16.0 |
| quickjs-emscripten (NG) | 5.17 | 0.29 | 16.0 |
| monty | 31.6 | 0.70 | 3.9 |

Compile: the module, once per process. New VM: a new instance and VM, ready to evaluate
(median). Memory: linear memory; measured at d01ce5dc8f, 2026-11-06, load 1.7.
<!-- /gen -->

A new djs VM is ready in <!--n engines/djs/startup-ms .2-->0.13<!--/n--> ms, against
<!--n engines/quickjs/startup-ms .2-->0.20<!--/n--> ms for QuickJS and
<!--n engines/quickjs-ng/startup-ms .2-->0.26<!--/n--> ms for QuickJS-NG: the realm
comes from the base image in the module, so creating a VM copies an image instead of
running the code that builds every builtin (the [snapshots page](snapshots.md) says
how). Its idle memory is above the QuickJS releases': the base image is larger than the
heap a fresh QuickJS context allocates, and djs reserves a larger native stack, which
sets how deeply its parser may nest. The emscripten builds start at 16 MiB, emscripten's
initial heap, whatever runs in them.

## Run time

The time from handing a fresh VM the script and its tool answers to having the result:
parsing and compiling the body, the tool calls through the bridge, and the script's work.

<!-- gen:status engines-run -->
![Run time per workload and engine, in ms on a logarithmic scale](gen/chart-engines-run.svg)

| workload | djs | quickjs-wasi | QuickJS | QuickJS-NG | quickjs-emscripten | quickjs-emscripten (NG) | monty |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| sequential | 0.44 | 0.40 | 0.37 | 0.37 | 0.41 | 0.40 | 0.72 |
| fanout | 1.04 | 1.61 | 1.19 | 1.16 | 1.79 | 1.65 | 1.59 |
| media | 0.30 | 0.47 | 0.90 | 0.43 | 1.47 | 0.97 | 0.95 |
| inspect | 0.26 | 0.24 | 0.20 | 0.19 | 0.29 | 0.31 | 0.81 |
| loop | 0.77 | 2.63 | 0.83 | 2.10 | 1.10 | 4.35 | 1.87 |
| text | 1.44 | 1.66 | 1.50 | 1.46 | 1.90 | 2.38 | 3.24 |
| json | 1.67 | 2.16 | 1.78 | 1.88 | 2.64 | 2.75 | 3.88 |
| ts | 2.05 | 2.35 | 1.96 | 2.11 | 2.92 | 2.99 | 3.00 |
| heavy | 11.8 | 16.0 | 15.0 | 16.0 | 18.7 | 29.4 | 13.0 |
| relative to djs | 1.00× | 1.37× | 1.14× | 1.18× | 1.58× | 1.90× | 2.01× |

Milliseconds per run (median), in a warm process; the last row is the geometric mean of
each engine's time over djs's; measured at d01ce5dc8f, 2026-11-06, load 1.7.
<!-- /gen -->

djs has the lowest time over the set, and the largest leads where a body moves data:
`fanout`, `json` and `media` pass large tool results through `JSON.parse` and strings
(djs parses and writes JSON in about a third of QuickJS's time, see the operations
below), and `heavy` spends most of its time splitting, matching and scanning file texts.
`loop` is a regular expression replace over prose in a loop, where Bellard's QuickJS is
close and QuickJS-NG and quickjs-wasi are far behind. djs is behind on the shortest
bodies, `sequential` and `inspect`, by a few hundredths of a millisecond: compiling the
body costs about the same in both, and what is left is a handful of small builtin calls
(`Object.entries`, `toFixed`), each dearer in djs (below). On `ts` djs compiles the
TypeScript itself, while the other engines get JavaScript whose types the host stripped
beforehand, outside the timing. monty is fast on `heavy`, whose string splitting and
regular expressions are native in its Rust implementation, and slower on the rest; its
times include the same JSON bridge in Python.

## Memory

The instance's linear memory after the run: what the run cost, since wasm memory does not
shrink.

<!-- gen:status engines-memory -->
![Linear memory after the run per workload and engine, in MiB on a logarithmic scale](gen/chart-engines-memory.svg)

| workload | djs | quickjs-wasi | QuickJS | QuickJS-NG | quickjs-emscripten | quickjs-emscripten (NG) | monty |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| sequential | 0.7 | 1.4 | 0.4 | 0.5 | 16.0 | 16.0 | 5.1 |
| fanout | 1.8 | 2.5 | 1.5 | 1.6 | 16.0 | 16.0 | 5.6 |
| media | 1.0 | 2.1 | 1.1 | 1.1 | 16.0 | 16.0 | 5.5 |
| inspect | 0.6 | 1.3 | 0.4 | 0.4 | 16.0 | 16.0 | 5.1 |
| loop | 0.9 | 1.5 | 0.5 | 0.6 | 16.0 | 16.0 | 5.1 |
| text | 1.2 | 1.6 | 0.6 | 0.7 | 16.0 | 16.0 | 7.1 |
| json | 1.4 | 2.2 | 1.2 | 1.3 | 16.0 | 16.0 | 5.9 |
| ts | 1.6 | 2.2 | 1.1 | 1.2 | 16.0 | 16.0 | 5.8 |
| heavy | 4.6 | 3.4 | 3.9 | 4.0 | 16.0 | 16.0 | 7.2 |
| relative to djs | 1.00× | 1.53× | 0.72× | 0.77× | 12.62× | 12.62× | 4.54× |

Linear memory after the run, MiB; the last row is the geometric mean of each engine's
over djs's; measured at d01ce5dc8f, 2026-11-06, load 1.7.
<!-- /gen -->

djs needs more memory than the QuickJS releases built here and less than quickjs-wasi,
monty and the emscripten builds. Part of the gap is constant: the larger idle footprint
above. The rest comes from the garbage collector. djs's collector traces the live heap
when the allocation since the last collection crosses a threshold, about as large again
as what survived, so garbage stays until then; QuickJS counts references and frees most
garbage the moment it is dropped. `heavy`, which allocates a few megabytes of
short-lived lines and strings, shows it most.

## Operation by operation

Single operations on data shaped like the workloads', each in a loop, against the faster
of the two QuickJS releases. Above 1, djs is slower.

<!-- gen:status engines-ops -->
![djs's time over the best QuickJS build's, per operation, on a logarithmic scale around parity](gen/chart-engines-ops.svg)

| operation | djs, ms | QuickJS, ms | djs / QuickJS |
| --- | ---: | ---: | ---: |
| `JSON.parse` of 600 query rows | 1.01 | 2.80 | 0.36 |
| `JSON.stringify` of 600 query rows | 1.04 | 3.01 | 0.35 |
| `split("\n")` of file texts | 1.26 | 2.73 | 0.46 |
| `join("\n")` of their lines | 0.48 | 0.85 | 0.56 |
| `includes` per line | 3.08 | 2.51 | 1.23 |
| template literal appends | 5.54 | 4.26 | 1.30 |
| `trim` | 11.2 | 3.64 | 3.08 |
| `slice` | 7.73 | 3.83 | 2.02 |
| `padStart` | 8.21 | 3.09 | 2.66 |
| `toLowerCase` | 18.4 | 10.5 | 1.76 |
| `charCodeAt` loop | 10.2 | 6.46 | 1.59 |
| `String(i)` | 6.07 | 3.38 | 1.80 |
| `toFixed` | 5.78 | 3.03 | 1.91 |
| `Math.max` | 0.70 | 0.49 | 1.42 |
| `Math.floor` | 5.90 | 2.68 | 2.20 |
| `match(/function \w+/g)` | 2.13 | 5.12 | 0.42 |
| `replace(/\s+/g, " ")` | 8.17 | 7.75 | 1.05 |
| `exec` of a line pattern | 9.24 | 13.4 | 0.69 |
| `matchAll` with `/gm` | 4.79 | 5.38 | 0.89 |
| `new RegExp` | 1.62 | 1.81 | 0.90 |
| `map` | 3.44 | 3.46 | 0.99 |
| `sort` with a comparator | 11.2 | 5.78 | 1.93 |
| `sort` of strings | 2.28 | 2.38 | 0.96 |
| `push` | 8.43 | 7.75 | 1.09 |
| `Map.set` with string keys | 18.6 | 10.2 | 1.83 |
| `Set.add` with string keys | 10.9 | 7.25 | 1.50 |
| object spread | 6.55 | 4.49 | 1.46 |
| `Object.entries` and `fromEntries` | 0.45 | 0.02 | - |
| `await` of a value | 5.51 | 17.1 | 0.32 |
| `Promise.all` of 20 | 4.44 | 7.94 | 0.56 |

Milliseconds per loop of the operation (js/bench/engines/ops.ts), the setup's time
subtracted; QuickJS: the faster of the two latest releases; measured at d01ce5dc8f,
2026-11-06, load 2.4.
<!-- /gen -->

JSON, splitting and joining, regular expression matching, sorting strings, `await` and
`Promise.all` are faster in djs. What is still slower is a group of small string and
number builtins (`trim`, `padStart`, `slice`, `toLowerCase`, `String(i)`, `toFixed`,
`Object.entries`, `Math.floor`): djs writes most builtins in a TypeScript subset
compiled to its own bytecode, and although the common ones take a fast path, a call
still costs more instructions than a call of QuickJS's C functions, which shows when the
operation itself is tiny. `sort` with a comparator runs the comparator, a script
function, for every comparison, and calling it from the sort still costs more than in
QuickJS; `Map.set` and `Set.add` with string keys also take more instructions per call.
These matter in a loop of tens of thousands of calls; in the workloads they are a small
part of the time.

## Passing data through the bridge

Moving data between the host and the sandbox is what [cheap data
passing](goals.md#cheap-data-passing) is about: a tool result going in, a run's result or
a tool call's arguments coming out. The workloads above pass every tool answer the same
way in every engine, as JSON text, so that they compare the engines. This section instead
compares the bridges: each engine gets the best path its embedding offers
([`js/bench/engines/bridge.ts`](../../js/bench/engines/bridge.ts)).

| engine | value in | value out | a tool call |
| --- | --- | --- | --- |
| djs | the host encodes the value in the wire format, the VM decodes it into cells; a long ASCII string is copied straight into a string the VM reserved | the reader decodes the value out of the arena in place; the VM does nothing | the call's record names the arguments; the reader reads them, the result goes in as above |
| QuickJS, QuickJS-NG | `JSON.stringify` on the host, the UTF-8 written into linear memory, `JS_ParseJSON` there; strings by `JS_NewStringLen`, bytes as a copied `ArrayBuffer` | `JS_JSONStringify` in the VM and its UTF-8 read in place, then `JSON.parse`; strings as their UTF-8, bytes by pointer | a C function serializes its argument with `JS_JSONStringify` and calls the host, which answers with JSON text or a string |
| quickjs-wasi | JSON text parsed by `JSON.parse` in the VM, or its handle walk (`hostToHandle`) | `JSON.stringify` in the VM, or its handle walk (`dump`) | `newFunction` with the same conversions |
| quickjs-emscripten | JSON text parsed in the VM, or objects built through handles | `dump`, which is `JSON.stringify` in the VM and `JSON.parse` on the host | `newFunction` with the same conversions |
| monty | JSON text and `json.loads`, or its value arena, which the component model lowers into the module | `json.dumps`, or the value arena lifted out | an external function: the run stops, the host answers with a value arena |

Where an engine has two paths, each number below is the faster one's: quickjs-wasi's
handle walk wins on arrays of numbers and loses on everything else, and monty's JSON text
beats its value arena for anything larger than call arguments. The payloads: call
arguments; a search tool's result as an object, of 1 KB, 100 KB, 1 MB and 10 MB of JSON;
source text of 1 MB and 10 MB (ASCII) and 1 MB of prose with accented letters, dashes and
CJK; 100,000 floating-point numbers; a 1 MiB `Uint8Array`; and objects nested 100 deep
(monty's `json.loads` stops at about 200). Every value must read back equal to what went
in. Each time is the median of one operation in fresh VMs; small payloads repeat within
a sample.

### In

<!-- gen:status bridge-in -->
![Putting a value into the VM, per payload and engine, in ms on a logarithmic scale](gen/chart-bridge-in.svg)

| payload | size | djs | quickjs-wasi | QuickJS | QuickJS-NG | quickjs-emscripten | quickjs-emscripten (NG) | monty |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| call arguments | 102 B | 0.002 | 0.002 | 0.001 | 0.001 | 0.003 | 0.003 | 0.025 |
| JSON, 1 KB | 1 KB | 0.006 | 0.009 | 0.008 | 0.008 | 0.015 | 0.021 | 0.055 |
| JSON, 100 KB | 100 KB | 0.319 | 0.774 | 0.649 | 0.7 | 1.09 | 1.19 | 0.828 |
| JSON, 1 MB | 1,000 KB | 3.13 | 7.41 | 6.36 | 6.97 | 10.9 | 12 | 7.79 |
| JSON, 10 MB | 10,000 KB | 31.6 | 81.5 | 73.2 | 78.6 | 117 | 128 | 97.9 |
| ASCII, 1 MB | 1,000 KB | 0.151 | 0.411 | 0.369 | 0.376 | 3.63 | 3.69 | 2.21 |
| ASCII, 10 MB | 10,000 KB | 1.08 | 4.05 | 3.62 | 3.66 | 36.5 | 37.2 | 24.9 |
| non-ASCII, 1 MB | 1,000 KB | 1.8 | 4.91 | 2.6 | 3.16 | 5.39 | 6.19 | 2.31 |
| 100k numbers | 1,076 KB | 1.02 | 3.51 | 8.94 | 19.9 | 8.38 | 14.1 | 6.53 |
| bytes, 1 MiB | 1,049 KB | 0.19 | 0.143 | 0.139 | 0.158 | 0.085 | 0.088 | 0.091 |
| nested 100 deep | 4 KB | 0.02 | 0.036 | 0.028 | 0.032 | 0.051 | 0.053 | 0.092 |
| relative to djs | | 1.0× | 2.0× | 1.8× | 2.1× | 3.9× | 4.4× | 4.4× |

Milliseconds per value put into a fresh VM (median), each engine's best path; the last
row is the geometric mean of each engine's time over djs's; measured at d01ce5dc8f,
2026-11-06, load 2.1.
<!-- /gen -->

### Out

<!-- gen:status bridge-out -->
![Reading a value out of the VM, per payload and engine, in ms on a logarithmic scale](gen/chart-bridge-out.svg)

| payload | size | djs | quickjs-wasi | QuickJS | QuickJS-NG | quickjs-emscripten | quickjs-emscripten (NG) | monty |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| call arguments | 102 B | 0.001 | 0.003 | 0.002 | 0.002 | 0.004 | 0.005 | 0.022 |
| JSON, 1 KB | 1 KB | 0.002 | 0.017 | 0.011 | 0.015 | 0.036 | 0.044 | 0.026 |
| JSON, 100 KB | 100 KB | 0.131 | 1.31 | 0.904 | 1.21 | 2.76 | 3.4 | 0.528 |
| JSON, 1 MB | 1,000 KB | 1.23 | 12.6 | 8.51 | 12 | 28.8 | 35.1 | 5.13 |
| JSON, 10 MB | 10,000 KB | 13.2 | 130 | 89.6 | 123 | 285 | 350 | 52.2 |
| ASCII, 1 MB | 1,000 KB | 0.075 | 1.2 | 0.321 | 0.266 | 0.66 | 0.89 | 0.183 |
| ASCII, 10 MB | 10,000 KB | 0.135 | 11.4 | 2.64 | 2.59 | 6.45 | 8.79 | 1.4 |
| non-ASCII, 1 MB | 1,000 KB | 0.038 | 2.57 | 1.73 | 1.63 | 2.02 | 2.24 | 0.818 |
| 100k numbers | 1,076 KB | 0.379 | 8.56 | 17.9 | 19.3 | 53.1 | 99.8 | 8.8 |
| bytes, 1 MiB | 1,049 KB | 0.06 | 0.033 | 0.03 | 0.03 | 0.1 | 0.098 | 0.2 |
| nested 100 deep | 4 KB | 0.011 | 0.077 | 0.059 | 0.07 | 0.179 | 0.233 | 0.053 |
| relative to djs | | 1.0× | 10.6× | 6.7× | 7.7× | 17.9× | 22.5× | 7.3× |

Milliseconds per value read out of a VM (median), each engine's best path; the last row
is the geometric mean of each engine's time over djs's; measured at d01ce5dc8f,
2026-11-06, load 2.1.
<!-- /gen -->

### Throughput and memory

<!-- gen:status bridge-rate -->
| engine | JSON 1 MB, in | out | JSON 10 MB, in | out | ASCII 10 MB, in | out | non-ASCII 1 MB, in | out | bytes 1 MiB, in | out |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| djs | 320 | 813 | 317 | 758 | 9,233 | 74,074 | 555 | 26,596 | 5,507 | 17,505 |
| quickjs-wasi | 135 | 79 | 123 | 77 | 2,470 | 875 | 204 | 389 | 7,328 | 32,067 |
| QuickJS | 157 | 118 | 137 | 112 | 2,761 | 3,789 | 385 | 578 | 7,527 | 34,953 |
| QuickJS-NG | 144 | 84 | 127 | 82 | 2,733 | 3,866 | 316 | 614 | 6,637 | 34,380 |
| quickjs-emscripten | 92 | 35 | 86 | 35 | 274 | 1,551 | 186 | 494 | 12,395 | 10,486 |
| quickjs-emscripten (NG) | 83 | 29 | 78 | 29 | 269 | 1,137 | 162 | 446 | 11,956 | 10,656 |
| monty | 128 | 195 | 102 | 192 | 402 | 7,128 | 432 | 1,222 | 11,460 | 5,251 |

MB per second: the payload's size over the median time; measured at d01ce5dc8f,
2026-11-06, load 2.1.
<!-- /gen -->

<!-- gen:status bridge-memory -->
| engine | idle | JSON 10 MB in | and out | ASCII 10 MB in | and out |
| --- | ---: | ---: | ---: | ---: | ---: |
| djs | 0.6 | 44.6 | 44.6 | 10.1 | 10.1 |
| quickjs-wasi | 1.3 | 42.2 | 52.0 | 20.4 | 20.4 |
| QuickJS | 0.3 | 55.8 | 55.8 | 38.5 | 38.5 |
| QuickJS-NG | 0.4 | 60.4 | 60.4 | 38.5 | 38.5 |
| quickjs-emscripten | 16.0 | 42.0 | 83.1 | 24.2 | 24.2 |
| quickjs-emscripten (NG) | 16.0 | 50.4 | 83.1 | 24.2 | 24.2 |
| monty | 5.0 | 82.6 | 108.6 | 14.5 | 32.6 |

Linear memory, MiB: of a fresh VM, after putting the value in, and after reading it back
out; measured at d01ce5dc8f, 2026-11-06, load 2.1.
<!-- /gen -->

### End to end

Three whole runs, each in a fresh VM, with a host function `tool` that every engine calls
synchronously: a 1 MB search result of which the script returns one path, a 1 MB file
that the script edits with one `replace` and passes to a second call, and a thousand calls
with small arguments and results.

<!-- gen:status bridge-e2e -->
![Whole runs that pass data through a tool, per engine, in ms on logarithmic scales](gen/chart-bridge-e2e.svg)

| case | djs | quickjs-wasi | QuickJS | QuickJS-NG | quickjs-emscripten | quickjs-emscripten (NG) | monty |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| one field of 1 MB | 3.26 | 7.42 | 6.69 | 7.45 | 11.8 | 12.9 | 29.2 |
| edit a 1 MB file | 0.295 | 1.69 | 4.02 | 5.66 | 11.8 | 11.7 | 2.66 |
| 1,000 small calls | 3.04 | 3.07 | 1.68 | 2.07 | 5.22 | 5.57 | 20 |

Milliseconds per run in a fresh VM (median): compiling and running the script, its tool
calls through each engine's host function; measured at d01ce5dc8f, 2026-11-06, load 2.1.
<!-- /gen -->

## Dumps

djs can write a run that waits on tool calls as a [continuation dump](dumps.md) and
continue it later in a new sandbox. Here every workload runs through the sandbox with
asynchronous host tools and is dumped at every idle point, every time it waits on a call:

<!-- gen:status engines-dumps -->
![djs's continuation dumps per workload: size, and the time to write and to load one](gen/chart-engines-dumps.svg)

| workload | idle points | dump, KiB (max) | gzip, KiB (max) | write, ms | load, ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| sequential | 4 | 1.2 (2.3) | 0.6 (0.9) | 0.31 | 0.34 |
| fanout | 15 | 74.3 (139.1) | 17.1 (33.5) | 0.56 | 0.66 |
| media | 2 | 0.4 (0.4) | 0.3 (0.3) | 0.14 | 0.24 |
| inspect | 0 | | | | |
| loop | 8 | 8.6 (15.5) | 3.1 (5.2) | 0.48 | 0.37 |
| text | 5 | 52.2 (53.0) | 9.2 (9.4) | 0.81 | 1.01 |
| json | 3 | 121.9 (122.0) | 20.5 (20.6) | 4.21 | 3.06 |
| ts | 1 | 1.7 (1.7) | 0.8 (0.8) | 0.77 | 0.44 |
| heavy | 97 | 42.1 (114.9) | 10.3 (25.2) | 0.53 | 0.73 |

Medians over a workload's idle points; load: a new sandbox with the run loaded; measured
at d01ce5dc8f, 2026-11-06, load 1.7.
<!-- /gen -->

Most dumps are a few kilobytes, and gzip takes the larger ones to between a fifth and a
third; they hold the run's live state, so a dump taken while `json` holds 600 query
rows, or while `fanout` holds the file texts it has read, is as large as that data.
Writing one and loading it into a new sandbox takes about as long as starting a VM and
running a small body; the `json` dumps take longest, as they encode and decode the most
objects. `inspect` makes no tool call, so it never waits and has nothing to dump.

## Re-running

```sh
scripts/bench-engines.sh          # fetch and build the engines, run, print the tables
scripts/bench-engines.sh --setup  # fetch and build only
scripts/status.py engines         # three runs; record the medians for this page
cargo run -q -p djs-docs -- regen # rewrite the tables
```

The engines go into `vendor/engines` of the primary checkout: npm packages pinned with
their integrity hashes by
[`js/bench/engines/package-lock.json`](../../js/bench/engines/package-lock.json), wasi-sdk
and the QuickJS release sources pinned by SHA-256 in
[`scripts/bench-engines.sh`](../../scripts/bench-engines.sh). Run it on a quiet machine:
the harness prints the load it started at, and recorded numbers should come from a load
below about 4.

## Where to look

- [`js/bench/engines/run.ts`](../../js/bench/engines/run.ts): the harness, what it times and how
- [`js/bench/engines/engines.ts`](../../js/bench/engines/engines.ts): each engine behind one interface
- [`js/bench/engines/prelude.js`](../../js/bench/engines/prelude.js),
  [`prelude.py`](../../js/bench/engines/prelude.py): the stub tools inside the engines
- [`tests/bench/codemode/`](../../tests/bench/codemode/): the workloads, their Python
  translations and [`tools.ts`](../../tests/bench/codemode/tools.ts), the invented tool data
- [`scripts/bench-engines.sh`](../../scripts/bench-engines.sh): fetching and building the engines
