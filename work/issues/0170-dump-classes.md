# 0170: Classes and private fields in dumps; the loader's privileged `$dump`

status: done
agent: portable
branch: issue/0170-dump-classes
depends-on: -
touches: crates/djs-core/src/vm/dump*.rs, the loader entry point (Vm::load_dump, new), privileged builtins for $dump (builtins/ or vm/), crates/djs-wasm (vm_load_dump), js/src/sandbox.ts (load), tests/dump/, docs/functions-as-code.md
guide: -

## Goal
Classes are in scope for continuation dumps (owner, docs/continuation-dumps.md "Classes and
private fields"): today version 1 refuses classes, methods using `super` and objects with
private fields. Make them dumpable, and introduce the loader mode the later primitives
(0171) build on.

## Acceptance criteria
- [x] `Vm::load_dump(text, opts)`: runs a dump as a Script with a `$dump` object the loader
      passes in, holding privileged primitives that an ordinary Script cannot reach. Version 1
      dumps load through it unchanged; `Sandbox.load`, the REPL's `.restore` and the wasm
      export use it.
- [x] Classes (declarations and expressions, `extends`, static blocks and static fields,
      accessors, computed keys) are written as their verbatim source inside a factory (they
      capture like closures), followed by their static state; methods with `super` (in
      classes and object literals) keep their home object.
- [x] Instances: made with the class's prototype and their own properties; subclasses of
      builtins (Map, Array, Error, Promise settled, …) through the builtin, then the prototype.
- [x] `$dump.private(obj, C, "#x", value)` for private fields, and brands for private
      methods/accessors, against the class the dump recreated.
- [x] Round-trip tests in vm::dump_tests for each of the above, refusal tests for what stays
      refused, and the checker (D2355) covering classes.
- [x] Coverage: how many of the harvested and private scripts define classes (vm::dump_harvest
      counts), in the result.

## Notes
- Decide the exact `$dump` handover (a parameter of the loader, a temporary global made
  non-writable, …); record it as a decision. A dump must never get the primitives by running
  as an ordinary Script.
- tests/dump/v1.js must keep loading (D2353).

## Log
- Coordinator OK'd (1) `jump_if_loading` (compiler/class.rs, ops.rs, run.rs, heap flag,
  class snapshot), (2) deleting js/proto + js/test/dumpproto.test.ts (D2564), (3) every new
  dump is `// djs-dump 2` (format 2 = Script run by `Vm::load_dump` with `$dump`).
- Static elements: first tried skipping the static initializer's call; static private
  methods then went missing (their closures live only in the initializer's hidden
  bindings). Final: the static initializer installs its private methods and then
  `jump_if_loading` to its return (D2580). Default module 443,460 -> 443,581 gzip.
- Loader (D2581): vm/dump_load.rs. `$dump` is a global property; primitives are host
  functions with reserved ids served by the loader's run loop; v1 runs as a plain Script.
  `heap::private_entries` added (own_keys hides private names; the 0167 dumper's private
  check never fired, classes were refused before it mattered).
- Classes (D2582): members assigned after discovery; class + members one union-find set;
  factory writes classes first (parents first) with locals; heritage wrapper; keep lists;
  static state; private fields/brands via `$dump.private`; HOME methods only inline in
  their home literal. Checker: a function passes if one same-text template keeps its names
  (a class's constructor and field initializer share the class text). Factory lines are
  no longer indented past the first line: multi-line function texts changed (a 0167 bug).
- Coverage: harvest 300/300 equal, no classes; private 2,607 dump and load, 2,601 equal;
  10 define classes, 9 of them equal.
- Sizes: dump feature (default + dump) 479,059 -> 491,966 gzip (+12.9 KB: classes and the
  loader); main module +121 B.
- Docs: functions-as-code.md (format 2, loader, classes, refusals), continuation-dumps.md
  (format 2 definition), cli.md (.restore); guide: generated ref-opcodes/status/builtins
  (op renumbering) only.

## Follow-ups
- Computed class keys and heritage paths are evaluated again at load (side effects would
  repeat); a refusal for impure computed keys would need the key expressions.
- Classes defined with TypeScript syntax dump their TypeScript text (like functions).
- The dump feature grew by 12.9 KB gzip; packaging is 0177's decision (D2564).

## Result
- `Vm::load_dump(text, &LoadOptions)` (vm/dump_load.rs): format 2 with the loader's `$dump`
  (primitive `private`), version 1 as a plain Script; `vm_load_dump`, `Sandbox.load`, the
  REPL's `.restore` use it. Every new dump is `// djs-dump 2`.
- Classes, class expressions, `extends` (name or path), accessors, computed keys, static
  methods/fields/blocks, private fields, methods and accessors (static too), instances,
  subclasses of builtins, `super` in classes and object literals: round-trip tests in
  vm::dump_tests; refusals for decorated classes, heritage expressions, private scope used
  outside the class; checker test for classes.
- `jump_if_loading`: static initializers skip fields and blocks while a dump loads.
- js/proto and its gate test removed (D2564). Header grammar fixed.
