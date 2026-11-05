# 0207: Make plain dumps small: measure where the bytes go, then cut them

status: in-progress
agent: pjs
branch: issue/0207-plain-dump-size
depends-on: 0201, 0202 (on main)
guide: snapshots.md (if the format's shape changes)
touches: vm/dump*.rs (in coordination with 0203), scripts/dump-costs/, work/research/0207/, tests/scores

## Goal
The owner's priority: **reduce the total size of plain dumps** (format 3, `DumpOptions::plain`),
substantially, without losing what makes them work: they run as plain Scripts in djs and node,
they continue identically to the uninterrupted run, re-dumps stay bounded (0202), and they
stay readable enough to debug.

Go deep. This is first an investigation, then an implementation:

1. **Measure.** Build a size breakdown tool (extend scripts/dump-costs/ or a new
   scripts/dump-size/): for every corpus idle point (difftest `--plain-out DIR`), the v2
   programs, examples/sleep-dump and pi codemode sessions (the pi suite with dumps), break
   each dump's bytes down by category: helper prelude (`$dump.R/L/advance`), builtin
   lookups (`$dump.bN = ...`), data (objects, arrays, strings), function texts written for
   closures (`$dump.nN = function …`), continuation bodies (`$dump.KaN`), origin function
   texts (`$dump.FaN`, which repeat the original function's text), `$djs` records (segs,
   scaf, cmap), scaffolding (`$djs_*` temporaries, loop scaffolds, blank lines and
   indentation), the final block. Report raw and gzip sizes, totals and per-case
   distributions, and the largest dumps with what dominates them.
2. **Find the waste.** Candidate ideas to evaluate (not a list to implement blindly):
   - text duplication: the origin function `Fa` repeats code that the continuation `Ka` and
     closure functions `nN` already contain (e.g. sleep-dump writes `step` twice and the
     whole script body twice). Can the origin be shared with the run's own function, sliced
     from one text, or written once and referenced by both?
   - the whole codemode source written several times (closures, origin, continuation);
   - `$djs` records: needed only for re-dumps; could they be smaller (relative offsets,
     packed strings) or derived from data already in the dump?
   - helpers written in every dump even when unused; names (`$dump.Ka6`, `$djs_t0`)
     longer than needed; whitespace and blank lines;
   - data encodings: repeated strings, shared structure, big arrays/objects, numbers;
   - things dumped that are unreachable from what the continuation actually needs (pruning
     by liveness: dead variables in a frame, captured scopes nobody reads);
   - builtins written as lookups when the target realm already has them;
   - whether a minified mode vs a readable mode is worth having (owner values readability;
     propose, don't decide).
   Estimate each idea's saving on the measured corpus before building it.
3. **Write it up** in work/research/0207/README.md (findings, numbers, ranked proposals with
   expected savings and costs) and send the coordinator a summary before the big changes,
   so the owner can weigh in on trade-offs (readability, format changes).
4. **Implement** the agreed wins in mergeable slices, each with before/after numbers, and
   add a size score (tests/scores or the dump-costs leg) so the gate catches regressions in
   total plain dump size.

## Constraints
- Another agent (pjr, issue 0203) is changing vm/dump_plain.rs, dump_cont.rs, dump_origin.rs
  and dump_flight.rs at the same time. Keep slices small, rebase often, and tell the
  coordinator before a large restructuring of those files so the two of you can sequence it.
- Format 2 is retired in 0205: spend nothing on format 2's size.
- Acceptance per merge: the plain-dump gate leg (djs and node over every corpus idle point,
  chains of 50) and every other gate leg stay green; the size score goes down.
- djs is unreleased (D024): change the format in place.

## Context
docs/plain-js-dumps.md (the design; section 5 "Pruning"), D2830–D2833, D2860–D2872,
D2890–D2898, crates/djs-core/src/vm/dump_*.rs, scripts/dump-costs/ (0198, D2800–D2802),
examples/sleep-dump/out/sleep.dump.js (a 2.7 KB dump of a 12-line script).

## Log
- Measurement tool: scripts/dump-size/dump-size.ts (structure scanner of the exporter's
  output: categories, marginal gzip per category, line provenance, repeats by category,
  `env.*` for the corpus prelude / harvest harness). Producers: difftest `--plain-out`
  (1,197 corpus dumps), `DJS_PLAIN_OUT` in dump_resume_tests (146 v2/golden dumps),
  sleep-dump, pi suite (147 dumps; a temporary uncommitted 2-line patch of js/pi/vm.ts).
  Estimator for the ideas: work/research/0207/estimate.ts. Results: work/research/0207/
  report.md, estimates.md; write-up README.md.
- Finding: 64% of corpus bytes are the test environment (corpus prelude state, harvest
  harness with its synthetic results), which pi's next mode never dumps; real-session
  dumps are dominated by continuation + origin + record + fixed overhead.
- Side note: the pi suite in --next --dump-every-idle --plain on a debug dump wasm had 7
  new failures ("passes undefined arguments and results through", "exposes tools under
  normalized identifiers"); not investigated (maybe debug-build only).

- Write-up: work/research/0207/README.md with ranked proposals (A–D clear wins, E–L
  trade-offs, score on corpus without env). Sent the summary to the coordinator; waiting
  for the owner on E–I. Next: A (record defaults) and B (`(0, f)` members) as small
  merges, C after checking with pjr.

- Owner/coordinator decisions: A, B, score approved; C approved (after pjr's slice, keep
  bindings named by scaf/cmap and bindings closures in K may capture); E–J replaced by
  D2834/D2835 ($djs host object with helpers and `source(ref)`, header `"djs-dump";`).
  Order: (a) header rename, (b) A+B+score, (c) helpers on $djs + js/src/dump-runtime.js,
  (d) $djs.source, (e) C.
- pi failures (release plain): 3x "exposes tools under normalized identifiers" = refusal
  "stopped again inside its rewritten statement" for `return { calls: [await a(), await
  a(), await a()] }` (dump_origin, 0203's open work); reported to the coordinator.
- Slice (b): A = D2900 (record defaults), B = D2902 (`key: (0, f)`), score = D2901
  (tests/scores/dump-size.tsv, in the gate leg "plain dumps (djs and node)").
  Before → after: corpus w/o env 8,827,779 → 7,996,238 raw (−9.4%), 2,623,606 →
  2,555,267 gzip (−2.6%); v2 533,139 → 526,329 raw (−1.3%), 191,848 → 190,536 gzip.

- (b) merged (ee39f80a). Slice (a): header `"djs-dump";` / `"djs-dump typescript";`
  (D2835) in the exporter, readers (CLI, sandbox.ts, difftest, plain-node.mjs), tests,
  docs; `djs resume --explain` says `format plain`.

- (a) merged (01501401). (c) in progress (WIP commit): crates/djs-core/src/vm/dump_runtime.rs
  holds HELPERS (moved out of dump_plain.rs; regexpIterator self-contained via a lexical
  `matchAll`), `script_text()`, `module_text()` (→ js/src/dump-runtime.js, test
  `dump_runtime_js_is_current`, needs RUST_KEEP in gate.py), `Vm::dump_runtime()`. Still
  to do: exporter writes `$djs.H` in plain mode (shared writers dump.rs/dump_flight.rs need
  a mode switch since format 2 still uses `$dump.H`; dump_cont.rs:1864 prologue), drop the
  `globalThis.$dump = { helpers }` content, hosts (CLI dumps.rs, difftest djs.rs, core
  tests plain_load, wasm export vm_dump_runtime, sandbox.ts loadPlain, pi vm.ts loadPlain,
  plain-node.mjs, sleep-dump.ts resumeInNode, js tests), docs.
- New scope from the coordinator (owner shocked by sleep-dump 2,751 B): target ≈ 700 B
  hand-written version (work/research/0207/target-sleep.js), items: 1 temps only when an
  operand is not pure to re-evaluate; 2 loop resumption without the L wrapper when no
  break/continue targets the loop; 3 continuations as immediately called functions with
  frame values as arguments; 4 records shrink to {f, at} if the exporter can regenerate K's
  layout deterministically (investigate first, report). Add sleep-dump to the score.

- Item 4 investigated and reported: records shrink to {f, at} only if K's text is
  canonical (no values inside K: IIFE arguments, canonical scaffold names, closures that
  capture none of K's bindings outside K); re-dump regenerates K from (f, at, K's
  parameter names) and requires exact text equality, holes `{f, at, h}` where value text
  must stay. Coordinator agreed; a mismatch is a clear refusal (no fallback, D024).
  Layout fields are 7.4% of pi bytes (record 17.3%), v2 13.7% (19.7%), corpus 1.1% (2.1%).
- Owner decision D2836: plain dumps only with frozen builtins; builtin operands are
  written as their expression or path, no temps. Next slice after (c).
- (c) done: helpers on `$djs` (D2903). dump_runtime.rs (HELPERS, script_text, module_text,
  Vm::dump_runtime / dump_runtime_from); exporter writes `$djs.H` in plain mode
  (`Dumper::rt()`, format 2 keeps `$dump.H`), K captures `$djs_R = $djs.R`; hosts: CLI,
  difftest, core tests, wasm `vm_dump_runtime(vm, len)` with the text from
  js/src/dump-runtime.js `script` (the text in the shipped module cost +2.3 KB gzip, over
  the ceiling; passed from JS it is +414 B), Sandbox, pi adapter, plain-node.mjs (runtime
  evaluated in each link's context), sleep-dump resumeInNode. The chain test's size check
  counts lines copying the runtime's code out (a re-dump writes the JS `fromSync`
  wrapper's closures where the run had the engine's record). Docs: plain-js-dumps.md §4
  (members one by one), host.md, guide/snapshots.md, sleep-dump README (refreshed, 2285 B).
  Score (after rebasing on 0203's iterator helpers, whose new helpers U M FM CC CI HD
  moved to the runtime too): corpus 8,062,700 → 7,950,316 raw, 2,577,552 → 2,533,166
  gzip; v2 652,232 → 578,286 raw, 234,344 → 206,934 gzip.

- (c) merged (d0fb2682). D2836 slice: the exporter refuses plain dumps without frozen
  builtins; hosts (D2904): CLI `run/resume --plain` and `repl --plain` make frozen VMs
  (`.plain on` without warns), Sandbox `plainDumps` default follows `vm_frozen_builtins`,
  difftest plain variant frozen and compared only for cases with idle points (24 corpus
  cases change builtins on purpose, none dumps), core plain tests `plain_vm()`, js tests
  and sleep-dump with `frozenBuiltins: true`. No host or test that dumps plain had to be
  dropped. Sizes unchanged.

- D2836 slice merged (cdb30b72). Sleep-dump in the score: core test
  `the_sleep_example_dumps` dumps the example's SCRIPT as the Sandbox does (codemode body
  with `ms`, console.log and async tools.sleep host functions, fixed deadline) and writes it
  to DJS_SLEEP_OUT; the gate leg scores it as `sleep_*` (2,321 B raw, 938 gzip; target
  ≈700). Items 1–3: asked pjr how to sequence them with 0203's dump_cont.rs work.

### pjr
- D2883: classes naming themselves (pjf's "cyclic dependencies": `class A {…}` in a
  factory scope whose methods/constructor name `A`, class expressions on globals),
  private-name bindings in frames dead like other class hidden bindings, a class's own
  static private field at its initial literal needs no patch (else a refusal that names
  class and field). Corpus case codemode/classes-self.js, test
  plain_dumps_of_self_naming_classes.
- D2882 `$djs.source(name, hash, params)` (slice d): codemode bodies' origin by reference.
  Core: DumpOptions::source_refs, vm::script_hash, Vm::source_function, origin of
  KIND_ASYNC_BODY through the wrapper's map. Hosts: CLI (`djs run` refers, `djs resume`
  reads the file at that path; the REPL writes text), Sandbox (this process's scripts or
  the `sources` option, `inlineSources`; hash checked in JS, `scriptHash`, so the shipped
  modules grow by 5/13 bytes), wasm `vm_set_dump_source_refs`, sleep example in node.
  pi and the difftest are unaffected: their codemode function is an arrow in a Script, not
  a body. Sleep example dump 2,104 → 1,730 B.
- 0211 items 3 (a suspended async method of a body class: its continuation goes where the
  class's member was placed) and 4 (template substitutions before the site get `k`
  records; dump_cont.rs, agreed). Corpus cases class-method-suspended, redump-template.
  Merged with D2882 (7496c0e8).
- D2886: source refs for the one function a host's wrapper script makes (pi's codemode
  arrow, the difftest's case function); pi adapter and difftest resolve it, plain-node
  stubs it. pi suite (106 dumps, next mode with dumps): median 824 → 782 B raw, largest
  1,253 → 1,095 B, total 91.1 → 81.4 KB (gzip 47.4 → 47.0 KB). TS dumps stay TypeScript
  when a named script has non-type TypeScript outside the dumped part.
- D2884: pjz's re-dump items 6 (functions/generators declared in loop bodies), 7 (`await
  using` disposal after copied text), G (await in `finally` in a loop on a chain), H
  (disposal block with a function sharing a `let`); continuation in an unmerged factory
  inside a block waits for the block's activation. Corpus case codemode/redump-loops.js,
  test plain_redumps_of_loops_finally_and_using. Classes C/D (class in a nested scope of
  shared variables, "a cell without its variable") are not done: a cause of their own (the
  class's inner name binding gets a scope of its own); format 2 refuses them too.
- D2885: pjz's items 3, 4, 5 and batch 2 (binary left operands, operator chains,
  template substitutions, awaited callee keys, assignments whose value awaits twice) and
  the classes C/D (inner name binding no factory variable; a frame-shared class binding
  declared by the continuation). Corpus cases codemode/redump-operands.js,
  codemode/classes-in-body.js; test plain_redumps_inside_rewritten_statements.
- Item 1 (D2905): dump_reread.rs decides per activation which call temporaries K reads
  again (Act::rereads, Ctx::rereads); dump_cont `call` writes the original call text when
  callee and `this` reread, `operand` for arguments. Test plain_dumps_read_operands_again.
  Score: corpus 7,980,365 → 7,868,547 raw; v2 578,286 → 546,263; sleep 2,321 → 2,104.
  Seen while writing the test (not from this change, not investigated): a getter of an
  object literal that captures a variable (`{ get m() { n++; … } }`) makes the dump's check
  fail with `get: ,` in the written text.

- Item 1 merged? (pending merge, rebased). Score per case (D2906): tests/scores/dump-size.tsv
  has `SUITE CASE RAW GZIP` lines; growth of a case fails, new/removed cases ask for
  --update. My tests moved to dump_size_tests.rs (a submodule of dump_resume_tests) to stop
  the end-of-file conflicts with 0203.

- pjz's ordering bugs (fixed; pjz's corpus cases codemode/plain-fuzz/{async-generator-for-await, generator-capturing-for-of, class-instance-loop-closure, accessor-pair-after-site} lose their plain-xfail markers): the callback now runs in
  parts at the ends of the continuation's blocks it needs (D2907, dump_plain
  split_callback, Ctx::go(block)); the accessor pair after a site: `$djs.DD` defines a
  getter's or setter's descriptor without the other half, as the literal does.

- pjz J: `for await` whose body always leaves the loop: the compiler leaves the dead
  `for-await.next` out of the site's map, the scaffold now takes `undefined` for a missing
  `next` (for-of too; `next_method`). Corpus case codemode/plain-fuzz/for-await-break.js
  (also the async generator suspended at its await with a `break`, pjz's "'return' of
  undefined", fixed by D2907).

- Batch merged (d56f59f0). Item 2 (D2908): for-of/for-in whose body has no break/continue
  and whose iterator has no `return`: `const $djs_i = it; { rest } for (Head of
  $djs_I($djs_i)) Body`; `$djs.I` takes `it.next` when `next` is left out (re-read, D2905
  rule in dump_reread); dump_origin reads the record's constants while K is in the rest.
  Unused `$dump.bN` builtins are no longer written. Test
  plain_dumps_continue_loops_after_the_rest. Sleep 2,104 → 1,962; corpus 8,154,483 →
  8,132,481; v2 551,509 → 545,726.

- Item 3 (D2909): the body's bindings are K's parameters, the values its call's arguments
  (consts the text may assign stay declarations, `may_assign`); K written in place with
  `$djs.k(K, record[, proto])(args)`; no blank lines and no braces for blocks that declare
  nothing. Tests plain_dumps_take_the_frame_as_arguments. Score: corpus 8,132,481 →
  7,741,422 raw; v2 545,726 → 497,622; sleep 1,962 → 1,751 (gzip 716; target 736/415).
  Remaining sleep bytes: the origin text `$dump.Fa6` (437 B, (d) pjr), the records (item
  4), `$dump.` names of single-use values (`$dump.n4`, `$dump.i5`, `$dump.p1`), dead
  bindings (`const label = "first"`, C).

- pjz 0211 round 2 (D2910): values of the callback by part (`made`, Ctx::made_in);
  bindings after their part or assigned after a deeper part, scaffold constants too.
  Corpus cases plain-fuzz/instance-before-callback.js, for-await-promise-all-chain.js.

- C (D2911): liveness pruning in dump_cont (P::Decl lines, P::Scope blocks, `prune`);
  dump_origin: a dead parameter reads as undefined on re-dump. v2 497,821 → 494,430,
  sleep 1,377 → 1,339 (gzip 697). Data that only dead bindings reach is still written in
  the first dump (flight renders all slots); a re-dump drops it.

- D2912 (review findings): k/tmp records only where K can stop again in the construct;
  block-end `site` records only with `using`; merged segments; bare `await` where it may
  stand. Sync-generator loop with Promise.all (pjz 0211): `$djs.L` `set` takes the
  iterator late. Docs refreshed: examples/sleep-dump/README.md (output, "The dump",
  `--explain`, which needs the script beside the dump: sleep-dump.ts writes out/sleep.js),
  docs/continuation-dumps.md (the plan example and its `--explain`). Sleep 1,339 → 1,247 B
  (gzip 648); v2 494,430 → 471,916. Not done from the review: the records' always-zero
  fields (records go with item 4), inlining single-use values (`$dump.n4 = []`).

- Item 4 plan (investigation). Records now: v2 59,198 B of 471,916 raw (12.5 %, gzip 15.6 %:
  segs 21 K, scaf 17 K, cmap 12 K), sleep 192 of 1,247 (15 %), corpus 180 K of 7.29 M (2.5 %).
  K's text depends on values in two ways: value texts inside it (the awaited promise,
  block code, scaffold constants, go calls, temporaries, the head's parameters) and
  decisions taken on values that change which of `f`'s text K copies and where its own
  constructs are (rereads, closeless loops, a pattern's done iterator, completion kinds,
  `using` triples). Only the second kind matters for origin tracking: the layout (segs,
  scaf, cmap) is where copied spans (`P::V`) and marks are, and generated text in between
  only shifts offsets. Plan:
  - dump_cont: every value-dependent decision that changes the shape (the sequence of
    copied spans, marks, literal temporaries) goes through `G::steer(n)`, which records `n`
    on a tape when writing and replays it when regenerating. A literal temporary written
    in place becomes a node of its own (its text is read back from K for cmap: numbers,
    booleans, null, undefined; string temporaries become constants).
  - Writing: generate K as now, then again from a canonical `Act` (the site, the slot
    names; every value a placeholder; a Ctx that answers placeholders) with the tape. The
    two piece lists must have the same shape (else refusal: a bug, never a fallback); the
    generated runs between structural pieces whose texts differ are the holes. Record:
    `{ f, at: "await#2", h: [run, len, ...], r: [tape] }` (empty fields left out).
  - Re-dump (dump_origin record()): f's map, its site `at`, the canonical Act, regenerate
    with `r`; walk the pieces against K's own text: fixed runs and copied spans must equal
    it, holes take their lengths; with the holes' texts put back the layout pass gives the
    write's segs/scaf/cmap exactly. Mismatch: refusal.
  - Later: the awaited promise and scaffold constants as K parameters (no hole at all).
  Estimate: ~75 % of the record bytes (v2 ~ -40 KB, sleep ~ -130 B, corpus ~ -130 KB).

- Item 4 (D2913): records are `{ f, at, h, r }`. dump_cont_canon.rs (new): K as pieces
  (runs, copies, marks, in-place literals), `G::steer` tape for every value-dependent
  shape decision (rereads, closeless, literal temporaries, pattern done, completion kinds,
  using records/triples/state), the canonical activation and Ctx, holes (runs whose text
  differs and does not read as the canonical one with a token per placeholder),
  `layout_of` for re-dumps (TypeScript: K's blanked types are spaces). dump_origin
  `record()` replaced: f's map and site `at`, the canonical K, the layout. String
  temporaries are constants now (re-dumps make no strings). Tape: trailing zeros dropped,
  a number of bits when all are yes/no. Test plain_records_make_the_layout_again.
  Score: v2 471,916 → 424,241 raw (gzip 177,258 → 153,023; records 59 → 10.6 KB); sleep
  1,247 → 1,093 (648 → 563; records `{ f: $dump.n2 }` and `{ f: $dump.Fa6, r: 54 }`);
  corpus 7,286,070 → 7,108,975. Dump time (corpus first dumps, release): ~5-10 % more
  (the second generation; the generator's liveness `render` is its largest part).
  Docs: plain-js-dumps.md §2.3, continuation-dumps.md, guide snapshots.md (example from a
  real run), examples/sleep-dump/README.md.
- 0207 stays open (owner): after item 4, fluff's (0216) ranked queue. pjr (0215) makes
  dump_plain's schedule/keys_in/dep_items/creator_block indexed after this merge.

- After the rebase onto 0203/0213: literal operands (strings without escapes, decimal
  numbers, true/false/null) are read again (dump_reread `is_literal`), as string
  temporaries are constants now; operands of K's own `$djs_R`/`$djs_C` call are always
  constants (origin tracking finds the original's temporaries in K's call only where it is
  the original's: a reread `new.arg` there was lost). Score vs main: v2 473,214 → 426,479,
  corpus 7,425,150 → 7,243,739, sleep 1,247 → 1,093.
- Queued (pjz 0211 batch 3): a `finally` whose normal end is dead (`if (true) continue`)
  has no `completion.kind` slot: "missing slot completion.kind@N".

- Batch after item 4: pjz's dead-end finally (corpus plain-fuzz/finally-dead-end.js);
  liveness renders each scope once (output identical; canon::generate ~2.4 % of the dump
  thread); D2914: object literals (`{ path, content: await … }`, spreads), binary left
  operands and template substitutions read again; rereads decided on the site's path (the
  call/binary at the same offset as its callee/left operand was a latent bug). Corpus
  7,243,739 → 7,216,900. Found: for a codemode body, the first dump's `act.site.pos` is
  relative to the body while its syntax is relative to the written `async function (…) {`
  (22 bytes apart in sleep); dump_cont uses `act.site.pos` for the `site` mark of a `using`
  disposal (follow-up, 0203's area).
- Candidates (writer, after pjr's indexing): one K for the activations of one closure at one
  site (corpus ~93 KB: 463 duplicate K texts in factories, Promise.all over map), with the
  awaited promise a parameter; inlining single-use values.

- fluff's queue (0216, work/research/0216/README.md) is the list now (coordinator's order).
  0/7 (D2915): positional records `$djs.k(K, f, at, r, h, proto)`, `K.$djs = [f, at, r, h]`;
  a generator's prototype is its function's (runtime default `f.prototype`); a function's
  default `prototype` held elsewhere is written `F.prototype` (dump.rs `fn_protos`); an
  unstarted generator with another prototype gets `Object.setPrototypeOf`. Corpus case
  language/generator-prototype.js. Score: v2 426,479 → 414,762; sleep 1,093 → 1,079;
  corpus 7,243,739 → 7,187,487.
  1 (evaluated): `h` and `r` stay. Measured now: v2 h 546 B + r 1,709 B of 415 KB (0.5 %),
  corpus h 7.4 KB + r 5.0 KB of 7.2 MB (0.17 %). Deriving `r` from K's text needs a search
  over decisions at re-dump (generation is not in text order: liveness and helpers come
  after), deriving `h` an expression/statement skipper whose ambiguous cuts would map
  wrongly; both cost re-dump time for a fraction of a percent.

- 3 (D2916): no final try/finally; the last statement is the completion value (`$dump.a6;`,
  in parentheses unless a reference, `void 0;` without handles: the CLI reads a promise
  completion as "run"); hosts delete `$djs` and `$dump`: `Vm::end_dump_load` (CLI, difftest,
  core tests), Sandbox and pi's adapter by running JS, node hosts themselves (plain-node,
  sleep-dump example, js/test/bindings). Score: v2 414,762 → 400,554, sleep 1,079 → 1,008
  (gzip 531), corpus 7,276,634 → 7,181,966. pi tests (next, dump at every idle point, debug
  module): one new failure, the worker backend's output-limit test, also without dumps
  (debug module speed, not this change).

- 5 (D2917): array iterators over a lone array as `[rest].values()`; loops over builtin
  iterators iterate them directly (tape `for-of.direct@N`; dump_origin's for-of reads K's
  own loop's iterator/next when it is no `$djs.L` wrapper); scaffold constants as K
  parameters; `while`/`do`/`for` rest-then-loop (no break/continue; `for` with `let` and a
  closure in the loop keeps the scaffold). Score: v2 400,554 → 395,149; sleep 1,008 → 925
  (gzip 489); corpus 7,181,966 → 7,134,411. Docs: sleep README transcript and explain,
  continuation-dumps example, guide snapshots example (real outputs).
- Left from item 4 (operands): string temporaries are constants since D2913 (139 in
  corpus+v2, ~3.5 KB); compound/member targets, numbers in template parts, user objects'
  data properties (fluff's "objects the dump has finished writing").

- D2918: assignments with the site on the right written as the original's (names, members,
  computed keys, compound old values where the target holds them now). v2 395,119 → 393,692.

## Follow-ups
- Exporter: write objects the dump runtime made (`$djs.fromSync`'s wrapper, `closeThrow`'s
  handler) as their helper call again instead of the runtime's closures (chain dumps of a
  `for await` over a sync iterator, ~1.6 KB each).

## Result
