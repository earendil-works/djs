# Portable snapshots: VM state that survives a djs upgrade

Status: research report (issue 0141), for the owner. Since adopted in adapted form:
continuation dumps (docs/continuation-dumps.md, D2560-D2565) take this design's idle-point
export, site naming and operand roles, write it as a Script, and re-enact builtin state
instead of describing it by schema. The original report follows; nothing in it was built in
the engine. The prototype and every number below are reproducible with the commands at
the end. It takes the opposite stance from docs/snapshot-versioning.md (0140), which
recommends journal replay and rejects migration and logical snapshots: here we assume
that snapshots *must* survive engine upgrades, and ask what the best design for that is
and what it costs.

The owner's question: *what would a heap snapshot design require that is more portable
between versions? Surely there has to be a better way.*

## The short answer

- **There is a better way than refusing or migrating images.** At an idle point (the
  script waits on tools), the old engine *exports* the VM as a logical graph described in
  terms of the source and the spec, and the new engine *imports* it by compiling the same
  source again and building its own frames from that description. Nothing in the
  snapshot is a pc, an offset, a slot number, a cell layout or an intrinsic id.
- **It works for djs, though it fails for most systems, because the program does not
  change.** An upgrade replaces the engine, never the script: both engines compile the
  same source, so every suspension point means the same thing on both sides and only has
  to be named in source coordinates instead of pcs. Erlang, Smalltalk, Temporal or
  Restate face a harder problem, new *user* code under old frames, and all of them pin
  the old code or rely on hand-written migrations (section 6).
- **Prototyped and measured across 406 commits.** A prototype outside the engine (2,300
  lines of Rust, `scripts/snapshot-research/portable`, builds unchanged against both
  trees) exports a waiting session on main before 0080 (1b6c2098) and imports it on
  today's main (bcdc6649), and the other way around. Between those builds:
  - objects moved from property tables to shapes, the base image went from 404 to
    322 KB, and the number of cell kinds from 22 to 24;
  - 46% of the suspended activations resume at another pc;
  - every builtin closure's intrinsic id moved (`PromiseAllResolveElement` 350 → 397);
  - Promise.all's element record gained a field.

  None of that reaches the format. On the 300 harvested codemode cases, all 1,057 idle
  points the prototype tried transfer in both directions:
  - 1,008 continue exactly like the uninterrupted run;
  - 49 continue like the *importing* build's own run, where the two builds behave
    differently (stack-trace columns, D2031).
- **Real scripts need a small, fixed vocabulary.** Of 4,881 awaits in 2,535 codemode
  scripts:
  - 48.6% hold nothing but bindings;
  - 4.4% also hold a for-of iterator;
  - 47.0% hold evaluated operands of an enclosing expression, 70% of them `text(await …)`.
  - None is inside a finally block or a switch.

  Ten operand roles plus the for-of iterator record cover every await site.
  Restricting portable snapshots to statement-level awaits would cover half the scripts,
  so the roles are needed.
- **The price is a contract, not the code.**
  - The compiler has to name its suspension points, bindings and operand roles at
    `await` (a side table, computed on demand), and keep them stable or ship a
    migration.
  - Every builtin closure kind and builtin record needs a named, versioned schema (25
    closure kinds today; codemode uses 5).
  - The engine has to read the format versions it promises.

  Estimate: 4–6 weeks in three phases, then the contract.
- **Hard limits:**
  - only quiescent points: no snapshot at an arbitrary fuel stop;
  - behaviour changes of the new engine apply from the restore on;
  - anything without a spec-level meaning cannot travel: a frame inside a builtin, the
    exact image bytes, canonical hashes, OOM and fuel points;
  - a recorded stack trace travels as text.
- **Recommendation:** journal replay (0140) stays the durable record for pi *now*. It
  needs no engine contract, and pi has it. Build portable snapshots when one of these
  becomes true: sessions get long or compute-heavy enough that replay is too slow;
  tool results cannot be replayed; djs ships to hosts that keep VMs across releases.
  Start with phase 1 (section 7), which already covers every harvested idle point. Put
  the contract in place first, at no cost: a test that pins the operand roles at every
  corpus await, so lowering changes become visible decisions today.

## 1. What a snapshot holds, by how portable it is

A waiting VM's state falls into three layers:

| layer | what | portable? |
| --- | --- | --- |
| data | objects, arrays, strings, numbers, Maps and Sets, Dates, errors, property order and attributes, prototypes | yes, and proven elsewhere: V8's ValueSerializer and Erlang's term format are stable across versions. Shapes, ICs, atoms, hash indexes are caches the importer rebuilds |
| code | functions, closures and their environments | yes, if code is named by content: the script's source travels (or its hash), the new engine compiles it, and a function is its source span. The span is what `Function.prototype.toString` returns, so the spec fixes it |
| control | suspended async functions (pc, locals, operand stack), promise reactions, builtin closures (promise resolvers, Promise.all elements) and their records, pending host calls | only with a compiler- and engine-independent description. This is the hard part, and 0088 measured it at 100–500 bytes per waiting session |

