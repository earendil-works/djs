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
- 2026-11-06 (lazy): approved: feature `lazy` (native + dump module, off in main/slim),
  image-neutral (images stay in one process and module, D2560), seal compiles all lazy
  templates; gate's stressed difftest and test262 legs run `--lazy`.
- 2026-11-06 (lazy): batch 1 built. `compiler/lazy.rs` (`compile_lazy`), `resolve::Unit`
  / `Plan` / `lazy_roots` / `emit_lazy`, the `lazy` op (`interp/run/lazy.rs`),
  `VmOptions::lazy_functions` (opt-in), `Vm::compile_lazy_templates` (from `seal`),
  `--lazy` in djs-difftest and djs-test262 (their new `lazy` features), D2990, D2991,
  guide compiler.md "Lazy functions", TESTING.md. Lazy record: the template's line table
  (pc 0 = the function's start; pc 1/2 carry the head offset and context bits as columns).
  Capture attributes in bits 16-17 of the descriptors (`closure` masks them). Pinned
  smaps changed (4 harvest cases: pool split moved pcs), accepted.
  Size: non-lazy builds were +1.1 KB gz at first (pool/emit restructuring inlined into
  `finish_unit`); all lazy-only code now behind cfg: slim +43 B gz (378,129 / ceiling
  378,201), main +145 B (442,678 / 442,831).
  Found: djs-test262/djs-difftest depend on djs-core without default features, so they
  needed their own `lazy` feature (my first `--lazy` runs were eager).
  Tests: vm::lazy_tests (results eager = lazy, early errors at load, identity of 3.4k
  lazily compiled functions of the corpus and golden dumps, seal); test262 active in both
  modes; difftest `--lazy` whole corpus, and stressed (suspend 97/1009, gc, determinism)
  on codemode, codemode-harvest, language, dquickts; `--plain-dumps --lazy` on codemode.
  Measured (native release, load ~10-20): typescript.js 183-200 -> 116 ms, golden dumps
  9.3 -> 6.2-6.9 ms, dump runtime 0.26 -> 0.155 ms, harvest 13.2-14.0 -> 11.6-12.5 ms.
  Known gap for batch 2: `compile_lazy` converts the whole Script source to UTF-8 per
  compile (O(script) per first call); the dumper does not read lazy templates yet, so
  `lazy_functions` stays off by default.

## Follow-ups

- Batch 2: Script source as UTF-8 (O(span) lazy compiles, toString); the dumper reads lazy
  templates through scratch compiles (`global_refs`, `code_globals`, harvest); lazy on by
  default in lazy builds; eval and `new Function` lazily.
- Batch 3 (with pjr, pjs, through the coordinator): lazy plain-dump loads, maps on demand.
- Batch 4: small-function threshold, `fclosure` taking the compiled template, phase-1
  emission off inside lazy subtrees.
- The decisions index (docs/decisions-index.md) needs D2990/D2991 (coordinator).

## Result
