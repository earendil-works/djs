# 0188: Is our continuation-dump suspension optimal? State machines vs suspension maps (research)

status: done
agent: critic
branch: issue/0188-state-machines
depends-on: 0169, 0171, 0172, 0184
guide: -
touches: docs/suspension-design-review.md (report), work/scratch/0188/ (prototypes), work/research/0188/ (prototype kept for reproducibility); no engine changes on main

## Owner
"Investigate if the way we suspend for a continuation dump is optimal. Would it not be saner
to rewrite the pending frame as basically a state machine, similar to how we decompose the
builtins when we transpile to Rust? Investigate and critique."

## Today (read first)
docs/continuation-dumps.md, docs/functions-as-code.md, ARCH.md "Images and dumps",
D2560–D2565, D2570 (suspension maps), D2581–D2585 (loader, $dump.resume), D2610–D2628
(exporter, re-enactment, finally as gosub indexes, using/for-in/... primitives).
A suspended activation is dumped as: the function's source text, the site name
(`await#N`), bindings by name, the operand stack by role (`call.callee`, `call.this`, ...),
loop records, and `$dump.*` primitives that re-enact in-flight builtin state; the loader
recompiles the dump's text, finds the site's resume pc and rebuilds the frame. The
compiler's suspension maps (roles per await) are a pinned contract (D2563).
The alternative to evaluate: bgen's `@native` builtins are compiled into resumable Rust
state machines (D1632); the analogue for user code would be lowering async functions and
generators (all, or those that can suspend at idle points) into explicit state machines
(regenerator/TypeScript-downlevel style: a `switch (state)` over resume points, locals and
spilled temporaries in a state object, try/finally and loops as states), so a suspended
frame is plain data `{state, vars...}` and resuming is calling the machine.

## Questions
1. A fair description and critique of the current design: what is complex (roles, the
   operand stack, re-enactment, finally/gosub, loader primitives, the pin contract), what
   breaks across versions, what it costs (code, size of the dump module, dump/load time),
   where the remaining refusals come from.
2. The state-machine design(s), concretely: where the transform happens (source-to-source
   in the dumper/loader only? in the compiler for every async function? only when a dump
   is taken?), what a dump looks like, how temporaries at an await are named (the transform
   spills them; does that remove the role vocabulary?), how try/catch/finally, loops,
   for-of/for-await iterators, `using`, generators' `yield*`, destructuring and builtin
   in-flight state (Promise.all, iterators) map; source fidelity (would dumps contain
   transformed code? can the original text be kept with a state map?); stack traces and
   `Function.prototype.toString`.
3. Costs and risks: speed of transformed code (wasm instructions on the workloads/corpus if
   all async functions are lowered), code size, compile time, spec conformance (test262
   async/generator suites), determinism, suspend-anywhere (images at any instruction stay),
   cross-version stability (state numbers vs site names), the size of the change.
4. Prototype the strongest variant on a few corpus programs (scratch) far enough to compare
   dump size, load time and code complexity with today.
5. Recommendation: keep, switch, or a hybrid (e.g. transform only at dump time for the
   frames being dumped), with reasons. Be blunt.

## Deliverable
docs/suspension-design-review.md.

## Log
- Read continuation-dumps.md, D2560–D2628, D1632, the smap pins and today's exporter/loader.
- Prototyped variant R (dump-time residual continuations) as an acorn rewriter of today's
  dumps (work/research/0188/residualize.mjs). Harvested idle points: 993/1063 identical,
  0 different, 70 refused (factory placement). v2 programs: 95/177, 0 different.
  Harvested dumps −42% raw / −18% zstd, load 0.48×.
- Chained R (dump a resumed residual again): size doubles per generation (2.3 → 257 KB in
  8 links). R rejected as a format; SM (spill at suspend, named slots) recommended.
- Incident: chain.sh dumped resumed runs at idle point 1, which is the loaded state; the
  exporter writes the identical dump there (fixed point), so the uncapped loop ran 5 h and
  wrote 880,944 files until the coordinator killed it. Now dumps at idle point 2, capped
  at 50 links, wrapped in timeout.sh. Recorded as a finding (idempotence is correct).
- Measured TypeScript down-levelling under djs: 2.4× (es2015) and 5.4× (es5) slower on an
  await-heavy loop; same job order.
- Guide: none affected.

## Result
docs/suspension-design-review.md: switch to **SM**, named-slot suspension states in the
bytecode compiler (spill at suspend; finally completions, iterator records, using,
switch and pattern state in slots named construct@span); no generated code in dumps;
open host calls carry host state with save/resume hooks (build first). It includes an
8-step migration plan, risks and performance opportunities. Proposed decisions
D2720–D2722 are in the report, not yet appended to decisions.md (coordinator's call).

## Follow-ups
- Steps 1–8 of the report's migration plan.
- Anonymous functions written as factory properties get stack name `$fN` after a resume:
  write them as `(0, async () => …)`.
- Unit test: dump → load → dump at the loaded state is byte-identical.
