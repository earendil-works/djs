# Suspension design review: state machines vs suspension maps

Status: research report (0188), 2026-11-04. Decisions D2720–D2722 (proposed with this
report). Prototype: [work/research/0188](../work/research/0188/README.md).

## The question

The owner asked whether the way djs suspends a frame for a continuation dump is the
right one, or whether the pending frame should be a state machine, like the bgen builtins
transpiled to Rust (D1632). The owner's framing: **anything that waits on the host should
be able to suspend, be dumped and continue later**, and doing that should be simple for a
host. The sleep-dump example (examples/sleep-dump, ~170 lines of host code) is too
complex for that. The owner leans to state machines and asked for a verdict, the variant
to build and a migration plan.

## Verdict

**Switch to state machines, in the compiler, with the state described by source-derived
names. Do not put generated code into dumps.** Keep almost everything else of the
current design.

1. **Build "SM": canonical suspension states in the bytecode compiler.** At every
   `await`/`yield` the compiler moves everything in flight into named frame slots:
   operand-stack temporaries, iterator records, `finally` completions, `using` records,
   switch discriminants and destructuring state. The operand stack is empty while a frame
   is suspended. A suspended frame is then plain data `{fn, state, slots}`: `fn` is the
   function's verbatim text (as today), `state` is the site name (`await#2`, as today),
   and `slots` maps a name to a value. A name is either a binding (`x`, `x@scope`) or a
   temporary named by its construct and source span (`call.this@41`). This is bgen's
   rule (D1632: "Rust locals never live across a state boundary") applied to user
   functions. The bytecode already is the state machine (the pc is the state). What
   changes is that the state at a suspension point is fully named.
2. **Open host calls become data too:** `{id, fn, args, state}`. The host function
   declares `save`/`resume` hooks, and the Sandbox does the bookkeeping (timers,
   re-issue, delivery). This is independent of (1), cheap, and is what makes the sleep
   example short. Build it first.
3. **Reject the two other state-machine variants**, each on measured grounds:
   - *Dump-time residual code* (variant R: the dump carries a generated continuation per
     frame). The prototype got further than I expected: 93.4% of the harvested idle
     points round-trip exactly, codemode dumps get 42% smaller and load twice as fast.
     But the scheme does not survive being dumped again. A resumed continuation dumped a
     second time doubles the dump per generation (2.3 KB to 257 KB after 8 dumps). The
     code a dump carries has to be the original function's, so the state has to be data.
   - *Lowering every async function at the source level* (regenerator, TypeScript
     down-levelling). It is 2.4× (via generators) to 5.4× (switch machine) slower on
     await-heavy code, and the code is 2.4–7× larger. It also changes the function text
     that `toString` and stack traces show.
4. **Be honest about what SM buys.** Today's format is already "state as data": a site
   name, bindings, and operands named by role. The state-machine idea does not remove
   the cross-version contract. Something must still say which values a suspended frame
   holds and what they are called. SM makes that contract **smaller, uniform and
   independent of the operand-stack discipline**. It also deletes code: the role stack,
   operand placement, gosub return-address mapping, the `loops` list, and the second
   compile at load. Most of today's remaining refusals are heap shapes, not frames, and
   SM does not fix those.

The rest of this report gives the evidence.

## 1. Today's design, described fairly

A dump is a Script. Data and closures are written as code (factories over shared
variables, `$dump.cell` for a variable a frame shares with closures), classes as their
verbatim text, builtin state in flight as the public calls that make it again (D2562), and
each suspended activation as

```js
$dump.resume(fn, { at: "await#2", awaiting, this, args, bindings: {...},
  operands: [["call.callee", f], ["call.this", o]], loops: [{kind, iterator, next}], queue })
```

The exporter reads the frame with the function's **suspension map**: compile the
function's script again with recording on. The parser keeps a stack of operand roles and
records it at each site (D2570). The loader compiles the dump's text again, also with
recording on, finds the site's resume pc, checks that the bindings, operand roles and
loops it was given equal its own map, and builds the frame (D2584). The maps of every
corpus and e2e await are pinned (D2563: 1,063 sites, tests/smap, 1.2 MB). A diff in the
pins is a format decision.

What it gets right, and SM keeps:

