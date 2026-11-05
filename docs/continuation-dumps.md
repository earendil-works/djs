# Continuation dumps: persisting a waiting session as code

Status: as built. The format is the plain dump of docs/plain-js-dumps.md (D2830–D2836,
issues 0199–0207); format 2, a Script that only `Vm::load_dump` could run with
privileged `$dump` primitives (D2560–D2565, D2580, D2581, issues 0169–0178), was retired
in 0205 (D2920). Builds on dumps of values (docs/functions-as-code.md, D2353) and
replaces heap images as the way a session is persisted (D2700).

## The short answer

- A waiting codemode session is written as a **dump**: an ordinary JavaScript Script that
  recreates the session's data and functions *and* its suspended activations, open host
  calls and in-flight promises. Every suspended activation is a **continuation**, a
  function of the same kind whose body is the rest of the original from where it waits
  (D2830). The Script runs in djs and in node, given the host's functions and one host
  object, `$djs` (D2834); no engine machinery runs it.
- Dumps are taken only at **idle points**: no jobs, no frames on the stack, no parked sync
  call (D2561). The externally visible effects of a codemode run are its tool calls, and
  a run is idle while it waits on them; whatever happens in between reruns
  deterministically.
- In-flight builtin state is written as the **public calls that recreate it**
  (`Promise.all([1, p])`, an iterator advanced past the steps it took), not described by
  engine schemas (D2562). What needs a few lines of code is a helper of the **dump
  runtime**, the members of `$djs` every dump shares (D2903).
- Dumps are written only for VMs with **frozen builtins** (D2836): the exporter may then
  write a builtin a frame already looked up as the expression that finds it again.
- A dump of a run that resumed a dump writes the *original* functions again, not the
  continuations (**origin tracking**, D2833, D2870): re-dumps stay the size of a dump of
  the uninterrupted run.
- **One dumper, in Rust** (D2564, D2831). Wasm hosts get it in a second module of the same
  build (packaging B, D2704); loading a dump needs no dumper at all, since it is code.
- Heap images stay, but only **inside a process**: the base image, pi's prelude image as a
  startup cache, the stress and determinism checks, and the transport between the two
  modules.

## What a dump looks like

A codemode turn (tests/dump/v2/plan.src.js, its tools answering through `later`):

```js
async function main() {
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
  print(`${report.flat().reduce((a, b) => a + b, 0)} TODOs in ${seen.size} files`);
  return report;
}
main().then((r) => print(JSON.stringify(r)));
```

