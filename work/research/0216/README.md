# 0216: the byte-minimal plain dump, and every byte between it and main's

Research for 0207 and its follow-ups (no exporter code changed). Measured on main at
65309049. The owner's goal: dumps as small as possible, no fluff, loading as fast as possible.
pjr's constraint (0215): no data encodings that JS code decodes at load; every item below
says what it does to load time.

## Short answer

The byte-minimal dump keeps the user's text as written and everything a re-dump needs, and
drops almost everything generated around it. For 8 hand-written representative dumps it is
**36% smaller raw and 34% smaller gzip** than main's, and every one of them continues
identically in node and (where a djs host can run them) in djs. In the two dumps measured,
loading it takes **26% and 37% fewer instructions**: nothing in the queue slows loading.

| set | main raw | main gzip | minimal raw | minimal gzip | how measured |
|---|---:|---:|---:|---:|---|
| 8 hand-written cases | 23,927 | 8,226 | 15,420 (−36%) | 5,390 (−34%) | `account.mjs`, exact |
| sleep-dump | 1,247 | 648 | 716 (−43%) | 411 (−37%) | exact (0207's target: 736 / 415) |
| pi suite, 106 dumps | 60,037 | 38,104 | ≈31,000 (≈−48%) | ≈24,500 (≈−36%) | mechanical rules measured (37,248 / 28,220), plus the hand-only items as the 3 pi cases show them (−14 … −38% raw past the rules) |
| v2 + golden, 192 dumps | 475,037 | 178,453 | ≈286,000 (≈−40%) | ≈107,000 (≈−40%) | mechanical 371,913 / 140,323 measured; hand-only items as the 3 v2 cases show them (≈−23%) |
| corpus without env, 342 cases | 7,425,150 | 2,520,306 | 7,000,461 (−5.7%) with rules 1–6; ≈ −10% with the rest | 2,358,365 (−6.4%); ≈ −8% | rules 1–6 measured by dump-size `--score` (its env scanner needs main's names); the rest estimated (estimates.md); the corpus is mostly tool results and other data |

With the owner/host items on top (seed only when used, the source named by hash alone,
no `"use strict"`, pi's call state left out), a pi dump loses another ≈9 KB raw / 6 KB
gzip over the suite: ≈ −63% raw / −51% gzip against main. The sleep dump would be ≈640 B.

The three biggest exporter items, in every set: **the records' layout fields** (0207 item
4, in progress; ≈11% raw, 11–15% gzip), **writing single-use values where they are used**
(host calls, `Promise.all`, data, activations, callbacks; 7–13%), and **the final
`try/finally`** (host deletes `$djs`; 13% of a pi dump, 8% gzip).

## Method

- Producers (scratch, `work/scratch/0216/`): difftest `--plain-out` (1,342 corpus dumps),
  `DJS_PLAIN_OUT`/`DJS_SLEEP_OUT` with `plain_dumps_at_every_idle_point`
  `the_sleep_example_dumps` (192 v2 dumps, the sleep dump), and the pi suite in next mode
  with a dump at every idle point (`pi-tests.ts --next --dump-every-idle` on the release
  dump module, 106 dumps, a temporary two-line write in `afterIdle` of js/pi/vm.ts, not
  committed; all tests pass).
- [rules.mjs](rules.mjs): the mechanical part of the minimal form, as text rewrites
  ([scan.mjs](scan.mjs) is a small lexer). [suite.mjs](suite.mjs) applies them to whole
  suites ([suites.md](suites.md)), cumulatively, per rule.
- [cases/](cases/): the hand-written minimal dumps. Each `steps.mjs` turns main's dump
  (`main.js`, unchanged) into `minimal.js` by categorized steps (rules, then exact text
  replacements); [account.mjs](account.mjs) applies them, checks each replacement matches
  once and reports raw and gzip −9 per step ([account.md](account.md)). Every byte of the
  difference is in one step, so the steps are the accounting.
- [estimate.mjs](estimate.mjs) ([estimates.md](estimates.md)): upper bounds over whole
  suites for items only the hand-written cases do; [owner.mjs](owner.mjs)
  ([owner.md](owner.md)): the owner/host items on top of the minimal dumps.
- Checks. node: [check.mjs](check.mjs) runs main's and the rewritten dump under one generic
  host and compares the transcripts (pi's tools, `later`, `print`, stubs for host values);
  [plain-node-min.mjs](plain-node-min.mjs) is scripts/plain-node.mjs with the minimal
  format's host changes, for the corpus. djs: `djs resume` ([check-djs.sh](check-djs.sh), v2
  shapes) and the Sandbox ([check-sandbox.ts](check-sandbox.ts), the sleep dump), with a
  one-line shim for the positional `$djs.k` after the header (djs's runtime is main's).
  Load: [instr.sh](instr.sh), instructions retired by a release `djs resume`.

Results of the checks:

| what | node | djs |
|---|---|---|
| hand-written cases (8) | all identical | sleep (Sandbox), v2-plan, v2-operands, v2-generator identical; pi and corpus shapes have no djs host outside their suites |
| pi suite, rules.mjs (106) | 106 identical | – |
| v2 suite, rules.mjs (192) | 190 identical, 2 differ only in `Math.random` (node cannot seed) | 192 identical |
| corpus, rules.mjs (1,337 links) | 1,337 identical (plain-node-min) | – |

What cannot be checked yet is the **re-dump** of a minimal dump: it needs the exporter to
make K's layout again from `f` and the site (0207 item 4). The section on records says what
each minimal dump keeps for it and why that is enough.

## The minimal dump

The sleep example, 716 bytes (main: 1,247; [cases/sleep](cases/sleep)):

```js
"djs-dump";
"use strict";
$djs.seed(2135587861,-1640531527);
$djs.k(async function (ms, step, t0, results, $i) {
results.push(await $djs.k(async function (label) {
const r = await $djs.resume(2,"tools.sleep",[5000],{"due":1793822825106});
return label + ": " + r;
},step)("first"));
for (const label of $i) {
  results.push(await step(label, ms));
}
console.log("done after", Date.now() - t0 >= 2 * ms ? "at least" : "less than", 2 * ms, "ms:", results.join(", "));
return results;
},$djs.source("sleep.js","b48bd687f864a8d6","ms"))(5000,async function step(label, ms) {
  console.log(label, "sleeps", ms, "ms");
  const r = await tools.sleep(ms);
  return label + ": " + r;
},1793822820106,[],["second"].values())
```

A pi dump, the suite's largest (910 → 307 bytes; [cases/pi-largest](cases/pi-largest)):

```js
"djs-dump";
"use strict";
$djs.seed(2033933741,-1539143467);
$djs.k(async (tools) => {
return { all: ALL_TOOLS, calls: ["dash", await $djs.resume(2,"call",["call","my-tool",undefined],{"id":2}), await tools.mcp__docs__search()] };
},$djs.source("codemode.js","f593729dd2bd2a94","",1),2)($djs.host("tools"))
```

(main writes the half-built object as `$dump.a2_0 = { all: $djs.host("ALL_TOOLS") }`, a
`$djs_D` capture, a temporary, `($djs_D($djs_t0, "calls", [...]), $djs_t0)`, a 300-byte
record, `$dump.Fa2`, `$dump.p1`, `$dump.a2` and the `try/finally`.)

The others: [pi-median](cases/pi-median/minimal.js) (568 → 315),
[pi-promise-all](cases/pi-promise-all/minimal.js) (859 → 347),
[v2-plan](cases/v2-plan/minimal.js) (docs/continuation-dumps.md's example; 2,535 → 1,682),
[v2-operands](cases/v2-operands/minimal.js) (the largest v2 dump, 17 activations; 7,695 →
3,718), [v2-generator](cases/v2-generator/minimal.js) (1,583 → 901),
[corpus-generator](cases/corpus-generator/minimal.js) (8,530 → 7,434, of which 7 KB is the
difftest prelude, which gets only the mechanical rules).

The rules the minimal dumps follow:

1. **Header**: `"djs-dump";` and `"use strict";` (the latter is an owner item).
2. **Seed** `$djs.seed(lo,hi)`; `next` only where it is not one past the highest reopened id.
3. **No holder and no final statement.** A value used once is written where it is used,
   in the same evaluation order (host hooks in id order, reactions on one promise in their
   order, activations in their order); a value used more than once that is not a global of
   the program is a `const` of a block around the dump's own statements (`{const $g=…; …}`).
   The Script's completion value is its last expression: the run's promise (`$djs.k(…)(…)`),
   or with several handles the null-prototype object. Without handles the host reads no
   completion value. The host deletes `$djs` after the Script (the CLI already does).
4. **Continuations** are written where their activation is first used, often inside
   another continuation's site operand (`results.push(await $djs.k(…)("first"))`): they run
   at the same moment, since a continuation's prefix runs while the dump loads.
5. **Records** `$djs.k(K, f[, site][, proto])`: the origin and the site, nothing else.
   `site` is the await's number, the string only for other sites (`"yield#1"`), left out
   for the first await; `proto` left out where it is `f.prototype`.
6. **Operands** evaluated before the site are written as the original's text wherever that
   gives the saved value on the dumped state (D2905, extended from call operands to every
   operand: object and array literals under construction, compound assignments' old
   values, `new` callees, data properties of ordinary objects the dump has finished
   writing). No temporaries, no `$djs.R/C/D/Q`, no `$dump.aN_k`.
7. **Loops** whose bodies cannot break or continue: the rest of the iteration, then the loop
   as written (D2908, extended from for-of/for-in to `while` and C-style `for` whose
   variable no closure captures). A loop over a builtin iterator without `return` iterates
   the iterator itself (`for (const label of $i)`), the iterator a parameter of K; an array
   iterator nothing else holds is written over the elements left (`["second"].values()`,
   not `$djs.advance(Array.prototype.values.call(["first","second"]), 1)`). A helper the
   prefix calls only while loading is called as `$djs.L(…)`, without a capture.
8. **Callbacks** (`$dump.gaN`) and factory plumbing (`Object.assign($dump.sN, …)`,
   `var x = $dump.sN.x`) go where their values are used once: closures are written where
   the original makes them (a function declaration of K, a reaction's arrow at the site).
9. **Builtins as methods** where frozen builtins make it the same (`p.then(f)`, not
   `Promise.prototype.then.call(p, f, undefined)`), trailing `undefined` arguments left out.
10. **Whitespace**: the user's text as written; generated text without optional spaces.
    Newlines between statements stay (they cost nothing in gzip).

## Records: what a re-dump really needs

A re-dump reads a continuation's frame and writes it as an activation of its origin, so it
needs to map K's site and slots to `f`'s. pjs's plan (0207 item 4, D2913 on its branch):
records `{f, at, h, r}`, K's layout made again from `f`, the site and a canonical K, and
checked against K's text. Field by field:

| field | needed? | minimal | why |
|---|---|---|---|
| `f` | yes | a reference (`step`, `main`) or `$djs.source(…)` in place | the origin; nothing else names it |
| `at` | yes, cheaply | `2`, `"yield#1"`, left out for `await#1` | derivable by regenerating K for each site of `f` and keeping the one that matches (K's text differs per site), but that multiplies re-dump time by the sites of `f`; one or two bytes are cheaper |
| `segs`, `scaf`, `cmap` | no | – | the layout item 4 regenerates (11% of pi bytes, 15% of v2 gzip) |
| `h` (holes) | no | – | every hole is a value written at an expression position (the awaited operand, an argument, an initializer, a loop head). The re-dump compiles K to read its frame anyway, and K's suspension map has the spans of every construct around K's site; the matcher takes the hole's extent from them (or from a skip of one expression) instead of a recorded length |
| `r` (tape) | rarely | – | a steered decision changes K's text, so the matcher can take the alternative that matches K's text at that point (deterministic, with a lookahead to the next fixed piece). The exporter can check at write time whether text-guided steering finds the recorded layout and write `r` only where it does not |
| proto (`k`'s third argument) | rarely | left out where it is `f.prototype` | main writes a fresh `{ __proto__: %GeneratorPrototype% }`, which is also a bug (below) |
| `$djs.k` itself | yes | `$djs.k(K,f,at)` | the record has to be on K; the positional form saves `{ f: `, `, at: ` and `}` |

pjs's "later" idea, the awaited promise as a parameter of K, costs bytes in the general
case (`, $p` and an argument) and is only worth it where it lets several activations share
one K (item 11).

## The accounting

Every byte between main's dump and the minimal one, by category. Totals are over the 8
cases (exact; per case in [account.md](account.md)); "suites" says what the mechanical rule
measures over the whole suite where there is one.

| category | cases Δ raw / gzip | suites (rules.mjs) | why main writes it | how it goes | cost | load |
|---|---:|---|---|---|---|---|
| record.layout: `segs`, `scaf`, `cmap` | −2,857 / −1,157 | pi −11.1% / −10.8%; v2 −11.1% / −14.8%; corpus w/o env −2.6% | origin tracking reads them | item 4 regenerates the layout; `h`, `r` derived (above) | regeneration in the re-dump (time), a refusal on mismatch | faster: no record arrays compiled and allocated |
| inline: single-use values | −1,505 / −411 | host values and lone resumes only: pi −7.3% / −3.3%; v2 −0.7%; all kinds (upper bound, estimates.md): pi 12.8%, v2 6.1% | every value gets a `$dump.X` (format 2's model, one line per value) | write a value used once at its use when the order of host hooks, promise reactions and activations is kept | the writer needs per-value use counts and an order check; text reads less linearly | faster: no global property set/get per value |
| reread: operands as text | −1,063 / −230 | estimates.md: v2 1.6%, pi 1.1% | D2905 covers call callee/`this`/arguments only, through frozen objects | extend to every operand kind, and to data properties of ordinary objects written before K runs | exactness argument per kind (no getters, proxies, later patches) | faster: no helper calls (`$djs.R/C/D/Q`) |
| final: `try { (x); } finally { delete … }` | −604 / −232 | pi −12.7% / −8.0%; v2 −3.3% | the dump deletes `$dump` and `$djs` (D2890) | no global holder (rule 3), the host deletes `$djs` after the Script; the completion value is the last expression | each host deletes `$djs` itself (CLI does; Sandbox, pi adapter, difftest, plain-node add a line) | neutral |
| loop: scaffolds and iterators | −604 / −214 | estimates.md: sleep 8%, v2 1.1% | placeholder scaffolds for `while`/`for`; `$djs.I` and a captured record for builtin iterators; `$djs.advance` from the start | rule 7 | the regenerator learns the same forms | faster at load and in the run: native iteration instead of the helpers' JS `next` |
| callback: `$dump.gaN`, factory plumbing | −577 / −190 | estimates.md: corpus 1.4%, v2 0.3% | the callback runs the part of the dump that needs K's closures (D2907) | rule 8 where its values are used once at the site | the writer places expressions into K's site operand | faster: fewer closures and property sets |
| record.encoding: positional, site number, proto | −451 / −137 | pi −1.8%; v2 −1.0% | object literal record, `"await#N"` strings | rule 5 (and the proto fix) | runtime `k(K, f, at, proto)` | neutral |
| holder: `globalThis.$dump = {}`, block consts | −204 / −129 | – | one global namespace for the dump | rule 3 | a block's consts must not be captured by closures that live on (the exporter knows captures) | faster: lexical instead of global property access |
| ws: generated whitespace | −170 / −25 | pi −1.0% / −0.4%; v2 −1.2% / −0.1% | readable spacing | rule 10 | less readable generated text (owner) | neutral |
| keep: no-op `$djs.keep` | −166 / −34 | estimates.md: v2 ≤0.7% | written for every class | only for classes the run deleted members of | none | faster |
| builtins.methods | −129 / −37 | pi −0.5%; v2 −0.6% | builtin paths with `.call` | rule 9 where the receiver has no own property of that name | the check | neutral |
| names: `$djs_x`, `$dump.` in the env | −110 / −9 | pi −0.8% − 2.3%; v2 −1.1% − 2.6% | self-describing generated names | `$x`, `$d.` (owner) | readability | neutral |
| unused: an operand nothing reads | −30 / −8 | – | `$dump.a40_0 = Base.prototype` for a super method | do not write it | none | faster |
| seed.next | −22 / −14 | pi −0.5% | always written | rule 2 | none | neutral |
| decl: `var g = function* g` | −15 / −9 | – | format 2's writer writes global generator functions as `var` (why is unclear) | a declaration as the original's | check why it was done | neutral |

Not in the cases but in the suites: **same function, same site** (h179: nine `$KaN`
continuations identical but for the awaited promise, ≈2.2 KB of 26.8 KB; corpus 0.2%
by estimates.md), the difftest prelude's **name fixups** (`Object.defineProperty(f, "name",
{ value: "echo" })`, 2,773 in the corpus, 178 KB) and **freezes** (8,055 lines, 200 KB:
`Object.freeze(ALL_TOOLS[0])` could be `Object.freeze` around the literal where the object
needs no later patch), and **data whitespace** (`{ id: "x", name: "y" }` → `{id:"x",name:"y"}`:
part of the whitespace rule; raw only).

## The queue (exporter, no owner decision needed)

Ranked by what they save on a real session's dump (pi), then v2. Gzip figures are from
the cases or the suites as above; "load" is never worse.

| # | item | pi raw / gzip | v2 raw / gzip | sleep | notes |
|---|---|---:|---:|---:|---|
| 1 | Records without layout (0207 item 4); `h` and `r` derived from K's syntax and text, written only where needed | −11% / −11% | −11% / −15% | −161 B | pjs is on it; take `h` from K's map, steer by text |
| 2 | Single-use values in place, all kinds, order-preserving (rules 3–4) | −7 … −13% / −3 … −8% | −1 … −6% | −111 B | includes the handle as the completion value and continuations in site operands |
| 3 | No final statement: the host deletes `$djs`; the completion value is the last expression | −13% / −8% | −3% / −3% | −72 B | an interface change (D2890): five hosts |
| 4 | Operands as the original's text, every kind (rule 6) | ≈−1% suite; −12% / −7% on pi-largest | −2% | – | extends D2905 |
| 5 | Loops: rest-then-loop for `while`/`for`, builtin iterators iterated directly, array iterators over what is left, no captures for load-time helpers (rule 7) | – | −1% … −3% | −115 B | extends D2908 |
| 6 | Callbacks and factory plumbing at the site (rule 8) | ≈−0.5% suite; −22% / −15% on pi-promise-all | −0.3% | – | |
| 7 | Positional records, site as a number, proto derived (fixes the generator prototype bug) | −1.8% / −1.6% | −1.0% / −1.1% | −14 B | runtime `k` changes |
| 8 | Block consts instead of the `$dump` holder for values used more than once | small | −0.5% | −23 B | needs the capture check |
| 9 | `keep` only for classes with deleted members; no unused operands | – | −0.7% | – | |
| 10 | Builtins as methods; `next` of the seed only where not derivable | −1.0% | −0.7% | −3 B | |
| 11 | One continuation per (function, site) with the awaited promise as a parameter | – | 0.1% | – | harvest dumps with `Promise.all(xs.map(async …))`: up to 8% |
| 12 | Freeze at creation; compact name fixups (the difftest prelude, so mostly the score) | – | – | – | corpus env |
| 13 | Liveness of a placeholder loop's head binding (`const x = 1;` that nothing reads) | – | small | – | |

Items 2, 4, 6 and 8 share one piece of machinery: per-value use counts with the evaluation
order of the dump, which the writer's scheduler (D2907, D2910) mostly has.

## Owner and host decisions (separately)

Measured on top of the minimal dumps ([owner.md](owner.md)); pi is the pi suite after
rules.mjs.

| item | pi raw / gzip | cases raw / gzip | what it costs |
|---|---:|---:|---|
| Seed only when `Math.random` was used before the dump (an unused generator's state is unobserved: any seed is consistent with what the run saw) | −8.9% / −7.8% | −1.7% / −3.7% | the engine tracks "used since seeded" (one bit); a resumed run's random numbers differ from the uninterrupted run's if it calls `Math.random` for the first time after the dump (they were unpredictable to it either way); D2832/D2897 |
| The source named by hash alone, 32 bits (`$djs.source("54e8cfc5")`) | −8.5% / −7.1% | −1.1% / −2.1% | the host looks up its scripts by hash and keeps the wrapper kind and params itself; a 32-bit hash collides sooner (D2882, D2886) |
| `"use strict"` left out: hosts compile a dump as strict code (djs always does, D015) | −4.0% / −3.4% | −0.7% / −1.3% | node hosts prepend it; a dump is no longer a self-describing strict Script |
| pi: an open call's state left out where it equals the call id (114 of 121) | −3.0% / −2.9% | −0.2% / −0.4% | pi's adapter (host) change, not the exporter's |
| Generated names short (`$djs_i1` → `$i1`, `$dump.n2` → `$2`) | −0.8 … −2.3% / ≈−0.5% | −0.7% / −0.2% | readability of generated names |
| Generated whitespace (rule 10) | −1.0% / −0.4% | −1.1% / −0.5% | readability; done in the minimal dumps above |
| No newlines between generated statements | −1.0% / −0.2% | −0.1% / +0.1% | readability; **not worth it** (gzip gains nothing) |
| Header `"djs-dump";` | – | – | D2835; kept |

## Not proposed (load time, pjr's constraint)

- **Shared repeated strings or structure** (a table and references, 0207's E10/E11: up to
  3% of the corpus without env, raw only): references decoded at load cost more than
  literals compiled natively; and gzip already removes most of the repetition.
- Run-length or packed encodings of arrays, numbers as base-36 strings: decoded by JS.
- A minified mode of the user's code: excluded by the goal, and 0207 measured < 1%.
- Native decoders stay fine where they exist (`$djs.bytes` = `Uint8Array.fromBase64`).

Load time measured (release `djs resume`, best of 5, minus an empty dump's 24.4 M
instructions; load plus the rest of the run, which is the same work for both):

| | main | minimal | |
|---|---:|---:|---:|
| v2-plan | 2.60 M | 1.92 M | −26% |
| v2-operands | 8.55 M | 5.35 M | −37% |
| v2 suite, rules.mjs only (192 dumps, one run each) | 554 M | 488 M | −12% |

## Findings on the way

- **Bug: a resumed generator's prototype.** A generator suspended at a yield is written
  with `$djs.k(K, rec, { __proto__: %GeneratorPrototype% })`, a fresh object, while its
  function's `prototype` (made again by the dump) is another one: after a load
  `Object.getPrototypeOf(g) === counter.prototype` is false (true in the run). Reproduced
  with `djs run --dump-after 1` on a 3-line program; plain-js-dumps.md §9 has the case where
  the prototype was replaced. Item 7's "proto left out where it is `f.prototype`" fixes the
  common case.
- `$djs.keep(C, [])` is written for every class, also when nothing was deleted (v2-operands:
  six no-op calls).
- An operand nothing reads (`$dump.a40_0 = Base.prototype` for a `super` method's
  continuation).
- Global generator functions are written `var counter = function* counter(…) {…}`, other
  functions as declarations.
- The head binding of a placeholder loop's rest is declared though nothing reads it
  (`const x = 1;`).
- dump-size.ts's environment scanner does not recognize rewritten names (`$d.`): scores of
  experiments like these need the rules that keep main's names.

## Reproducing

```sh
S=work/scratch/0216; R=work/research/0216
DJS_SLEEP_OUT=$S/sleep DJS_PLAIN_OUT=$S/v2 cargo test -p djs-core --lib -- the_sleep_example_dumps plain_dumps_at_every_idle_point
target/debug/djs-difftest --engine djs --quiet --no-diff --plain-out $S/corpus
# pi: in js/pi/vm.ts afterIdle, write dumped.dump to a file (not committed), then
DJS_WASM=$PWD/target/djs-dump.wasm node js/pi/pi-tests.ts --next --dump-every-idle --out target/pi-codemode-dump
node $R/suite.mjs --out $S/mech sleep=$S/sleep pi=$S/pi v2=$S/v2 corpus=$S/corpus
node $R/check.mjs --dirs $S/v2 $S/mech/v2; node $R/plain-node-min.mjs $S/mech/corpus; $R/check-djs.sh $S/v2 $S/mech/v2
node $R/account.mjs; node $R/check.mjs $R/cases/sleep/main.js $R/cases/sleep/minimal.js
```