- **Text as code.** Functions and classes keep their verbatim text, so `toString`, source
  positions inside a function and TypeScript text survive (D2615). Nothing is decompiled.
- **Names come from the source, not the build.** `await#N` is an ordinal in the text,
  bindings are names (with scope offsets), roles are constructs. A dump never holds a pc
  (gosub return addresses are written as indexes, D2627).
- **Refuse, never misread** (D2583). Every primitive checks against this build.
- **Idle points only** (D2561), so no native frame, parked call or half-run instruction
  is ever in a dump. Suspend-anywhere stays an in-process matter (images, fuel slices).
- **Coverage.** At 0172 every one of 1,164 harvested and 5,257 private-script idle
  points dumped, loaded and compared equal.

## 2. Where it hurts

| | today |
| --- | --- |
| Compiler hooks | ~130 role push/pop sites across 12 compiler files; smap.rs 699 lines plus 548 of tests |
| Vocabulary | 35+ operand roles (`call.arg[i]`, `array.element[i]` up to 256 then `array.literal`, `template.part[i]`, `assign.old`, `pattern.*`, `finally.value`/`finally.return`, `using.record`, `switch.discriminant` written twice, …), 7 compiler-made site kinds, loop records as a separate list |
| Loader | dump_resume.rs 472 lines: operand placement by role, gosub index → pc, loops, queue, hidden locals |
| Exporter | the activation part of dump_flight.rs (~600 of 1,976 lines), role-specific edges (for-in targets, using records, from-sync `next` elision) |
| Load | compiles the dump text **twice** (normal, then recording for the maps) |
| Contract | the operand stack layout at each await: role order, which values the compiler keeps on the stack, gosub emission order (D2627: "a compiler that orders them differently bumps COMPAT_VERSION") |

Three weaknesses come from **coupling to the operand stack**:

1. **Brittleness across builds is about stack discipline, not semantics.** A dump stays
   loadable only while the compiler keeps the same values on the operand stack, in the
   same order, at that await. Examples of such changes:
   - 0184 changed a for-await close from `v ret it` to `v ret`;
   - D2620 reordered a codemode body's `<state>` local;
   - djs keeps up to 256 array elements and then switches representation (that is
     where `array.element[i]` and `array.literal` come from);
   - templates keep their parts rather than a prefix.

   A register allocator, superinstructions or a different call sequence would hit the
   pins too, even though the meaning of the program did not change.
2. **Every construct needs its own role.** Each new construct that can sit around an
   await needs a role (pushed and popped at the right place in the single-pass parser),
   an exporter rule and a loader rule. Constructs no one instrumented get
   `unsupported:<construct>`. 0184 spent most of its time on this: `finally`, `using`,
   destructuring, `yield*`, for-in and switch.
3. **`finally` is the worst of it.** A suspended finally block has a return address on
   the stack, so the format has to name a gosub by its index in emission order.

Remaining frame-related refusals (D2627, D2628):

- a site inside a class definition (the class under construction is on the stack);
- closing a for-await iterator on a resumed generator's `return(v)` path;
- an async generator awaiting a `return()` value;
- a finally's saved completion value in Script/eval code.

All the **other** refusals are heap shapes, and no frame design changes them:

- weak collections and revoked Proxies;
- a for-in loop over a Proxy;
- generator objects with their own properties or another prototype;
- `allKeyed`;
- reactions of foreign promise constructors;
- decorators;
- computed keys evaluated again;
- private names outside their class.

The pain the owner sees in **host code** (the sleep example) is not frames at all. The
dump knows an open call only as `(id, name, args)`. Everything else, like the deadline,
the timer, re-arming and delivery, is host bookkeeping beside the dump (`HostState`,
`arm`, `finish`, a manual `begin`/`advance`/`settle` loop).

## 3. The state-machine variants

| variant | where the transform happens | what a frame is in a dump |
| --- | --- | --- |
| **A** regenerator-style source lowering of every async function | compiler front end, always | `{state: 7, vars}`, with numbers from the transform |
| **R** residual continuation | the exporter, only for dumped frames | generated JS code: the rest of the function |
| **SM** spill at suspend, named slots | bytecode compiler, at suspension sites only | `{at: "await#2", slots: {name: value}}` |

