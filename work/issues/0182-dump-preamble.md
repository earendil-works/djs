# 0182: No helper preamble in dumps; computed class keys

status: in-progress
agent: portable
branch: issue/0182-dump-preamble
depends-on: 0178
guide: -
touches: crates/djs-core/src/vm/dump*.rs (writer + loader primitives), tests/dump/ (golden dumps change only with a decision, D2619), examples/sleep-dump/README.md (the dump text), docs/continuation-dumps.md, docs/functions-as-code.md

## Owner
Looking at examples/sleep-dump's dump: "Why does the example dump all these roots? I don't
really understand."

## Goal
1. The exporter writes helper code into the dump: `$dump.builtin = (path) => { const roots
   = {14 internal roots…}; …path parser… }` (the whole table even when one entry is used)
   and `$dump.advance = (it, n) => …`. Format 2 dumps only load through the loader, which
   hands them `$dump` and has the builtin registry (D2371). Make `$dump.builtin(path)` (by
   registry path, refusing an unknown path) and `$dump.advance(it, n)` **loader
   primitives**, and stop emitting any helper preamble: a dump contains only its data,
   functions and `$dump.*` calls. Golden dumps are rewritten under a decision (D2619).
2. Computed class member keys (`[k] = 1`, `[k]() {}`) are evaluated again where the dump
   writes the class (ReferenceError at load, or silently another `k`): refuse them, or
   write the keys by value. No silent wrong output.
3. Update the sleep-dump README's dump text and docs.

## Log
- 2026-11-04 (portable): helpers -> loader primitives (vm/dump_prims.rs: builtin, advance,
  fromSync, closeThrow, spent, rejected, bytes, fn, keep), read-only on `$dump`; no
  helper preamble (D2622). `$dump.builtin` uses registry::resolve_reachable and requires a
  base cell. advance first wrote the iterator index as a double: a second dump of the
  loaded VM read it as 0 (caught by difftest --dump-every-idle, h172); now an int, test
  dumps_carry_the_random_state_and_iterators_again chains dumps with two awaits per
  iteration. new_builtin_closure factored out of %NewBuiltinClosure (intrinsics.rs).
- Computed keys (D2623): the parser records the computed keys each function evaluates
  in its suspension map; the exporter refuses keys in a class's or method's text that an
  enclosing function evaluates, unless literal or a well-known symbol. Test
  computed_keys_evaluated_again_are_refused.
- Coordinator addition (0176): Math.random's state, `$dump.random(lo, hi)` first in every
  dump (D2624); hash key unaffected.
- Goldens rewritten under D2625 (87 -> 50 KB with three new programs: spent, bytes,
  random); steps.dump.js stripped of helpers. examples/sleep-dump/README.md: the dump
  (3,159 -> 1,308 bytes), the explain positions and the paragraph on helpers.
  djs-cli/tests/repl.rs: three expected dump line numbers moved by one (the random line).
  Docs: continuation-dumps.md (primitives table, computed keys, random),
  functions-as-code.md (example, builtins, refusal table). Guide: none affected.

## Result
