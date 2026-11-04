# 0201: Exporter: plain-JS continuations in Rust (behind a flag), djs run of plain dumps, node harness and chain check in the gate

status: done
agent: pjx
branch: issue/0201-plain-exporter
depends-on: 0200 (interface first)
guide: -
touches: crates/djs-core/src/vm/dump*.rs (a new plain writer next to format 2), crates/djs-difftest (--dump-every-idle writes plain dumps behind a flag), scripts/ (node harness), tests

## Goal
docs/plain-js-dumps.md section 8 "Recommendation and plan", step 2.

## Context
docs/plain-js-dumps.md (the design, results and plan: read all of it), the prototype in
work/research/0199/ (plain.mjs, normalize.mjs, freevars.mjs, the harness and chain checks),
decisions D2830-D2833, and today's format 2 exporter (vm/dump*.rs, D2610-D2628, D2730-D2735,
D2800-D2802). Acceptance at every merge: dumps run as plain Scripts in djs **and node** and
continue identically to the uninterrupted run over every corpus idle point they cover,
re-dumps stay bounded (cap chains at 50 links), and the existing gate checks stay green.
djs is unreleased (D024): no compatibility code for format 2 once it is retired.

## Log

### Plan (pjx, start)
- Exporter in plain mode (`DumpOptions::plain`) still writes format 2 lines (the checker
  compiles them) but writes plain forms at the source: builtins `$dump.bN` captured at
  the top, open calls `$djs.resume(id, name, args[, state])`, host fns by name, no
  `$dump.random` (`$djs.seed(lo, hi)` first). It records an IR next to `out`: factories
  (scopes, vars, props with captured boxes), activations (fn node, site, slots with
  cells, awaiting, this, args, queue). References to closures of merged factories (a
  frame shares their variables) inside data become exporter patches.
- vm/dump_plain.rs: the writer (port of plain.mjs toPlain after parsing: placement,
  scheduling with callbacks, emission, helpers); vm/dump_cont.rs: the continuation
  generator (port of gen()), fed by pjc's site-structure records (0200).
- Interface agreed with pjh/coordinator: header `"djs-dump 3";`, host hooks under the
  global `$djs` (`seed`, `resume`), completion value = the single handle, `$dump`
  deleted in a final try/finally.
- Testing: difftest `--dump-every-idle --plain` (in-process chains, cap 50 links), with
  `--plain-out DIR` for a node harness (scripts/plain-node.mjs) comparing transcripts.

### State (pjx, first session)
- Exporter (`DumpOptions::plain`): IR recorded in dump.rs emit_group / dump_flight.rs
  emit_act; plain forms at the source ($dump.bN builtins, $djs.resume, host fns by name,
  refusals for privates/using/closeThrow/iterator helpers); closures of merged groups
  inside data become exporter patches (`plain_patched`), which replaces the prototype's
  placeholders.
- vm/dump_plain.rs: refs scanner, writer (placement, scheduling with callbacks,
  emission, helpers, `"djs-dump 3"`, `"use strict"`, `$djs.seed`, completion value =
  handle). vm/dump_cont.rs: the generator against pjc's smap_syntax types (untested
  until 0200's recording lands).
- difftest `--plain-dumps` (chain cap 50, `$djs.seed` host fn, `$djs.resume` =
  `(id, name, args) => later(...args)`); all codemode cases refuse with "no syntax
  recorded at the site" until 0200 lands.
- Interfaces agreed (coordinator, pjh): `$djs.seed(lo, hi)`, `$djs.resume(id, name,
  args[, state])`, one handle, header `"djs-dump 3"` (`typescript` variant), host
  functions written inline as their host_fn_name expression (evaluated during the load).

### Second session (pjx)
- Rebased on 0200 (site syntax recorded) and 0206 (patterns, TS ranges), and on 0204
  (pjh's host side; `djs-cli` `dump_names` now passes `plain` through).
- Fixes found by the corpus: dependencies of a factory or a statement are what its
  evaluation reads (`eager_refs` skips function and class bodies; a global named inside
  a closure is read when it runs, and a continuation of an earlier dump names that
  dump's `$dump`); a group's block is placed where its frame declares its live variables;
  values written by a global's name that a factory variable shadows are read before the
  factory (also fixes the same latent case in format 2); continuations of continuations
  get fresh scaffold/temporary names and no second helper declaration; continuation keys
  avoid member names (an earlier dump's `$Ka18`); the final block parenthesizes the
  handle and deletes `$djs` too (pjh, D2890); several handles by name (D2895).
- difftest: `--plain-out DIR` (dumps, output before each, the plain run's transcript and
  clock); a refusal ends a case's dumps; `$djs.resume` through `Vm::open_host_call`
  like the hosts; TS plain dumps compile as TypeScript.
- scripts/plain-node.mjs (node check, refusal ratchet tests/plain-refusals.txt), gate leg
  "plain dumps (djs and node)", TESTING.md paragraph, D2860/D2861.
- djs-core tests: `plain::tests` (scanner), `plain_dump_chains_continue_like_the_run`
  (the v2 fixtures as chains: 35 links; refused: operands (new.target/super),
  asyncgen (for-await site), reenact (async-from-sync close, a promise reacted to
  before the dump can make it)).
- Guide: no page describes plain dumps yet (none affected).

## Follow-ups
- 0202: origin tracking. Today a continuation of a continuation keeps the earlier ones
  as its `$djs.f` (linear growth, ~3 kB per link in h261); stale `$dump.` names inside
  old continuation texts are harmless (never run) but ugly.
- 0203: for-await sites (codemode/async-helpers.js idle point 8), new.target/super,
  patterns and defaults (now on paths, 0206), yield*, update expressions, tagged
  templates, async-from-sync close, iterator helpers, `using`.
- TS plain dumps as plain JS (strip every function text the dump writes, D2853), then
  run them in node too (plain-node.mjs skips `"djs-dump 3 typescript"`).
- `djs run --plain` (pjh's run.rs has the `plain` field "once the plain exporter
  lands"): the exporter is in; wiring the flag is CLI work.
- The node check maps only V8's `Cannot read properties of ...` wording; other
  engine-specific messages would need entries.

## Result
Plain dumps (format 3, D2830) behind `DumpOptions::plain`: `vm.dump(&DumpOptions {
plain: true, .. })` returns a Script starting `"djs-dump 3";` that a host runs with
`$djs = { seed, resume }` (D2890); its completion value is the `run` handle (or every
handle by name, D2895). Over the whole difftest corpus 1,197 idle points: every case is
dumped at each idle point as a chain of up to 50 links and continues identically in
djs; 1,187 links also run in node identically (TS dumps skipped in node); 1 case refuses
(for-await site, 0203), listed in tests/plain-refusals.txt. Check: `djs-difftest
--engine djs --plain-out DIR` then `node scripts/plain-node.mjs DIR --refusals
tests/plain-refusals.txt` (the gate leg). Known gaps: the 0203 constructs refuse; chain
growth until 0202.