An image (D1320, D2260) stores all three as engine memory. That is why it is fast, and
why it is tied to its build (0140: 72% of all merges change the build hash). A portable
snapshot stores the first layer as data, replaces the second by references into the
source, and gives the third a vocabulary of its own.

## 2. The key observation: the program does not change

Every system that persists continuations runs into code that changes under a suspended
frame:

- Smalltalk keeps the old CompiledMethod alive;
- Erlang keeps two versions of a module;
- Temporal and Restate pin an execution to the deployment it started on;
- Lua's Pluto/Eris, Stackless Python and Javaflow require the same bytecode.

They have to: a frame of `f` version 1 has no meaning in `f` version 2 unless a person
writes the migration.

djs is in a different position. In an upgrade **only the engine changes**. The script
(pi's codemode body) and pi's prelude are the same source text on both sides. So the
question is never "what does this frame mean in new code" but "how does the new
*compiler* lay out the same source". Both compilers implement the same spec, so the state
of a suspended activation has a description that both can produce and consume: *which
await of which function, which bindings hold which values, and which already-evaluated
operands the spec's evaluation order keeps*. That is the spec's own model of an
execution context, and it does not mention bytecode.

The importer can therefore treat the snapshot as a spec-level description and *compile
its own frame* from it: the resume pc is "after the await at offset X in this build's
code", the local slots are "where this build keeps binding `out` of the block at offset
Y", the operand slots are "where this build keeps the callee of the call at offset Z".
If the new compiler has no such place (it lowered the construct differently), the import
refuses with a precise reason, or a migration maps the old roles to new ones.

## 3. Design

### 3.1 When: at quiescent points only

A portable snapshot is taken when the VM is **idle**:
- the job queue and the host inbox are empty;
- no frame is on the value stack;
- every continuation lives in heap objects: async-function states, promise reactions,
  builtin closures.

This is pi's natural durability point, since the VM waits on tools. It removes the
untranslatable state:
- native frames and `@native` state numbers;
- suspended builtins (the builtin language has no `await`, so builtins keep their state
  in records, never frames: 0088 checked every idle point);
- parked calls and fuel debt.

Images keep their advantage of suspending anywhere (G5); the portable form does not try
to compete there.

### 3.2 Format sketch

The prototype writes JSON; a real format would be binary (varints, a string table,
deduplicated shapes as in 0088's encoder) and zstd-compressed.

```text
header      format version, exporting engine (informational), semantic level
scripts     [{name, source | source hash, goal}]       code, by content
reference   the world the importer rebuilds before the graph: "djs realm" plus host
            globals by name, optionally a prelude stage (pi: prelude hash + start data)
roots       pending host calls [{seq, function name, args, promise}], named handles,
            patches to reference objects (global properties a prelude added; pi's lockdown)
nodes       the graph, each node one of:
  object      class, proto, [key, attributes, value | {get, set}], extensible
  array       proto, elements, extra properties
  closure     fn {script, start, end}, captures by binding name, own properties
  box         value (a captured binding; identity matters, closures share it)
  async       fn, callee, site {line, col}, this, argc, args, bindings [[name, value]],
              operands [[role, value]], result promise
  promise     state, result, handled, [reaction {capability, onFulfilled, onRejected}]
  builtin-closure  intrinsic name (spec name), data
  record      schema name, fields by name
  collection  Map | Set, entries in insertion order
  date, bound function, array iterator {target, index, kind}, host function {id, async}
values      undefined, null, booleans, numbers (NaN, ±Infinity, -0 spelled out),
            strings, {ref: node}, {builtin: path}
```

Not in it: bytecode, pcs, slot numbers, cell kinds, shapes, inline caches, atoms, hash
indexes, GC state, intrinsic ids, realm slot numbers. The importer recomputes all of them.

### 3.3 Naming everything without the engine

| thing | named by | stable because |
| --- | --- | --- |
| a script | its source (or hash: the host has the text, as pi has its prelude) | content |
| a function | (script, source span) | the span is `Function.prototype.toString`'s, fixed by the spec |
| a suspension point | (function, source position of the `await` keyword); compiler-made awaits (for-await, async generator `yield`/`return`, `await using`) get a kind tag with the construct's position | the source |
| a binding | name plus the source position of its declaring scope (the prototype uses name + occurrence: 14 of 5,209 harvested activations have a repeated name) | the source |
| an operand | a role from a fixed vocabulary (3.5) | the spec's evaluation order |
| a builtin object | its path from the global object or a named intrinsic: `globalThis/Array/prototype/push`, `%ArrayIteratorProto%/next`; resolved by *walking* the path in the new realm | the spec's property paths |
| a builtin closure | the spec name of its abstract closure (`PromiseAllResolveElement`) plus its data | the spec |
| a builtin record | a schema name and field names (`PromiseElement {alreadyCalled, index, values, capability, remaining, keys}`) | versioned: a new field gets a default, a dropped one must be at its default |
| a host function | the host's id (the host re-registers its functions on import, as on a restore today) | the host's contract |
| a pending host call | its sequence number in the host's own record, function name and arguments; the importer re-issues it into the VM, the host recognizes it and does not run it again | the host's contract |

Builtin paths must name only objects the base realm has: the exporter compares with a
fresh realm. A global that a script or prelude added is state, not a builtin. Its
object is exported as a node, and the global property is a patch. The prototype learned
this the hard way: the corpus prelude's `text` and `tools` live on the global object.

### 3.4 Export and import

**Export** (by the engine that wrote the state, i.e. the old one):
1. Collect, then walk from the roots: pending host calls, named handles, patched
   reference objects.
2. For each object, emit a reference by builtin path, or a node in BFS order.
3. For each async state, read its frame and match the resume pc to its await. Then:
   - name the locals with the template's binding names;
   - name the operands with this build's suspension map (3.5);
   - emit the result promise.
4. Refuse with a reason for anything that has no name:
   - an unknown class or record schema;
   - a symbol-keyed property (not built in the prototype);
   - a generator or async generator, which the prototype does not cover;
   - a hidden local holding a value;
   - a privileged closure that no path reaches.

**Import** (by the new engine):
1. Build the reference world: a fresh realm, the host's globals, and for pi optionally
   its prelude (see 3.7).
2. Compile every script and index the templates by (script, span). Nothing runs.
3. Re-issue the pending host calls, so that the VM's host-call table and promises are
   the importer's own. Their promises become the snapshot's pending promise nodes.
4. Allocate a cell for every node, then fill them in a second pass, so that cycles are
   free:
   - properties through the ordinary define path, which rebuilds shapes and indexes;
   - Map and Set through `collection_set`, which rebuilds hash indexes;
   - closures from their template, with captures by name in *this* build's capture
     order;
   - builtin closures by looking up the intrinsic name in this build's table;
   - records laid out by this build's schema.
5. For each activation, build the frame this build would have:
   - find the await at the recorded source position, and resume after it;
   - lay out this build's header and argument padding;
   - put each binding into this build's slot for it, and the state object into
     `<state>`;
   - compute this build's suspension map at that await, which must hold the snapshot's
     roles; put the operand values in its order;
   - set the Generator's slots.
6. Apply the patches and register the named handles, then `run`: the VM is idle, and
   the host delivers the next result.

The prototype implements exactly this through djs-core's public heap API. A real
implementation would live in djs-core, next to the collector, which already knows
every cell.

### 3.5 What the compiler must guarantee

Per `await` (and `yield`), a **suspension map**, computed on demand by compiling the
function again with a flag (it costs nothing at run time and nothing in images):
- the await's source position and kind;
- for every local slot, the binding it holds: name and declaring scope position, or a
  stable name for hidden locals (`<state>`, `<fields N>`, private names);
- for every operand slot below the awaited value, its role.

The role vocabulary that covers every harvested await (section 5):

| role | example | held at await sites (2,535 scripts) |
| --- | --- | ---: |
| `call.callee`, `call.this`, `call.arg[i]` | `text(await t())`, `out.push(await t())`, `f(a, await t())` | 2,244 (46%) |
| `object.literal` (+ `object.key` for a computed key) | `text({path, content: await t()})` | 164 (3.4%) |
| `binary.left` | `` `=== ${f}\n` + await t() `` | 27 |
| `assign.object`, `assign.key`, `assign.old` | `out[k] = await t()`, `sum += await t()` | 26 |
| `array.literal` | `[a, await t()]` | 11 |
| `template.prefix` | `` `${a}: ${await t()}` `` | 4 |
| `new.callee` | `new X(await t())` | 1 |
| `iterator.record` (iterator, next method) of an enclosing for-of | `for (const f of files) {… await …}` | 532 (10.9%) |

Not seen in the corpus but needed eventually: for-in and for-await iterator records, the
`switch` discriminant, a finally block's pending completion (spec: a completion record:
normal, return v, throw v, break label), destructuring under way (iterator and source),
`using` disposal stacks, `yield*` delegation. Each is a spec concept with a small value
description; each has to be added before scripts that need it can be exported. Until
then the exporter refuses them.

The **contract**: for the same source, a new compiler produces the same set of roles at
each suspension point, possibly in another order and in other slots. A lowering change
at an await has two options:
- it keeps the roles, which is the common case: between the two prototype builds, 1,057
  of 1,057 idle points had identical role sequences, although 46% of their resume pcs
  moved;
- it ships a migration from the old roles to the new ones.

A test pins the role sequence of every await in the corpus, as the compiler's snapshot
tests pin bytecode; a change shows up as a diff that has to be accepted deliberately.

The values in the slots must also mean the same: a key operand is "the key *after*
ToPropertyKey" or "before". If a lowering moves that conversion across an await, the
roles differ (`assign.key` vs `assign.key.converted`) and the change needs a migration.
The prototype's roles are weaker than this. They are labels derived from the
instruction that produced each slot (`value value object-literal` for `text({…: await
t()})`), which is enough to show the mechanism. They would not notice a compiler that
swaps callee and `this`; real stack maps must come from the compiler, which knows the
roles when it emits the call.

### 3.6 What the runtime and the builtins must guarantee

- **Builtin identities**:
  - paths from the global object and the named intrinsics keep resolving (a rename
    keeps an alias);
  - the spec names of builtin closure kinds stay (25 kinds today, `NewBuiltinClosure`).
- **Record schemas**: every record a builtin closure or a builtin object keeps across
  an await has a schema with named fields and a version:
  - Promise.all/allSettled/any elements and their counter;
  - finally's `{onFinally, constructor}`;
  - async-from-sync iterators;
  - async generator requests.

  A new field gets a default, so a new build can read an old record; the prototype does
  this for `keys` (Promise.allKeyed, D1972). A field that changes meaning needs a
  migration.
- **Internal state of builtin objects** in spec terms: an Array Iterator is
  {target, next index, kind}, a Map is its entries in order, a Date its time value. The
  engine's representation is free (D1800's fast stepping did not change the iterator's
  observable state).
