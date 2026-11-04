# 0202: Origin tracking: K.$djs read back; frames of continuations written as activations of their origin; corpus chain check

status: done
agent: pjx
branch: issue/0202-origin-tracking
depends-on: 0201
guide: -
touches: vm/dump*.rs, tests

## Goal
docs/plain-js-dumps.md section 8 "Recommendation and plan", step 3.

## Context
docs/plain-js-dumps.md (the design, results and plan: read all of it), the prototype in
work/research/0199/ (plain.mjs, normalize.mjs, freevars.mjs, the harness and chain checks),
decisions D2830-D2833, and today's format 2 exporter (vm/dump*.rs, D2610-D2628, D2730-D2735,
D2800-D2802). Acceptance at every merge: dumps run as plain Scripts in djs **and node** and
continue identically to the uninterrupted run over every corpus idle point they cover,
re-dumps stay bounded (cap chains at 50 links), and the existing gate checks stay green.
djs is unreleased (D024): no compatibility code for format 2 once it is retired.

## Log

### pjx
- Design (D2870): instead of the prototype's text normalization, the exporter reads a
  continuation's activation as its origin's at visit time (vm/dump_origin.rs,
  `Dumper::origin_frame`, called from `visit_activation` in plain mode): the frame is
  rebuilt in the origin's layout from its suspension map, so edges, slots and the
  generator see an activation of `f`; `K` and its `$djs_*` locals are simply not
  reached any more.
- Generator: records `cmap` (the rewritten statement's temporaries: K's constant or the
  literal), a finally's completion in its `scaf` record, the for-of scaffold's mark at
  the loop itself (after a label). Writer: `K.$djs.f` is always a function (the run's by
  name, else `$dump.F<act>` from its text); a generator's continuation gets the
  generator object's prototype (`Act::proto`); the activation's deps include its
  function's expression.
- Checks: `plain_dump_chains_continue_like_the_run` now also compares every link's size
  with a dump of the uninterrupted run at that idle point (within 512 bytes; without
  origin tracking the generator and combinators fixtures fail it); plain-node.mjs
  reports a continuation of a continuation (scaffold names renamed by `fresh`).
- Corpus: all 1,197 idle points continue identically in djs, 1,187 links in node, the
  same single refusal; total plain dump text 36.0 MB -> 24.5 MB; the largest drift over a
  chain is state (h232: 18-25 kB with 5 parallel calls), not growth.
- Guide: no page affected.

## Follow-ups
- 0203: refused here: a continuation that stopped again inside its own rewritten
  statement (the `$djs_R(...)` array literal holds the state), a for-in scaffold (the
  origin's for-in iterator is an internal object), a codemode body held by reference.

## Result
Plain dumps of runs that resumed plain dumps are written from the original functions'
texts (D2870): chains stay bounded (each link about the size of a dump of the same state
from the uninterrupted run). No API change; `K.$djs` gained `cmap` entries and finally
records, and `f` is always a function.
