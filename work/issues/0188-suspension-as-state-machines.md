# 0188: Is our continuation-dump suspension optimal? State machines vs suspension maps (research)

status: in-progress
agent: critic
branch: issue/0188-state-machines
depends-on: 0169, 0171, 0172, 0184
guide: -
touches: docs/suspension-design-review.md (report), work/scratch/0188/ (prototypes); no engine changes on main

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

## Result
