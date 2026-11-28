# Benchmarks

How fast does djs start and run the scripts an agent writes, and what does it cost
in download and memory, next to the QuickJS builds a host would otherwise embed? This
page compares them on workloads modeled on real agent codemode sessions, all in
WebAssembly, the way a host runs them, and says how to re-run every number.

> **In short.** Seven engines (djs, four QuickJS builds of both QuickJS lines, and monty,
> a Python subset) run nine codemode workloads in wasm in one node process, each run in a
> fresh VM, and must all return the same result. djs starts a VM fastest and has the
> lowest run time over the set; it is ahead where scripts move JSON, strings and tool
> results, and behind on the smallest bodies and on some single string and number builtins. Passing
> data through the host bridge, each engine its best way, djs takes values in about twice
> as fast and reads them out six to nine times as fast; it is behind on bytes coming in and
> on many tiny calls. Its
> module is the largest of the JavaScript engines, it takes about a tenth more memory
> than the QuickJS releases built here over the set. `scripts/bench-engines.sh` fetches,
> builds and runs everything; the tables and charts are generated from the recorded
> results.

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
  host interfaces. [Passing data through the bridge](#passing-data-through-the-bridge)
  compares the host interfaces.
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
| quickjs-wasi | QuickJS-NG for WASI, which can save and restore its memory, version 3.6.2 | the npm package's `quickjs.wasm` |
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
| djs | 995 KB | 431 KB | 364 KB | none |
| quickjs-wasi | 637 KB | 287 KB | 247 KB | 28 KB |
| QuickJS | 966 KB | 332 KB | 262 KB | none |
| QuickJS-NG | 1,087 KB | 371 KB | 295 KB | none |
| quickjs-emscripten | 503 KB | 231 KB | 199 KB | 8 KB |
| quickjs-emscripten (NG) | 529 KB | 247 KB | 212 KB | 8 KB |
| monty | 24,593 KB | 7,628 KB | 4,804 KB | 55 KB |

Module bytes as shipped; gzip -9, brotli quality 11; measured at c81310b606, 2026-11-21;
d0bc2eef16, 2026-11-21, load 3.1.
<!-- /gen -->

djs's module is the largest of the JavaScript engines: it carries the TypeScript front
end and the base image of an initialized realm (every builtin object, coded to
<!--n wasm/base-coded kb-->45 KB<!--/n-->), which spares each new VM from building them.
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
| djs | 1.14 | 0.07 | 0.4 |
| quickjs-wasi | 5.27 | 0.24 | 1.3 |
| QuickJS | 5.16 | 0.20 | 0.3 |
| QuickJS-NG | 1.99 | 0.26 | 0.4 |
| quickjs-emscripten | 1.15 | 0.24 | 16.0 |
| quickjs-emscripten (NG) | 6.82 | 0.28 | 16.0 |
| monty | 31.7 | 0.56 | 3.9 |

Compile: the module, once per process. New VM: a new instance and VM, ready to evaluate
(median). Memory: linear memory; measured at d0bc2eef16, 2026-11-21, load 3.1.
<!-- /gen -->

A new djs VM is ready in <!--n engines/djs/startup-ms .2-->0.07<!--/n--> ms, against
<!--n engines/quickjs/startup-ms .2-->0.20<!--/n--> ms for QuickJS and
<!--n engines/quickjs-ng/startup-ms .2-->0.26<!--/n--> ms for QuickJS-NG: the realm
comes from the base image in the module, so creating a VM copies an image instead of
running the code that builds every builtin (the [images page](images.md) says
how). Its idle memory equals QuickJS-NG's and is one 64 KiB page above QuickJS's: the
base image is larger than the heap a fresh QuickJS context allocates. The emscripten
builds start at 16 MiB, emscripten's initial heap, whatever runs in them.

## Run time

The time from handing a fresh VM the script and its tool answers to having the result:
parsing and compiling the body, the tool calls through the bridge, and the script's work.

<!-- gen:status engines-run -->
![Run time per workload and engine, in ms on a logarithmic scale](gen/chart-engines-run.svg)

| workload | djs | quickjs-wasi | QuickJS | QuickJS-NG | quickjs-emscripten | quickjs-emscripten (NG) | monty |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| sequential | 0.38 | 0.37 | 0.37 | 0.38 | 0.41 | 0.37 | 0.61 |
| fanout | 0.84 | 1.57 | 1.22 | 1.13 | 1.78 | 1.65 | 1.43 |
| media | 0.20 | 0.47 | 0.91 | 0.43 | 1.49 | 0.98 | 0.81 |
| inspect | 0.24 | 0.23 | 0.20 | 0.19 | 0.29 | 0.31 | 0.65 |
| loop | 0.70 | 2.60 | 0.83 | 2.10 | 1.11 | 4.38 | 1.66 |
| text | 1.24 | 1.65 | 1.49 | 1.44 | 1.90 | 2.39 | 2.90 |
| json | 1.39 | 2.12 | 1.79 | 1.88 | 2.65 | 2.74 | 3.54 |
| ts | 1.75 | 2.31 | 1.97 | 2.10 | 2.94 | 2.99 | 2.74 |
| heavy | 8.87 | 15.6 | 15.0 | 16.3 | 19.0 | 29.4 | 11.9 |
| relative to djs | 1.00× | 1.60× | 1.38× | 1.42× | 1.90× | 2.27× | 2.12× |

Milliseconds per run (median), in a warm process; the last row is the geometric mean of
each engine's time over djs's; measured at d0bc2eef16, 2026-11-21, load 3.1.
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
| sequential | 0.5 | 1.4 | 0.4 | 0.4 | 16.0 | 16.0 | 5.1 |
| fanout | 1.4 | 2.5 | 1.6 | 1.6 | 16.0 | 16.0 | 5.6 |
| media | 0.8 | 2.1 | 1.1 | 1.1 | 16.0 | 16.0 | 5.5 |
| inspect | 0.4 | 1.3 | 0.4 | 0.4 | 16.0 | 16.0 | 5.1 |
| loop | 0.6 | 1.5 | 0.5 | 0.6 | 16.0 | 16.0 | 5.1 |
| text | 0.9 | 1.6 | 0.6 | 0.7 | 16.0 | 16.0 | 7.1 |
| json | 1.2 | 2.2 | 1.1 | 1.3 | 16.0 | 16.0 | 5.9 |
| ts | 1.3 | 2.2 | 1.1 | 1.2 | 16.0 | 16.0 | 5.8 |
| heavy | 3.9 | 3.4 | 2.4 | 2.4 | 16.0 | 16.0 | 7.2 |
| relative to djs | 1.00× | 1.99× | 0.89× | 0.94× | 16.45× | 16.45× | 5.92× |

Linear memory after the run, MiB; the last row is the geometric mean of each engine's
over djs's; measured at d0bc2eef16, 2026-11-21, load 3.1.
<!-- /gen -->

Over the set djs needs about a tenth more memory than the QuickJS releases built here,
and half of quickjs-wasi's or less, a fraction of monty's and the emscripten builds'. Where
a body holds large tool results (`fanout`, `media`) djs needs less than the QuickJS
releases; on the small bodies it is level or a page above, the idle footprint. The gap
on `text` and `heavy` comes from the garbage collector. djs's collector traces the live
heap when the allocation since the last collection crosses a threshold, about as large
again as what survived, so garbage stays until then; QuickJS counts references and frees
most garbage the moment it is dropped. `heavy`, which allocates a few megabytes of
short-lived lines and strings, shows it most.

## Operation by operation

Single operations on data shaped like the workloads', each in a loop, against the faster
of the two QuickJS releases. Above 1, djs is slower.

<!-- gen:status engines-ops -->
![djs's time over the best QuickJS build's, per operation, on a logarithmic scale around parity](gen/chart-engines-ops.svg)

| operation | djs, ms | QuickJS, ms | djs / QuickJS |
| --- | ---: | ---: | ---: |
| `JSON.parse` of 600 query rows | 1.10 | 3.00 | 0.37 |
| `JSON.stringify` of 600 query rows | 0.94 | 3.11 | 0.30 |
| `split("\n")` of file texts | 1.90 | 2.84 | 0.67 |
| `join("\n")` of their lines | 0.61 | 0.88 | 0.69 |
| `includes` per line | 2.15 | 2.60 | 0.83 |
| template literal appends | 4.85 | 4.45 | 1.09 |
| `trim` | 4.45 | 3.78 | 1.18 |
| `slice` | 4.56 | 3.98 | 1.15 |
| `padStart` | 3.95 | 3.12 | 1.27 |
| `toLowerCase` | 5.18 | 10.6 | 0.49 |
| `charCodeAt` loop | 7.39 | 6.48 | 1.14 |
| `String(i)` | 4.23 | 3.45 | 1.23 |
| `toFixed` | 3.62 | 3.18 | 1.14 |
| `Math.max` | 0.54 | 0.56 | 0.96 |
| `Math.floor` | 2.65 | 2.82 | 0.94 |
| `match(/function \w+/g)` | 1.78 | 5.25 | 0.34 |
| `replace(/\s+/g, " ")` | 7.91 | 8.19 | 0.97 |
| `exec` of a line pattern | 9.90 | 13.7 | 0.73 |
| `matchAll` with `/gm` | 3.67 | 5.49 | 0.67 |
| `new RegExp` | 1.60 | 1.90 | 0.84 |
| `map` | 3.07 | 3.58 | 0.86 |
| `sort` with a comparator | 7.61 | 6.03 | 1.26 |
| `sort` of strings | 2.27 | 2.43 | 0.93 |
| `push` | 8.05 | 7.87 | 1.02 |
| `Map.set` with string keys | 15.6 | 10.2 | 1.53 |
| `Set.add` with string keys | 9.74 | 6.92 | 1.41 |
| object spread | 1.07 | 4.50 | 0.24 |
| `Object.entries` and `fromEntries` | 0.00 | 0.11 | - |
| `await` of a value | 5.95 | 17.1 | 0.35 |
| `Promise.all` of 20 | 4.49 | 8.01 | 0.56 |

Milliseconds per loop of the operation (js/bench/engines/ops.ts), the setup's time
subtracted; QuickJS: the faster of the two latest releases; measured at d0bc2eef16,
2026-11-21, load 2.9.
<!-- /gen -->

JSON, splitting and joining, `toLowerCase`, regular expression matching, `includes`,
`map`, `new RegExp`, object spread, `await` and `Promise.all` are faster in djs;
`replace`, sorting strings, `push`, `Math.max` and `Math.floor` are at about parity. The
hot string and number builtins run as Rust that is generated from their TypeScript
source, so what is left of the gap in `trim`, `slice` and the `charCodeAt` loop is
mostly the call itself:
finding the function and leaving the interpreter's loop to run it cost more than a call
of QuickJS's C functions, which shows when the operation is tiny. `String(i)`,
`toFixed` and template literals come within about a quarter of QuickJS: integers are
written without the floating-point formatter and `toFixed` of an ordinary number
without big-integer arithmetic, and what remains is mostly allocating the short result
string. `padStart` here pads `String(i)`, so it pays for that, a second call and a
second string. `sort` with a comparator
runs the comparator, a script function, for every comparison, and calling it from the
sort still costs more than in QuickJS; `Map.set` and `Set.add` with string keys also
take more instructions per call. `Object.entries` and `fromEntries` are a fraction of a
millisecond in both. These matter in a loop of tens of thousands of calls; in the
workloads they are a small part of the time.

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

Where an engine has two paths, each number below is the faster one's: the handle walks of
quickjs-wasi and quickjs-emscripten win on arrays of numbers, whose decimal text is slow
to parse, and lose on most other values, and monty's JSON text beats its value arena for
anything larger than call arguments. The payloads: call
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
| JSON, 1 KB | 1 KB | 0.006 | 0.009 | 0.008 | 0.008 | 0.015 | 0.022 | 0.053 |
| JSON, 100 KB | 100 KB | 0.317 | 0.763 | 0.645 | 0.699 | 1.09 | 1.17 | 0.828 |
| JSON, 1 MB | 1,000 KB | 3.14 | 7.35 | 6.4 | 6.95 | 11 | 11.9 | 7.86 |
| JSON, 10 MB | 10,000 KB | 31.5 | 81.6 | 72.6 | 78.4 | 118 | 128 | 98.4 |
| ASCII, 1 MB | 1,000 KB | 0.141 | 0.413 | 0.382 | 0.387 | 3.7 | 3.71 | 2.26 |
| ASCII, 10 MB | 10,000 KB | 1.21 | 4.19 | 3.79 | 3.84 | 36.6 | 37 | 23.8 |
| non-ASCII, 1 MB | 1,000 KB | 1.96 | 5.04 | 2.67 | 3.27 | 5.41 | 6.16 | 2.31 |
| 100k numbers | 1,076 KB | 1.01 | 3.45 | 8.82 | 19.5 | 8.43 | 13.8 | 6.54 |
| bytes, 1 MiB | 1,049 KB | 0.232 | 0.14 | 0.134 | 0.148 | 0.084 | 0.088 | 0.089 |
| nested 100 deep | 4 KB | 0.02 | 0.036 | 0.029 | 0.032 | 0.051 | 0.053 | 0.088 |
| relative to djs | | 1.0× | 2.0× | 1.8× | 2.0× | 3.8× | 4.2× | 4.2× |

Milliseconds per value put into a fresh VM (median), each engine's best path; the last
row is the geometric mean of each engine's time over djs's; measured at d0bc2eef16,
2026-11-21, load 3.3.
<!-- /gen -->

djs takes JSON-like values in about half the time of the fastest QuickJS path: a 1 MB
search result in <!--n engines/bridge/djs/json-1m/in-ms .1-->3.1<!--/n--> ms against
<!--n engines/bridge/quickjs/json-1m/in-ms .1-->6.4<!--/n--> ms for QuickJS. The host writes the value once in the wire
format and the VM builds every object at its final size in one pass, where the others
serialize it to JSON text on the host and parse that text again in the VM. A long ASCII
string is one copy into a string the VM reserved for it (<!--n engines/bridge/djs/text-10m/in-ms .1-->1.2<!--/n--> ms for
10 MB, against <!--n engines/bridge/quickjs/text-10m/in-ms .1-->3.8<!--/n--> ms for QuickJS, which first encodes the text into
its linear memory and then copies it into a string). Text with non-ASCII characters
gains less: it is encoded to UTF-8 and transcoded back in the VM. Numbers gain as much,
since the wire format carries them as binary values instead of decimal text to be parsed.

Bytes are where djs is behind: a `Uint8Array` is copied into the encoder's buffer, then
into the staging buffer, then into the VM's own bytes, three copies where the others
make two. The emscripten builds pay the most for strings, because their glue
encodes them to UTF-8 in JavaScript, byte by byte.

### Out

<!-- gen:status bridge-out -->
![Reading a value out of the VM, per payload and engine, in ms on a logarithmic scale](gen/chart-bridge-out.svg)

| payload | size | djs | quickjs-wasi | QuickJS | QuickJS-NG | quickjs-emscripten | quickjs-emscripten (NG) | monty |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| call arguments | 102 B | 0.001 | 0.004 | 0.002 | 0.002 | 0.004 | 0.005 | 0.022 |
| JSON, 1 KB | 1 KB | 0.003 | 0.017 | 0.011 | 0.015 | 0.036 | 0.044 | 0.027 |
| JSON, 100 KB | 100 KB | 0.128 | 1.31 | 0.901 | 1.21 | 2.76 | 3.4 | 0.531 |
| JSON, 1 MB | 1,000 KB | 1.24 | 12.6 | 8.58 | 12.1 | 28.8 | 35 | 5.21 |
| JSON, 10 MB | 10,000 KB | 13.3 | 128 | 90.4 | 123 | 287 | 354 | 52.5 |
| ASCII, 1 MB | 1,000 KB | 0.075 | 1.2 | 0.322 | 0.265 | 0.675 | 0.909 | 0.216 |
| ASCII, 10 MB | 10,000 KB | 0.146 | 11.5 | 2.64 | 2.65 | 6.47 | 8.94 | 1.4 |
| non-ASCII, 1 MB | 1,000 KB | 0.124 | 2.58 | 1.76 | 1.72 | 2.01 | 2.27 | 0.814 |
| 100k numbers | 1,076 KB | 0.459 | 8.07 | 17.9 | 19.5 | 52.8 | 101 | 8.83 |
| bytes, 1 MiB | 1,049 KB | 0.073 | 0.033 | 0.03 | 0.03 | 0.092 | 0.095 | 0.201 |
| nested 100 deep | 4 KB | 0.011 | 0.077 | 0.059 | 0.07 | 0.18 | 0.233 | 0.053 |
| relative to djs | | 1.0× | 9.1× | 5.8× | 6.6× | 15.3× | 19.4× | 6.4× |

Milliseconds per value read out of a VM (median), each engine's best path; the last row
is the geometric mean of each engine's time over djs's; measured at d0bc2eef16,
2026-11-21, load 3.3.
<!-- /gen -->

Reading out is where the designs differ most. djs's reader decodes the value straight out
of the VM's memory, and the VM does no work at all; everyone else first has the VM write
the value as JSON text, copies the text out and parses it on the host. A 1 MB search
result comes out of djs in <!--n engines/bridge/djs/json-1m/out-ms .2-->1.24<!--/n--> ms, against
<!--n engines/bridge/quickjs/json-1m/out-ms .1-->8.6<!--/n--> ms for QuickJS and <!--n engines/bridge/monty/json-1m/out-ms .1-->5.2<!--/n--> ms for monty, whose
`json.dumps` is the fastest of the JSON paths here. A string comes out at the speed of a
memory copy, since its characters are already in the form a host string needs. A
`Uint8Array` is copied out in all of them; the reader's copy costs a few hundredths of a
millisecond more than reading the bytes in place by a pointer.

### Throughput and memory

<!-- gen:status bridge-rate -->
| engine | JSON 1 MB, in | out | JSON 10 MB, in | out | ASCII 10 MB, in | out | non-ASCII 1 MB, in | out | bytes 1 MiB, in | out |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| djs | 319 | 807 | 317 | 753 | 8,256 | 68,587 | 510 | 8,045 | 4,522 | 14,404 |
| quickjs-wasi | 136 | 80 | 123 | 78 | 2,388 | 869 | 199 | 388 | 7,495 | 31,969 |
| QuickJS | 156 | 117 | 138 | 111 | 2,641 | 3,789 | 375 | 569 | 7,837 | 34,953 |
| QuickJS-NG | 144 | 83 | 127 | 81 | 2,601 | 3,767 | 306 | 582 | 7,104 | 34,493 |
| quickjs-emscripten | 91 | 35 | 85 | 35 | 273 | 1,546 | 185 | 498 | 12,424 | 11,361 |
| quickjs-emscripten (NG) | 84 | 29 | 78 | 28 | 270 | 1,118 | 162 | 441 | 11,970 | 11,049 |
| monty | 127 | 192 | 102 | 190 | 420 | 7,127 | 432 | 1,229 | 11,782 | 5,230 |

MB per second: the payload's size over the median time; measured at d0bc2eef16,
2026-11-21, load 3.3.
<!-- /gen -->

<!-- gen:status bridge-memory -->
| engine | idle | JSON 10 MB in | and out | ASCII 10 MB in | and out |
| --- | ---: | ---: | ---: | ---: | ---: |
| djs | 0.4 | 42.2 | 42.2 | 14.6 | 14.6 |
| quickjs-wasi | 1.3 | 42.2 | 52.0 | 20.4 | 20.4 |
| QuickJS | 0.3 | 36.7 | 56.6 | 19.4 | 19.4 |
| QuickJS-NG | 0.4 | 41.3 | 51.1 | 19.4 | 19.4 |
| quickjs-emscripten | 16.0 | 42.0 | 83.1 | 24.2 | 24.2 |
| quickjs-emscripten (NG) | 16.0 | 50.4 | 83.1 | 24.2 | 24.2 |
| monty | 5.0 | 82.6 | 108.6 | 14.5 | 32.6 |

Linear memory, MiB: of a fresh VM, after putting the value in, and after reading it back
out; measured at d0bc2eef16, 2026-11-21, load 3.3.
<!-- /gen -->

The memory after putting a 10 MB value in is mostly the value itself in each engine's
representation. djs keeps a little more than QuickJS for JSON: its staging buffer stays
as large as the largest wire encoding it was given. Reading the value out adds nothing
in djs, while the engines that go through JSON text need room for the text, which in the
emscripten builds doubles their memory. A 10 MB ASCII string takes about its own size in
djs, one copy; in QuickJS it is there twice, as the UTF-8 the host wrote and as the
string.

### End to end

Three whole runs, each in a fresh VM, with a host function `tool` that every engine calls
synchronously: a 1 MB search result of which the script returns one path, a 1 MB file
that the script edits with one `replace` and passes to a second call, and a thousand calls
with small arguments and results.

<!-- gen:status bridge-e2e -->
![Whole runs that pass data through a tool, per engine, in ms on logarithmic scales](gen/chart-bridge-e2e.svg)

| case | djs | quickjs-wasi | QuickJS | QuickJS-NG | quickjs-emscripten | quickjs-emscripten (NG) | monty |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| one field of 1 MB | 3.26 | 7.36 | 6.69 | 7.45 | 11.8 | 12.8 | 29.8 |
| edit a 1 MB file | 0.37 | 1.7 | 4.04 | 5.56 | 11.8 | 11.8 | 2.89 |
| 1,000 small calls | 3.22 | 3.05 | 1.69 | 2.05 | 5.28 | 5.6 | 20.1 |

Milliseconds per run in a fresh VM (median): compiling and running the script, its tool
calls through each engine's host function; measured at d0bc2eef16, 2026-11-21, load 3.3.
<!-- /gen -->

The case of a 1 MB result of which the script reads one field is decided by putting the
result in: every engine, djs included, builds the whole value before the script runs, so
djs's lead is that of the table above, and reading one string out costs nothing anywhere.
Editing a 1 MB file shows both directions: the file goes in as one copy, and the edited
text comes out to the second call through the reader, in
<!--n engines/bridge/djs/e2e/edit-1m-ms .2-->0.37<!--/n--> ms against <!--n engines/bridge/quickjs/e2e/edit-1m-ms .1-->4.0<!--/n--> ms for QuickJS, whose host function has to
write the megabyte as JSON text inside the VM and parse it again on the host.

A thousand small calls are where djs is behind the minimal C embedding:
<!--n engines/bridge/djs/e2e/calls-1000-ms .1-->3.2<!--/n--> ms against <!--n engines/bridge/quickjs/e2e/calls-1000-ms .1-->1.7<!--/n--> ms. With small values the conversion is
cheap everywhere, and what remains is the call itself: djs's run returns to the host at
every call, the host reads the call's record, puts the result in and runs the VM again,
while the C embedding calls the host function from inside the interpreter (the other
QuickJS builds do too, but every value crosses through their JavaScript glue, and they
are slower than djs). monty stops and resumes its run for every call, which costs it <!--n engines/bridge/monty/e2e/calls-1000-ms .0-->20<!--/n--> ms.

In short, djs's two halves pay off differently. The wire format makes the way in about
twice as fast as JSON text for structured data and a single copy for long ASCII strings.
The reader makes the way out six to nine times as fast for structured data and more for
strings, because no engine work and no text happen in between. Bytes coming in and many tiny calls are the places where the
simpler paths of the C embedding are still ahead.

## Re-running

```sh
scripts/bench-engines.sh          # fetch and build the engines, run, print the tables
scripts/bench-engines.sh --setup  # fetch and build only
scripts/status.py engines         # three runs; record the medians for this page
scripts/status.py bridge          # only the bridge benchmark
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
- [`js/bench/engines/engines.ts`](../../js/bench/engines/engines.ts): each engine behind one
  interface, with its paths in and out for the bridge benchmark
- [`js/bench/engines/bridge.ts`](../../js/bench/engines/bridge.ts): the bridge benchmark, its
  payloads and end-to-end cases; [`driver.c`](../../js/bench/engines/driver.c): the C embedding
  of the two QuickJS releases
- [`js/bench/engines/prelude.js`](../../js/bench/engines/prelude.js),
  [`prelude.py`](../../js/bench/engines/prelude.py): the stub tools inside the engines
- [`tests/bench/codemode/`](../../tests/bench/codemode/): the workloads, their Python
  translations and [`tools.ts`](../../tests/bench/codemode/tools.ts), the invented tool data
- [`scripts/bench-engines.sh`](../../scripts/bench-engines.sh): fetching and building the engines
