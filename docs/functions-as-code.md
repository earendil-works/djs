# Functions as code: dumps

Status: built (issue 0167). `Vm::dump` in djs-core (feature `dump`), the REPL's `.dump`
and `.restore`, a djs-wasm export (feature `dump`, off by default) behind
`Sandbox.dump`/`Sandbox.load`, and a TypeScript prototype of the same dumper over the heap
bytes (js/proto) for comparison. Decisions: D2352 (wasm size), D2353 (the format and its
promise), D2354 (which dumper a wasm host uses).

Next: format 2, continuation dumps, which also carry suspended activations and replace heap
images as persistence; the TypeScript dumper is dropped (D2560-D2565,
docs/continuation-dumps.md).

The owner's question: a harness builds functions in one session and wants them back later,
"even after weeks, where heap snapshots are potentially no longer usable", with captured
state that is often data pulled by codemode and sometimes binary, and the persisted form
readable.

## The short answer

- **A dump is a JavaScript Script that recreates the bindings when run.** Not an image and
  not a serialization format that needs a reader: functions are their own source text,
  data are literals and constructor calls, builtins are referenced by name. Loading a dump
  is running it in a fresh VM. Any later djs runs it, because it only uses the language
  and the standard builtins.
- **Builtins are never written.** `Math.max` is written `Math.max`; the few builtins no
  global path reaches (`%ArrayIteratorPrototype%`) go through a ten-line resolver the dump
  carries. Builtins the session changed (`Array.prototype.sum = ...`, `delete Math.hypot`)
  are written as the code that changes them; with frozen builtins there are none.
- **Closures stay closures.** Closures that share captured variables are made by one
  factory per group, with the variables' current values and the functions' verbatim text;
  sharing stays shared and nothing in the user's code is rewritten.
- **Current state, by default and only.** A dump captures values at the time of the dump.
  History (the scripts that built them) is the job of the host's journal.
- **What cannot be written is refused, by path:** `p: a pending promise`,
  `lib.run~cache: a Proxy`. Nothing is silently dropped.
- **Measured:** every one of the 300 harvested codemode cases and 2,607 of 2,627 private
  pi scripts (the rest do not compile or run in the harness, or end with a closure over a
  binding still in its TDZ) dump, load into a fresh VM and print the same bindings. A
  dump written today loads in a build 800 commits older and gives the same answers.
- **Size:** the Rust dumper adds 35.6 KB gzip to the wasm module (8.0%), so djs-wasm
  leaves it out by default. The TypeScript prototype costs 13.7 KB gzip of JS and no wasm,
  produces the same text, and is twice as fast on V8. Recommendation (D2354): the Rust
  dumper is the reference (tests, the native CLI); a wasm host such as pi uses the
  TypeScript one, cross-checked against Rust on the same cases.

## What a dump looks like

The committed fixture `tests/dump/v1.js`, from this session:

```js
function makeCounter(start) { let n = start; return { inc: () => ++n, get: () => n }; }
const counter = makeCounter(40); counter.inc();
const bytes = new Uint8Array(100).map((_, i) => i * 3);
const small = new Int16Array([-1, 2, 300]);
const key = { k: 1 };
const index = new Map([[key, 'object key'], ['when', new Date(1700000000000)]]);
const graph = { name: 'root', children: [] }; graph.children.push({ parent: graph });
const frozen = Object.freeze({ list: [1, 2] });
const tag = Symbol.for('app.tag');
const re = /ab+c/gi;
const iterProto = Object.getPrototypeOf([][Symbol.iterator]());
var legacy = { [tag]: 'tagged', big: 12345678901234567890n };
Array.prototype.last = function () { return this[this.length - 1]; };
```

is (helpers shortened):

