# 0241: ES modules: module goal, loader, linking and evaluation (M1–M3, M6, M7)

status: open
agent: -
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
- [ ] module goal compiles every module syntax form; early errors (duplicate exports, `export`
      of an undeclared name, `import` bindings assigned, `await`/`yield` rules, ...)
- [ ] loader API on `Vm`, graph loading, linking with link-time SyntaxErrors
- [ ] evaluation in the builtin language, stackless (a module body can suspend on a sync host
      call and run out of fuel mid-evaluation), errors cached per module
- [ ] live bindings, cycles (including a function exported from a module whose body has not run
      yet), namespaces exotic, import.meta.url
- [ ] graphs survive `collect`, image round trips and the suspension/GC stress modes the e2e
      tests have
- [ ] host reads of exports
- [ ] unit tests + e2e tests (crates/djs-core/tests); the existing gate legs green; the shipped
      wasm module's size within its ceiling
- [ ] docs/compiler.md section on the module goal; ARCH.md non-goals and "Semantics stance"
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

## Follow-ups

## Result
