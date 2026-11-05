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

- 2026-11-06 (lazy): batch 1 merged (main efe2316c). Batch 2: the test262 leg also runs
  eager, in parallel; Script source as UTF-8 bytes (D2992, COMPAT_VERSION 3; lazy compiles
  copy only the span, toString converts only the function); the dumper reads lazy
  templates (compiled ones through `IC`, unrun ones through the script's `free_names`,
  D2993); djs-wasm's `dump` sets `lazy_functions` (djs-difftest's `dump` now turns on
  djs-wasm's too, so djs-abidrive matches the dev dump module); the plain dumps leg runs
  `--lazy`. Checked: corpus plain dumps lazy = eager (only clock values differ), plain-node
  on the lazy dumps, native-vs-wasm images, js tests, `scripts/pi-dump-every-idle.sh` on
  the release dump module (0 new failures), test262 both modes.
- 2026-11-06 (lazy): batch 2 merged (main 76623ae8). Batch 3 (coordinator: pjr/pjs stay
  out of `Vm::compile`, `dump_cache`, `script_maps`, map lookups): lazy plain-dump loads with
  the load's maps as a partial cache entry, others on demand (`compile_lazy_with_maps`,
  `lazy_function_maps`, `LazyIndex`, `Dumper::creator_maps`), D2994; globals of lazy
  functions recorded at every lazy compile (`compile_lazy_with_globals`, `LazyGlobals`),
  the dumper's per-function compile only as a fallback; eager rules for call arguments,
  parenthesized functions, async functions and generators, eager dump runtime, D2995.
  `vm::lazy_tests::lazy_maps_are_eager_maps` (1.4k maps). Corpus plain dumps lazy = eager
  (text identical but clock values), plain-node OK, pi-dump-every-idle OK, test262 both.
  Numbers in work/research/0218/README.md "Batch 3 results": corpus load + re-dump -3.6%,
  pi load/dump -4/-5%, records/log/binary within noise.
- 2026-11-06 (lazy): batch 3 merged (main 03d541f2). Batch 4 measurements: no phase-1
  emission in lazy subtrees skipped (phase-1 encode is 3.2% of a lazy corpus load in all,
  parse 49%, resolve 34%; below the 3-5% bar). The +4% on running the codemode cases
  lazily is the harness compiling the corpus prelude.js per VM (80% of runtime lazy
  compiles); pi seals its prelude (compiled once): with the prelude eager, lazy runs are
  -1.7% (1,852 vs 1,884 M). A size threshold (512-1024 B) trades the load gain away; none
  kept. Compile cache across loads (blob of compiled cells, import with atom remapping)
  estimated and proposed to the coordinator (~-20-25% of pi's resumeSession).

## Follow-ups

- Eval code and `new Function` still compile eagerly (builtins/eval.rs has only the heap;
  it would need the VM's option).
- Continuations in plain dumps are function expressions assigned and called later
  (`$dump.Ka5 = async function …; $dump.Ka5.call(…)`); async keeps them eager now, but a
  sync continuation or callback the writer parenthesized would be eager by D2995 (pjs).
- Batch 4: small-function threshold, `fclosure` taking the compiled template, phase-1
  emission off inside lazy subtrees.
- The decisions index (docs/decisions-index.md) needs D2990/D2991 (coordinator).

## Result
