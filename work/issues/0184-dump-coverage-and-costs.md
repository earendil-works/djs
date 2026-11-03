# 0184: Close the remaining dump refusals; measure dump size and load time

status: in-progress
agent: portable
branch: issue/0184-dump-coverage
depends-on: 0182
guide: -
touches: crates/djs-core/src/compiler/ (suspension maps for the unsupported constructs), crates/djs-core/src/vm/dump*.rs, tests/dump/v2/ (new golden programs under D2619), tests/smap pins, scripts/snapshot-research or scripts/dump-costs (measurements), docs/continuation-dumps.md

## Goal
Dumps are the persistence format (D2560); before images are retired (0177):
1. **Remaining refusals** — describe what a suspended frame holds at, and dump/resume:
   `switch` case tests (the discriminant twice on the stack), `finally` blocks (completion
   records), `using`/`await using` regions, destructuring, `yield*` (the delegated iterator),
   closing an async iterator, `for-in` (the key iterator), RegExp string iterators, iterator
   helpers if feasible. Each: the map roles, the loader's resume side, re-enactment, golden
   programs, every-idle-point tests. Refuse cleanly whatever stays out.
2. **Costs vs images** (the owner's comparison): dump size (raw and zstd) and write/load time
   (wasm, the dump module) next to session images/containers for: an empty Sandbox, typical
   pi waiting sessions (scripts/snapshot-research cases, the harvested corpus idle points:
   distribution), and data-heavy sessions (records, a 5 MB log, binary buffers). Recommend
   whether anything (e.g. zstd of dump text, slices/shared strings in dumps) is needed before
   0177 retires images.

## Log

## Result
