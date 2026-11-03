# 0170: Classes and private fields in dumps; the loader's privileged `$dump`

status: in-progress
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
- [ ] `Vm::load_dump(text, opts)`: runs a dump as a Script with a `$dump` object the loader
      passes in, holding privileged primitives that an ordinary Script cannot reach. Version 1
      dumps load through it unchanged; `Sandbox.load`, the REPL's `.restore` and the wasm
      export use it.
- [ ] Classes (declarations and expressions, `extends`, static blocks and static fields,
      accessors, computed keys) are written as their verbatim source inside a factory (they
      capture like closures), followed by their static state; methods with `super` (in
      classes and object literals) keep their home object.
- [ ] Instances: made with the class's prototype and their own properties; subclasses of
      builtins (Map, Array, Error, Promise settled, …) through the builtin, then the prototype.
- [ ] `$dump.private(obj, C, "#x", value)` for private fields, and brands for private
      methods/accessors, against the class the dump recreated.
- [ ] Round-trip tests in vm::dump_tests for each of the above, refusal tests for what stays
      refused, and the checker (D2355) covering classes.
- [ ] Coverage: how many of the harvested and private scripts define classes (vm::dump_harvest
      counts), in the result.

## Notes
- Decide the exact `$dump` handover (a parameter of the loader, a temporary global made
  non-writable, …); record it as a decision. A dump must never get the primitives by running
  as an ordinary Script.
- tests/dump/v1.js must keep loading (D2353).

## Log

## Follow-ups

## Result
