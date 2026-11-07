# 0243: Synthetic modules, import() through the loader, modules in djs-embed (M4, M5)

status: done
agent: embed
branch: issue/0243-module-loader-import-embed
depends-on: 0240, 0241
touches: crates/djs-core/src/vm (module API), crates/djs-core/src/interp, builtins/import.ts, builtins/module.ts, crates/djs-embed, crates/djs-test262 (dynamic-import tests), tests/test262.toml, docs/guide/rust-api.md, docs/guide/language.md
guide: docs/guide/rust-api.md, docs/guide/language.md

## Goal

M4, M5 and the embedding side of M2/M7 (`~/Development/gondolin/native/docs/djs-hooks.md` §3):

- **M4 synthetic modules.** The host defines a module without source: a key and its exports
  (names and values: host functions, wire-decoded data, anything the embed API can make), as
  ECMA-262's Synthetic Module Records (the JSON modules proposal's shape). Importers link to it
  like any module; its exports are live (the host may set them before evaluation).
- **M5 `import()`** goes through the same loader: in a VM with modules, `import(spec)` (in
  modules and scripts; the referrer is the active module's key or the script's name) becomes an
  async host call the host answers by loading a graph (its loader, sync) and handing back the
  module, or an error; the engine links and evaluates it in a later run and settles the promise
  with the namespace (per EvaluateImportCall / ContinueDynamicImport; errors reject). Without
  the feature D1582's behavior stays.
- **djs-embed**: `Source::module(...)` for the policy entry and a loader trait (resolve/load,
  in-memory module maps); synthetic modules from host functions (an `@sdk/net`-style module whose
  exports are host functions and constant data); calling an exported function of the entry
  module as the async entry point (`onRequest` exported from the policy module); snapshots that
  carry the evaluated graph (M6); host access to exports (M7); dynamic `import()` answered by the
  embed loader automatically in the drive loop.
- test262: the dynamic-import tests that become possible (0242's runner) into active.

## Acceptance criteria
- [x] synthetic modules in djs-core and djs-embed
- [x] import() through the loader, async, with errors rejecting; referrer passed
- [x] djs-embed module entry points and snapshots of evaluated graphs, tested
- [ ] test262 dynamic-import group active as far as supported; ratchet updated (0242 has
  not started: no fixture loader in djs-test262 yet; see Follow-ups)
- [x] guide pages

## Log

### djs-core (66b242ad)
- **Synthetic modules**: `Vm::define_synthetic_module(key, &[(name, Value)])` makes a record
  (interp/module.rs `new_synthetic`) with status EVALUATED, metadata of ENV_VAR entries and
  local exports, and a Record of Boxes as its body (a closure for source modules: the class
  marks it synthetic). Linking, ResolveExport, namespaces and `module_export` work unchanged;
  importers share its Boxes, so `Vm::set_module_export` is live. The loader is never asked for
  a key that exists already (vm/module.rs `module_record`), so resolve maps the specifier to
  the key and that is all.
- **import()**: in embed builds the compiler emits `%ModuleOp(specifier, options, referrer)`
  (`this` undefined; referrer = `opts.filename`, the module key or script name, a string
  constant) instead of `%DynamicImport` (const-cut by `crate::MODULES`: wasm modules
  unchanged). module.ts `ImportCall`: if the VM has no import function (NsOp 4) it is
  D1582's DynamicImport; else the same checks (attributes: any key is a SyntaxError, as for
  static imports), then a call of the hidden async host function (fn id
  `vm::IMPORT_FN_ID = u32::MAX`, args specifier, referrer) and `then(FinishImport)` (NsOp 7:
  a builtin closure of ModuleOp with op 6) which finds the linked record (NsOp 5), runs
  `EvaluateGraph` (ModuleEvaluate split; errors kept per module) and returns the namespace
  (NsOp 6; a namespace exporting `then` is thenable, as in the spec).
  Host side: `Vm::set_dynamic_import(on)` (stored in the Modules record slot 2, so images
  carry it), `import_request(call_id)`, `answer_import(call_id, loader)` (=
  `load_module_from(spec, Some(referrer))` + resolve with the key, or reject with the load
  error), `load_module_from`.
- The `Intrinsic::Modules` Record grew from 2 to 4 slots (import fn, finish closure):
  COMPAT_VERSION 6 → 7.
- tests/smap/e2e/dynamic_import.smap: the await's pc moved (302 → 318) by the referrer push
  (pins are made by embed builds; the dump modules have no embed and compile the old code).