```js
// djs-dump 1: 12 bindings (counter, bytes, small, key, index, graph, frozen, tag, re, iterProto, legacy, makeCounter).
// Run it as a Script to recreate them (djs REPL: .restore FILE). Builtins are
// referenced by name, never written.
globalThis.$dump = {};
$dump.bytes = (s) => { /* base64 -> ArrayBuffer with atob */ };
$dump.builtin = (path) => { /* resolves "%ArrayIteratorPrototype%", "get Map.prototype.size", ... */ };
// inc, get sharing n
$dump.s1 = (() => {
  let n = 41;
  return { inc: () => ++n, get: () => n };
})();
const counter = { inc: $dump.s1.inc, get: $dump.s1.get };
const bytes = new Uint8Array($dump.bytes("AAMGCQwPEhUYGx4h..."));
const small = new Int16Array([-1, 2, 300]);
const key = { k: 1 };
const index = new Map([[key, "object key"], ["when", new Date(1700000000000)]]);
$dump.n3 = { parent: undefined };
$dump.n2 = [$dump.n3];
const graph = { name: "root", children: $dump.n2 };
const frozen = { list: [1, 2] };
const tag = Symbol.for("app.tag");
const re = /ab+c/gi;
const iterProto = $dump.builtin("%ArrayIteratorPrototype%");
var legacy = { big: 12345678901234567890n, [tag]: "tagged" };
function makeCounter(start) { let n = start; return { inc: () => ++n, get: () => n }; }
// Builtins the session changed.
Array.prototype.last = function () { return this[this.length - 1]; };
// Links that close cycles, and attributes.
$dump.n3.parent = graph;
Object.freeze(frozen);
void delete globalThis.$dump;
```

The roots are top-level declarations of the Script, as the session had them: the global
lexical record's bindings as `let`/`const`, the global object's own properties as `var`
(or the function declaration itself, when its text declares that name). `$dump` is a
temporary namespace for the helpers and for objects that are referenced twice or sit on a
cycle; it is deleted at the end, and a dump that needs none of it has none.

## How a dump is written

The dumper walks the graph from the selected bindings breadth-first, recording every value
it reaches with its path (`lib.run~cache`, `index<entries>[1]`, `x.[[Prototype]]`), how
often it is referenced, and its edges. Then it finds cycles (Tarjan's strongly connected
components) and groups of closures that share captured variables (union-find over the
closures and the variable boxes they capture), and writes each root in order, inlining
what is referenced once and not on a cycle and naming the rest `$dump.nN`. Output follows
the walk's order (roots in order, keys in property order), so it is deterministic without
sorting.

### Builtins

Builtins are named by their canonical paths (D2371) on the **base image** the VM was made
from, not on the session's heap: the base's cells sit at the same offsets in every VM made
from it, so a cell below the base's end is a builtin and its path is the one the base
gives it, even if the session assigned it to a global of its own. A path made of
identifiers and property keys (`Array.prototype.map`, `Symbol.iterator`) is an expression;
the others (`%TypedArray%`, `get Map.prototype.size`, `Int8Array.[[Prototype]]`) are passed
to `$dump.builtin`, which knows an expression for each realm root that has no global
(`Object.getPrototypeOf([][Symbol.iterator]())` for `%ArrayIteratorPrototype%`). Internal
values of builtins (a prelude helper reached through a capture, `%intrinsic:...%`) are
refused: they are not part of the language, and their identity is not something a later
build can promise.

Builtins the session changed are found by comparing the heap's builtin prefix with the
base image (a byte compare first, then object by object): every changed, added or deleted
property is written as an assignment, `Object.defineProperty` or `delete`, a changed
extensibility as `Object.preventExtensions`, a changed prototype is refused. With frozen
builtins (D2304) the prefix cannot change and this step is skipped.

### Values

- Primitives as literals (`-0`, `NaN`, `12345678901234567890n`, strings with escapes for
  quotes, line terminators and lone surrogates).
- Ordinary objects as literals in property order. A non-default prototype becomes
  `__proto__:` (or a later `Object.setPrototypeOf` when it closes a cycle); accessors whose
  functions are written in place become `get x() {...}`; methods become `m() {...}`.
  Non-default attributes are patched after the data with `Object.defineProperty`, keeping
  the key order (a placeholder holds the place); `Object.freeze`, `seal` and
  `preventExtensions` come last, so the patches can still run.