- **Behaviour**: the continuation runs with the new engine's semantics, as a replay does.
  The difference is that only the *future* runs on the new engine; what happened before
  the snapshot is baked into the state (section 10).

### 3.7 The reference world, and pi

pi's prelude creates about 260 closures and 350 boxes (0088). Its lockdown freezes about
700 builtin objects, so the global object, `tools` (a Proxy) and every frozen builtin
differ from a fresh realm. There are two ways to carry that:
- **as graph**: export the prelude's objects like any others, and the frozen builtins
  as patches (attributes and extensibility per builtin path). This is simple and
  self-contained. It costs the prelude's state in every snapshot: 0088 measured 19 KB
  zstd against the bare base.
- **as a reference stage**: the importer runs pi's prelude (same source, same start
  data, deterministic since D2262) on the new engine, and the snapshot names the
  prelude's objects by path from the prelude's handles (`handle[1]~pending`). It carries
  only what the session changed: 0.9–2.2 KB in 0088's encoder. The cost is matching
  prelude-made objects by path, which 0088's encoder prototyped and the analyzer's
  structural diff (D2264) does for images.

The corpus prelude of the harvested cases is small and does not freeze anything. The
prototype carries it as graph plus a patch of the global object's added properties.

### 3.8 Next to images

Images stay the same-build fast path: they are exact, take ~1 ms, and can be taken at
any instruction. The portable form is needed only when the build changes. Two ways to
produce it:
- **always written alongside**: every `snapshot()` also exports (a few ms, a second
  blob). Restore uses the image on the same build, the portable form otherwise. No old
  engine is needed at upgrade time.
