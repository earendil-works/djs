# Plain-JavaScript dumps: continuations with origin tracking

Status: research report (0199), 2026-11-04, and the plan it led to, carried out in
0200–0207; format 2 was retired in 0205 (D2920), so plain dumps are the only format and
"today" below means format 2 as it was. Decisions D2830–D2836. Prototype:
[work/research/0199](../work/research/0199/README.md). The format as built:
docs/continuation-dumps.md.

## The question

The owner, after seeing a format 2 dump: "It's no longer valid JavaScript. It requires
quite a bit of machinery in the interpreter for this to work. That invalidates the whole
goal that it's just executable JavaScript." A format 2 dump runs only under
`Vm::load_dump`, with privileged `$dump` primitives: `$dump.resume(fn, {at, slots})` builds
a frame at a compiled resume pc; `cell`, `TDZ`, `advance`, `forIn`, `using`, `private`, … exist
only while a dump loads.

The task: prototype dumps that run as ordinary JavaScript, given only the host's functions,
in djs **and** in node, by writing every suspended frame as a *continuation function*
(critic's variant R, docs/suspension-design-review.md §3), and fix R's blocker, growth on
re-dump, by **origin tracking**.

## The short answer

**It works, and it is the better format.** I recommend switching (D2830), with the
transform in the Rust exporter (D2831), in steps (§8).

- **Every harvested idle point runs as a plain Script and continues identically**, in djs
  (`djs run`, no loader) and in node (`vm.Script` plus a 30-line harness that provides
  `later`/`print`): 1,161 of 1,161. Overall 1,345 of 1,408 idle points (harvested,
  tests/dump/v2 and golden programs, a 54-await stress program) transform; every one that
  transforms continues identically, except where the run itself differs (`Math.random`'s
  state, wall clock, engine error texts, a builtin node lacks).
- **Re-dumps stay bounded.** A dump of a run that resumed continuations is mapped back to
  the original functions and sites (*origin tracking*) and the next continuation is
  written from the original text again. Chains of dump → plain → run to the next idle
  point → dump … over every idle point: all 286 harvested programs (1,161 links) continue
  identically; the plain dump's size follows today's dump of the same state (R's
  loop-nested: 2.3 → 257 KB over 8 links; here a constant 3.0 KB; a 50-link chain of nested
  loops stays at 3.3–4.3 KB while today's goes 1.2–2.3 KB).
- **What needed a helper now has a plain encoding**: TDZ variables shared with closures are
  exact (the continuation *is* the factory of its frame's scopes, §2.2), and the loader
  primitives become plain helper functions or plain code (§4). R's 70 refusals are fixed
  (§3). Two things have no plain encoding: **private fields of instances** (only code in
  the class body can write them) and **`Math.random`'s state**; both need a hook from the
  host or the class (§4).
- **Smaller and faster on codemode**: harvested dumps −32% raw, −7% zstd -3 (median per
  dump 0.79 raw, 0.99 zstd); load to the first idle point 0.78× of today's `load_dump`
  (median, release). Small synthetic dumps get bigger and a little slower (+109% raw,
  1.28× load): a fixed ~1–2 KB of helpers and origin records.
- **What it buys beyond the owner's point**: the dump no longer depends on the compiler.
  Today a dump is loadable only while a later compiler keeps the same named slots at the
  same site (the D2563 pins are a format contract). A plain dump is JavaScript: any later
  djs, or node, runs it. Suspension maps stay, but as the exporter's way of *reading* a
  frame, not as a contract.
- **What it costs**: the exporter grows a source-level transform (the continuation
  generator, ~1,800 lines of JS in the prototype) that needs each site's statement path
  with spans, which djs's single-pass compiler can record at the site (§6). The loader,
  `$dump.resume`, the privileged primitives and the recording compile at load go (§7).

## 1. Today, and what R left open

A format 2 dump is a Script run by `Vm::load_dump` with a privileged `$dump`
(docs/continuation-dumps.md). An activation is `$dump.resume(fn, {at, awaiting, this,
args, slots})`: the loader compiles the dump with recording on, finds `at`'s resume pc and
writes the slots into a frame by name. Variables a frame shares with closures go through
`$dump.cell(() => x)` (the factory hands out the variable's box), a variable in its TDZ is
`$dump.TDZ` or a cell taken before its declaration ran.

R (0188) wrote each activation as a residual function, "the rest of `fn` from the site",
in plain code, and got 993 of 1,063 harvested dumps right. It was rejected for three
reasons:

1. **Growth.** A resumed residual dumped again is written as the old residual plus a
   residual of it: 2.3 KB → 257 KB over 8 links.
2. **70 refusals.** A function and the variables its frame shares lived in different
   factories; the residual had to be written inside both.
3. **Lossy TDZ.** A shared variable in its TDZ became a plain `let` assigned later.

## 2. Design

### 2.1 A continuation per activation

For an activation of `f` stopped at site `s`, the dump writes a function `K` of `f`'s kind
(async function, arrow, generator, async generator) whose body is:

- **the prefix**: the bindings live at `s` declared with their values (`const k = "y"`),
  and the closures of the frame's scopes made in place (§2.2);
- **the site's statement rewritten**: values evaluated before the site come from the dump,
  the site itself is `(await <the promise it waits on>)` (or a placeholder `(yield)`), the
  rest of the statement is `f`'s text. An operand evaluated before the site is written
  as `f`'s text again where that is pure and gives the saved value on the dumped state
  (D2836, D2905): an identifier whose binding holds it, a callee `b.name` whose `b` holds
  the saved `this` and whose `name` is found on a frozen builtin (`results.push((await
  p))`). Otherwise the call keeps its evaluated callee and `this`: `$djs_R(callee, self,
  [arg0, (await p), <rest>])` with `$djs_R` = `Reflect.apply` captured while the dump
  loads;
- **the loops around the site** as real loops over the saved iterator record. Where the
  body cannot jump to the loop (no `break` or `continue` in it) and the iterator has no
  `return` to close it, the rest of the current iteration comes first and the loop then
  goes on as written over the record: `{ rest } for (const v of $djs_I(iterator))
  body` (D2908). Otherwise `for (const v of $djs_L(iterator, next))`, whose first step is
  a placeholder so the rest of the current iteration runs first inside the loop, and whose
  later steps (and `break`, `return`, exceptions closing the iterator) are the original's;
- `try`/`catch`/`finally`, `switch`, labels, `while`/`do`/`for` kept around the rest, a
  suspended `finally` written as its rest followed by the completion that entered it;
- **the rest of `f`** verbatim.

`K` is called once, with `f`'s `this`; its prefix runs to the site's `await` while the dump
loads. Generators are primed with one `next()` to their placeholder `yield`; an async
generator's queued requests are made again with `next`/`throw`/`return` on it, so their
promises are the ones data holds. An example (golden `loop-nested`, the second iteration
of the outer loop; [examples](../work/research/0199/examples/)):

```js
$dump.Ka4 = async function () {
const $djs_R = $djs.R, $djs_L = $djs.L;
let i = 0;
{ let $djs_f1 = true; while ($djs_f1 || (i < 2)) { if ($djs_f1) { $djs_f1 = false; {
{ let $djs_f3 = true; for (const $djs_v3 of $djs_L($dump.i2, $dump.b0)) { if ($djs_f3) { $djs_f3 = false; {
const a = "y";
{
{ let $djs_f5 = true; for (const $djs_v5 of $djs_L($dump.i3, $dump.b1)) { if ($djs_f5) { $djs_f5 = false; {
const b = "p";
const $djs_t0 = $dump.a4_0;
$djs_R($djs_t0, undefined, [0, "y", "p", (await $dump.p1)]);
} } else { const b = $djs_v5; print(i, a, b, await later(a + b)); } } }
}
} } else { const a = $djs_v3; {
      for (const b of "pq") print(i, a, b, await later(a + b));
    } } } }
i++;
} } else {
    for (const a of ["x", "y"]) {
      for (const b of "pq") print(i, a, b, await later(a + b));
    }
    i++;
  } } }
};
Object.defineProperty($dump.Ka4, "$djs", { value: { f: main, at: "await#1", segs: [114,46,5, …], scaf: [["while",39,95], …], cmap: […] } });
$dump.a4 = $dump.Ka4.call(undefined);
```

### 2.2 The continuation is the factory of its frame's scopes

This is what fixes R's refusals and its TDZ loss. Today a variable a frame shares with
closures lives in a factory and is handed to the frame as a `$dump.cell`. Here the
**continuation declares it**, and the closures that capture it are **made inside the
continuation**, in the block that declares it:

```js
// h() { const get = () => late; …; const v = await later(1); let late = v * 2; … }
$dump.Ka3 = async function () {
let early = "ReferenceError";
Object.assign($dump.s2, { get: () => late });    // the closure, made where `late` lives
const get = $dump.s2.get;
const v = (await $dump.p1);
let late = v * 2;                                 // `late` is in its TDZ until here: exact
return early + " " + get();
};
```

A variable in its TDZ is simply a declaration in the rest of `K` that has not run yet. The
closures are handed to the rest of the dump through `$dump.sN` (the factory's name, so
the rest of the dump's text is unchanged). The harvested dumps hold 3,019 activations and
2,350 factories whose variables a frame shares, all merged into continuations; none of
their shared variables happens to be in its TDZ at an idle point, so the TDZ case is
covered by tests/dump/v2/cells and `x-long` (56 dumps, all identical; R wrote them as
plain `let`s).

The exporter's factories are scope *groups* (one factory can hold variables of two frames
when a closure captures both), so the prototype places every piece by what it captures:

- a variable a frame takes as a cell is declared in the continuation block of the scope
  that declares it;
- a closure goes to the innermost block among the variables it captures (free variable
  analysis of its text);
- variables no frame takes (a finished loop iteration's scope) stay a block of their own
  inside that place (`{ let path = "a"; … }`; one still in its TDZ is declared after a
  `break` out of its labelled block, so it never initializes);
- a continuation goes where its function goes. A function made in a live frame (the
  codemode body's inner `async (tools) => {…}`, a `todos` helper run by `Promise.all`) gets
  its continuation **inside the outer continuation**.

Two orderings follow from this, both plain code:

- **Callbacks.** An inner continuation exists only once the outer one's prefix ran, but
  the outer one awaits a promise made from the inner one (`await Promise.all(batch.map(todos))`).
  The outer prefix calls a callback the dump defines (`$dump.ga13()`) right before its
  site's statement; the callback runs the part of the dump that needs the new closures
  (starting the inner activation, the `Promise.all`). 866 of the 1,161 harvested dumps use
  one; 824 nest continuations.
- **Placeholders.** Data a frame needs can hold a closure that frame's prefix makes (a
  `log` array holding `() => i + j` from the current iteration). The data is written with
  `undefined` there and patched right after the continuation's call (202 harvested dumps).

### 2.3 Origin tracking

Each continuation carries its origin as a plain property, `K.$djs = {f, at, segs, scaf,
cmap}`:

- `f`: the original function (a reference when the run holds it, else its text, §5);
- `at`: the site of `f` that `K` stands for;
- `segs`: `[kStart, fStart, length]` for every stretch of `K`'s text copied from `f`;
- `scaf`: the constructs `K` made itself (the loop scaffolds, the site's own `await`) with
  the offsets of the originals;
- `cmap`: the temporaries of `K`'s rewritten site statement and the slots of `f` they stand
  for (`$djs_R(callee, …)`'s first argument is `f`'s `call.callee@41`).

A dump of a run that resumed continuations (today's exporter, reading `K`'s frames with
`K`'s suspension maps) is first **normalized**: every activation of a continuation becomes
an activation of `f`. `K`'s site maps to `f`'s through the segments; `K`'s slots to `f`'s
names (bindings by name, temporaries by offset, the loop scaffold's iterable unwrapped to
the saved `iterator`/`next`, the `cmap` for a frame still inside the rewritten statement);
`K`'s own `$djs_*` locals are dropped, and so is what only they kept alive (old
continuations, their helpers). Then the next continuation is generated from `f`'s text.
The code a dump carries is always the original's: that is what keeps it bounded.

## 3. Results

Measured over every idle point (at most 40, 60 for the stress program, per program) of
the harvested corpus (with a one-line shim for pi's `text`/`store`/`load`), the
tests/dump/v2 programs, the golden programs and `x-long` (nested loops, a closure over a
per-iteration variable, a helper reading a `const` declared after the loops, 54 awaits).
The e2e dump mode has no idle points (finished VMs) and is not applicable.

**Single dumps** (`check.mjs`: the output before the dump plus the plain dump's output must
equal the uninterrupted run's):

| corpus | idle points | transformed | djs: identical (+ env) | node: identical (+ env) |
| --- | --- | --- | --- | --- |
| harvested | 1,161 | **1,161** | 1,143 + 18 | 1,106 + 55 |
| v2 + golden | 193 | 130 | 128 | 124 |
| x-long | 54 | 54 | 54 | 54 |

"env" is a difference of the run, not of the dump, and the same as today's resume shows:
`elapsedMs` from `Date.now()` taken before the dump (3 harvested programs), V8's error
messages (`Cannot read properties of undefined` for djs's `cannot read property`). The
remaining differences: `Math.random`'s state (2 dumps, §4) and node lacking
`Iterator.concat` (4 dumps).

**Chains** (`chain.mjs`: dump at idle point 1, then plain → run to idle point 2 → dump →
normalize → plain …, at most 50 links; every link's plain dump also run to the end in
node):

| corpus | programs | links | continue identically | longest chain |
| --- | --- | --- | --- | --- |
| harvested | 286 | 1,161 | **286** | 29 |
| v2 + golden + x-long | 45 | 184 | 34 (11 refused at some link) | 50 |

Size per link stays at today's dump of the same state plus a constant: the largest drift
of `plain − today` over a whole harvested chain is 1.4 KB (h231, where plain is 4–6 KB
*smaller* than today at every link). Examples:

| program | links | plain, per link | today, same states |
| --- | --- | --- | --- |
| golden loop-nested (R: 2.3 → 257 KB) | 8 | 3,042 3,051 3,042 3,051 … | 864–865 |
| golden combinators (R: 2.7 → 80 KB) | 7 | 3,523 → 2,822 | 1,783 → 1,407 |
| h155 | 13 | 13,085 → 15,437 → 14,900 | 10,626 → 13,534 → 12,991 |
| x-long | 50 | 3,268 … 4,265 | 1,199 … 2,258 |

**R's 70 refusals** (function and shared variables in different factories) are all fixed:
they are the nested-continuation and callback cases of §2.2.

### Sizes and times

| | harvested (1,161) | v2 + golden + x-long (184) |
| --- | --- | --- |
| raw | 15.70 MB → 10.65 MB (**−32%**) | 0.32 MB → 0.67 MB (+109%) |
| zstd -3 | 3.39 MB → 3.14 MB (**−7%**) | 134 KB → 245 KB (+82%) |
| per dump, median ratio | raw 0.79, zstd 0.99 | raw 2.29, zstd 1.77 |
| load to the first idle point, median | 297 µs → 237 µs (**0.78×**; p10 0.61, p90 0.92) | 125 → 154 µs (1.28×) |
| JS transform (prototype, node) | 1.2 ms median | 0.9 ms |

Load is `Vm::load_dump` against compile + run of the plain Script as an ordinary Script,
both to the first idle point, best of 10 fresh VMs each, interleaved, release build
(work/research/0199/timing; the machine was loaded, load average ~9, so the ratios are the
result, not the microseconds). Without pruning (§5) harvested dumps would be +17% raw and
+31% zstd: the codemode body's text that already ran is what today's dumps carry and
plain dumps do not.

The small dumps pay a fixed price: the helper object (~700 bytes; since D2903 the host's), an origin record per
activation, and a loop body written twice (the rest of the current iteration and the body
for later ones). Codemode dumps are dominated by data and the body's text, where plain dumps
win.

## 4. What still needs a helper

The plain dump defines `$dump` as an ordinary global object at its top and deletes it at the
end (its completion value is the run's promise, which `djs run` and the node harness take
as the run). Every loader primitive has a plain counterpart, or goes away:

| today | plain dump |
| --- | --- |
| `$dump.resume` | a continuation (§2) |
| `$dump.cell`, `$dump.TDZ` | the continuation declares the variable (§2.2) |
| `$dump.pending(id, name, args)` | the host function called again: `later("ya")`, `tools.read({…})` (see below) |
| `$dump.handle("run", p)` | the Script's completion value |
| `$dump.builtin(path)` | the intrinsic's plain expression, captured at the top (`Object.getPrototypeOf([][Symbol.iterator]()).next`) |
| `advance`, `fn`, `keep`, `spent`, `rejected`, `bytes` | plain helper functions (one line each) |
| `fromSync` | a `for await` over the sync iterator record (the async-from-sync wrapper is the language's own) |
| `forIn` | a key iterator over the keys left that skips deleted ones |
| `regexpIterator` | `RegExp.prototype[Symbol.matchAll]` on the dump's clone (its `lastIndex` set first) |
| `random(lo, hi)` | **none**: plain JavaScript cannot set `Math.random`'s state |
| `private(obj, C, "#x", v)` | **none**: only code in `C`'s body can add `#x` to an object |
| `issued(n)` | not needed: the host numbers its calls |

What the prototype refuses, by idle points (v2 and golden programs only; none in the
harvested corpus), and whether a plain encoding exists:

| refusal | idle points | plain encoding |
| --- | --- | --- |
| `using` declarations in flight | 11 | yes: `using` again over `{[Symbol.dispose]: saved method bound to the resource}`, in order |
| `closeThrow` (async-from-sync close in a reaction) | 9 | yes: a plain function with the same steps (its identity is never visible) |
| `yield*` (`yield*`, `yield*.await` sites) | 10 | yes: `yield*` over a wrapper of the saved inner record whose first step is the current result |
| site in a destructuring pattern / assignment target | 10 | yes: the rewrite rules of the other expressions |
| site in a `case` test | 4 | yes: the switch from that case with the saved discriminant |
| `finally` entered by `break`/`continue` | 4 | yes: a label on the scaffold loop, `break L` after the rest |
| `for-await.close` site, `for await` over an async iterator with the site in its body | 6 | yes: the rest of the iteration before the loop, closing as AsyncIteratorClose does (an explicit `try`/`finally` with a state flag) |
| iterator helpers in flight | 6 | yes when the counter is passed on: `underlying.map((x, i) => fn(x, i + k))` |
| `for await` step over a sync iterator | 2 | yes, as above |
| private fields of instances | 1 | no, without changing the class's text |
| `Math.random`'s state | (2 dumps run wrong) | no, without a host hook |

**Open host calls.** Re-issuing the call is exact for the corpus (`later` answers with its
argument). A real host needs to know that a call is an old one (deliver a recorded result,
or not repeat a side effect), so the plain dump should call a function the host provides,
`resume(id, name, args, state)`, with today's `pending` arguments (D2832). That is host
code, not engine machinery; the Sandbox's tasks (D2771) already have the hooks.

**`Math.random`.** The same: a host function `seed(lo, hi)` the dump calls first, which djs
implements and other hosts may ignore (D2832). Without it a resumed run's random numbers
differ, in node anyway.

**The interface as built (0204, D2890, D2895, D2897).** The hooks are the methods of one
global object, `$djs`, that the loading host defines and the dump deletes in its final
`finally` (with `$dump`): `$djs.seed(lo, hi, next)` is the dump's first call, with
`Math.random`'s state and the id the dumped VM's next host call gets; `$djs.resume(id,
name, args, state)` stands for each open call (`state` left out when the call has none).
Both are called only while the Script itself is evaluated. A djs host reopens call `id`
under that id (`Vm::open_host_call`) and, when the Script has stopped, sets the id counter
to `next` (or one past the highest reopened id): the hooks are sync calls, which take ids
of their own, so without `next` the resumed run's calls would get other ids than the
uninterrupted run's. Host functions the dump holds as values are written as the host's
name for them, an expression evaluated while the dump loads (`tools.read`; pi's
`$djs.call`). The Script's completion value is what the host named: the value itself for
one handle named `run`, else a null-prototype object of all of them by name.

**Frozen builtins (D2836).** Plain dumps are written only for VMs with frozen builtins
(`VmOptions::frozen_builtins`, D2304; pi's `next` mode has them): the exporter refuses
otherwise. It may then assume that no builtin changes between a value's evaluation and
the load. The hosts follow: `djs run` with its dump flags, `djs resume` and `djs repl
--frozen` make frozen VMs, a Sandbox or pi session without frozen builtins cannot dump
(D2920), and the difftest's plain-dump variant runs frozen and is compared only for cases
with idle points (a case that changes builtins on purpose and takes no
dump would differ for that alone).

**The runtime (0207, D2903).** The format's fixed helpers are members of `$djs` too, next
to the hooks: the host provides them, so a dump does not carry them (they were ~700 bytes of
every small dump). They are the same code for every dump, the format's small standard
library: `crates/djs-core/src/vm/dump_runtime.rs` defines them once; a djs host gets the
object from `Vm::dump_runtime()` and adds its hooks; a host without djs imports
`js/src/dump-runtime.js` (generated from the same table, one named export per helper and
`helpers`, all of them) and sets `globalThis.$djs = { ...helpers, seed, resume }`. The
Sandbox and pi's adapter pass that file's `script` (the same as a Script) to the wasm
export `vm_dump_runtime`, so that the shipped module does not carry the text. A continuation captures the
helpers it calls (`const $djs_R = $djs.R`), since `$djs` is gone once the dump has loaded.
Each helper is a plain function that uses neither `this` nor another member, so that the
names can become imports later. The members:

| member | what it is |
| --- | --- |
| `seed(lo, hi, next)` | hook: `Math.random`'s state, the next call id (above) |
| `resume(id, name, args, state)` | hook: an open host call (above) |
| `source(name, hash, params)` | hook: the codemode body `name` the host ran, which the continuations name as their origin instead of a second copy of its text (D2882); djs hosts compile it again and check the hash, node returns any function (it never dumps again) |
| `R`, `G`, `C`, `A` | `Reflect.apply`, `Reflect.get`, `Reflect.construct`, `Object.assign`: a call, read or construction whose callee or receiver was evaluated before the site |
| `D(o, k, v)` | defines a data property (an object literal's member written after the site) |
| `P(k)` | ToPropertyKey, as a computed key does it |
| `S(o, keys)` | a pattern's rest property: CopyDataProperties without `keys` |
| `Q(a, ...v)` | appends elements to an array being built (an argument list or array literal with a spread, evaluated around the site) |
| `L(it, next)` | an iterable over a saved iterator record whose first step is a placeholder: a loop's or pattern's rest of the current step runs in it |
| `I(it, next)` | the same without the placeholder (a pattern element still to take, a loop going on after the rest of its iteration, D2908); without `next`, the iterator's own |
| `E()` | an iterable that is done (a pattern's iterator that was) |
| `Y(it, next, async, ...first)` | a `yield*`'s inner record; its first step is `first` if given, else a placeholder |
| `U(value, method, kind)` | a `using` resource added again: an object whose dispose method calls the saved `method` on `value` (`kind` 4: async) |
| `M(f, k)` | an iterator helper's callback whose counter already passed `k`: calls `f` with the counter shifted |
| `FM(it, next, f, k, inner, innerNext)` | a `flatMap` helper stopped inside its inner iterator, made one step before and stepped over a placeholder |
| `CC(cur, curNext, items)` | an `Iterator.concat` stopped inside an item, the same way |
| `CI(m, o)` | a concat item: an iterable whose iterator is `m` called on `o` |
| `HD()` | a completed iterator helper |
| `N(it, next, ...first)` | the async iterator of a `for await` over a saved record; its first step is `first` if given |
| `X(it)` | AsyncIteratorClose of a throw: a promise that always fulfills |
| `O(r)` | the check that an iterator result is an object |
| `fromSync(it, next)` | the async-from-sync iterator of a `for await` over a sync iterator |
| `closeThrow(it)` | its handler that closes the sync iterator when a step's value rejects, and rethrows (a re-dump writes the objects these two made as the calls again, D2951) |
| `forIn(o, keys)` | a `for-in`'s key iterator over the keys left, skipping deleted ones |
| `advance(it, n)` | steps an iterator `n` times, returns it |
| `regexpIterator(re, s, flags)` | a `matchAll` iterator over `re` and `s` (`flags & 4`: done) |
| `fn(o)` | the method, getter or setter of a one-member object literal (methods keep their kind) |
| `spent(k)` | a settled promise's resolve (`k` 0) or reject (`k` 1) function: calling it does nothing |
| `rejected(e)` | a rejected promise that counts as handled |
| `bytes(b64)` | an `ArrayBuffer` from base64 |
| `k(K, record, proto)` | gives continuation `K` its origin record (`K.$djs`) and a generator continuation its `prototype`, and returns `K`: the dump writes `$dump.aN = $djs.k(async function (…) {…}, {…})(args)` (D2909) |
| `keep(o, keys)` | deletes a class's own members other than `keys`: the ones the run deleted since its evaluation |

## 5. Pruning: why plain dumps are smaller

A function nothing but its activation holds (the codemode body; an inner callback the
frame runs) is not written as a function at all. Its continuation's origin is its text,
**pruned**: the top-level statements before the site's statement are gone (they never run
again; declarations keep a skeleton, `const results = 0;`, for their names and kinds), and
what remains is written as **pieces of the continuation's own text** (`[start, length]`
spans where `K` copied `f` verbatim, strings for the rest), so text a continuation holds is
not written twice. Normalization rebuilds the origin from the pieces. This is the part of
R's size win that survives origin tracking, and it answers critic's §6 "drop executed body
text".

## 6. Where the transform runs

The prototype is JavaScript (acorn) that post-processes today's format 2 dumps:
`plain.mjs` (the continuation generator, placement, scheduling, helpers; 1,800 lines),
`normalize.mjs` (origin tracking; 340), `freevars.mjs` (160). That was the right way to find
the design, but not to ship it:

- djs's CLI, REPL and wasm hosts need it in Rust, and a JS parser in the dump module is
  the wrong dependency (D2564's argument against a TypeScript dumper applies).
- Post-processing parses the dump the exporter just wrote and re-derives what the exporter
  knew: which factory is a scope group of which frame, which closures capture what. The
  normalizer even recognizes its own loop wrappers in the re-dump's factories.

**Recommendation: the Rust exporter writes plain dumps directly (D2831).** It already reads
every frame by SM's slot names, which is exactly the generator's input (`call.callee@41`,
`for-of.iterator@18`, `completion.kind@23`), and it knows the scope groups and the closures'
captured variables without a free-variable analysis. What it lacks is the structure of the
code around the site, since the compiler is single-pass and has no AST. The compiler can
record it at each site the way it records the role stack (D2570): the statements on the
path from the function body to the site with their kinds and spans (block, loop with its
head and body spans, `try` parts, `switch` cases, label), the declarations of each block,
and the expression constructs around the site with their spans, which it already names.
The generator then splices `f`'s text by spans. The origin record is data the exporter
writes and reads back; normalization becomes "read `K`'s frame with `K`'s map, translate
the names through `K.$djs`".

## 7. What can be deleted

With plain dumps as the only format:

- **the loader**: `Vm::load_dump` becomes "compile and run a Script"; dump_resume.rs (331
  lines), the privileged `$dump` and every primitive in dump_prims.rs (472), the resume and
  refusal paths in dump_load.rs (771), the recording compile of the dump's text at load
  (D2735), the loader half of the golden and refusal tests;
- **the cross-version contract of the suspension maps**: the pins (tests/smap, 1.2 MB)
  stop being a format promise (D2563). They stay useful as the exporter's regression test,
  but a renamed slot no longer orphans old dumps: old dumps do not mention slots;
- **`Loaded` bookkeeping** for handles and open calls: the host's own functions see them;
- **the "every later djs loads it or refuses" machinery** (D2560's refusal side, D2583):
  the promise becomes "it is JavaScript".

What stays: idle points only (D2561); the exporter's frame reading by suspension maps;
re-enactment of builtin state as public calls (D2562, now without primitives); classes by
their text; images in-process (suspend-anywhere, base image, stress modes); packaging B
(D2704).

## 8. Recommendation and plan

Switch to plain-JS dumps. The owner's goal holds in full for the codemode corpus, both
engines run the dumps, re-dumps are bounded, codemode dumps get smaller and load faster,
and the engine loses the loader and the format's dependency on the compiler. The price is
a source-level transform in the exporter, which the prototype shows to be about the size of
today's activation writer plus loader.

| step | what |
| --- | --- |
| 1 | Compiler: record each site's statement path, block declarations and expression constructs with spans next to the suspension map (no behaviour change; pinned). |
| 2 | Exporter: the continuation generator in Rust behind a flag; plain dumps for the codemode subset; `djs run` of a plain dump and the node harness in the gate (`--dump-every-idle` writes plain dumps; the chain check as a gate leg with a 50-link cap). |
| 3 | Origin tracking in the exporter: `K.$djs` read back, frames of continuations written as activations of their origin; the chain check over the corpus. |
| 4 | The refusals of §4 that have plain encodings, construct by construct, against the golden programs. |
| 5 | Host hooks: `resume(id, name, args, state)` for open calls, `seed` for `Math.random` (Sandbox tasks call them). |
| 6 | Delete the loader, `$dump.resume`, the primitives, the recording compile; format 2 goldens become plain goldens; images stay. |

Risks:

- **Correctness moves from the loader to a source transform.** The continuation must
  behave exactly like the rest of `f`, including job order. The prototype's checks (every
  idle point, every chain, both engines) found and fixed ordering bugs (combinators, queued
  requests, patches of regexps' `lastIndex`) that a gate leg would keep found.
- **Readability.** A plain dump is code, but generated code: scaffolds (`$djs_f1`,
  `$djs_L`) and origin records. `djs resume --explain` still has a job.
- **Stack traces** of a resumed run show the continuation's positions (today: the dump's).
- **Small dumps** get bigger by a constant; trim the helper object and the origin records
  (§3) before measuring again.

## 9. Findings on the way

- **Exporter bug**: a generator function whose `prototype` property was replaced is
  dumped without it (`G.prototype = letters.prototype` before the dump; after resume
  `Object.getPrototypeOf(g) === letters.prototype` is `false`, it was `true`). Follow-up.
- **Exporter**: an activation's `this` that is an arrow's (no own `this`) is written as the
  call's receiver (`this: { $f3: …, $Ka10: … }` for `$dump.s2.$Ka10()`); harmless but it
  keeps the receiver alive. The plain dump calls arrows as `(0, K)()`.
- **Exporter**: dumping a VM where a generator method was called with a `this` that holds
  the global `$d` object failed with "the dump: does not compile" (no further detail);
  not reduced further.
- **Template literal parts** are named by the delimiter before them (the backtick or the
  `}`), not by the part's first character (docs/continuation-dumps.md "Slot names" says
  "the element's own start").

## 10. Refusals that remain (0203)

The constructs of section 4 have plain encodings in the exporter (D2871–D2877), except
for one, which plain dumps refuse:

- **Private names on instances** (D2878): an object that carries a class's private field,
  method or accessor (`#n`, `#bump`). Only that class's own construction can add a private
  name to an object, and the dump does not rewrite class text (no-op initializers,
  injected static methods) to make that possible. The refusal names the class and the
  private name. Classes with static private members, and private methods called on the
  class, are not affected. Instances like this that are live across an await are rare in
  codemode; tests/dump/v2/plain-refusals.txt lists the one fixture
  (golden/class-fields).

**Class text without its static code (0209, D2950).** A plain dump makes a class by
evaluating its text, and that would run its static blocks and static field initializers
again (format 2's loader skipped them; a plain dump has no loader). Their effects are
already in the dumped heap and the dump writes them after the class: static values as
patches, members deleted since as `$djs.keep`. So the class text is written with that
code blanked by spaces, newlines and offsets kept: `static { log.push(1) }` becomes
`static {             }`, `static n = f();` becomes `static n       ;`. A static private
field or auto-accessor keeps a literal initializer (`static #c = 0`), since only the
class's code can set it; one with any other initializer is refused. The visible cost:
after a load, `Function.prototype.toString` of such a class shows the blanks. Computed
keys run at evaluation too; the exporter already refuses any that is neither a literal
nor a well-known symbol.

**TypeScript parameter properties (0209).** `constructor(public x: number)` also assigns
`this.x`, so it is not erasable: the dump keeps its TypeScript (`"djs-dump typescript";`),
as for an enum (D2879).

## Decisions

Proposals for the coordinator, not added to docs/decisions.md by this report.

- **D2830**: Continuation dumps become plain JavaScript: every suspended activation is a
  continuation function (the rest of the original from its site) that is also the factory
  of its frame's scopes; the dump defines nothing privileged and runs as an ordinary Script
  given the host's functions. Format 2 and its loader primitives are retired once the
  plain exporter covers the corpus.
- **D2831**: The plain dump is written by the Rust exporter; the compiler records each
  site's statement path, block declarations and expression constructs with spans, and the
  exporter splices the original text. The prototype in JavaScript is not shipped.
- **D2832**: Open host calls and `Math.random`'s state are restored through host
  functions the dump calls (`resume(id, name, args, state)`, `seed(lo, hi)`), not through
  engine primitives.
- **D2833**: Re-dumps use origin tracking: a continuation carries `$djs = {f, at, segs,
  scaf, cmap}`, and the exporter writes the frame of a continuation as an activation of its
  origin, so a dump always carries the original code (pruned of what cannot run again) and
  its size stays bounded over any number of re-dumps.
