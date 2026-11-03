# 0182: No helper preamble in dumps; computed class keys

status: open
agent: -
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

## Result
