# How continuation dumps are written

[Continuation dumps](dumps.md) shows what a dump is to a host: a plain Script, written
while the run waits for its tools, that recreates the run when any later djs runs it. This
chapter is about the writer: how it gets from a heap of frames, promises and iterators to
that Script, and how it keeps a dump of a resumed run as small as the first one.

> **In short.** The writer reads each suspended frame by its **suspension map**, a
> description by name of what the frame holds at that `await` or `yield`, which the
> compiler records on demand. It writes the frame as a **continuation**: a function of the
> same kind whose body is the rest of the original from where it waits. Builtin state in
> flight (promises, combinators, iterators) is written as the public calls that put it
> there. Every continuation carries a small **origin record**, so a dump of a resumed run
> is written from the original code again and does not grow. Fixed helpers come from the
> host as the **dump runtime**, and the script the run started with is named by
> reference.

## From a heap to a Script

A dump is taken only at an **idle point** ([dumps](dumps.md#idle-points)): no frame on
the stack, no job queued, and the run waits for async host calls. There, every piece of
the run in progress is an async function or generator stopped at an `await` or `yield`,
which the compiler can describe by name. Nothing is in the middle of an instruction, a
builtin or a regular expression match.

The writer starts from the roots a host names: the open host calls, in the order they
were made; the values the host wants back, such as the run's result promise; and the
global bindings the run made. It walks the heap from there and writes what it finds:

- **data** as the literals and constructor calls that make it, an object reached twice
  or on a cycle once, on `$dump`, and patched where a cycle closes;
- **builtins** by the expression that finds them (`Array.prototype.push`), from a table
  of the base's builtins and their paths that is built with the base, never by their
  contents;
- **functions** by their source text, closures that share variables made by one factory,
  so that sharing stays shared;
- **classes** by their text with their static code blanked out (its effects are in the
  heap and written after the class), so evaluating the class again runs nothing twice;
- **what is in flight**, the suspended activations, pending promises and iterators, as
  the rest of this chapter describes.

The result is checked before it is returned: the writer compiles the dump and compares
every function it wrote with the original. A name the original captured must be captured
in the dump too, and a global it read must still be a global, or the loaded function
would read another binding than it did; such a dump is refused instead.

All of it assumes **frozen builtins**. The writer reads a builtin the run looked up as
the expression that finds it again, and a method a half-done call already read as a
second read of it. That gives the same value only if no builtin changes between the run's
read and the load, which frozen builtins guarantee ([the language](language.md#frozen-builtins)).

## Suspension maps

A suspended frame is a slice of the value stack copied into a `Generator` cell
([suspension](suspension.md#generators-and-async-functions)): slots without names. To
write it as source, the writer needs to know what each slot is. The compiler can tell, and
records it as a **suspension map** for every `await` and `yield` of a function:

- the **site**, by kind and number: `await#2`, `yield#1`, `for-await.next#1`;
- the **bindings** live there, by name and scope, with the slots that hold them;
- the **temporaries**: the values a construct keeps on the operand stack while a nested
  expression is evaluated, named by the construct and the source offset of what keeps
  them (`call.callee@41`, `for-of.iterator@18`, `completion.kind@23`);
- the **syntax around the site**: every statement and expression on the path from the
  function's body down to the site, with the spans of its parts (a loop's head and body, a
  `try`'s blocks, a call's arguments).

The compiler is single-pass and builds no syntax tree ([no syntax
tree](goals.md#no-syntax-tree)), so the maps cannot be read off a tree afterwards.
Instead the parser keeps a stack of names as it emits code: it pushes a construct's name
when it leaves a value on the operand stack and pops it when the value is consumed, so at
a site the name stack is the operand stack below the awaited value. While recording, it
also keeps every syntax node that encloses a site; everything else stays text that the
writer copies by span. A normal compile records nothing: the maps are computed when a
dump needs them, by compiling the function's script again with recording on, and kept for
the VM's later dumps. A site whose names don't add up to the stack depth of the compiled
code, or one inside a class definition, whose half-made class no dump could write, is
marked unsupported rather than misnamed.

> **Trade-off.** Recording maps on demand over keeping them with the code. Every compile
> and every VM stays as small and fast as without dumps, and the maps exist only for the
> functions that are suspended when a dump is written. The cost: a dump compiles the
> scripts of its suspended functions a second time, which is a large part of a small
> dump's cost; the dump module's caches ([below](#the-dumpers-caches)) take most of it
> away for a host that dumps the same scripts again and again.

The dump itself names no slot. The maps are only the writer's way of reading a frame of
this build, so a later djs with a different compiler loads the dump all the same.

## Continuations

For an activation of a function `f` stopped at a site, the writer writes a function `K`
of `f`'s kind (async function, generator, async generator; arrows as arrows) whose body
is the rest of `f` from the site:

- **the prefix**: the bindings live at the site, as `K`'s parameters with the frame's
  values as arguments, or declared in `K` where closures share them (below);
- **the site's statement, rewritten**: the site itself becomes an `await` of the promise
  the frame waits on (a generator's a placeholder `yield`), values the statement had
  evaluated before the site come from the frame, and the rest of the statement is `f`'s
  text. An operand that is pure to evaluate again and gives the saved value is written as
  `f`'s text (`results.push(await p)` reads `results.push` again); any other keeps the
  saved value, through a helper such as `$djs.R`, which is `Reflect.apply`;
- **the loops around the site**: the rest of the current iteration first, then the loop
  as written over the saved iterator, or a loop whose first step is a placeholder where
  the body can jump to the loop. `try`, `switch`, labels and `using` scopes stay around
  the rest; a suspended `finally` is its rest followed by the completion that entered it;
- **the rest of `f`**, verbatim.

`K` is written where it is called and called once, with `f`'s `this`. Its prefix runs to
the site while the dump loads, so the loaded run waits exactly where the original did. A
generator's continuation is primed with one `next()` to its placeholder `yield`; an async
generator's queued requests are made again on it, so their promises are the ones the
data holds.

**The continuation is the factory of its frame's scopes.** A variable a frame shares with
closures is declared inside `K`, in the block where `f` declares it, and the closures
that capture it are made right there. So sharing survives the dump, and a variable still
in its temporal dead zone is simply a declaration in the rest of `K` that has not run yet.
It also decides where continuations go: a continuation of a function made in a live frame
is written inside the continuation of that frame, and a value the dump uses once is
written where it is used, as long as that keeps the order in which the load does things.

Here a run pauses inside `measure`, which `Promise.all` waits for, while an array
iterator has taken one step:

```js live
const words = ["alpha", "beta", "gamma"];
const it = words.values();
async function measure(w) {
  debugger;            // the run waits here, as on a tool call
  return w.length;
}
const lengths = await Promise.all([1, measure(it.next().value)]);
for (const w of it) lengths.push(w.length);
console.log(lengths.join(" "));
return lengths;
```
<!-- gen:run -->
```text
1 5 4 5
```
<!-- /gen -->

The dump at the pause has two continuations: the body's, which awaits a `Promise.all`
made again with its settled element in place, and inside that call `measure`'s, which
waits on the open call of the pause. The iterator `it` is made again as an array iterator
at the step it had reached:

<!-- gen:dump -->
```js
"djs-dump";
globalThis.$dump = {};
$dump.n2 = async function measure(w) {
  debugger;            // the run waits here, as on a tool call
  return w.length;
};
$dump.n5 = ["alpha", "beta", "gamma"];
$djs.k(async function (it) {
const lengths = await Promise.all([1, $djs.k(async function (w) {
await $djs.resume(1, "debugger", []);
return w.length;
}, $dump.n2)("alpha")]);
for (const w of it) lengths.push(w.length);
console.log(lengths.join(" "));
return lengths;
}, $djs.source("example.js", "1694bb51"))($djs.advance(Array.prototype.values.call($dump.n5), 1));
```
<!-- /gen -->

## Builtin state, re-enacted

What builtins hold in flight is engine state: a promise's reactions, the elements a
`Promise.all` has collected, how far an iterator has gone. The writer does not describe
that state; it writes the public calls that put it there, so the dump depends on the
language, not on the engine's internal layout:

- a **pending promise** is written by what made it: an open host call (`$djs.resume`), an
  activation (its continuation's result), a reaction of another promise (`then`, `catch`
  or `finally` called again with the same handlers), a combinator (`Promise.all` and its
  kin called again, the elements that had settled written as their values), or
  `new Promise` whose resolving functions go to whoever held them. A promise that
  nothing can settle any more is `new Promise(() => {})`. Each promise's reactions are
  added again in their order, so jobs run in the original order;
- an **array iterator** is made again over its array and stepped as far as it had gone
  (`$djs.advance`), or, where nothing else holds the array, made over what is left of it;
- **iterator helpers** (`map`, `take`, `flatMap`, ...) are made again by the public
  methods over the saved underlying iterator, with a callback's counter shifted by the
  steps already taken;
- a **`for in`** loop goes on over the keys it had left, skipping ones deleted since; a
  **`matchAll`** iterator is made again on a copy of the regular expression; a `for
  await` over a sync iterator, a `yield*` delegation and the resources of a `using`
  scope go on through small wrappers of the saved records.

> **Trade-off.** Re-enacting through public calls over describing the engine's records.
> A record layout would be quicker to write and could capture anything, but it would tie
> every dump to the build that wrote it, which is what dumps exist to avoid. Public calls
> mean the same in every djs, and in node. The cost: the writer has to know how to
> re-enact every kind of builtin state, construct by construct, and it refuses what it
> cannot (below).

## Origin tracking

Once a dump is loaded and runs, the suspended frames are frames of the continuations, not
of the original functions. Dumped again as they are, the next continuation would be
written from `K`'s text, which holds the previous dump's prefix and scaffolding, and a
chain of resumes would grow with every link.

So each continuation carries its **origin record**: `$djs.k(K, f, at, r, h)` attaches it
to `K` as it is made. It names

- `f`, the original function: a reference when the run holds it, else the script it came
  from ([below](#sources-by-reference));
- `at`, the site of `f` that `K` stands for (`2` for the second `await`);
- `r`, the decisions the writer took on the run's values that shaped `K`'s text: which
  operands it reads again, a completion's kind; as one number when they are all yes or no;
- `h`, the stretches of `K`'s text that depend on values and that `f`'s site alone does not
  give.

When the writer meets a frame of `K`, it writes `K` again from `f`, `at`, `r` and `h`,
which tells it which stretches of `K`'s text are copies of `f`'s and which are its own
constructs. Through that layout, `K`'s site maps to `f`'s, `K`'s bindings to `f`'s by name
and `K`'s temporaries to `f`'s slots. The frame is then read as a frame of `f`, and the
next continuation is written from `f`'s text, as in the first dump. The code a dump
carries is always the original's, so a dump stays the size of a dump of the uninterrupted
run however many times the run was resumed. To keep this exact, the writer writes every
continuation twice, as it is and again from its origin record, and refuses a dump where
the two differ in more than the recorded stretches.

## Sources by reference

A codemode script runs as the body of an async function the host compiled, which has no
function text of its own in the heap. Its continuation's origin is the script, and the
host has that script already. So a dump names it by reference: `$djs.source(name, hash,
params)`, with the file name the run was started under, its parameters and a 32-bit hash
of the parameters, the compile options and the text. The loading host compiles the script
again, checks the hash and hands back the function, which the continuations name as their
origin and never call. A host that would rather not keep scripts asks for the text in the
dump instead ([the script, by reference](dumps.md#the-script-by-reference)).

So the body's text is in the dump once, as the rest that the continuation holds; the part
that already ran, which never runs again, is not in it at all.

## The dump runtime

A continuation needs a few lines of help here and there: `Reflect.apply` for a call whose
callee was evaluated before the site, an iterable over a saved iterator whose first step
is a placeholder, an iterator advanced by `n` steps, an `ArrayBuffer` from base64, the
call that attaches an origin record. These helpers are the same in every dump, so the
host provides them as members of `$djs`, next to its hooks, instead of each dump carrying
them:

| member | what it is |
| --- | --- |
| `seed`, `resume`, `source`, `host` | the host's hooks: `Math.random`'s state and the next call id, an open call, the script by reference, a host value by name ([dumps](dumps.md#djs-what-the-loading-host-provides)) |
| `R`, `G`, `C`, `A` | `Reflect.apply`, `Reflect.get`, `Reflect.construct`, `Object.assign`, captured while the dump loads |
| `L`, `I`, `E` | iterables over a saved iterator record, with or without a placeholder first step, and one that is done |
| `advance`, `forIn`, `regexpIterator`, `fromSync`, `M`, `FM`, `CC`, `Y`, `U`, ... | builtin state in flight: iterators, helpers, `yield*` and `using` |
| `k`, `keep`, `bytes`, `spent`, `rejected`, ... | an origin record, a class's deleted members, binary data, settled promises |

The helpers are defined once, in Rust, as a table of JavaScript texts; the file a host
without djs imports, [`js/src/dump-runtime.js`](../../js/src/dump-runtime.js), is written
from the same table, and a test keeps it current. A djs host evaluates the runtime in the
loading VM as a Script of only the helpers the dump names, adds its hooks and defines the
object as `$djs`; the wasm module takes the runtime's text from the host, so the shipped
module does not carry it. Each helper is a plain function that uses neither `this` nor
another member, and a continuation captures the ones it calls (`const $djs_L = $djs.L`),
since `$djs` is deleted once the Script has run.

The hooks are sync host functions of the loading VM. `$djs.resume(id, name, args, state)`
opens call `id` of the host function `name` again under the same id and returns its
promise; once the Script has stopped, the host sets the id the run's next host call gets,
so the resumed run numbers its calls as the uninterrupted one would.

## What the writer refuses

The writer refuses a value it cannot write as JavaScript that recreates it, and names it
by its path from the roots; the run goes on and only the dump fails.
[Continuation dumps](dumps.md#what-cannot-be-dumped) lists what a script can meet. The
reasons come from the mechanism above:

- **No public call recreates it.** A weak collection's contents depend on what else is
  reachable, and nothing lists them. A private name can be added to an object only by its
  class's own construction. A revoked `Proxy` keeps neither its target nor its handler,
  so there is nothing to make it from. A then-reaction on a promise of another
  constructor has no call that puts it there.
- **Recreating it would be visible.** The keys left in a `for in` over a `Proxy` would
  have to come from calls of its traps, which the host's code would see; a decorated
  class would run its decorators again; a computed key would be evaluated again unless
  its value cannot differ.
- **The map cannot name it.** A site inside a class definition, or a frame the map does
  not describe, has no continuation.
- **The check fails.** A function whose names would resolve to other bindings after the
  load.

## The dumper's caches

The writer is in the dump module, a build of djs-wasm with the dumper that a host loads
next to the shipped module ([the module builds](host.md#the-module-builds)). To dump, the
host copies the VM's image into a VM of that module and writes the dump there. The host
keeps that instance and VM between dumps, replacing only its heap with the next image,
so what the VM derived once (its suspension maps among it) serves the next dump too. The
table of the base's builtins and their paths is built with the base and part of the
module, so no dump walks the base to find them.

A host that resumes each dump in a new instance, and dumps it again at the next idle
point, compiles the same texts in every instance: the script the run started with (the
answer to `$djs.source`) and its own fixed snippets. The dump module keeps two caches for
it, as bytes the host holds per module:

- **The compile cache.** A compile only appends cells to the arena and interns atoms. A
  compile done through `vm_compile_cached` records both, in order: the cells it appended
  and every atom it interned between them. The next VM imports that record instead of
  compiling: it copies the cells, rebases the references inside them, and interns the
  same names at the same points, renumbering atom ids where it numbered them otherwise.
  It ends up byte for byte as if it had compiled the text itself, which the determinism
  checks compare.
- **Suspension maps.** After a dump that compiled maps, `vm_dump_maps_export` gives them
  as bytes, and `vm_dump_maps_import` hands them to the next VM before its dump. The maps
  are data of the text and its options alone, so the dump is the same text either way.

The dump module also compiles functions lazily, at their first call, and the writer reads
a function that never ran from its text: a dump of a lazy VM is the same text as of an
eager one.

## Where to look

- [`crates/djs-core/src/vm/dump.rs`](../../crates/djs-core/src/vm/dump.rs): the writer:
  roots, data, functions, classes, refusals and the checker.
- [`crates/djs-core/src/vm/dump_flight.rs`](../../crates/djs-core/src/vm/dump_flight.rs):
  what is in flight: open calls, promises and their reactions, combinators, iterators;
  [`crates/djs-core/src/vm/dump_flight_helpers.rs`](../../crates/djs-core/src/vm/dump_flight_helpers.rs):
  iterator helpers.
- [`crates/djs-core/src/vm/dump_plain.rs`](../../crates/djs-core/src/vm/dump_plain.rs):
  the Script: factories merged into continuations, placement, the completion value;
  [`crates/djs-core/src/vm/dump_plain_inline.rs`](../../crates/djs-core/src/vm/dump_plain_inline.rs):
  values written where they are used.
- [`crates/djs-core/src/vm/dump_cont.rs`](../../crates/djs-core/src/vm/dump_cont.rs):
  continuations; [`crates/djs-core/src/vm/dump_reread.rs`](../../crates/djs-core/src/vm/dump_reread.rs):
  operands read again; the other `dump_cont_*.rs` files beside it: loop heads,
  destructuring, `yield*`, `using`.
- [`crates/djs-core/src/vm/dump_origin.rs`](../../crates/djs-core/src/vm/dump_origin.rs)
  and [`crates/djs-core/src/vm/dump_cont_canon.rs`](../../crates/djs-core/src/vm/dump_cont_canon.rs):
  origin tracking.
- [`crates/djs-core/src/compiler/smap.rs`](../../crates/djs-core/src/compiler/smap.rs)
  and [`crates/djs-core/src/compiler/smap_syntax.rs`](../../crates/djs-core/src/compiler/smap_syntax.rs):
  suspension maps and the syntax around a site;
  [`crates/djs-core/src/compiler/smap_codec.rs`](../../crates/djs-core/src/compiler/smap_codec.rs):
  the maps as bytes.
- [`crates/djs-core/src/vm/dump_runtime.rs`](../../crates/djs-core/src/vm/dump_runtime.rs):
  the dump runtime's one definition; [`js/src/dump-runtime.js`](../../js/src/dump-runtime.js):
  the same for hosts without djs.
- [`crates/djs-core/src/vm/compile_cache.rs`](../../crates/djs-core/src/vm/compile_cache.rs):
  the compile cache.
- [`tests/dump/v2/`](../../tests/dump/v2/): programs that wait, and golden dumps that
  every later djs must still load.
