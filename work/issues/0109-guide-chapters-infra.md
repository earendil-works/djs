# 0109: Guide chapters: builtins, host interface, testing

status: in-progress
agent: infra
branch: issue/0109-guide-chapters-infra
depends-on: 0103 (part 1: djs-docs and the authoring guide)
guide: docs/guide/ (three new tour chapters; SUMMARY.md)
touches: docs/guide/ (the three chapters, SUMMARY.md entries, generated regions via regen), nothing else

## Goal
Help the docs agent's 0103 by writing the three tour chapters closest to infra's work,
following docs/guide/README.md (the authoring guide: "In short" box, trade-off notes,
"Where to look", decisions; live examples where they help):
- **Builtins:** the leaf rule, the builtin language and bgen, the bytecode vs native
  backends (D1631–D1635), @fast entries (D1850, D1910), dense leaves, the intrinsic table,
  collect-and-retry in natives (D1900), why this layering and what it costs.
- **Host interface:** djs-wasm's ABI, the Sandbox, values in (wire) and out (reader),
  async host calls and the inbox (D1670), deterministic mode and journals (D1680, D1730),
  the pi adapter, and the trade-offs.
- **Testing:** test262 groups and the ratchet, difftest against dquickts with the shared
  prelude, the stress modes (suspension, GC stress, determinism, native vs wasm, both
  backends), the gate and merge flow, and why each exists.

## Acceptance criteria
- [ ] Three chapters, accurate against the code and decisions, readable for a human new to
      the project, with generated regions where data exists (via djs-docs).
- [ ] The docs agent reviews them for consistency of voice, terms and structure before
      merge (coordinate through the coordinator).
- [ ] `djs-docs check` green.

## Log
- Wrote docs/guide/builtins.md, host.md and testing.md (with img/run-loop.svg) and
  linked them in SUMMARY.md. Facts were checked against the code, ARCH.md, TESTING.md,
  docs/host.md, docs/wire.md and the decisions; numbers are taken from those documents or
  counted in the sources (builtins, intrinsics, @native, @fast).
- Generated material: gen:ops call_intrinsic, two live examples with gen:run (builtins:
  a getter in the includes slow path and a stack trace through `map (native)`; testing: a
  codemode case), and gen:decisions on each page. `djs-docs check` is green.
- Waiting for the docs agent's review (voice, terms, structure) before merge.

## Follow-ups

## Result
