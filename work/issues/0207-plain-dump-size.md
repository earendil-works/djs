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

## Follow-ups

## Result