- **export on drain**: snapshots stay images. At a deploy, a one-off job loads the old
  module, restores each stored snapshot and exports it. Sessions then resume on the new
  build by import. The old module is needed only during the deploy, not as a long-lived
  registry, and no per-snapshot cost is paid.

Either way the exporter is the *old* engine's code and the importer the *new* one's, so
each engine only ever reads its own memory layout and the format.

## 4. The prototype

`scripts/snapshot-research/portable` is a standalone crate on djs-core's public API,
outside the workspace and the gate. The same sources build against the older tree (copy
the directory into a `git archive` of the old commit, build with `--features old`, which
only switches the one record schema that changed). Parts:
- `session.rs`: two hosts.
  - A script as an async body with `tool(name, arg)` (async) and `emit()` (sync).
  - Harvested corpus cases, run as the difftest runner runs them: the corpus prelude,
    `host.output`/`host.done`, `api.run`. Each fake tool awaits an async host function
    `__tick()` first, so every tool call is an idle point, as with a real host.
  - Tool results depend only on the call's sequence number.
- `export.rs`, `import.rs`: sections 3.2–3.4.
  - Covered:
    - plain objects, arrays, closures and boxes, promises with reactions, async
      activations;
    - builtin closures and records (PromiseElement, Counter, Finally);
    - Map and Set, Date, Error (stack as text), bound functions, Array Iterators;
    - host functions, the global-object patch.
  - Refused: everything else.
