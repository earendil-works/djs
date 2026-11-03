# Continuation dumps: persisting a waiting session as code

Status: plan, agreed with the owner (2026-11-03). Decisions D2560–D2565; issues 0169–0178.
Builds on dumps (docs/functions-as-code.md, D2353) and on the portable-snapshot research
(docs/portable-snapshots.md, 0141), and replaces heap images as the way a session is
persisted.

## The short answer

- A waiting codemode session is written as a **format 2 dump**: a JavaScript Script, like
  a version 1 dump, that recreates the session's data and functions *and* its suspended
  activations, open host calls and in-flight promises. Any later djs either loads it and
  continues the run, or refuses it with a reason (D2560).
- Dumps are taken only at **idle points**: no jobs, no frames on the stack, no parked sync
  call (D2561). The externally visible effects of a codemode run are its tool calls, and
  a run is idle while it waits on them; whatever happens in between reruns
  deterministically.
- In-flight builtin state is written as the **public calls that recreate it** (re-enacted:
  `Promise.all([1, p])`, an iterator advanced with `next()`), not described by engine
  schemas (D2562). What plain JavaScript cannot express goes through a handful of
  privileged loader primitives that exist only while a dump loads.
- The compiler names each `await`/`yield` and describes what a suspended frame holds there
  (its **suspension map**); a test pins the maps, so a lowering change at an await is a
  visible decision (D2563).
- **One dumper, in Rust** (D2564). The TypeScript dumper of D2354 is dropped. Whether wasm
  hosts get it in the main module or in a second module of the same build is decided by
  measurement after images are retired; the plan leans to the second module.
- Heap images stay, but only **inside a process**: the base image (every VM starts from
  it), pi's prelude image as a startup cache, the stress and determinism checks, and as
  the transport between the two modules. Snapshot containers, session images, reference
  compression, `.snapshot`, `Sandbox.bytes()`/`fromBytes()` and pi's
  `snapshot()`/`resumeVm()` go once dumps cover the harvested corpus (0177).
- pi's prelude keeps in-flight state of its own (the pending call table). djs builds the
  **shape pi should move to** in its own adapter (D2565): tools as engine async host
  calls, no in-flight state in the prelude. pi is not changed yet.
- Classes and private fields are in scope (they are refused by version 1 dumps today).

## What a format 2 dump looks like

Format 2 is defined as **a Script run by `Vm::load_dump` with the `$dump` the loader
passes in** (D2581, 0170): every dump this build writes starts with the directive
`"djs-dump 2";` (D2640; before 0180 the comment `// djs-dump 2`), also when it holds only
values; activations are what continuation dumps add to it. The
loader defines `$dump` as a global property (not writable, not enumerable) holding the
primitives as host functions only its run loop serves, removes it when the load ends, and
still runs version 1 dumps as plain Scripts.

A codemode turn:

```js
const files = await tools.glob({ pattern: "src/*.ts" });
const seen = new Map();
async function todos(path) {
  const body = await tools.read({ path });
  const n = body.split("\n").filter((l) => l.includes("TODO")).length;
  seen.set(path, n);
  return n;
}
const report = [];
for (const batch of [files.slice(0, 2), files.slice(2)]) {
  report.push(await Promise.all(batch.map(todos)));
}
text(`${report.flat().reduce((a, b) => a + b, 0)} TODOs in ${seen.size} files`);
```

stopped in the second batch: `src/c.ts` was read and counted, the read of `src/d.ts`
(host call 5) is open. Two activations are suspended (the run, inside `report.push(await
…)`, and `todos("src/d.ts")`, inside `tools.read`), with a half-finished `Promise.all`
between them. Its dump (tests/dump/v2/plan.dump.js loads a version of it):

