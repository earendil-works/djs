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

## Follow-ups
- Exporter: write objects the dump runtime made (`$djs.fromSync`'s wrapper, `closeThrow`'s
  handler) as their helper call again instead of the runtime's closures (chain dumps of a
  `for await` over a sync iterator, ~1.6 KB each).

## Result
