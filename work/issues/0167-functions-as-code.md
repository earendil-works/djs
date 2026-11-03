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

## Follow-ups

## Result
