# 0167: Persist functions and their closures as readable code (design + prototype)

status: in-progress
agent: portable
branch: issue/0167-functions-as-code
depends-on: 0141, 0142, 0145, 0160
guide: - (later)
touches: docs/functions-as-code.md (design), scripts/snapshot-research/portable (extend 0141's crate) or a new scratch crate, work/scratch/0167/; no engine changes on main

## Owner
A harness builds functions, snapshots them and reuses them: "self-extending code, or using
one iteration to write some code to use again later, even after weeks (where heap snapshots
are potentially no longer usable)". Closures capture "stuff that codemode pulled usually,
but this can potentially be binary data too, in buffers". The persisted code should be
visible (readable). Whether persisting captures current state or history: undecided.

## Design to work out (defaults, challenge them)
- **Durable form = code + data**, independent of engine internals (survives upgrades):
  selected globals (functions, classes, closures, plus the data they reach) exported as a
  readable JS module: function bodies from their source (0141: code by source span),
  captured bindings rewritten to environment objects (`env$1.count`), shared captures share
  one env object, object identity kept within the exported graph, an env/data section
  emitted as literals.
- **Binary data**: ArrayBuffer/typed arrays/DataView in captured state, emitted readably
  (base64 literal + a tiny decode helper, or a side blob for large buffers referenced by
  hash). Note the conflict with the slim build (0160 drops typed arrays): say what that
  means for pi (keep typed arrays in pi's build, or represent binary another way).
- **Current state by default** (the export captures the values at export time); keep the
  defining script(s) as provenance in a header comment or alongside, so history can be
  replayed if wanted. Show both options with their costs.
- **Images as the fast path**: the same-build reuse stays an image restore (~1 ms); the
  code export is the durable/inspectable form, re-imported when the image is refused
  (dependency descriptor changed, D2265) or for review/editing.
- **Live things**: tool handles (`tools`, `text`, ...) are re-bound by name on load; pending
  promises, suspended generators, Proxies, host objects and patched builtins are refused
  with a clear message naming the value's path.
- **Edits**: a model or human may edit the exported module; re-importing it is just running
  it (plus restoring the env data).

## Deliverable
docs/functions-as-code.md with the design, and a prototype: export selected globals from a
VM to a module, reload it in a fresh VM and in a VM of a build from a different commit
(reuse 0141's old-tree setup), including binary buffers and shared closures; coverage on
helper-library-style scripts (from the harvested corpus, scripts/harvest: find scripts that
define reusable functions), the refusal list, and sizes. No engine changes.

## Log
- Scope (coordinator, owner): `.dump` = semantic serialization written as JS, not an image;
  builtins referenced, never dumped. Widened to a real implementation in djs-core
  (`Vm::dump`), REPL `.dump`/`.restore`, a djs-wasm export + `Sandbox.dump/load`, behind a
  default-on cargo feature `dump`; doc covers going beyond the REPL.
- 0141's portable crate no longer builds on main (FuncTemplate split into FuncInfo, D2444;
  closure template in ELEMENTS, D2448): not extended; the dumper lives in djs-core.
- Builtin names: `registry::canonical_names` over a heap of the *base image* (prefix cells
  have the same offsets in every VM of the base), so a user global aliasing a builtin
  never names it. Paths without `%`/`~`/`get `/`[[Prototype]]` are JS expressions;
  the others go through the loader's `$builtin(path)`; `~N`/`%intrinsic:` refuse.
- Closures: one factory per group of closures that share captured boxes (verbatim source
  text, no identifier rewriting); captured values that close a cycle are set after the
  data through the factory's `$set`.

- Built: `crates/djs-core/src/vm/dump.rs` (`Vm::dump(opts, names)`, `Vm::dump_values`,
  `DumpOptions { base, host }`, `DumpError`, `HEADER = "// djs-dump 1"`), feature `dump`
  (default in djs-core, IMAGE_NEUTRAL, left out of SOURCE_HASH's feature list in build.rs);
  tests `vm/dump_tests.rs` (8 round trips + refusals, `DJS_DUMP_SHOW=1` prints the dumps);
  REPL `.dump <file|-> [names]`, `.restore` runs a dump in a fresh VM (djs-cli enables
  djs-core/dump); djs-wasm `vm_dump` (feature `dump`, NOT default: +58 KB gzip measured
  443,590 -> 501,768; the gate's debug wasm build adds it); js `Sandbox.dump(names)` /
  `load(text)` (Script goal), js/test/dumpload.test.ts.
- `registry::reachable_names` = canonical_names without the prelude-functions pass: that
  pass compiles the prelude source and linked ~66 KB gzip (prelude text) into wasm.
- Coordinator (after the size report): keep dump off in djs-wasm; shrink the Rust dumper
  (no BTreeMap / sort / format! on the dump path; heap tables, insertion order,
  crate::text builder D2339) and prototype a host-side TS dumper over js/src/reader.ts to
  compare (coverage, speed, JS code size), recommend one in the doc.

## Follow-ups

## Result