- Arrays with holes and extra properties; Map and Set by their entries (entries that close a
  cycle are added afterwards, all of them in order); Date, RegExp (literal, or `new
  RegExp` when the source would not survive as a literal), wrappers (`Object(1)`),
  settled promises (`Promise.resolve(v)`, a rejected one through a helper that marks it
  handled), errors (`new TypeError(msg)` plus the `stack` the error had), bound functions
  (`f.bind(this, ...args)`).
- Symbols: well-known ones are builtins (`Symbol.iterator`), registered ones
  `Symbol.for(key)`, others `Symbol(description)`, made once when used twice.
- Binary data: an ArrayBuffer is `$dump.bytes("base64")`; views are constructor calls on
  their buffer (`new DataView(buf, 1, 2)`, `new Uint8Array(buf, 2, 2)`), so views of one
  buffer still share it. A small typed array that owns its whole buffer is written as its
  elements (`new Float64Array([1.5, -2])`).

### Functions and closures

A function is its source text (`Function.prototype.toString`'s slice of the script). A
function that captures nothing is written where it is used (`tag: (s) => lib.fmt(s)`).
Closures that capture variables are made by a **factory per group**, where a group is the
closures that share captured variables, transitively:

```js
// add, all sharing state, api
$dump.s1 = (() => {
  let state = $dump.n2;
  let api;
  return { add(x) { state.items.push(x); return api; }, all: () => state.items, $set(v) { ({ api } = v); } };
})();
const store = { add: $dump.s1.add, all: $dump.s1.all };
...
$dump.s1.$set({ api: store });
```

The variables get their current values; a value that is on a cycle through the closures
(here `api` is the object holding the closures) is set after the data through the
factory's `$set`. A closure stored in one of the group's own variables is declared under
it (`function fmt(x) {...}` inside the factory), so recursion and mutual references work
as written. The functions keep their names: a factory key or a `let` gives an arrow its
name as the original binding did, and a name the context does not give is patched with
`Object.defineProperty(f, "name", ...)`.

**Why factories and not environment objects.** The other design rewrites captured
variables into properties of an environment object (`env$1.count`), which needs a parser
and an identifier rewriter that understands scoping, `eval`-free code, shorthand
properties and destructuring, and turns the user's code into something else. Factories
keep every function's text verbatim, need no parser (the dumper only reads the compiler's
names for the captured slots), and are ordinary JavaScript a person or a model can read
and edit. The price is the indirection `counter = { inc: $dump.s1.inc }` and one group
per set of sharing closures.

User globals that a function reads or writes by name (found in its bytecode:
`get_global`, `put_global`, `typeof` of a global) are dumped with it, so dumping `lib` also
writes the `fmt` and `calls` it uses. Host globals (`console`, `tools`, ...) are never
written: values reachable from them (two levels deep) are referenced by name
(`tools.read`), and the loading VM's host provides them under the same names. A captured
variable that holds the host's value under the host's own name (a body's `tools`
parameter) is left to the global.

### The checker

A dump that loads but reads other bindings than the session did would be silent wrong
output, the worst result a dump can have (0168: a root that was itself a closure over a
parameter, `const blah2 = blah(2)`, came out as the bare text `() => a`). So after
writing, the dumper compiles the dump (with the lines that write no function cut down to
the names they declare: data cannot change what a name resolves to) and compares every
function it wrote with the original, by the compiler's own resolution (D2355):

- every variable the original captured is captured in the dump too (a variable of the
  factory it is written in), or is read as a global that holds it: a root holding the
  variable's box (`dump_values`), or a root or host global of the same name holding its
  current value;
- every global the original read or wrote stays a global (no factory variable hides it)
  and exists after loading if it existed before: a root, a host global or a builtin
  global (a name that was undefined before may stay undefined);
- the dump compiles.