```js
"djs-dump 2";
$dump.builtin = (path) => { /* as in version 1 */ };
$dump.files = ["src/a.ts", "src/b.ts", "src/c.ts", "src/d.ts"];
$dump.report = [[2, 0]];
$dump.batches = [["src/a.ts", "src/b.ts"], ["src/c.ts", "src/d.ts"]];
$dump.s1 = (() => {
  let seen = new Map([["src/a.ts", 2], ["src/b.ts", 0], ["src/c.ts", 1]]);
  async function todos(path) {
    const body = await tools.read({ path });
    const n = body.split("\n").filter((l) => l.includes("TODO")).length;
    seen.set(path, n);
    return n;
  }
  return { todos, $cells: { seen: $dump.cell(() => seen) } };
})();
$dump.f1 = async function () {
  const files = await tools.glob({ pattern: "src/*.ts" });
  const seen = new Map();
  async function todos(path) { /* … verbatim … */ }
  const report = [];
  for (const batch of [files.slice(0, 2), files.slice(2)]) {
    report.push(await Promise.all(batch.map(todos)));
  }
  text(`${report.flat().reduce((a, b) => a + b, 0)} TODOs in ${seen.size} files`);
};
$dump.it1 = $dump.batches[Symbol.iterator]();
$dump.it1.next();
$dump.it1.next();
$dump.p5 = $dump.pending(5, "tools.read", [{ path: "src/d.ts" }]);
$dump.a2 = $dump.resume($dump.s1.todos, {
  at: "await#1",
  awaiting: $dump.p5,
  this: undefined,
  args: ["src/d.ts", 1, $dump.batches[1]],
  bindings: { path: "src/d.ts", body: $dump.TDZ, n: $dump.TDZ },
});
$dump.p6 = Promise.all([1, $dump.a2]);
$dump.run = $dump.resume($dump.f1, {
  at: "await#2",
  awaiting: $dump.p6,
  this: undefined,
  args: [],
  bindings: {
    files: $dump.files,
    seen: $dump.s1.$cells.seen,
    todos: $dump.s1.todos,
    report: $dump.report,
    batch: $dump.batches[1],
  },
  operands: [["call.callee", Array.prototype.push], ["call.this", $dump.report]],
  loops: [{ kind: "for-of", iterator: $dump.it1,
            next: $dump.builtin("%ArrayIteratorPrototype%.next") }],
});
$dump.handle("run", $dump.run);
```

