# Source modules for Rust hosts

djs's native Rust API executes an acyclic subset of JavaScript source modules.
This is an incremental implementation of ES modules, not full ESM conformance or
a complete replacement for QuickJS in Rust pi.

## Loading and execution

`djs_core::vm::modules::ModuleGraph` owns a module cache and binding environments.
Implement `ModuleLoader` to resolve static specifiers into canonical identities and
load UTF-8 JavaScript. The core has no filesystem, package or URL policy.
`load_source` compiles and links the entire dependency graph before any body runs.
Modules already registered through the native binding API can supply synthetic
host exports without a loader call.

```rust
use djs_core::value::Value;
use djs_core::vm::{Status, Vm, VmOptions};
use djs_core::vm::modules::{ModuleGraph, ModuleLoader, SourceError};

struct Loader;
impl ModuleLoader for Loader {
    fn resolve(&mut self, _: &str, specifier: &str) -> Result<String, SourceError> {
        Ok(specifier.trim_start_matches("./").into())
    }
    fn load(&mut self, id: &str) -> Result<String, SourceError> {
        match id {
            "counter.js" => Ok("export let count = 0; export function inc() { count++; }".into()),
            "main.js" => Ok("import {count, inc} from './counter.js'; inc(); export {count};".into()),
            _ => Err(SourceError::Loader(format!("unknown module {id}"))),
        }
    }
}

# fn main() -> Result<(), SourceError> {
let mut vm = Vm::new(VmOptions::default())?;
let graph = ModuleGraph::new(&mut vm)?;
graph.load_source(&mut vm, "main.js", &mut Loader)?;
graph.begin_evaluation(&mut vm, "main.js")?;
loop {
    match graph.poll_evaluation(&mut vm, 10_000)? {
        Status::OutOfFuel => continue,
        Status::Done(_) => break,
        other => return Err(SourceError::Loader(format!("module stopped: {other:?}"))),
    }
}
assert_eq!(graph.read_export(&mut vm, "main.js", "count")?, Value::int(1));
graph.release(&mut vm);
# Ok(())
# }
```

For host calls, handle `Status::HostCall` and `Status::Idle` through the existing
VM APIs, deliver results and poll again. `OutOfFuel` also yields between completed
dependency bodies. Do not run unrelated VM calls during a graph evaluation.
A successful evaluation is cached, and later evaluations reuse it. A thrown value
is retained in the arena and returned again without rerunning the failed body.
Load/link failures are cached separately. A new canonical identity or graph is
needed to retry changed source. Host allocator failure abandons execution and
prevents a partially executed module from being restarted.

## Supported source semantics

- Side-effect imports, default imports, named imports and aliases, including mixed
  default/named imports.
- Exported `let`, `const`, `var`, functions, classes and destructured declarations;
  local export lists and aliases; default expressions, functions and classes;
  explicit named reexports from another module.
- Live binding reads in both module bodies and nested functions. Imported bindings
  cannot be assigned or initialized by the importer. Local constants reject writes,
  and uninitialized lexical bindings throw a ReferenceError, including for `typeof`.
- Within-module function hoisting and `var` initialization to undefined. Modules
  have private top-level declarations, strict semantics and top-level `this` equal
  to undefined, including through arrows.
- Host-controlled canonical identities, dependency-order execution, once-only
  loading/evaluation, synthetic exports, fuel suspension, parked host calls, GC and
  snapshot restoration during evaluation.

The compiler parses import/export declarations with its token machinery. It
allocates shared arena binding records and resolves references to them, including
references from nested functions. Two bytecode operations read and write those
records. There is no source-text rewriting or copying imported values into locals.
Default expressions evaluate to a value once, as JavaScript requires; `export {x
as default}` exports the live binding instead.

## Arena ownership and snapshots

Module records, binding cells, body functions, dependency identities, evaluation
states, the evaluation queue/cursor and cached exceptions are ordinary arena
objects rooted by one host handle. Compiled function constant pools retain the
binding records they reference. Temporary loading and parsing vectors disappear
when loading returns. The graph adds no dependency or cell layout.

Save `graph.handle_index()` alongside the VM image. Restore that image with
`Vm::from_bytes`, then reattach with `ModuleGraph::from_handle_index(index)`.
Release the root once, and do not reuse the index afterward. Returned export
values are raw VM values; give them their own handles before running or collecting
the VM. Internal graph objects are never exposed to JavaScript.

The native `declare` / `declare_binding` / `export` / `import` / `initialize` /
`set` APIs remain available for host-created bindings. Declare local bindings and
exports before linking a native cyclic graph. Synthetic values can include existing
host function and promise values. Native cyclic cells do not imply cyclic source
evaluation support.

## Remaining requirements

- Graph-wide declaration instantiation, including function export initialization
  during linking rather than on entry to each body, and cyclic source linking. Source
  cycles are explicitly rejected before execution, rather than evaluated in an
  incorrect order.
- Namespace imports/exports, namespace exotic objects, export-star resolution and
  ambiguity checks. Namespace imports and star exports currently fail compilation.
- `import.meta`, dynamic import through the host loader, top-level await and async
  module graph evaluation. `import.meta` and top-level await are rejected; dynamic
  import retains the existing no-loader rejection even when a static graph is used.
- Import attributes, source-phase/deferred imports, TypeScript source-module grammar,
  asynchronous loading, module names with lone UTF-16 surrogates, a wasm module-loading
  ABI and a module-aware test262 runner.
- Full module grammar/early-error conformance, broader module test262 coverage and
  pi-rs replacement integration.

Codemode and ordinary Script compilation keep their existing behavior. Static
source modules use the dedicated graph API. The supported subset is substantial
enough to execute ordinary acyclic source graphs, but the remaining requirements
still block a completeness claim and the Rust pi replacement PR.