Anything else refuses the dump with the function's path (`blah2: captures `a`, which is
not in scope where the dump writes the function`). The check costs a compile of the cut-down
dump (djs compiles about 30 MB/s natively), so mostly the size of the functions and
factories written.

### What is refused

Everything below fails the dump with one line per value, `path: reason`:

| value | why |
| --- | --- |
| a pending promise | its reactions are live state of the run |
| a suspended generator or async function | an activation, not a value (images and journals keep those) |
| a Proxy | its behavior is its handler's code at every operation |
| a host function the host does not re-bind by name | the host owns it |
| a class, a method that uses `super` | not written yet: a class needs its whole body and its static state; a `super` method its home object |
| an object with private fields | `#x` cannot be set from outside the class |
| a binding in its TDZ, or a closure over one | there is no value to write |
| a captured `this`, `arguments` or `new.target` | not a variable a factory can declare |
| a builtin's internal value, a builtin with no name | see "Builtins" |
| a builtin whose prototype was changed | not expressible as a patch the base's builtins accept |
| weak collections, WeakRef, FinalizationRegistry, resizable buffers, iterators | live or unordered state |
| two captured variables of the same name in one group | a factory has one scope |
| a function the checker finds reading another binding than before | see "The checker" |

## Current state, history and provenance

A dump is the state at the time of the dump. The alternative, keeping the scripts that
built the state and replaying them, is what the host's journal (D1730) already does, and
it has the opposite costs: replay re-runs tool calls (or needs their recorded results),
takes the scripts' time again, and a replayed session diverges where the engine changed
behavior; in exchange it can recreate everything, frames included. A dump needs neither
the tools nor the time and only holds values, which is what "functions to use again
later" means. Provenance belongs next to the dump, not in it: the host can store the ids
of the turns that defined each binding in its metadata (below), or append the defining
script as a comment for a reader, at the cost of the dump's size.

## Images are the fast path, dumps the durable form

The same build restores a session from its image in about a millisecond with everything
intact, including suspended activations. An image is refused when its dependency
descriptor (D2265) no longer matches. A dump is the form that outlives that: slower to
load (it compiles and runs), limited to values, readable and editable. A host keeps both:
the image while the build lasts, the dump for later builds, for review, and for edits.

## The host API

