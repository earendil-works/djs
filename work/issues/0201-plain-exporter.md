# 0201: Exporter: plain-JS continuations in Rust (behind a flag), djs run of plain dumps, node harness and chain check in the gate

status: in-progress
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


## Result