- `roles.rs`: operand roles by abstract interpretation of the bytecode, standing in for
  the compiler's suspension map.
- `heapx.rs`: frames, templates and builtin paths.
- `sites.rs`: the corpus runner.

### Results

Two builds 406 commits apart: OLD = 1b6c2098 (main before 0080), NEW = bcdc6649 (main
after 0140, the same engine as 0140's N+1).

| | OLD | NEW |
| --- | --- | --- |
| build hash | 7b5d8711 | b175525f |
| cell kinds | 22 (property tables only) | 24 (shapes, D1831) |
| base image | 404 KB | 322 KB |
| `PromiseAllResolveElement`, `PromiseRejectFunction` intrinsic ids | 350, 345 | 397, 392 |
| Promise.all element record | 5 fields | 6 (`keys`, D1972) |

Four small cases, statement-level, for-of with `out.push(await …)` and a template
literal, Promise.all over async arrows, try/finally in a helper called with
`sum += await step(n)`, `.finally` and `.catch` on a tool promise:
- every idle point exported on OLD and imported on NEW finishes with the uninterrupted
  run's output;
- for example, the for-of case's activation resumes at the same pc 102 on both
  builds with roles `iterator iterator.next callee:push this`;
- in other scripts the pcs move (the earlier disassembly of the same for-of shows
  `await` at 85 on OLD and 87 on NEW).

The harvested corpus: 300 cases, 278 of them with idle points; up to 25 idle points
tried per case:

| | |
| --- | --- |
| idle points tried | 1,057 |
| export → import on the same build, transcript as uninterrupted | 1,057 on NEW, 1,057 on OLD |
| OLD → NEW | 1,057 imported: 1,008 continue exactly like OLD's run, 49 like NEW's own run (the builds differ there: stack-trace columns, D2031) |
| NEW → OLD (a downgrade; `keys` dropped at its default) | the same: 1,008 + 49 |
| activations at those idle points | 5,209, all mapped; 46% resume at a different pc on the other build, frame sizes identical, every builtin closure id different |
| snapshot size (JSON, includes the corpus prelude's state and the case's embedded fake results) | median 27 KB, 6.3 KB with zstd -3; max 239 KB / 27 KB |
| collected image at the same points | median 368 KB (NEW), 455 KB (OLD) |
| time, native dev build (opt-level 1) | export 3.6–3.7 ms median, of which ~3 ms are two walks of a realm for builtin paths (1.5 ms each: the fresh realm's and the session's), which a table of builtin names in the engine replaces; the rest ~0.6 ms. Import 0.75–0.8 ms |

Refusals, checked by hand:
- A session holding a suspended sync generator is refused at export: "an activation of
  kind 0 in state 1", plus the generator prototype's methods, which no builtin path in
  the prototype reaches.
- A snapshot whose operand roles were swapped is refused at import: "operand roles
  differ at (6, 12): snapshot [iterator, iterator.next, this, callee:push], this build
  [iterator, iterator.next, callee:push, this]".
- A snapshot whose script source was edited by one space is refused: "no function at
  (script.js, 0, 387) in this build". A real format checks the source hash first.

Snapshot node counts over all of them:
- 33,664 objects, 29,959 closures, 27,312 boxes, 11,294 promises, 5,206 async
  activations;
- 3,572 builtin closures: `PromiseAllSettled{Resolve,Reject}Element`,
  `PromiseAllResolveElement`, `PromiseRejectFunction`;
- 2,489 records.

## 5. Coverage of real codemode scripts

**Static** (`await-sites.mjs`, an ECMAScript AST, no compiler): every await of the 2,535
distinct scripts of 0130's harvest (4,881 awaits; 371 inside nested async functions; 12
scripts do not parse as plain JavaScript). Categories:
- A, nothing held but bindings;
- B, also an enclosing for-of / for-in iterator;
- C, also expression operands;
- D, a finally block, a switch or a destructuring under way.

| | await sites | scripts by their worst site |
| --- | ---: | ---: |
| no await | – | 44 (1.7%) |
| A | 2,373 (48.6%) | 1,213 (48.1%) |
| B | 215 (4.4%) | 180 (7.1%) |
| C | 2,293 (47.0%) | 1,086 (43.0%) |
| D | 0 | 0 |

What the C sites hold:

| held | sites |
| --- | ---: |
| callee | 1,817 (37.2%), 1,599 of them `text(await …)` and 176 `image(await …)` |
| callee + this | 242 (5.0%): `out.push(await …)`, `JSON.parse(await …)` |
| callee + object literal | 152 (3.1%): `text({path, content: await …})` |
| callee + this + binary left | 17 |
| assignment target (object, key) | 26 |
| array literal | 11 |
| object literal | 6 |
| others (template prefix, binary left alone) | 14 |