### A: lower every async function (rejected)

I measured TypeScript's down-levelling under djs (release build, whole-process time, on a
loaded machine; work/research/0188/ts-downlevel-bench.js: 200,000 iterations of `s +=
await i` with a try/finally):

| | time | code |
| --- | --- | --- |
| native async/await | 0.17 s | 745 B |
| es2015 (`__awaiter` over generators) | 0.40 s (2.4×) | 1,782 B |
| es5 (`__generator` switch machine) | 0.92 s (5.4×) | 5,393 B |

The job-order probe (ts-downlevel-ticks.js) gave the same interleaving in all three, so
modern down-levelling is tick-exact. The real costs are speed and size:

- the functions' text changes (`toString`, stack positions);
- `arguments`, `super` and per-iteration bindings need helper closures;
- the state numbers are still the transform's, so they are a pinned contract like
  today's maps.

A buys nothing for durability and costs on every run.

### R: residual continuations in the dump (prototyped; rejected as the format)

**The idea.** The exporter, which reads the frame with this build's maps, writes for
each suspended activation a function that runs **the rest** of it:

- the statements before the site are dropped;
- the bindings live at the site are declared from their dumped values;
- the suspended statement is rewritten with the evaluated operands in place
  (`$apply(callee, this, [arg0, await p])`);
- each enclosing loop becomes a loop over the saved iterator record that runs the rest of
  the current iteration first;
- try/catch is kept around the rest;
- a suspended finally runs its rest and then the completion that entered it
  (`throw v`, `return v`, `break L`);
- switch, async generators (their request queue is made again by calling `next`) and
  generators (primed to their yield) are covered too.

The continuation is written where the original's free names resolve (in its factory), so
it shares variables with closures without `$dump.cell`. Nested functions are copied
verbatim. The loader needs no `resume` and no maps: the dump is plain JavaScript plus
four helpers that would be loader primitives (`$apply`, `$loop`, `$define`, `$prime`).

**The prototype.** work/research/0188/residualize.mjs, about 600 lines with acorn. It
rewrites today's dumps and is checked by resuming both and comparing the continued
output:

| corpus | dumps | identical | different | refused |
| --- | --- | --- | --- | --- |
| harvested codemode (all idle points, ≤40/case) | 1,063 | **993 (93.4%)** | **0** | 70 |
| tests/dump/v2 programs (built to hit every role) | 177 | 95 | 0 | 82 |

- **Harvested refusals:** all 70 are one exporter limitation. The function and its
  frame's shared variables are in different factories, so the continuation needs
  factories nested like the original scopes.
- **v2-program refusals:** constructs the prototype does not lower: `yield*`, for-await,
  for-in, `using`, class methods with `super`/private names, `new` with spread,
  assignment patterns.
- **Lossy case:** a variable shared with a closure that is still in its TDZ (very common
  in codemode: hoisted helpers that read a `const` declared after the first await)
  cannot be initialized from outside in plain JavaScript. The prototype makes it a
  plain `let` that the rest assigns.

Measured against today's dump of the same state (load = best of 10 fresh-VM loads,
release build):

| | harvested (993) | v2 programs (95) |
| --- | --- | --- |
| raw size | 11.68 MB → 6.76 MB (**−42%**) | 126 KB → 215 KB (+71%) |
| zstd -3 | 2.47 MB → 2.02 MB (−18%) | 59 KB → 91 KB (+53%) |
| load, median | 224 µs → 110 µs (**0.48×**) | 105 µs → 89 µs |

Codemode dumps shrink because a codemode body's text is held only by its own activation.
The residual drops the part that already ran, including the big literals the model
wrote, whose values are in the data anyway. Loads are faster because there is one
compile and no recording compile or map resolution. Where the function is still needed
(named functions), the dump carries it **and** its residual, so it grows.

**Why R is not closed under dumping.** A resumed residual is an ordinary function. Dump
the run again later and the exporter writes that function's text (the old residual) plus
a residual of it. The prototype chained this: dump, residualize, resume, dump again, and
so on (chain.sh, capped at 50 links):