The dump has no comments (D2641; the `/* … */` are this page's shortening). Read top to
bottom:

- The directive `"djs-dump 2";` names the format (D2640). `$dump` comes from the loader
  with `resume`, `cell`, `pending`, `handle`, `private` and `TDZ`, which exist only while
  the Script runs; everything else is plain JavaScript.
- Data, then the closures: `todos` shares `seen` with the run's frame, so its factory
  hands out the variable's cell too (`$cells.seen`), which the run's bindings take, so
  the frame and the closure share one variable again.
- `$dump.f1` is the turn's script as the host compiled it: never called, it supplies the
  code for the run's activation.
- The for-of's iterator, past the first batch: advancing an array iterator runs no user
  code, so doing it again gives exactly the same state.
- Host call 5, issued before the dump. On load the host decides: deliver a recorded
  result, issue it again, or reject it.
- `todos("src/d.ts")`, called by `batch.map`, so with (element, index, array); its
  `resume` returns the activation's result promise. `Promise.all` over the second batch
  has element 0 (`src/c.ts`) resolved to 1: a plain 1 in its place gives the same record.
- The run, in the second iteration of the for-of. `operands` are the values evaluated
  before the await, named by role, bottom first; the loader puts them where this build's
  compiler keeps them. `$dump.handle` gives the host its handle on the run's result.

Where each activation stopped is what `djs resume --explain` prints, for the file at
hand (tests/dump/v2/plan.dump.js, this example with `later` as the tools' transport):

```text
tests/dump/v2/plan.dump.js: format 2; 1 open call, 2 suspended activations; handles: run
#5 later("TODO four\nok\nTODO five")
  async function todos at dump.js:19:18
      const body = await tools.read({ path });
                   ^
    path = 'src/d.ts', body = <uninitialized>, n = <uninitialized>
  async function main at dump.js:38:17 (run)
      report.push(await Promise.all(batch.map(todos)));
                  ^
    files = [ 'src/a.ts', 'src/b.ts', 'src/c.ts', 'src/d.ts' ], seen = Map(3) { … }, todos = [AsyncFunction: todos], report = [ [ 2, 0 ] ], batch = [ 'src/c.ts', 'src/d.ts' ]
```

Loading runs it as a Script, drains the job queue (here the `Promise.all` job that fills
element 0), and leaves the VM idle, waiting on call 5, where the original run was.

## Design

### Idle points only (D2561)

A dump is taken when `run` returned `Idle`, or before a run starts. No fuel debt, no
parked sync call, no native frame, no regex in mid-match, no half-run instruction can be
in it. A host that has to persist a run that is computing gives it fuel to reach an idle
point, and otherwise kills it: the run since its last tool result reruns the same way,
which is what determinism (D1720) guarantees. Within a process, suspending anywhere (fuel
slices) is unchanged.

### The loader primitives (D2562)

Plain JavaScript writes the data, the closures (version 1's factories), classes,
settled promises, re-enacted builtin state and promise reactions (`.then` calls in their
original order). The loader adds only:

| primitive | what |
| --- | --- |
| `$dump.resume(fn, {at, awaiting, this, args, bindings, operands, loops, …})` | builds a suspended activation of `fn` at the named site, subscribes it to `awaiting`, returns its result promise (async functions) or its generator object (generators: `yield#N`, no `awaiting`) |
| `$dump.cell(() => x)` | the variable cell an arrow that captures exactly one variable closes over, so a frame and closures share it |
| `$dump.pending(id, name, args)` | an open host call with the host's id; its promise |
| `$dump.handle(name, value)` | a named handle for the host (the run's promise, a REPL input) |
| `$dump.private(obj, C, "#x", value)` | a private field (or brand) of the class `C` the dump recreated |
| `$dump.TDZ` | a binding before its declaration ran |
| `$dump.issued(n)` | the id of the host's next call (0171) |

`resume` compiles nothing new: `fn` was made from its verbatim source by the dump. It
compares the given bindings, operand roles and loops with this build's suspension map at
`at` and refuses on any difference. The primitives are privileged builtins reachable only
through the `$dump` the loader passes in; a dump run as an ordinary Script has none of
them.

As built (0171, D2583–D2585):

- `resume(fn, {at, awaiting, this, args, bindings, operands, loops, queue})`. `fn` must
  be made by the dump's own text: the loader compiles that text again with recording on,
  so the map's slots and resume pc are those of `fn`'s code. `bindings` names every
  binding in scope at the site (`name`, or `name@scope` with the declaring scope's
  offset when two of a name are in scope) and nothing else; hidden locals are made by the
  loader (`<state>`, `this`, `new.target`, `<active>`, `<home>`) or given in `bindings`
  (`arguments`). `operands` is a list of `[role, value]` pairs bottom first (roles
  repeat: nested calls), `loops` a list of `{kind, iterator, next}` outermost first
  (for-in loops are refused for now: their iterator is internal), `queue` an async
  generator's requests `{type: "next"|"throw"|"return", value, promise}` with pending
  promises the dump made. Unknown keys, a missing or extra binding, operand, loop or
  queue entry, a non-promise `awaiting` at an await or any `awaiting` at a yield, and an
  unsupported site refuse the load with the reason.
- A captured binding gets a new Box, or the Box of a `$dump.cell(() => x)` (also in its
  TDZ, when the cell is taken before the declaration runs); `$dump.TDZ` and cells are
  values only `resume` takes.
- `pending(id, name, args)`: the host maps `name` to its function id
  (`LoadOptions::host_fn`; an unknown name refuses). `issued(n)`: the id the host's next
  call gets (else one past the largest open id). `handle(name, v)`: a heap handle the host
  gets back. `Vm::load_dump` returns the open calls and the handles; the VM is idle.

### The exporter as built (0172, D2610–D2614)

`Vm::dump(&DumpOptions { base, host, host_fn_name, handles }, names)` writes format 2 with
whatever is in flight when the VM is idle (no frames, no jobs, no parked call, no result
delivered and not applied yet; else it refuses: D2561). The host names its functions
(`host_fn_name`, the inverse of the loader's `host_fn`) and the values it needs back
(`handles`, written last as `$dump.handle(name, v)`; the exporter adds none itself: a host
passes, say, its run's promise as "run").

- **Roots:** the open async host calls (written first, in the host call table's order, as
  `$dump.pending(id, name, args)`), the handles, the selected globals. Everything in
  flight is found from them: a pending promise's reactions lead to the activations that
  await it (and their frames), to `then` handlers and derived promises, to combinator and
  resolving-function records.
- **Pending promises** are written by what made them: an open call, an activation (its
  result promise is `$dump.resume`'s), reaction `j` of another promise (`then`, `catch`,
  `finally` called again with the same handlers), a combinator (`Promise.all`/
  `allSettled`/`any` called again with the settled elements' values, or rejected promises
  for their reasons, in place), `new Promise((res, rej) => ...)` when resolving functions
  are held (user code's, `race`'s, an async function that returned a promise), else
  `new Promise(() => {})`. Each promise's reactions are added in their order; a dump
  whose dependencies would add them in another order is refused. Data that holds such a
  promise gets it by a patch at the end, so an activation can need that data first.
- **Activations:** the suspension map comes from compiling the function's script again
  with the options it was compiled with (Script cells keep them, `Script::OPTIONS` and
  `Script::PARAMS`; D2610), into the dump's scratch copy of the base. The frame is read
  by the map: parameters and bindings in scope by name (`name@scope` when shadowed), the
  `arguments` object if the function has one, operands by role, loops as `{kind,
  iterator, next}`, an async generator's request queue. Bindings that closures share are
  handed over as `$dump.cell`s from their factory (`$cN` keys). The dump's text has no
  comments (the owner's call, D2641: `djs resume --explain` shows where each activation
  stopped, in the dump); `Vm::dump_report` returns, per resumed activation, its location in the dump, the
  function, the site, and the site's file, line, column and source line. Generators that
  have not started are called again (their parameters must run no code); finished ones
  are made finished.
- **Re-enacted iterators:** array, string (by code points), Map and Set iterators are
  made on their target and advanced (`$dump.advance`); a collection only an iterator
  holds is made again with its entries. A `for await` over a sync iterable gets the
  dump's own async-from-sync iterator (`$dump.fromSync`, which steps the same way: the
  object is never visible to user code). Proxies are `new Proxy(target, handler)`.
- **Still refused, with their path:** sites the maps call unsupported (`finally`, `using`,
  destructuring, `yield*`, closing an async iterator), for-in loops, revoked Proxies,
  iterator helpers and RegExp string iterators, weak collections, a generator object
  with another prototype or own properties, keyed combinators (`allKeyed`), an async
  generator awaiting a `return()` value, then-reactions on promises of other
  constructors.
- **TypeScript:** a dump that writes a function of a TypeScript script says so in its
  directive (`"djs-dump 2 typescript";`) and is compiled as TypeScript (D2615).
- **Not carried:** the VM's clock and `Math.random`'s state are the host's to set again;
  unhandled-rejection bookkeeping of promises the session has not looked at yet.

### Re-enactment

Builtin state in flight is written as the calls that make it, wherever the spec makes the
result indistinguishable:

- `Promise.all`/`allSettled`/`any`/`race` and `allKeyed`: called again with the settled
  elements' values (or reasons, for allSettled/any) in place of their promises;
- `then`/`catch`/`finally` reactions: added again in their order; resolving functions
  through `new Promise((res, rej) => …)`;
- array, Map, Set, string and regexp-string iterators: made and advanced with `next()`;
  iterator helpers by their spec state where it is replayable;
- async-from-sync iterators, `for await` state: by their underlying iterator;
- generators and async generators: `$dump.resume` at a `yield` site, since advancing
  one runs user code. Async generators also carry their request queue.

Builtins the run patched are fetched as the originals through `$dump.builtin` (pi's
lockdown freezes them anyway). Whatever has no re-enactment and no primitive is refused
with its path, as in version 1.

### Suspension maps (D2563)

Per `await`/`yield`, computed by compiling the function again on demand (no cost at run
time, nothing in the heap):

- the site: `await#N`/`yield#N` in the function's text order, compiler-made sites (`for
  await`, async generator `yield`/`return`, `await using`) with a kind;
- the bindings live at the site: name and declaring scope, hidden locals by stable name;
- the operand roles below the awaited value, from the vocabulary of
  docs/portable-snapshots.md §3.5 (`call.callee`, `call.this`, `call.arg[i]`,
  `object.literal`, `object.key`, `binary.left`, `assign.*`, `array.literal`,
  `template.prefix`, `new.callee`), produced by the compiler where it emits the
  construct, not inferred;
- the enclosing loops and their iterator records; `switch` discriminants, a `finally`'s
  pending completion, destructuring under way, `using` stacks, `yield*` delegation as
  they are added.

As built (0169, D2570), djs's lowering keeps a few values the vocabulary above does not
name: template literal parts until they are joined (`template.part[i]`, no
`template.prefix`), up to 256 leading array elements (`array.element[i]`, then
`array.literal`), the object of `o[await k]` (`member.object`), and the arguments of `new`,
`super(…)` and spreads (`new.arg[i]`, `super.arg[i]`, `call.args`). Compiler-made sites are
named `yield.await`, `yield.return`, `return.await`, `for-await.next`, `for-await.close`,
`await-using` and `yield*`.

The pin test commits the maps of every await in the corpus and the e2e async tests, like
the bytecode snapshot tests. Accepting a diff means accepting that dumps stopped at those
sites by an older build refuse (or adding a mapping).

### Classes and private fields

Classes are written as their verbatim source inside a factory (they capture like
closures), followed by their static state. Methods using `super` keep their home object
because the class or object literal is evaluated again. Instances are made with the
class's prototype; subclasses of builtins through the builtin (`new Map(entries)`) and then
the prototype. Private fields, private methods and brands go through `$dump.private` with
the recreated class, whose private names are the new ones. This also applies to version 1
dumps of values.

### Open host calls

The dump lists every open call with the host's id. What happens on load is the host's
policy, the dump only states the facts: deliver a recorded result, issue the call again,
or reject it as interrupted. A tool with side effects should not be issued twice; pi's
host keeps calls by id already.

### Compatibility (D2560)

Format 2 promises: every later djs loads the dump and continues, or refuses it with a
reason. It never misreads. Data and functions are as durable as version 1. Activations
depend on the suspension maps (D2563) and on the re-enactments staying exact. Golden
format 2 fixtures are loaded by every gate run; a periodic check exports with an old
commit and imports with the new one (0178).

### One dumper, packaging open (D2564)

The exporter and the loader are Rust in djs-core. A TypeScript copy would have to repeat
every role, re-enactment and class case, kept byte-identical by a gate check, and would
read the engine's generator frames, reaction lists, host call table and builtin closure
kinds; the suspension maps would come from wasm anyway, and the CLI and REPL need Rust
regardless. For wasm hosts two packagings use the same code:

- **A.** one module with the `dump` feature on (+32.4 KB gzip for version 1 today, plus
  continuations and classes), raising the size ceiling, with what 0177 removes as offset;
- **B.** two modules of one build: `djs.wasm` without the dumper and `djs-dump.wasm` with
  it. Images are the transport inside the process: persist = take the idle VM's image,
  load it into the dump module, write the dump; resume = load the dump in the dump module,
  take its image, continue in the small module. The feature is image-neutral (D2352), so
  both modules read each other's images. The main module grows by nothing.

0177 measures both after the image persistence code is gone; the plan leans to B.

### pi's future shape (D2565)

pi's prelude (tests/pi/prelude-source.ts) keeps a `pending` Map of resolve/reject
functions per call id, `nextId`, `finished`, the store's writes and the reactions of
`run()` on the body's promise. All of that would have to travel in a dump as prelude
state. The shape pi should move to, which djs builds in its own adapter first:

- tools are engine async host functions (D1374), so an open call is `$dump.pending`;
- the body's promise is a host handle; `finished` and the store's writes live in the
  adapter;
- the prelude then holds no in-flight state and is rerun on load (it is deterministic,
  D2262); the dump names `tools`, `text`, `console` as host globals.

`tests/pi/prelude-next.ts` is derived from pi's prelude with these changes listed at its
top; `tests/pi/prelude-source.ts` stays a verbatim copy, and pi's suite runs against both.

## Testing

The invariant is the counterpart of `--suspend-every`: **at every idle point, dump, load
into a fresh VM, continue; the transcript must equal the uninterrupted run's.** A
difference is a hard failure, outside the ratchet.

The corpus's fake tools resolve through promise jobs, so its runs have no idle points.
0173 adds an async host function `host.later(v)`: the djs runner delivers it at the next
idle point, under qjs it is `Promise.resolve(v)`. The corpus prelude's fakes and the
harvest importer's in-script fakes return through it, so every tool call is an idle point
and the oracle's `.expected` files stay as they are.

| where | what |
| --- | --- |
| djs-core unit | suspension-map roles per construct and the pin test (0169); hand-written format 2 fixtures and refusals for the loader (0171); round trips per construct for the exporter (0172) |
| e2e | `DJS_E2E_MODE=dump`: the async tests with a dump round trip at every idle point |
| difftest | `--dump-every-idle` over `codemode` and `codemode-harvest`, in the gate next to `--suspend-every 97` |
| coverage | `vm::dump_harvest` at every idle point of the harvested cases and the 2,627 private scripts, counted by refusal reason |
| REPL | line-mode transcripts in `crates/djs-cli/tests/repl.rs` |
| CLI | `djs run --dump-after K` / `djs resume` for every K of a few scripts |
| Sandbox | `js/test/dumpresume.test.ts`: round trips at every idle point; refusals |
| pi | `pi-vm.test.ts` through the adapter; `node js/pi/pi-tests.ts --dump-every-idle` with prelude-next, every wait resumed in a new module instance |
| across builds | golden fixtures `tests/dump/v2/*.js`; the two-commit export/import check |

### REPL

The REPL gets a host-provided `tools`: `tools.x(args)` is an async host call answered by
hand. Inputs that wait do not block the prompt; an input's result prints when it settles.

```
> const r = await tools.read({ path: "a.ts" })
[waiting] #1 tools.read({"path":"a.ts"})
> .pending
#1 tools.read({"path":"a.ts"})  input 1, at repl:1:17
> .dump s.js
wrote 1.4 KB to s.js (1 activation, 1 pending call)
> .clear
> .restore s.js
loaded s.js into a fresh VM; waiting: #1 tools.read({"path":"a.ts"})
> .resolve 1 "export const x = 1;"
> r.length
19
```

- `.pending`; `.resolve <id> <json>`, `.reject <id> <message>`; `.bt <id|all>` for the
  suspended activations and their bindings;
- `.dump-every [on|off]`: dump and reload at every idle point (as `.suspend-every` does
  with images), printing `[dumped N bytes]`; `--tools answers.json` for scripted answers;
- `.dump` writes format 2 when something is in flight; `.restore` restores waiting
  inputs from their handles; `.snapshot` goes with 0177;
- CLI: `djs run --tools answers.json --dump-after K out.js` and `djs resume out.js
  --tools answers.json`; `djs run --dump-every-idle`.

### Sandbox API

`runAsync` awaits the host functions itself and cannot stop at an idle point, so it gets
a lower layer:

```ts
const run = sandbox.begin(code, params);   // runs to the first idle point or the end
run.state;     // "idle" | "done" | "threw"
run.calls;     // [{ id, fn: "tools.read", args }] still open
run.settle(id, { ok: true, value });       // delivers one result
await run.advance();                       // to the next idle point
const text = run.dump();                   // only while idle: a format 2 dump

const { sandbox: s2, run: r2 } = await Sandbox.resume(text, { functions, asyncFunctions });
r2.calls;      // the same open calls; the host settles or re-issues them
```

`runAsync` becomes `begin` + calling the async functions for `run.calls` + `advance` in a
loop. Journals (D1730) stay available on top; persistence does not need them.

## Issues

| issue | what | depends on |
| --- | --- | --- |
| 0169 | Suspension maps in the compiler, pin test | - |
| 0170 | Classes and private fields in dumps; the loader's privileged `$dump` | - |
| 0171 | Loader: `resume`, `cell`, `pending`, `handle`, `TDZ`; hand-written format 2 fixtures | 0169, 0170 |
| 0172 | Exporter: activations, open calls, re-enactment; `--dump-every-idle`, e2e dump mode, coverage | 0171, 0173 |
| 0173 | Idle points in the difftest corpus (`host.later`) | - |
| 0174 | REPL and CLI: tools, pending calls, waiting inputs, format 2 `.dump`/`.restore`, `run --dump-after`/`resume` | 0171 (the tools and pending-call part can land first) |
| 0175 | Sandbox run API (`begin`/`settle`/`advance`/`dump`/`resume`), wasm exports, both packagings built | 0172 |
| 0176 | pi's future shape: prelude-next, adapter mode, `dumpSession`/`resumeFromDump`, pi's suite with `--dump-every-idle` | 0175 |
| 0177 | Retire images as persistence; packaging decided by measurement; guide | 0176 |
| 0178 | Across builds: golden format 2 fixtures, the two-commit check | 0172 |

0169, 0170 and 0173 touch disjoint areas and run in parallel.