- **djs-core:** `Vm::dump(&DumpOptions { base, host }, names: Option<&[&str]>)` dumps the
  named globals (all of the VM's own when `None`: the global lexical record and the
  global object's properties the base does not have) plus the globals their functions
  read. `Vm::dump_values(opts, &[(name, value)])` dumps explicit values as `let` bindings;
  a value may be a captured variable's box (`Value::internal`), so a harness can dump a
  function scope's bindings and the closures that capture them refer to them by name.
  `Err(DumpError(lines))` lists every refusal. Feature `dump` (default on; image-neutral,
  it only reads the heap).
- **djs CLI:** `.dump <file|-> [names]` and `.restore <file>` in the REPL
  (docs/cli.md).
- **djs-wasm / JS:** `vm_dump` (feature `dump`, off by default, D2352);
  `Sandbox.dump(names?)` passes `console` and the sandbox's host functions as the host
  names, and `Sandbox.load(text)` runs a dump as a Script (not journaled). The prototype
  `dumpHeap(sandbox.heapView(), { base, host, names })` in js/proto/dump.ts does the same
  from the host side.

**Selecting what to persist.** A harness decides by name. The simplest rule for codemode
is to dump the bindings a script left at its top level (the body's locals, through
`dump_values` and their boxes) or the globals it assigned; a more deliberate one is a
`persist(name)` helper in the prelude that records names for the host to dump at the end
of the turn. Dumping names, not "everything", keeps dumps small and keeps scratch data
out; the closure walk brings along whatever the named functions need.

## Format version and the compatibility promise

The first line, `// djs-dump 1`, names the format. Version 1 means: a Script that only uses
the language, the standard builtins and the helpers it defines on `$dump`, builtins
referenced by D2371 paths, and the conventions above. **The promise: every later djs
loads a version 1 dump and recreates the same bindings.** It needs no reader to keep
(djs is unreleased, D024, and keeps no code for old formats; a dump needs none, since
loading it is running it). The committed fixture `tests/dump/v1.js` is loaded by a test
in every gate run; when the format changes on purpose, the version goes up and the old
fixture stays. Checked once by hand in the other direction: a dump written today loads in
a build 800 commits older (main at issue 0083, 9e7bcfd5) and gives the same answers.

What can still break a dump is what breaks any program: a builtin it names that a later
build removes (the path no longer resolves), a host name the new host does not provide,
or language behavior the functions rely on that changes. These are visible errors at load
time or later in the code, not silent misreads.

## The trust boundary

A dump is code. Loading one runs whatever it says, so a dump is exactly as trusted as the
scripts that made it, and an edited dump as trusted as its editor:

- Load dumps only into a sandbox, never into the host's own JavaScript, and into a
  **fresh** VM, so a dump cannot see or change another session's state.
- Use frozen builtins (D2304) in the loading VM when the dump must not change builtins:
  its "builtins the session changed" section then cannot take effect (an assignment to a
  frozen builtin fails, `Object.defineProperty` throws), so a host that wants patched
  builtins loads into an unfrozen VM on purpose.
- Fuel and memory limits apply as to any script; a dump of a large session costs time to
  load in proportion to its size (a 9.7 MB dump of 50,000 records is one Script).
- A model-edited dump is model code: run it under the same policy as the model's scripts.
  The format makes that easy to review, which is the point of making it readable.

## Storage and naming

One file per dump. A host names it by content (a hash of the text) and keeps the
metadata next to it: when, which build wrote it, the names it holds, the turns that
defined them (provenance), and its size. Binary data is inline base64 (4/3 of its size);
a later step can move large buffers into side blobs named by hash and have
`$dump.bytes` take a hash, which deduplicates buffers across dumps and keeps the text
small. The slim build (D2411) has no typed arrays but keeps `atob`/`btoa`:
there, binary data is strings already, and a dump writes strings, so nothing changes.

## How pi would use it

pi's codemode already persists JSON values with `store`/`load`. Dumps extend that to
functions and their state:

1. A turn defines helpers (`const parse = ...; const cache = new Map()`), and either the
   script calls `persist("parse", "cache")` or the host dumps the turn's top-level
   bindings.
2. The host keeps the session image as the fast path and the dump as the durable copy,
   with the metadata above.
3. A later turn, or a later session after an upgrade, starts a fresh VM, loads the dumps
   it wants (the image when the build still matches), and runs the script; `tools` and
   `console` are pi's own again.
4. The model can be shown a dump (it is code) and asked to fix or extend it; the result is
   loaded like any other.

## Rust or TypeScript

Two implementations of the same algorithm exist, and both write the same text (7 of the 8
round-trip cases byte for byte, the other differs only in the order of its roots; the
same refusals; the same 9.7 MB for 50,000 records):

| | Rust (`Vm::dump`, wasm `vm_dump`) | TypeScript (js/proto/dump.ts) |
| --- | --- | --- |
| code | ~2,800 lines; wasm +35.6 KB gzip (+101 KB raw), the checker included | ~1,550 lines; 40.2 KB minified, 13.7 KB gzip (+2.0 KB for the layout table it shares with the reader) |
| 1k / 10k / 50k records | 21.5 / 178 / 875 ms with the checker (12.9 / 99 / 508 before it) | 11.2 / 73 / 360 ms (+3 ms once per base for builtin names), no checker |
| reads the heap through | the engine's own accessors | the layout table, plus engine facts copied by hand |
| global reads of functions | from bytecode | from a scan of the source for identifiers (over-approximation) |
| errors' `stack` | kept | lost |
| native (CLI, embedders) | yes | no |

(Release wasm in node 25 vs the same node running the TypeScript, at machine load ~4.5;
N records with nested objects, dates, a shared Map, a cycle, N/10 counters and an
N×4-byte buffer. Natively the Rust dumper takes 40 ms for 10k records without the
checker; in wasm it allocates heavily and V8's strings and JIT win. The N/10 counters are
N/10 factories, which the checker compiles.)

The Rust dumper started at +58 KB gzip; most of that was `core::fmt`, `BTreeMap`, slice
sorts and string search reached from the dump path. It now uses one open-addressing index
of cells and atoms, insertion order, `crate::text` (D2339) and no string slicing (whose
panic path pulls `Debug` and Unicode tables). What is left is its own logic.

**Recommendation (D2354).** Keep the Rust dumper as the reference: it reads the heap
through the engine, so a layout change cannot make it misread, it serves the CLI and
native embedders, and its tests are the spec. A wasm host should use the TypeScript
dumper: the module stays under its size ceiling with no feature to enable, the host
already has the heap bytes, and it is faster. Before it leaves js/proto it needs (1) the
engine facts it copies by hand exported through layout.generated.ts (function flags,
intrinsic numbering, object-class slots, lazy function properties, collection and regexp
encodings), (2) the global-reads scan replaced by the two opcodes and their operand
layout, and errors' stack, and (3) a gate step that runs both on the same cases and
requires the same text, as the wire and reader cross-checks do (js/test/dumpproto.test.ts
now runs the comparison in the gate's js tests), and (4) the checker: it has no compiler
of its own, so it would compile the cut-down dump through the module (a `vm_compile`
into a scratch VM) or keep its structural guard (a closure of a group is only ever
written by its factory) as its only check.

## Coverage and results

- `crates/djs-core/src/vm/dump_tests.rs`: round trips through a fresh VM (shared
  closures, globals functions read, data identity and kinds with attributes and frozen
  objects, cycles through closures, binary buffers and views, builtins by name and
  through the resolver, patched builtins, symbol keys with attributes, function-scope
  bindings by box, host values), refusals by path, and the committed v1 fixture;
  closures from factories (over parameters, locals, nested scopes, `let` in loops,
  sharing a binding, capturing other closures), each run as one Script and line by line
  as the REPL does. The checker's own tests (`dump::check_tests`) feed it the texts it
  must refuse: 0168's bare closure, a global hidden by a factory variable, a global the
  dump lacks, a dump that does not compile.
- js/test/dumpproto.test.ts: the TypeScript prototype and the Rust dumper on the same
  cases (both input styles): identical text on all closure cases, both load right.
- js/test/dumpload.test.ts: `Sandbox.dump`/`load` across sandboxes with a host function,
  refusals, a dump that ends with a cycle patch.
- `vm/dump_harvest.rs` runs every harvested codemode case as the difftest does, then dumps
  the body's top-level bindings with `dump_values`, loads the dump into a fresh VM with
  stand-ins for the host globals, and compares the bindings (JSON, functions as text):
  - 300 harvested cases: 205 leave top-level bindings, 7 of them functions (8
    functions); 300 dump, 300 load, 300 equal; 397 KB of dumps. The equality compares
    functions as text, which is why it did not catch 0168; the checker does.
  - 2,627 private pi scripts (tools answer `""` in call order; counts only): 12 do not
    compile in the harness and 2 fail in it; 1,428 leave bindings, 123 of them functions
    (139 functions). 2,607 dump and load; 2,601 give equal bindings, the other 6 hold a
    tool function that the loading VM re-binds to its stand-in; 6 are refused, all for a
    closure over a binding still in its TDZ (the script stopped early under the fake
    tools); 1.36 MB of dumps.
  - Codemode scripts are mostly one-shot data pipelines: functions worth keeping are rare
    in them today, which is the case for making persistence an explicit `persist(name)`.
- Sizes (wasm `full` build, gzip -9): 443,468 without the dumper, 479,077 with it.

Run: `cargo test -p djs-core --lib vm::dump_tests`; coverage
`DJS_HARVEST=1 cargo test -p djs-core --lib vm::dump_harvest -- --ignored --nocapture`;
the comparison `DJS_WASM=<module with vm_dump> node js/proto/compare.ts [--big N]`.

## Open

- Classes (the body and its static state, private fields through the class's own code).
- Large buffers as side blobs by hash; dumps that share data.
- The TypeScript dumper's three steps above, and wiring it under `Sandbox.dump` when the
  module has no `vm_dump`.
- A `persist(name)` convention in pi's prelude and the host's storage of dumps and their
  metadata.
