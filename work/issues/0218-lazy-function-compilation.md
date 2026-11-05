# 0218: Lazy function compilation: compile a function's body on its first call

status: in-progress
agent: lazy
branch: issue/0218-lazy-function-compilation
depends-on: -
guide: compiler.md (and any page describing when code is compiled)
touches: crates/djs-core/src/compiler/, function templates/closure creation in vm/heap, tests; design first

## Goal
0215 (pjr) measured that compiling the dump Script is ~85% of loading a plain dump, and most
function bodies in a dump never run (closures kept for later, origins only used by re-dumps,
helpers). Codemode scripts have the same shape. Make function compilation lazy: the Script's
compile parses everything (all early errors still reported at load, as the spec requires) and
resolves scopes, but generates bytecode for a function body only when the function is first
called (or when something needs its template: toString needs only source text; re-dump needs
suspension maps of activations, which are running and therefore compiled).

Design first (work/research/0218/README.md, sent to the coordinator before building):
- what a lazy template holds (source span, scope/resolution info for its free variables so
  that captures are known at closure creation, flags), and what full compilation needs later;
- whether to re-parse the body on first call (V8-style preparse + full parse) or keep the AST /
  a compact pre-parsed form; memory and time trade-offs, in no_std/wasm;
- interaction with closures and captures (cells, boxes, scope analysis that today needs inner
  functions' bodies), class bodies, generators/async, eval/with, TypeScript stripping, source
  maps / suspension maps (smap), images/snapshots (a lazy template in a base image?), the GC,
  and the deterministic compile caches;
- expected wins: corpus load, pi codemode run start, dump load; measure with
  scripts/dump-costs/ and the benches; size impact on the shipped modules (ceilings are tight:
  main ~320 B, slim ~110 B headroom; a trade may need a decision).
Then implement in batched merges behind the gate, with test262 and the difftest as the safety
net. Coordinate with pjr (0215) who measures load, and pjs (0207) who changes dump text.

## Log

- 2026-11-06 (lazy): design in work/research/0218/README.md, sent to the coordinator
  before building. Profiled `compile` (typescript.js: parse 38%, `resolve::finish` 60%)
  and prototyped deferring phase 2 for lazy-eligible functions (patch in
  work/scratch/0218/proto.patch, reverted): typescript.js -40%, golden dumps -26%, dump
  runtime -35%, codemode harvest -10%. Proposal: full parse at load (all early errors),
  lazy template + `lazy` trampoline op, re-parse of the span on first call with a
  synthetic outer for captures, pools split along lazy roots so eager and lazy code are
  identical (maps), cargo feature `lazy` on in native/dump builds and off in main/slim
  (size). Waiting for the coordinator's go.

## Follow-ups

## Result