| program | links | residual size per link |
| --- | --- | --- |
| loop-nested | 8 | 2.3, 6.1, 12.5, 24.5, 42.9, 76.2, 137, **257 KB** |
| combinators | 7 | 2.7, 3.8, 5.9, 10.4, 20.0, 39.8, **80 KB** |
| today, any link | | ~0.9 KB (constant) |

It roughly doubles per generation. Each residual also duplicates the loop body (the rest
of the iteration, plus the verbatim body for later iterations). Pruning does not help: to
stay bounded, the exporter must map a continuation's frame back to the **original**
function's site. In other words, the code must be the original and the state must be
data. That is SM. Second-generation dumps also exposed helper code becoming user data
(the `$loop` wrappers got dumped as factories) and generator continuations failing
today's prototype check. Both are fixable, but they show that R pushes semantics into
generated JavaScript that every later dump must understand again.

R is still worth knowing about:

- it shows that the information today's exporter collects is enough to express
  continuations in plain JavaScript for nearly all real idle points;
- its size and load advantages show where SM should get its performance (section 6).

### SM: spill at suspend, named slots (the variant to build)

- **Where:** in the bytecode compiler, for every function that can suspend, always on.
  The single-pass parser already knows the role stack at each await (D2570). Before the
  `await`/`yield` instruction it emits a move of the k operand-stack values below the
  awaited value into k reserved hidden locals, and after it the reverse. That costs k
  moves per suspension, only at sites with k > 0 (about a third of the corpus sites:
  712 of 1,063 have no operands). Nothing changes elsewhere, so sync code and
  suspend-anywhere images are untouched.