stopped in the second batch: `src/c.ts` was read and counted, the read of `src/d.ts`
(host call 5) is open. Two activations are suspended (`main`, inside `report.push(await
…)`, and `todos("src/d.ts")`, inside `tools.read`), with a half-finished `Promise.all`
between them. `djs run --dump-after 5 plan.js tests/dump/v2/plan.src.js` writes (the
`contents`/`tools` lines of the program and `main`'s text shortened here):

```js
"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 6);
globalThis.$dump = {};
$dump.s2 = {};
async function main() { /* … verbatim … */ }
$dump.p1 = $djs.resume(5, "later", ["TODO four\nok\nTODO five"]);
$dump.n3 = new Map([["src/a.ts", 2], ["src/b.ts", 0], ["src/c.ts", 1]]);
$dump.n4 = ["src/c.ts", "src/d.ts"];
$dump.n7 = [[2, 0]];
$dump.i8 = $djs.advance(Array.prototype.values.call([["src/a.ts", "src/b.ts"], $dump.n4]), 2);
$dump.ga9 = () => {
$djs.k($dump.s2.$Ka5, $dump.s2.todos);
$dump.a5 = $dump.s2.$Ka5.call(undefined, "src/d.ts");
$dump.p6 = Promise.all([1, $dump.a5]);
};
$dump.a9 = $djs.k(async function (files, report) {
const $djs_I = $djs.I;
const seen = $dump.n3;
Object.assign($dump.s2, { todos: async function todos(path) {
    const body = await tools.read({ path });
    const n = body.split("\n").filter((l) => l.includes("TODO")).length;
    seen.set(path, n);
    return n;
  }, $Ka5: async function (path) {
const body = await $dump.p1;
const n = body.split("\n").filter((l) => l.includes("TODO")).length;
seen.set(path, n);
return n;
} });
var todos = $dump.s2.todos;
$dump.ga9();
const $djs_i1 = $dump.i8;
report.push(await $dump.p6);
for (const batch of $djs_I($djs_i1)) {
    report.push(await Promise.all(batch.map(todos)));
  }
print(`${report.flat().reduce((a, b) => a + b, 0)} TODOs in ${seen.size} files`);
return report;
}, main, 2, 54, [0,531])(["src/a.ts", "src/b.ts", "src/c.ts", "src/d.ts"], $dump.n7);
$dump.p10 = Promise.prototype.then.call($dump.a9, (r) => print(JSON.stringify(r)), undefined);
try { ($dump.p10); } finally { delete globalThis.$dump; delete globalThis.$djs; }
```

The dump has no comments (D2641; the `/* … */` and `[…]` are this page's shortening).
Read top to bottom:

- The directive `"djs-dump";` names the format (D2835; `"djs-dump typescript";` when a
  function's TypeScript is more than types, D2879). The Script is strict.
- `$djs.seed(lo, hi, next)`: `Math.random`'s state and the id the dumped VM's next host
  call gets (D2897). `$dump` is an ordinary global object the dump makes for its own
  locations and deletes at the end, with `$djs`.
- Host call 5, issued before the dump: `$djs.resume(id, name, args, state)` asks the host
  to continue it, under the same id. What it means is the host's policy (deliver a
  recorded result, issue it again, reject it); the hook returns the call's promise.
- Data: the Map, the batches, the report; the for-of's iterator advanced past the first
  batch (`$djs.advance` runs no user code).
- `$dump.a9` is what `main`'s continuation returned: an async function written in place,
  given its origin record by `$djs.k` and called at once (D2909). Its parameters are
  bindings live at the site (`files`, `report`), passed as the call's arguments; a binding
  shared with closures (`seen`) is declared, and the closures of the frame's scopes are
  made where they are declared (`todos` shares `seen` with the frame, so it is made inside
  the continuation). The rest of the current loop iteration comes first, the half-done
  `report.push(await …)` reading `report.push` again (D2905), then the loop goes on over the
  saved iterator, `for (const batch of $djs_I($djs_i1))` (D2908). The record names its
  origin (`f: main`, the site, the stretches copied from `main`), which a later dump reads
  back (origin tracking).
- `$dump.s2.$Ka5` is `todos("src/d.ts")`'s continuation. It is made by `a9`'s prefix (it
  captures `seen`), so the part of the dump that needs it, starting the activation and the
  `Promise.all` over it, is a callback the continuation calls once the block that makes it
  has its code (`$dump.ga9`, D2907).
- The Script's completion value is what the host named, here the run's promise
  (D2895).

Where each activation stopped is what `djs resume --explain` prints:

```text
plan.js: 1 open call, 2 suspended activations; handles: run
#5 later("TODO four\nok\nTODO five")
  async function $Ka5 at plan.js:43:14
      const body = await $dump.p1;
                   ^
    path = 'src/d.ts', body = undefined, n = undefined
  async function <anonymous> at plan.js:51:13
      report.push(await $dump.p6);
                  ^
    files = [ 'src/a.ts', 'src/b.ts', 'src/c.ts', 'src/d.ts' ], report = [ [ 2, 0 ] ], seen = Map(3) { … }, todos = [AsyncFunction: todos], batch = undefined
```

(`batch = undefined`: the rest of the iteration no longer reads it, so the continuation
leaves it out, D2911.)


Loading is running the Script with `$djs` defined, to its first stop that is not a sync
host call: the `Promise.all` job fills element 0 and the VM is idle, waiting on call 5,
where the original run was.

## Design

### Idle points only (D2561)

A dump is taken when `run` returned `Idle`, or before a run starts. No fuel debt, no
parked sync call, no native frame, no regex in mid-match, no half-run instruction can be
in it. A host that has to persist a run that is computing gives it fuel to reach an idle
point, and otherwise kills it: the run since its last tool result reruns the same way,
which is what determinism (D1720) guarantees. Within a process, suspending anywhere (fuel
slices) is unchanged.

### `$djs`: the hooks and the dump runtime (D2834, D2890, D2903)

The host defines one global object while a dump loads, and the dump deletes it in the
`finally` that ends its Script:

- `seed(lo, hi, next)`, the dump's first call; a djs host sets `Math.random`'s state and,
  once the Script has stopped, the id counter to `next` (or one past the highest reopened
  id): the hooks are sync calls with ids of their own (D2897);
- `resume(id, name, args, state)` per open call; a djs host reopens call `id` of its
  function `name` under that id (`Vm::open_host_call`) and returns its promise. An unknown
  name throws and the load fails;
- the runtime: `R`, `L`, `advance`, `fromSync`, `keep`, … (docs/plain-js-dumps.md §4 has
  the table), defined once in crates/djs-core/src/vm/dump_runtime.rs. A djs host gets the
  object from `Vm::dump_runtime()`, a host without djs imports js/src/dump-runtime.js. A
  continuation captures the helpers it calls (`const $djs_L = $djs.L`).

- `host(name)`: a host value by name (D2971). A host function the dump holds is written
  as `$djs.host(identity)` (`$djs.host("tools.read")`, D2970), a value the host made and
  names (a root: `DumpOptions::host_roots`, and the globals in `DumpOptions::host` by
  their names) as `$djs.host(name)`, its own data properties as reads on it
  (`$djs.host("tools").list`). The host returns the object it bound under that name.

Hosts
run dumps themselves (D2891): the CLI (crates/djs-cli/src/dumps.rs), the Sandbox
(js/src/sandbox.ts `loadPlain`), pi's adapter (js/pi/vm.ts), the difftest. Sync host
functions the dump's code calls while it loads are served as in a run.

### The exporter (D2610–D2614, D2860, D2870)

`Vm::dump(&DumpOptions { base, host, host_roots, host_fn_name, handles, call_states },
names)` writes the idle VM (else it refuses: D2561), and refuses a VM without frozen
builtins (D2836). The host names its functions (`host_fn_name`: their identities), the
globals it defines itself (`host`: re-bound by name, never written; each one's value a
root by its name), other values it made (`host_roots`), the values it needs back (`handles`, the
Script's completion value: the value itself for one named `run`, else a null-prototype
object of all, D2895) and its state of each open call (`call_states`, JSON text written as
`resume`'s fourth argument after a check that it is JSON data, D2770).

The exporter reads the heap as format 2's did (vm/dump.rs, vm/dump_flight.rs) and writes
its lines plus an IR of factories and activations, which the plain writer
(vm/dump_plain.rs) and the continuation generator (vm/dump_cont*.rs) turn into the Script
(D2860):

- **Roots:** the open async host calls (in the host call table's order), the handles, the
  selected globals. Everything in flight is found from them: a pending promise's
  reactions lead to the activations that await it (and their frames), to `then`
  handlers and derived promises, to combinator and resolving-function records.
- **Pending promises** are written by what made them: an open call, an activation (its
  continuation's result), reaction `j` of another promise (`then`, `catch`, `finally`
  called again with the same handlers), a combinator called again with the settled
  elements' values in place, `new Promise((res, rej) => …)` when resolving functions are
  held, else `new Promise(() => {})`. Each promise's reactions are added in their order.
- **Activations** are read by the function's suspension map (below), compiled again from
  its script on demand, and written as continuations by the site's syntax path (D2850):
  the prefix, the site's statement rewritten (values evaluated before the site come from
  the dump, a call keeps its evaluated callee and `this`), the loops, `try`, `switch`,
  labels and `using` scopes around it, and the rest of the function verbatim. Generators
  are primed to a placeholder `yield`; an async generator's queued requests are made
  again. Constructs with their own encodings: `for await` (D2872), destructuring (D2874),
  `yield*` (D2875), `using` (D2876), iterator helpers (D2877), loop heads and `var`
  (D2880), methods using `super` (D2871), classes a waiting frame shares (D2881).
- **Origin tracking:** a continuation's frame is read as one of its origin (`$djs.k(K, f, at, r, h)`: the origin, its site, the decisions and the holes that make
  `K`'s layout again, D2913, D2915), so the next continuation is written from the original's text (D2833,
  D2870, D2873).
- **Codemode bodies:** a run compiled as an async body has no function text of its own;
  its continuation is `async function (tools, console) {…}` over the body's text, and
  what only its activation holds is pruned (docs/plain-js-dumps.md §5).
- **Refused, with their path:** instances that carry private names (D2878), sites the
  maps call unsupported (inside a class definition), a for-in loop over a Proxy, revoked
  Proxies, weak collections, keyed combinators, then-reactions on promises of other
  constructors, computed keys the dump would evaluate again unless their value cannot
  differ (D2623), a codemode body whose frame differs as a function (D2617).
- **The checker** compiles the dump and compares every function it writes with the
  original: a name the original captured must be captured in the dump too, a global it
  read must stay a global (D2618).

### Suspension maps (D2563)

Per `await`/`yield`, recorded by a compile with recording on (no cost at run time,
nothing in the heap): the site (`await#N`/`yield#N`, compiler-made sites by kind), the
bindings live there, the temporaries the frame keeps, each by slot name (`call.callee@41`,
`for-of.iterator@18`, `completion.kind@23`: the construct and its offset in the
function's text, D2721, D2730, D2731), and the syntax around the site (D2850–D2853). The
pin test commits the maps of every await in the corpus (tests/smap).

With format 2 the maps were a contract between builds: a dump named slots, and a later
build loaded it only if its compiler kept the same slots at the same site. A plain dump
names no slot, so the maps are only the exporter's way of reading a frame of this build;
the pins are a regression test of the compiler, not a format promise.

### Classes

Classes are written as their verbatim source inside a factory (they capture like
closures), followed by the static state they changed (`$djs.keep` deletes members the
class no longer has). Methods using `super` keep their home object because the class or
object literal is evaluated again. Instances are made with the class's prototype;
subclasses of builtins through the builtin. An object that carries a private field,
method or accessor is refused: only the class's own construction can add a private name
(D2878).

### Open host calls

The dump lists every open call with the host's id and the host's state of it (D2770):
`$djs.resume(id, name, args, state)`. The engine keeps nothing of the state between
dumps. What happens on load is the host's policy: deliver a recorded result, issue the
call again (from its state), or reject it as interrupted. The Sandbox's tasks run that
policy from hooks the host function declares (D2771).

### Compatibility (D2560, D2920)

A dump is JavaScript that calls only `$djs`, so any later djs, or node, runs it; what it
depends on is the dump runtime and the language, not the compiler. The golden dumps
(tests/dump/v2/golden/, written once, run by every gate) hold the runtime and the engine
to that (TESTING.md "Dumps across builds"); djs is unreleased, so the runtime may still
change incompatibly by a decision (D024).

### One dumper; packaging B (D2564, D2704)

The exporter is Rust in djs-core (feature `dump`). Wasm hosts get it in a second module of
the same build: `djs.wasm` without the dumper and `djs-dump.wasm` with it. To dump, the
host takes the idle VM's image, loads it into the dump module and writes the dump there
(images are the transport inside the process; the feature is image-neutral, D2352). To
resume, the host runs the dump in the shipped module itself, which needs only
`vm_open_call`, `vm_dump_runtime` and `vm_set_next_call_id` (D2892). Hosts store dumps
zstd-compressed (D2703).

### pi's shape (D2565)

pi's prelude keeps in-flight state of its own (the pending call table, `finished`, the
store's writes). The shape pi should move to, which djs's adapter builds with
tests/pi/prelude-next.ts: tools are engine async host functions, so an open call is a
`$djs.resume`; the body's promise is a host handle; `finished` and the store live in the
adapter; the prelude holds no in-flight state and is rerun on load. docs/host.md
"pi's next mode" has the API.

## Testing

The invariant is the counterpart of `--suspend-every`: **at every idle point, dump, run
the dump in a fresh VM, continue; the transcript must equal the uninterrupted run's.** A
difference is a hard failure, outside the ratchet. The corpus's tools answer through the
async host function `host.later(v)` (0173), delivered at the next idle point, so every
tool call is an idle point.

| where | what |
| --- | --- |
| djs-core unit | the suspension-map pins (tests/smap); dumps of values and of waiting programs, the exporter's refusals and reports (vm/dump_tests.rs, vm/dump_resume_tests.rs); every idle point of the tests/dump/v2 programs plainly and in chains of re-dumps; the golden dumps |
| e2e | `DJS_E2E_MODE=dump`: every case's final state dumped, run and dumped again |
| difftest | `--plain-dumps` over the corpus, chains of at most 50 links, and each link run in node (`scripts/plain-node.mjs`); the gate leg "plain dumps (djs and node)", with the size score (D2901) |
| REPL, CLI | line-mode transcripts in crates/djs-cli/tests/it/repl.rs; `djs run --dump-after K` / `djs resume` for every K of a few scripts |
| Sandbox | js/test/dumpresume.test.ts and task.test.ts in both packagings; plain.test.ts |
| pi | pi-vm.test.ts through the adapter; `node js/pi/pi-tests.ts --next --dump-every-idle` (a full-gate leg) |

The REPL (`djs repl --frozen`: `.dump`, `.restore`, `.dump-every`, `.pending`, `.bt`) and
the CLI (`run --dump-after`, `--dump-every-idle`, `resume`, `resume --explain`) are in
docs/cli.md; the Sandbox's `begin`/`settle`/`advance`/`dump`/`loadRun` and tasks in
docs/host.md.

## History

| issues | what |
| --- | --- |
| 0169–0178 | format 2: suspension maps, classes, the loader and its primitives, the exporter, idle points in the corpus, REPL and CLI, the Sandbox run API and packagings, pi's next mode, images retired as persistence, golden dumps |
| 0191–0196 | slot names, construct temporaries in locals, one compile per load |
| 0199 | plain-JavaScript dumps: research and prototype (docs/plain-js-dumps.md) |
| 0200–0207 | site syntax in the maps, the plain exporter, origin tracking, the remaining constructs, host hooks, the dump runtime and size |
| 0205 | format 2 retired: the loader, `$dump.resume` and the primitives, the recording compile at load and the `jump_if_loading` opcode deleted; goldens rewritten as plain dumps (D2920) |
