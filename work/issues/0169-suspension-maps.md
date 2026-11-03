# 0169: Suspension maps in the compiler, pinned

status: open
agent: -
branch: issue/0169-suspension-maps
depends-on: -
touches: crates/djs-core/src/compiler/ (a side table computed on demand), a new pin test and its committed maps (tests/ or compiler/snapshots), docs/continuation-dumps.md (only to correct the plan)
guide: - (the guide is rewritten in 0177)

## Goal
The compiler contract of continuation dumps (docs/continuation-dumps.md "Suspension maps",
D2563): for every `await`/`yield` of a function, a description of what a suspended frame
holds there, independent of this build's slots and pcs, so a later build can rebuild the
frame from a dump (0171) and the exporter can name what it finds (0172).

## Acceptance criteria
- [ ] `compiler::suspension_map(...)` (name it as fits) recompiles a function on demand and
      returns, per site: its name (`await#N`/`yield#N` in text order; compiler-made sites of
      `for await`, async generator `yield`/`return`, `await using` with a kind), the resume
      pc, and for every frame slot what it holds: a binding (name + declaring scope), a hidden
      local by stable name (`<state>`, …), an operand by role, or nothing.
- [ ] Operand roles are produced by the compiler where it emits the construct (not inferred
      from instructions): at least the vocabulary of docs/portable-snapshots.md §3.5
      (`call.callee`, `call.this`, `call.arg[i]`, `object.literal`, `object.key`,
      `binary.left`, `assign.object`/`.key`/`.old`, `array.literal`, `template.prefix`,
      `new.callee`) and loop records (for-of/for-in/for-await iterator records).
- [ ] Constructs not covered yet (a `finally`'s pending completion, `switch` discriminant,
      destructuring under way, `using` stacks, `yield*`) are reported as "unsupported at this
      site" with the construct, never silently mislabeled.
- [ ] Unit tests per role and per loop kind; a test that a callee/this swap in the lowering
      would change the map.
- [ ] Pin test: the maps of every await/yield in tests/corpus (codemode, codemode-harvest)
      and the e2e async tests committed as text, compared like the bytecode snapshots, with an
      update mode. Coverage counts (sites, by role sequence, unsupported) in the issue result.
- [ ] Zero cost at run time and in images: nothing stored in templates.

## Notes
- The 0141 prototype (scripts/snapshot-research/portable/src/sites.rs, roles.rs) derived
  roles from instructions; read it for the shapes, but the roles must come from the compiler.
- Nothing here changes bytecode; compiler snapshot tests must stay unchanged.
- Parallel with 0170 and 0173 (disjoint areas).

## Log

## Follow-ups

## Result