- **Names:** a spilled slot is named by its construct and the source span of the value
  it holds, relative to the function's start: `call.callee@41`, `call.this@41`,
  `binary.left@88`, `array@120` (the array literal under construction), `template@200`.
  Constructs hold their state in slots of the same kind:
  - for-of/for-await: `iterator@S` and `next@S` (this replaces the `loops` list);
  - switch: `discriminant@S`, held once, not `dup`ed;
  - `using`: `using@S`;
  - destructuring: `pattern.*@S`;
  - finally: `completion.kind@S` (normal, throw, return, break or continue with its
    target, named by the target's span) and `completion.value@S`. This replaces the
    gosub return address, so a dump has no return-address indexes and the loader has no
    return-address mapping.

  A future compiler may evaluate in another order or keep other values on its stack
  between suspension points without touching the format.
- **A dump:** `$dump.resume(fn, { at, awaiting, this, args, slots: { files: …,
  "call.callee@41": Array.prototype.push, "call.this@41": $dump.n7, "iterator@312":
  $dump.i8, "next@312": $dump.builtin(…) } })`. There is one map: `bindings`, `operands`
  and `loops` become `slots`. The loader writes slots by name and jumps to the state's
  pc. Neither side reconstructs a stack.
- **Loops of the same frame** can be dumped any number of times: the code is the
  original's and the state is data, so dump size is bounded.
- **try/catch/finally:** the handler table is the code's, as today. A suspended finally
  has its completion in slots. A resumed generator's `return(v)` path sets
  `completion.kind = return` and enters the finally like any exit. This also removes
  the remaining for-await-close refusal.
- **Iterators, `yield*`, builtin state in flight:** unchanged. Re-enactment and the
  loader primitives (`$dump.advance`, `fromSync`, `forIn`, `using`, `iteratorHelper`)
  stay. They describe heap objects, not frames.
- **Source fidelity, `toString`, stack traces:** unchanged from today. The text is
  verbatim and positions inside a function are the original's. The function's location
  is in the dump. One existing nit: anonymous functions written as factory properties
  get an internal name (`at $f3` instead of `at <anonymous>` after a resume; `name` is
  patched, the stack name is not). Writing them as `(0, async () => …)` fixes it (a
  follow-up).
- **Speed:** k moves per suspension where k > 0, nothing elsewhere. finally completions
  in slots instead of gosub/ret are about the same instruction count. Measure on the
  bench before and after (risk 1).
- **Spec conformance and determinism:** the evaluation order and job counts are the
  compiler's, unchanged. test262's async and generator suites and the dump stress modes
  check it.
- **Cross-version stability:** the pinned contract becomes **the set of named slots per
  state**. It does not depend on stack order or emission order. It is still a contract.
  A compiler that keeps a new value across an await adds a slot, and older dumps lack
  it, so they refuse (or a default is added).

### Host calls as data (build first)

Today a dump writes `$dump.pending(id, name, args)` and the host keeps everything else.
Proposed:

```js
$dump.pending(5, "tools.sleep", [5000], { due: 1767225600000 })   // host's JSON state
```

```ts
const sandbox = await Sandbox.create({
  asyncFunctions: {
    "tools.sleep": {
      call: (ms) => sleep(ms),                                     // starts the work
      save: (call) => ({ due: call.startedAt + Number(call.args[0]) }),
      resume: (state) => sleep(Math.max(0, state.due - Date.now())), // after a load
    },
  },
});
const run = sandbox.start(SCRIPT, { ms });
const dump = await run.suspend();          // next idle point: save() per open call, dump
// … any time later, any process:
const result = await (await Sandbox.resume(dump, sameOptions)).result;
```

- **Engine:** an opaque JSON string per open call, carried by `vm_dump`/`vm_load_dump` and
  `Loaded.calls`.
- **Sandbox:** keeps the call table, runs `save` at dump time, runs `resume` (or `call`
  again when there is no `resume`, or rejects) at load, and drives the run.

A function without hooks keeps today's behaviour. The sleep example becomes about 40
lines with no `HostState`, `arm` or manual loop. pi's adapter gets the same thing for
tool calls.

## 4. Migration plan

Each step merges green on its own; djs is unreleased (D024), so formats change in place
and the golden dumps are rewritten under a decision (D2619).

| step | what | areas | can run in parallel with |
| --- | --- | --- | --- |
| 1 | **Host-call state.** Engine carries per-call JSON (`$dump.pending(id, name, args, state)`, `Loaded.calls[].state`, wasm exports). Sandbox: function descriptors `{call, save, resume}`, `run.suspend()`, `Sandbox.resume(…).result`. Rewrite examples/sleep-dump. | vm/dump*, djs-wasm, js/src/sandbox.ts, examples | 2–5 |
| 2 | **Naming contract** (D2721). Write the slot-naming scheme (construct@span) into docs/continuation-dumps.md; extend the pin format to list named slots per site. No behaviour change. | docs, compiler/smap.rs, tests/smap | 1 |
| 3 | **Spill at suspend.** The compiler emits the spill/unspill moves (or an await operand count) from the role stack. Suspension maps report slots instead of operand roles. The exporter and loader read and write slots; `operands` disappears from the format. Pins and goldens rewritten. Bump `COMPAT_VERSION` (frame meaning at awaits changed). | compiler, interp/run/suspend.rs, vm/dump_flight.rs, vm/dump_resume.rs, tests/smap, tests/dump | 1 |
| 4 | **Constructs into slots.** for-of/for-await records, switch discriminant, `using` record and destructuring state live in hidden locals at suspension points; `loops` disappears. | compiler (for_in_of, stmt, using, pattern), dump_* | 1, after 3 |
| 5 | **finally completions in slots.** In functions that can suspend (sync code may keep gosub; measure both). This removes gosub-index mapping (D2627) and the return-path refusals. | compiler/stmt.rs, generator.rs, dump_* | after 3 |
| 6 | **Delete.** The role vocabulary in its current form (Role enum, `unsupported:<construct>`, depth checks), operand placement and gosub mapping in dump_resume.rs, role-specific edges in dump_flight.rs. The format becomes `{at, awaiting, this, args, slots, queue}`. | compiler, vm/dump_* | after 4, 5 |
| 7 | **pi adapter.** prelude-next and the adapter use host-call hooks for tool calls; pi's suite runs with `--dump-every-idle`. | tests/pi, js/pi | after 1 |
| 8 | **Load performance.** Keep each function's state table (state → pc, slot layout) in its compiled form in the dump build, or in a per-script cache, so a load compiles once. | compiler, vm/dump_load.rs | after 6 |

Tests at every step:

- smap pins and golden dumps (rewritten only in steps 3–6, each time with a decision);
- `DJS_E2E_MODE=dump`;
- difftest `--dump-every-idle`;
- the harvested and private idle-point counts (`vm::dump_harvest`), which must stay at
  100%;
- test262 async-functions, generators and async-generators (spill and finally changes);
- the dump-then-load idempotence (below) as a unit test;
- benches before and after steps 3 and 5.

**What stays:**

- verbatim function and class text;
- scope-group factories and `$dump.cell` (still needed for TDZ cells: plain JavaScript
  cannot initialize an outer TDZ variable, as R showed);
- re-enactment and the builtin-state primitives;
- `$dump.pending`/`handle`/`issued`/`random`/`private`/`builtin`;
- idle points only;
- in-process images and suspend-anywhere;
- packaging B;
- the pin and golden discipline.

**What goes:**

- operand roles as a stack description;
- the `operands` and `loops` keys;
- `finally.return` gosub indexes;
- `switch.discriminant` written twice;
- the recording recompile at load (step 8);
- most of `unsupported:<construct>`;
- the host-side `HostState` pattern.

## 5. Risks

1. **Spill cost.** k moves per suspension with k > 0. It is cheap, but awaits sit in
   loops in codemode, so measure (bench, wasm instruction counts) at step 3.
2. **finally lowering.** Changing gosub/ret to completion slots touches try/finally
   semantics everywhere, including completion values. Restrict it to functions that can
   suspend, and run test262's statements/try, generators and async suites.
3. **The single-pass parser.** Spans must be known where a role is pushed. They are: the
   parser is at the construct then. Slot reservation per site raises the frame size by
   the maximum k, not the sum.
4. **The contract remains.** SM shrinks the contract, it does not remove it. Do not
   promise that any later compiler loads any old dump. Keep refusals exact.
5. **Concurrency.** Steps 3–5 all touch the compiler and dump_* files: run them in
   sequence, one agent at a time. Steps 1, 2 and 7 parallelize.
6. **Images.** The meaning of a frame at an await changes in step 3, so bump
   `COMPAT_VERSION` there. In-process only.

## 6. Performance opportunities (for a later optimization agent)

- **One compile per load** (step 8). R's measurements bound the gain: load time ~0.5×
  on codemode dumps. Today the loader compiles the dump text twice.
- **Drop executed body text.** A codemode body is held only by its activation, and R
  shrank those dumps by 42% raw / 18% zstd by not carrying it. With SM the text is
  needed to compile the state table. A body could instead be written as the original
  text minus spans the state can never reach again (only top-level statements before the
  site, outside every loop), keeping positions by padding. This is worth a decision of
  its own; it changes `toString` of the body, which only the run holds (D2617).
- **Spill elision:** constants, `const` bindings re-read after the await (djs already
  re-reads them), and values the awaited expression does not need.
- **Dump-then-load is idempotent today.** Resuming a dump and dumping again at its first
  idle point (the loaded state) writes byte-identical text. Hosts can skip re-dumping an
  unchanged run.

## 7. Findings on the way

- **The non-terminating chain was not an engine bug.** My chain script dumped each
  resumed residual at idle point 1. Idle point 1 of a resumed run is the loaded state
  itself (D2604 numbers idle points before an answer), and today's exporter writes the
  identical dump there. So the chain was at a fixed point and looped (it wrote 880,944
  files before the coordinator stopped it). The fixed point is correct behaviour and a
  good property, and should be a unit test. The script now dumps at idle point 2 and
  caps at 50 links.
- **The anonymous-function stack name after resume** (`$f3`): see section 3.
- **Computed member keys:** djs re-reads `const` keys after an await (`t[k] = await v`
  keeps only `assign.object`). This is correct because `k` cannot change, and it agrees
  with qjs and node on the probe.

## Decisions

These are proposals for the coordinator to append to docs/decisions.md if accepted. They
are not added by this report.

- **D2720:** Suspended frames become named-slot state machines in the bytecode compiler
  (SM); no generated code in dumps (R rejected: dumps of resumed continuations double per
  generation); no source-level lowering of async code (A rejected: 2.4–5.4× slower).
- **D2721:** Slot names are construct@span relative to the function start; states keep
  `kind#N`. The pins list slots per state.
- **D2722:** Open host calls carry host JSON state with `save`/`resume` hooks declared by
  the host function; the Sandbox keeps the bookkeeping.
