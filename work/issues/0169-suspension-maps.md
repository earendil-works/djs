# 0169: Suspension maps in the compiler, pinned

status: done
agent: bloat
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
- Design (D2570): a role stack in FuncState (`smap::Recorder`, only with the `dump` feature
  and only when `compile_maps` asks): constructs push the roles of the values they keep where
  they emit them and pop them where they consume them; `Parser::site` records each
  await/yield (kind, position, phase-1 index, scope, role stack); `rotate_code` moves the
  indexes along. Resolution maps a site to the final code (`map_pos`), takes the resume pc and
  checks the role count against `stack_depths`: a mismatch or an `unsupported:` role makes the
  site unsupported. 0141's inference from instructions is not used.
- Instrumented: calls (`prepare_call` per form, `parse_args_as` by kind, `emit_call` pops),
  tagged templates, `new`, `super(…)`, `super[k]`, `o[k]`, assignments (by lvalue kind, compound
  `assign.old`, logical assignment), binary operators that keep their left operand, array and
  object literals, template literals, `import()`, intrinsic calls, for-of/for-in/for-await
  records. Marked unsupported: switch case tests, finally blocks, using regions,
  destructuring, class definitions (one push in class.rs's `parse_class`), yield*, closing an
  async iterator (for-await close paths, the generator return path, whose handlers cut the
  stack).
- Probed ~65 constructs by hand (unit tests keep 56); no construct in the corpora or the e2e
  tests gives a count mismatch.
- Pins: tests/smap/<area>/<case>.smap, a slot table per function and one line per site (first
  draft printed every slot per site: 1.2 MB; now 116 KB), plus tests/smap/summary.txt. The pin
  test also compiles every case normally and compares the disassembly with the recording
  compile's: recording changes no code. `DJS_UPDATE_SMAPS=1` regenerates.
- Size: with `dump` off (the shipped modules) the hooks are empty; module gzip 443,460 →
  443,494 (+34), slim 379,049 → 379,137 (+88) against main 16b7438f (positions of scopes are
  stored only with `dump`; argument kinds are an enum, not function pointers, which cost +151).
- Corrected docs/continuation-dumps.md (the vocabulary as djs's lowering needs it; compiler-made
  site names). Guide: none (0177 rewrites it).
- Overlap: 0170 adds `jump_if_loading` in class.rs (static-init sites): resume pcs in pinned
  functions that define classes move; 0173 changes the harvested cases' fakes, which are in the
  pinned sources: whoever merges second runs `DJS_UPDATE_SMAPS=1` and checks the summary.

## Follow-ups
- Describe the constructs marked unsupported as dumps need them: switch case tests
  (discriminant), finally blocks (pending completion), using regions (disposal stack),
  destructuring (source, iterator), class definitions, yield*, closing an async iterator.
- `for (o[await k] of xs)`: the lhs is parsed before the loop's setup but runs after it; its
  sites report a count mismatch (unsupported) rather than roles.

## Result
`compiler::suspension_maps(heap, src, opts)` (djs-core, `dump` feature) recompiles a source and
returns per function its sites: `await#N`-style names per kind in source order, position, resume
pc, operand roles bottom first (or the unsupported construct), and every argument/local slot as a
binding (name + declaring scope position), hidden local, raw argument or nothing. Types:
`compiler::{FunctionMap, Site, Slot, Role, Loop, SiteKind}`. Decision D2570.

Pins: `compiler::smap_tests::pinned_maps` over tests/corpus/codemode, codemode-harvest and the
e2e cases with sites, committed in tests/smap/ (update with `DJS_UPDATE_SMAPS=1`). Coverage
(tests/smap/summary.txt):

| area | cases | sites | unsupported |
| --- | ---: | ---: | ---: |
| codemode | 6 | 25 | 1 (for-await close) |
| codemode-harvest | 300 | 881 | 0 |
| e2e | 9 | 157 | 52 |
| total | 315 | 1,063 | 53 |

By kind: 957 await, 30 yield, 11+11+11 async generator yield/await/return, 23 yield*, 11
await-using, 8 for-await.next, 10 for-await.close, 2 return.await. Described sites by role
sequence: 708 with no operands, 208 `call.callee call.this`, 21 for-of record, 18 `call.callee
call.this object.literal`, 12 for-of record + call, 9 for-of record + call + object literal, 8
for-await record, 7 nested calls, and 21 more in 12 rarer sequences (binary.left, assign.*,
array.element, template.part). Unsupported: 23 yield*, 19 using, 10 closing an async iterator,
1 destructuring; none from a count mismatch.

Known gaps (follow-ups): the unsupported constructs above (switch discriminant, finally
completion, using stacks, destructuring, class definitions, yield*, async iterator close); sites
in an lhs of `for (o[await k] of …)` would report a count mismatch (lhs code runs after the
setup but is parsed before it).