- vm::tests::reset_to_reuses_the_arena was fragile: the base crossed a 64 KiB boundary of
  `image_capacity` with module.ts's growth; the test now grows the arena for real and asserts
  that it did.
- Tests: vm/module_tests.rs synthetic_modules, dynamic_import_through_the_loader (referrers),
  dynamic_import_errors_reject, dynamic_import_off_rejects_as_without_modules,
  dynamic_import_survives_images.

### djs-embed (602c59a8)
- `Source::module` / `typescript_module` (SourceKind Module, TypeScriptModule), `Loader`
  trait (`&self`, Send + Sync, kept by the snapshot), `ModuleMap` (in-memory; `./` `../`
  joined to the referrer's key), `ModuleSource::{js, typescript}`, `SyntheticModule::new(key)
  .host_fn(HostFn).data(name, Data)`, `Builder::module`, `Builder::loader`,
  `Instance::start_export`/`call_export`/`export`, `Error::Load`. Synthetic keys win over
  the loader (exact specifier match). Setup turns dynamic import on, answers imports in
  its run loop; `Instance::step` answers imports itself (loops while only imports were
  waiting). Host fn id u32::MAX is refused (reserved).
- tests/it/modules.rs: the Gondolin policy scenario, stepping, dynamic import in calls,
  scripts importing, TS + custom loader, setup errors.

### Docs, sizes, rebase
- D3235 (djs-core design), D3236 (djs-embed API); index lines under "Embedding in Rust hosts".
- Guide: rust-api.md (new "Modules" section, in-short and setup-failure sentences),
  language.md (synthetic modules and `import()` in "Modules"; the wasm paragraph says a
  native VM without an import-answering host rejects too). Crate docs: djs-embed lib.rs
  "# Modules" with an example that tests/it/example.rs runs.
- Sizes (scripts/build-wasm.sh, after the djs-core change): shipped 426,824 gzip / 980,275
  raw (ceiling 426,865), slim 360,544 (ceiling 360,623): the compiler's module path is
  const-cut, module.ts is @embed-only, so nothing of this reaches the wasm modules.
- Rebased onto main 879ba761 (0244): took main's reset_to test fix (the same fix as mine),
  merged the index lines and language.md's paragraph, regen (registry gained
  %prelude:EvaluateGraph% and %prelude:ImportCall%).
- Checked: djs-core lib, e2e plain + dynamic_import/modules in all modes, djs-embed, clippy
  workspace + wasm32 release leg + djs-core feature combinations, fmt, djs-docs check.

## Follow-ups
- test262 dynamic-import: 0242's fixture loader should call `Vm::set_dynamic_import(true)`
  and answer `Vm::import_request` calls with `Vm::answer_import` (referrer = the test's
  file name, which is what scripts compile with), then move `dynamic-import` out of the skip
  group as far as it passes and update the ratchet.
- Fuel for `import()`: loading and compiling a graph inside `Instance::step` is not metered
  (it runs no JS; the memory cap applies). A host with untrusted large module maps may want a
  bound (e.g. on the source bytes a call may load).
- Synthetic module host functions are not in `Snapshot::host_fns()` (they are in
  `Snapshot::modules()`); a host dispatching by id should list both.
- No top-level await (0248) and no import attributes / JSON modules: `import(x, { with:
  {...} })` with any key rejects with a SyntaxError.
- The dumper knows nothing of module records (unchanged from 0241).

## Result
- **djs-core** (`embed` builds; D3235): `Vm::define_synthetic_module(key, &[(name, value)])`
  and `Vm::set_module_export` (live) for modules without source; `Vm::set_dynamic_import(on)`
  makes `import()` (modules and scripts) an async host call with fn id `vm::IMPORT_FN_ID`
  (args specifier, referrer = module key or script name), answered with
  `Vm::answer_import(call_id, loader)` (`import_request` to recognize one,
  `load_module_from` underneath); the next run evaluates the graph and fulfills with the
  namespace, errors reject (load, link, syntax, evaluation; evaluation errors are kept).
  Off by default: D1582's rejection. Images carry synthetic modules and the setting.
- **djs-embed** (D3236): `Source::module`/`typescript_module`, `Builder::loader(impl Loader)`
  (`ModuleMap` in memory), `Builder::module(SyntheticModule::new("@sdk/net")
  .host_fn(..).data(..))`, `Instance::start_export`/`call_export`/`export`, `import()`
  answered inside `step`, `Error::Load`. Snapshots hold the evaluated graph.
- Gaps: test262 dynamic-import not activated (needs 0242's loader), no TLA, no JSON
  modules, import loads unmetered by fuel.
