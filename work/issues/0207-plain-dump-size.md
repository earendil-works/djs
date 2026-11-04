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
- WIP on branch issue/0207-source-ref (not merged): `$djs.source(name, hash)` for codemode
  bodies (DumpOptions::source_refs, script_hash, Vm::source_function, origin of
  KIND_ASYNC_BODY via the wrapper's map); hosts not done yet.
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

## Follow-ups
- Exporter: write objects the dump runtime made (`$djs.fromSync`'s wrapper, `closeThrow`'s
  handler) as their helper call again instead of the runtime's closures (chain dumps of a
  `for await` over a sync iterator, ~1.6 KB each).

## Result
