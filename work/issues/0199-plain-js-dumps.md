# 0199: Dumps that are plain executable JavaScript (R with origin tracking), prototype

status: open
agent: -
branch: issue/0199-plain-js-dumps
depends-on: 0188, 0196
guide: -
touches: work/research/0199/ (the prototype, measurements), docs/plain-js-dumps.md (report); no engine changes on main unless a small, approved hook is needed

## Owner
After seeing a format 2 dump: "It's no longer valid JavaScript. It requires quite a bit of
machinery in the interpreter for this to work. That invalidates the whole goal that it's
just executable JavaScript." Today a format 2 dump needs Vm::load_dump and privileged
`$dump` primitives: `$dump.resume(fn, {at, slots})` builds a frame at a compiled resume pc,
and `$dump.cell/advance/forIn/using/private/...` exist only during a load.

## Goal
Prototype dumps that **run as ordinary JavaScript**, given only the host functions (tool
calls etc.): no privileged primitives, no engine support for resuming frames. Approach:
critic's variant R (docs/suspension-design-review.md §3 "R"; prototype in
work/research/0188/residualize.mjs): every active (suspended) frame is written as a
**continuation function**, the rest of the original function from its await, with live
values as parameters/locals; nested functions and everything else are copied verbatim.
R's blocker was growth on re-dump (a continuation of a continuation; 2.3 KB → 257 KB over 8
dumps). Fix it with **origin tracking**: each continuation records which original function
and which original state it stands for, and the exporter always regenerates from the
**original** text, mapping the continuation's current await back to the original state, so
dump size stays bounded across any number of re-dumps.

## Questions / acceptance
- Over every corpus idle point (harvested 1,164; tests/dump/v2 programs; e2e dump mode
  where applicable), a dump: (a) runs as a plain Script in djs **and in node** (with the
  host functions provided by a small harness), (b) continues identically to the
  uninterrupted run, (c) re-dumps with bounded size over long chains (cap every loop;
  50 links max, timeouts around whole scripts).
- What still needs a helper and why (TDZ cells shared with closures, in-flight builtin
  state: Promise.all, iterators, for-in, using, private names, generators' queues): find
  plain-JS encodings (factory shapes, public calls) or list each as a refusal with counts.
- Fix R's 70 refusals (variables shared across factories) or explain them.
- Where the transform runs: in the exporter (Rust, reusing the compiler's slot names from
  SM for live values) vs a JS-side prototype first; recommend.
- Sizes and load/resume times vs today's format 2 (scripts/dump-costs).
- A recommendation: switch the format to plain-JS dumps (and what can be deleted: the
  loader primitives, $dump.resume, the recording compile), or not, with reasons.

## Deliverable
docs/plain-js-dumps.md plus the prototype under work/research/0199/.

## Log

## Result
