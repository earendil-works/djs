# 0241: ES modules: module goal, loader, linking and evaluation (M1–M3, M6, M7)

status: done
agent: modules
branch: issue/0241-es-modules
depends-on: -
touches: crates/djs-core/src/compiler (module goal; compiler/module.rs is strip-types' module syntax today), crates/djs-core/src/interp (module records, environments, namespace objects, intrinsics), crates/djs-core/src/heap (new cell kinds, the namespace object class), crates/djs-core/src/vm (a module API, new file), builtins/ (module evaluation in the builtin language; import.ts), crates/djs-core/Cargo.toml (feature `modules`), crates/djs-prelude/Cargo.toml (feature pass-through), crates/djs-core/compat_inputs.rs, tests (unit and e2e), docs/compiler.md, ARCH.md (modules no longer a non-goal)
guide: docs/guide/language.md (the "ES modules" row and the import() example), docs/guide/compiler.md if it says there are no modules

## Goal

D3171. Requirements M1–M3, M6, M7 of `~/Development/gondolin/native/docs/djs-hooks.md` §3
(read it): policies written as modules that import host-provided libraries, compiled once into a
snapshot.

- **M1** a module goal (`Goal::Module`, `CompileOptions::module(name)`): `import`/`export` in all
  forms (default, named, namespace, re-exports, `export * as ns`, `export { x as default }`,
  string export names, `export default` expressions/functions/classes), always strict, top-level
  `this` undefined, `import.meta`. TypeScript modules too (`typescript: true`): type-only imports
  and exports erased (compiler/module.rs already parses these for strip-types).
- **M2** the host loader: a `Vm` API that loads a module graph through a host callback,
  synchronously, with no I/O in the engine: `resolve(specifier, referrer_key) -> Result<key,
  message>` and `load(key) -> Result<source, message>` (or one callback; your call, record it).
  Keys are opaque strings; a key already in the VM's module registry is not loaded again. Syntax
  errors and loader errors are reported as JS errors (SyntaxError / the host's message).
- **M3** linking and evaluation per spec (ECMA-262 16.2 Cyclic Module Records, without top-level
  await, which is 0248): live bindings, cycles, namespace objects (the exotic object of 10.4.6:
  null prototype, `@@toStringTag` "Module", sorted keys, non-extensible, writable+enumerable+
  non-configurable data properties that read through to the binding, TDZ ReferenceErrors),
  ResolveExport with star exports and ambiguity, `import.meta` (an object with at least
  `url` = the module's key, made once per module), link errors (a missing or ambiguous export is a
  SyntaxError before anything runs), evaluation errors recorded per module and rethrown on later
  evaluations.
- **M6** module records, environments and namespaces are heap cells reachable from the realm, so
  `collect`/`image()`/`from_bytes` carry linked and evaluated graphs; test that an image taken
  after evaluation restores and its exports still work (and live bindings stay live).
- **M7** host access: the namespace of a module and one export's current value by name, from Rust.

All of it behind the djs-core feature `modules` (default on, passed through djs-prelude like
`full`; off in djs-wasm), cut by a constant like `FULL` (D2411) so both builds type-check the
same code; without it `CompileOptions::module` compiles to a SyntaxError "modules are not
available in this build". Classify the feature in compat_inputs.rs (D2267).

## Acceptance criteria
- [x] module goal compiles every module syntax form; early errors (duplicate exports, `export`
      of an undeclared name, `import` bindings assigned, `await`/`yield` rules, ...)
- [x] loader API on `Vm`, graph loading, linking with link-time SyntaxErrors
- [x] evaluation in the builtin language, stackless (a module body can suspend on a sync host
      call and run out of fuel mid-evaluation), errors cached per module
- [x] live bindings, cycles (including a function exported from a module whose body has not run
      yet), namespaces exotic, import.meta.url
- [x] graphs survive `collect`, image round trips and the suspension/GC stress modes the e2e
      tests have
- [x] host reads of exports
- [x] unit tests + e2e tests (crates/djs-core/tests); the existing gate legs green; the shipped
      wasm module's size within its ceiling
- [x] docs/compiler.md section on the module goal; ARCH.md non-goals and "Semantics stance"
      updated; the guide's language page

## Notes (a suggested design; deviate where the code says otherwise, and record it)
- docs/compiler.md §5 (scopes, resolution pass, captures, hoisting) and §6.3 (closures: a
  function object's SLOTS hold its template and the Box refs of its captures) are the base.
- **Environments as boxes.** Make every top-level binding of a module (var, let, const, class,
  function declarations, imports, plus hidden ones such as the `default` export's
  `*default*`) a Box, created by the *linker*, not by the module function's prologue: link time
  needs them before any module body runs (cycles: InitializeEnvironment). One way: the module
  function receives its environment's boxes as arguments, and its prologue copies them into the
  binding slots instead of `new_box`; an import's box *is* the exporter's box (live bindings for
  free), and imports compile as const bindings (assignment throws). The template records, per
  environment entry, its name, kind and slot.
- **Hoisted functions at link time.** Top-level function declarations must exist before any
  body runs (a cycle can call them). The linker can instantiate their templates directly: their
  captures are all module-top-level bindings, i.e. environment boxes it holds. The module body
  then skips their hoisting.
- **Records.** A ModuleRecord cell (key, status, template or null, environment, requested
  specifiers and resolved records, import/export entries, namespace, error, DFS index and
  ancestor index for Tarjan, import.meta) in a registry reachable from the realm/roots. ResolveExport
  and link are leaf work (no user code) and can be Rust; mind recursion depth (explicit stack).
- **Evaluation** (InnerModuleEvaluation) calls module bodies, so it is in the builtin language
  (builtins/module.ts, `%Module*` intrinsics for the record bookkeeping). It returns a promise
  (spec `Evaluate()`); with no TLA it is settled when the run is done.
- `import()` keeps D1582's behavior in this issue (0243 routes it through the loader); a module's
  `import()` gets the referrer later, so leave room for it.
- Namespace objects: look at how the existing exotic objects (Proxy, String, arguments, typed
  arrays) hook into property access in heap/object.rs and the interpreter's fast paths;
  module namespaces must never take an inline cache or shape path that skips the exotic methods.
- test262's module tests come in 0242; use them locally early (`djs-test262 run --group skip
  language/module-code` once a runner can load fixtures) if it helps, but don't build the runner
  support here unless it is small.
- qjs can run modules (`~/Development/dquickts/qjs -m file.mjs`) as an oracle for behavior
  questions.
- Size: the shipped module must not grow beyond noise (the Rust side is cut by the feature; the
  prelude's new builtins are in the base image: keep them small, measure with
  scripts/build-wasm.sh and scripts/size/budget.py).

## Log

### Plan (modules agent)
Design (deviations from the Notes recorded as D3190+):
- **Compiler.** `Goal::Module` (bits goal 3), `FuncKind::Module` (top level, KIND_SCRIPT
  template), parser flag `module`: `await` reserved everywhere, top-level function
  declarations lexical (duplicates and var conflicts are errors), `return`/`new.target`
  errors. compiler/module.rs parses import/export for real when `module` (strip-types keeps
  its path) into a `ModuleInfo` (requests, imports, local/indirect/star exports).
  `compile_module(heap, src, opts) -> (template, meta)`; `meta` is a U32Array of atoms and
  ints (atoms never die).
- **Environment = the module closure's captures.** Every body-scope binding of the module
  function (var/let/const/class/function/import, hidden `*default*`, `*meta*` for
  import.meta) gets `Storage::Env(i)`: accessed with `get_cap/put_cap` (TDZ elision as for
  boxes), and inner functions capture it as `Capture::Outer(i)`. The body's prologue makes
  no boxes and does not hoist top-level functions. The linker (Rust) creates the closure
  of the module template and fills capture i with a Box: a fresh one (var undefined,
  lexical uninitialized, function = closure instantiated by the linker from the child
  template, whose captures are all `Outer(env)`), or the exporter's Box for an import
  (live bindings), a Box holding the namespace / the import.meta object.
- **Records** are `ObjectClass::Record` objects (slots: key, status, body closure,
  requested records, error, dfs index/ancestor, namespace, meta U32Array, ...), in a
  registry object (key atom -> record) in a new realm intrinsic slot: images carry them.
- **Linking** (ResolveExport, InitializeEnvironment) in Rust (leaf), explicit stacks.
- **Evaluation** in builtins/module.ts (`%ModuleEvaluate`, InnerModuleEvaluation without
  TLA, errors cached), called by `Vm::evaluate_module` via the installed intrinsic.
- **Namespaces** are Proxy objects (target: null-proto non-extensible object with a
  writable non-configurable data property per export + @@toStringTag; handler: a Record
  whose prototype holds builtin traps get/set/getOwnPropertyDescriptor/defineProperty/
  ownKeys over leaf intrinsics): every property path already sends proxies to the
  traps, so no IC or shape path can skip them, and no new exotic class is needed.
- **TS modules**: imports never referenced as values are dropped (type imports), local
  `export {T}` of a non-value name is dropped.
- Stages: (1) compiler, (2) records/loader/link, (3) evaluation/namespaces/meta,
  (4) images/stress/docs.

### Progress
- Stage 1 done (482fe2db): compiler/module.rs (real module items), module_meta.rs
  (metadata words), `Storage::Env` in resolve.rs, snapshot `snapshots/modules.js`,
  `compiler::tests::module_errors`. `modules` is a code-only feature (build.rs hash
  filter + compat_inputs neutral list): no image or base depends on it.
- Stages 2+3 done (a974bfe2): interp/module.rs (records, registry in
  `Intrinsic::Modules`, link, ResolveExport, namespaces as Proxies, the namespace
  leaves), vm/module.rs (`ModuleLoader`, `ModuleSource`, `Vm::load_module`,
  `evaluate_module`, `module_namespace`, `module_export`, `has_module`),
  builtins/module.ts (`ModuleEvaluate`, traps; @slim-omit), Exec::closure split into
  `run::alloc_closure_of` + `run::function_properties` (free fns). vm/module_tests.rs
  covers live bindings, namespaces, cycles, link/load errors, cached errors,
  import.meta, TS type imports, images and suspension.
- e2e: `// mode: module` cases (tests/data/e2e/modules.js, modules-errors.js; files split
  by `// file:` lines; gen-expected.sh runs them in qjs through a dynamic import, CASES=
  regenerates single cases). All four modes pass (dump mode skips the dump round trip).
- Size (D3190): the module builtins cost ~290 B gzip in the base, so `modules` became an
  image feature: bgen writes builtins/gen/prelude-nomodules.js (without files tagged
  `// @modules-only`), which full builds without modules (the shipped module) use; the
  slim prelude leaves them out too. Rust paths are cut by `crate::MODULES`; the parser's
  module state is a `ModuleSlot` (unit without the feature), the env index lives in
  `Var::param` (+1), one leaf `NsOp` and one builtin `ModuleOp` (evaluate + the five
  namespace traps as builtin closures with the op as data). Measured vs main 4771aa63:
  shipped 426,705 -> 426,760 gzip (ceiling 426,806), slim 360,319 -> 360,458 (ceiling
  360,485). gzip moves +-25 B with trivial edits: keep new live code behind MODULES.
- Gotcha: compiler/lazy.rs numbers FuncKinds (`KINDS`): new kinds go last.

### Blocked: the size ceilings (needs the coordinator)
Everything is implemented, tested and rebased onto main 017ae943 (regen clean); the merge
gate failed only on `size budget`. Measured (scripts/build-wasm.sh, main 017ae943 vs the
branch): shipped 426,780 -> 426,840 gzip (+60; raw 980,184 -> 980,274, +90), ceiling
426,806; slim 360,398 -> 360,533 (+135; raw 851,767 -> 851,876, +109), ceiling 360,485.
Main itself is 26 bytes under its ceiling (it grew 75 bytes since 4771aa63). What is left
of 0241 in the wasm modules is what no constant can cut: two INTRINSICS entries (`ModuleOp`,
`NsOp`) and their installed-array slots in the base, the realm's `Intrinsic::Modules`
slot, and enum arms (`Goal::Module`, `FuncKind::Module`, `VarKind::Import`,
`Storage::Env`) in compiler matches; everything else is behind `crate::MODULES`, and the
module builtins are out of both wasm preludes. gzip moves +-25 bytes with trivial edits
(measured several times). Tried and dropped: keeping the module state in the installed
array instead of a realm slot (bigger, and broke an e2e image check).
Proposed decision (as D3164 did): "D3192: Size of 0241; the size ceilings rise by what it
measured": `ceiling_gzip` 426,806 -> 426,840 and `slim_ceiling_gzip` 360,485 -> 360,533 in
tests/scores/size.tsv (plus the header line "raised by 0241, D3192"). With it, rerun
scripts/merge-to-main.sh from the worktree.

- Assigning an import binding is a TypeError at run time (as for `const`), not an early
  error: the spec has no early error for it (SetMutableBinding of an immutable binding).
- Guide: language.md (new "Modules" section, the in-short paragraph and the left-out
  table); compiler.md (guide) did not mention modules. docs/compiler.md 5.11, ARCH.md.

### Continuation: modules fold into `embed` (D3175)
- Rebased onto main 7237378f. The `modules` feature is gone: djs-core's `embed` is the image
  feature (IMAGE_FEATURES bit 3, in the build hash; `embed = ["djs-prelude/embed"]`), the
  constant `crate::EMBED` with `crate::MODULES` as its alias, `// @embed-only` and
  `builtins/gen/prelude-noembed.js` (now also hashed by build.rs, which the old
  prelude-nomodules.js was not). `embed` without `full` is a compile_error (lib.rs). D3193.
- djs-embed's base and djs-test262 (default feature `embed`) now have modules; djs-difftest
  builds djs-test262 without default features, so it stays without `embed`.
- Found: djs-embed's `the_memory_cap_cannot_be_caught` failed once the base had the module
  builtins: after the cap's OOM abort, the next call hit the cap again (no collection due).
  Fix in djs-embed (outside `touches:`, minimal): `fail` collects after an OOM abort; new
  test over five caps. D3194.
- Sizes re-measured (scripts/build-wasm.sh): shipped 426,839 gzip (+59 vs main), slim
  360,536 (+138). Ceilings raised by exactly that (D3192, approved by D3175): 426,865 and
  360,623.
- Checked: djs-core lib tests, e2e plain, clippy workspace + the gate's djs-core feature
  combinations (+ full,embed), bgen --check, djs-prelude/djs-embed/djs-bgen/djs-wasm/
  djs-cli tests, slim tests and slim bins, test262 language/ + built-ins/{Object,Proxy,
  Reflect} (ratchet OK), djs-docs check.
- Guide: language.md (modules come with the native build for Rust hosts), compiler.md,
  goals.md and typescript.md no longer say there are no ES modules at all.

## Follow-ups
- 0242: test262 module tests need a fixture loader in djs-test262 (`ModuleLoader` over the
  test's directory; `flags: [module]` compile with `Vm::load_module`); djs-test262 has
  `embed` (modules) by default now, djs-difftest does not.
- docs/guide/rust-api.md does not describe djs-core's module API (`Vm::load_module` etc.);
  djs-embed has no module API yet.
- HTML-like comments (`<!--`, `-->`) are still accepted in module code (the lexer has no
  goal flag); test262 checks it.
- Import attributes: any `with { ... }` key is a SyntaxError (no JSON modules yet, M8).
- `import()` from module code still rejects (D1582); 0243 routes it through the loader.
- The dumper (vm/dump.rs) knows nothing of module records; e2e's dump mode skips module
  cases' dump round trip.
- Namespace property access goes through Proxy traps (a builtin call per `ns.x`); named
  imports are direct capture reads. Worth a fast path only if hosts use namespaces hot.

## Result
ES modules in djs-core behind the `embed` feature (D3175, D3193: on in native default
builds, djs-prelude's defaults, djs-embed and djs-test262; off in every djs-wasm module and
in djs-difftest), D3190/D3191; size ceilings raised by the residual, D3192; djs-embed
collects after an out-of-memory abort, D3194.

- Compile: `compiler::compile_module(heap, src, &CompileOptions::module(key))` →
  (template, metadata); `compile()` refuses `Goal::Module`.
- Host API (`djs_core::vm`): implement `ModuleLoader { resolve(specifier, referrer) ->
  Result<key, msg>, load(key) -> Result<ModuleSource { text, typescript }, msg> }`;
  `vm.load_module(specifier, &mut loader) -> Result<key, error Value>` loads, compiles and
  links the graph; `vm.evaluate_module(key)` starts the evaluation (run until `Done(p)`; `p`
  is the Evaluate promise); `vm.module_export(key, name)`, `vm.module_namespace(key)`,
  `vm.has_module(key)`.
- Semantics: every import/export form, live bindings, cycles (hoisted functions exist at
  link time), Proxy-based namespaces with the 10.4.6 behavior, import.meta (`url` = key),
  link-time SyntaxErrors, evaluation errors kept per module; graphs survive collections,
  images and the e2e stress modes.
- Known gaps: no top-level await (0248), no import attributes/JSON modules, `import()`
  unchanged (0243), HTML comments not rejected in module code, no dumper support.