Held operands per await: 0 for 53.0%, 1 for 37.9%, 2 for 8.5%, 3 or 4 for 0.5%. 10.9%
of awaits sit in a for-of body. None is in a for-in, for-await, switch or finally.

**Dynamic** (the prototype on the 300 committed, redacted cases): at the 1,057 idle
points, the cases' own suspended activations (1,660, the harness's excluded) were:
- 947 at A sites, 33 at B and 680 at C;
- 588 idle points have only A activations, 33 have B, 436 at least one C;
- the most common role sequences: none (947), `value value object-literal` (290:
  `text({…: await …})`), `value value` (228: `text(await …)`), a for-of iterator plus a
  call (125).

So: a design limited to statement-level awaits would cover about half of real sessions;
the ten roles plus iterator records cover all of them.

## 6. Prior art

What each system did about code that changes under a suspended computation:

| system | what persists | across versions | lesson for djs |
| --- | --- | --- | --- |
| Smalltalk (Squeak, Pharo) | the whole image, including contexts (frames as objects: method, pc, stack) and compiled methods | the image carries its own bytecode. A frame keeps its old CompiledMethod when the method is recompiled, so old frames finish on old code. VM changes to the object format (Spur) were offline image conversions; a new bytecode set was introduced next to the old one, selected per method | code inside the image solves "new code, old frame" by keeping the old code. That is the engine registry of 0140, inside the image |
| Lua Pluto / Eris | closures, coroutines with their stacks, as bytecode prototypes and frames; C functions through a "permanents" table of names | tied to one Lua version: the bytecode and the frame layout are in the file | the permanents table *is* "builtins by name". Frames as bytecode do not travel |
| Stackless Python | tasklets pickled with their frames (code objects + instruction offset) | code objects are marshal data, specific to the Python version | same as Pluto |
| Gambit Scheme / Termite | serializable continuations, used to migrate processes | both ends must have the same compiled code | process migration works when code identity is guaranteed |
| Racket stateless servlets | continuations made serializable by a source transform (CPS, defunctionalized frames as structs) | valid for the same servlet code | a pinned source transform (0088's C') gives data-only continuations. The transform becomes the contract |
| Javaflow, Kilim (JVM) | bytecode instrumentation saves locals and the operand stack at suspension points and rebuilds them on resume | tied to the instrumented class bytes | the same mechanism as our importer (operand stack rebuilt from a description), but keyed to bytecode, not source |
| Erlang/OTP | processes, not persisted. Hot code loading keeps two versions of a module; a process moves to the new one at a fully qualified call. `code_change/3` migrates gen_server state by hand. The external term format is stable; funs carry their module's version hash and fail ("badfun") against other code | data: stable format. Code: two versions and explicit migration points | stable data format plus version-tagged closures; the user's code changes, so migration is manual |
| V8, SpiderMonkey | startup snapshots and code caches; structured clone / ValueSerializer for data | snapshots and code caches are checked against the exact build; the data serializer is versioned and stable | the same split as djs: images per build, data portable |
| Hermes | precompiled bytecode | bytecode version must match the VM | ship code and VM together |
| Temporal | an event history; workflow code is replayed | replay against new code must be deterministic. Changes are guarded with versioning markers (`patched`/`GetVersion`), or executions are pinned to worker build ids | replay plus pinning (0140's recommendation) |
| Restate | a journal of steps per invocation | in-flight invocations stay on the deployment they started on; new ones go to the new one | pinning |
| Golem (wasm) | an operation log, replayed into the wasm component | an "automatic" update replays the log on the new component (must stay compatible), a "manual" one uses snapshot functions the component itself exports | replay, or application-defined state, never engine memory |
| Cloudflare Durable Objects / Workflows | Durable Objects keep durable state only in their storage API, and memory is lost on eviction or deploy. Workflows re-run and skip completed steps by their persisted results | no continuations survive; state is explicit | for long-lived pi state (`store`) this is what pi already does |
| CRIU | a whole Linux process | needs the same binaries | an image, as djs's |
| Unison | code identified by the hash of its syntax tree; values and continuations refer to code by hash, and missing code can be fetched by hash | code identity is content, independent of names | content-addressed code is exactly how a portable snapshot should name functions: by (script hash, span) |

The pattern: every system that kept continuations across versions either kept the old
code running (Smalltalk, Erlang, Temporal, Restate) or shipped code inside the state and
froze the interpreter's formats (Pluto, Stackless, Javaflow). The ones with stable
formats are data-only (Erlang terms, ValueSerializer) or replay-based (Temporal, Golem,
Workflows). djs can do better than all of them only because its programs do not change
across an upgrade. A suspension point of *unchanged source* has a version-independent
meaning, and the new engine can re-derive its own frame from it. That is the
combination this design uses: Unison's content-addressed code, Pluto's permanents
(builtins by name), and Javaflow's rebuild of locals and operands, keyed to source
positions and spec roles instead of bytecode.

## 7. Costs

**Engine work, in phases:**
1. **Core (2–3 weeks).** Export and import in djs-core:
   - data, closures, boxes, promises, async activations;
   - Map, Set, Date, Error, bound functions, Array Iterators;
   - the five promise closure kinds and their records;
   - builtin paths;
   - the compiler's suspension maps for the ten roles and for-of records.

   A binary format and a host API (`Sandbox.exportPortable()`, pi's adapter, `djs heap`
   reading the format). The round trip on the harvested corpus joins the gate. This
   phase alone covers every harvested idle point.
2. **pi (1–2 weeks).**
   - Proxy, the lockdown as patches, or pi's prelude as a reference stage (3.7);
   - symbols;
   - the remaining builtin closure kinds and records, with schema versions;
   - golden snapshots in the gate (section 9).
3. **The rest, when needed.**
   - Generators and async generators: their request queue is a record;
   - for-await, finally completions, switch, destructuring, `using`;
   - class private names, WeakMap/WeakSet entries.

   None of these is used by codemode today (0130): the exporter refuses them until then.

Total 4–6 weeks for pi's needs, against 0088's estimate of 6–10 weeks. The difference is
what the prototype established:
- the public heap API suffices;
- frame rebuilding is ~150 lines;
- roles at awaits are few.

**Ongoing costs:**
- *Compiler*: lowering at suspension points becomes a contract with a test that pins it
  (section 9). Measured churn: no role change at any harvested await in 406 commits,
  while ops.rs and the for-of code changed several times (0140's churn table).
- *Builtins*: a schema per record and closure kind; a field added needs a default, a
  meaning changed needs a migration. Promise.all's record changed once in that span
  (D1972).
- *Format*: readers for the versions promised (say the last two releases' formats).

**Size:** the data plus small continuations.
- Compressed, about what an image container costs against pi's prelude image (2–9 KB
  for a waiting session, D2260).
- Smaller with pi's prelude as a reference stage (0088: 0.9–2.2 KB).
- Larger without one (+19 KB).
- The prototype's JSON, carrying the corpus prelude and the cases' embedded results:
  6.3 KB zstd median.

**Time:** an export is a heap walk plus one recompilation per function with suspended
activations (for the suspension maps); an import compiles the scripts and builds the
graph. Native dev build: 0.8 ms import, ~0.6 ms export without the prototype's realm
walks for builtin paths (3 ms, replaced by a table of builtin names in the engine).
That is comparable to a compressed image restore (1–1.5 ms in wasm), and far below a
replay of a long session.

**Risk:**
- a wrong mapping must refuse, never misbehave. The design refuses on any name it
  cannot resolve, any role set that differs and any unknown schema;
- the prototype's weak roles are the one place a silent swap could happen, hence
  compiler-emitted maps;
- builtin behaviour changes apply mid-session (as with replay);
- the contract slows down lowering changes at awaits.

## 8. Hard limits

- **Quiescent points only.** No portable snapshot at an `OutOfFuel` stop in a loop or
  inside a builtin: the state of a `@native` state machine or a half-run instruction has
  no spec description. Images keep that ability (G5).
- **The new engine's behaviour from the restore on.**
  - A bug fix, a changed error message or another libm result shows up in the
    remaining run.
  - Seen in the prototype: 49 continuations print the importing build's stack-trace
    columns.
  - Exact continuation across builds is only possible by keeping the old engine
    (0140's registry).
- **Determinism artifacts do not travel.**
  - The image after an import is not the image the old build would have had; canonical
    hashes (D1730) are per build.
  - Collection points, OOM points and fuel accounting restart with the new engine.
  - A logical hash of the exported graph can serve as the cross-build check instead.
- **Recorded stack traces** travel as text (the trace records pcs of the old build).
  An imported error has its `stack` as an own data property, which a script can tell
  apart.
- **Things without a spec-level identity** cannot be exported:
  - a privileged closure no path reaches;
  - a builtin record without a schema;
  - a hidden compiler local without a stable name.

  Each needs a name before it can travel; the exporter refuses until then.
- **Compiler contract at awaits.** A new lowering that changes the roles at an await
  without a migration makes old snapshots stopped there unimportable. That is a refusal,
  not a misbehaviour, but it is a lost session unless replay or the old engine is the
  fallback.
- **Builtin semantics of in-flight state.** If a builtin's algorithm changes how an
  in-flight record is used (not just its layout), the old record may not mean the same.
  Schema versions catch layout changes; semantic changes need review.
- **Host contract.** Host function ids and pending calls by sequence number must stay
  the host's own across the upgrade (they are today: the Sandbox names functions, pi
  keys calls).

## 9. How the gate would test it

- **Round trip at every idle point** of the harvested corpus on the current build: the
  prototype's `harvest` command (300 cases, 1,057 points, seconds in a dev build).
  Export, import into a fresh VM, finish, compare transcripts.
- **Suspension-map pins**: the roles at every await of the corpus (and of the e2e
  async tests) as a committed snapshot, like the compiler's bytecode snapshots. A diff
  means a lowering change at an await. Accept it only with a migration or an explicit
  "old snapshots at this site stop importing".
- **Golden portable snapshots**: a selection of idle points (one per role sequence and
  schema, ~50, about 300 KB compressed), exported by each release and committed, imported
  by every later build until the format window closes. Or build the old commit in CI and
  export there, as the prototype did (two minutes).
- **Schema checks**: the builtin record and closure tables, field names and versions in
  a generated reference, so a change to `NewElementRecord` without a schema bump fails.

## 10. Compared with journal replay (0140)

| | journal replay | portable snapshot |
| --- | --- | --- |
| what is stored | start data + every tool result | the live state (data + small continuations) |
| size | grows with the history (0088: 0.3–2.4 KB light, 58–296 KB heavy) | the live data: smaller when the script reduces its inputs (0088: 3.8 vs 58 KB for `sequential`), equal when it keeps them |
| restore time | re-executes the whole history (1–43 ms in 0088's cases; seconds for compute-heavy sessions) | proportional to the live state (~1 ms) |
| behaviour changes in the new build | the whole *past* re-runs on the new engine: a fix or a changed message in code that already ran can diverge (detected), and the session is lost | only the *future* runs on the new engine; the past is baked into the state |
| non-replayable tools | a problem (results must be stored, side effects must not repeat) | no issue: nothing re-runs |
| engine work | small: a cross-version replay mode, golden journals | 4–6 weeks + contract |
| contract | behaviour stability of everything a script did | lowering at awaits + builtin schemas + format readers |
| failure mode | divergence, detected | refusal at import, detected |
| pi today | already does it to survive restarts | nothing yet |

They are complementary. Replay needs no contract and pi has it, but its cost and risk
grow with the session's history. A portable snapshot's cost and risk are bounded by the
live state. A session can carry both: the portable snapshot as the fast cross-build
path, replay as the fallback when an import refuses.

## 11. Recommendation

1. **Now:** keep 0140's plan, with replay as the cross-version record and images as
   the same-build fast path. Add the cheap part of this design at once: the
   suspension-map pin test. It only needs the compiler to report the roles it already
   knows at each await, and it turns "the lowering around awaits changed" into a
   visible decision from today on. With it in place, adopting portable snapshots later
   does not inherit unknown churn.
2. **When sessions outgrow replay** (long, compute-heavy, non-replayable tools), or djs
   ships to hosts that keep VMs across releases: build phase 1 (2–3 weeks), with the
   harvested-corpus round trip in the gate. Then phase 2 for pi, preferably "export on
   drain" (3.8), so that no per-snapshot cost is paid and no old engine is kept beyond
   a deploy.
3. **Do not** pursue image migration (0140's option c): the portable form gets the same
   result without reading old layouts in new code.

## Reproducing

```sh
# the coverage of await sites in the private extraction (counts only)
node scripts/snapshot-research/await-sites.mjs --jsonl ../../work/scratch/0130/extract.jsonl \
    --acorn PATH/TO/node_modules/acorn
# the prototype on this build
(cd scripts/snapshot-research/portable && cargo build --offline)
target/debug/portable roundtrip scripts/snapshot-research/portable/cases/*.js
target/debug/portable harvest --sources OUT/src tests/corpus/codemode-harvest/*.js   # wrapped sources
node scripts/snapshot-research/await-sites.mjs --files OUT/src/*.js --sites OUT/sites.json \
    --acorn PATH/TO/node_modules/acorn
target/debug/portable harvest --sites OUT/sites.json --save OUT/new tests/corpus/codemode-harvest/*.js
# an older build: the same crate in a git archive of the older commit
git archive 1b6c2098 | tar -x -C OLD && mkdir -p OLD/scripts/snapshot-research
cp -r scripts/snapshot-research/portable OLD/scripts/snapshot-research/
(cd OLD/scripts/snapshot-research/portable && CARGO_TARGET_DIR=OLD/target cargo build --offline --features old)
OLD/target/debug/portable harvest --save OUT/old tests/corpus/codemode-harvest/*.js
target/debug/portable import-all OUT/old OUT/new        # OLD -> NEW
OLD/target/debug/portable import-all OUT/new OUT/old    # NEW -> OLD
```
