# 0152: What can djs learn from pico? (research)

status: done
agent: pico
branch: issue/0152-lessons-from-pico
depends-on: -
guide: -
touches: docs/lessons-from-pico.md (report), work/scratch/0152/ (the docs copy, notes); no engine changes

## Owner
"What could we learn from pico? Let a sub agent investigate." The owner provided pico's
documentation site as one HTML file: work/scratch/0152/pico-docs.html in the primary
checkout (a text extraction is next to it: pico-docs.txt, ~230 K chars). pico is a small
statically typed scripting language for LLM code mode with a C11 VM built to wasm (757 KB,
no imports, one VM per instance): JSON values, JSON Schema types, tool calls without
async/await, suspension at any instruction, images of a few KB, reference counting instead
of a collector, a JIT, a TypeScript host, benchmarks against QuickJS, djs and V8.

## Questions
- Read the whole document (tour, language reference, appendix: opcodes, cell kinds and
  layouts, image format, wasm exports, benchmarks, testing).
- For each area, compare with djs (use our docs: ARCH.md, the guide, docs/*.md reports
  such as frozen-builtins, empty-heap, goldilocks/portable snapshots, size-vs-quickjs) and
  identify concrete lessons: what pico does better or differently and whether djs could
  adopt it given that djs must run JavaScript (spec semantics, test262), e.g.:
  - value encoding and arena layout, cell kinds; image format and how images stay a few
    KB; what is in a fresh VM (standard library size, natives vs pico code: "functions that
    take callbacks are pico code");
  - suspension, fibers, fuel; host calls without async; determinism; snapshots across
    versions (instruction set / image versions);
  - reference counting vs our collectors; memory per VM;
  - compiler and bytecode density (83 opcodes), the JIT (what it buys in wasm);
  - the host interface and wasm ABI; testing approach (site examples as tests, evals:
    "can models write pico").
  - their benchmark numbers against djs: what they measure, whether they're fair/current,
    and what they say about our weak spots.
- Rank the lessons by value for djs's goals (codemode, snapshots, per-VM memory, module
  size, speed), with rough effort, and point each at the djs code or issue it touches
  (e.g. 0151's base shrink, the slim build 0149, snapshots 0142).

## Deliverable
docs/lessons-from-pico.md. Don't copy large parts of pico's text; summarize and cite
sections.

## Log
- Read pico's whole site (tour, reference, appendix) from the text extraction (folded to
  lines in work/scratch/0152/pico-folded.txt), and ARCH.md, the guide's snapshot,
  suspension, gc, heap and host chapters, and the reports (empty-heap, frozen-builtins,
  goldilocks/portable snapshots, size-vs-quickjs, slim-codemode) plus 0151's plan on the
  bloat branch.
- Checked pico's benchmarks: built djs-wasm (`wasm` profile) at 7d684dbb into
  work/scratch/0152/target, rewrote pico's compute cases from their descriptions
  (work/scratch/0152/w) and ran them through scripts/perf/wasm-run.ts and
  qjs-wasm-run.mjs. Found `[...Array(n).keys()]` 26x quickjs-wasi (D1800's iter_next
  declines on holes for keys iterators too), `Array.from({length}, f)` 2.9x, and
  map/reduce on tiny arrays 2x (0137's follow-up). pico's djs image number (336 KB) is a
  raw arena from before D2066/D2431; a new sandbox is 11.7 KB uncompressed, 1.2 KB zstd
  (work/scratch/0152/snap.ts).
- Wrote docs/lessons-from-pico.md. No decisions needed; guide: none affected.

## Follow-ups
1. Keys iterators over holes in `iter_plan` (and `Array.from({length}, f)`): 26x / 2.9x
   vs quickjs-wasi. Small.
2. Callback builtins on 0-2-element arrays (0137's follow-up), with pico's 9b shape as
   the benchmark (1.95x).
3. For 0151: one cell per function (code inline in the template), thin internal helpers
   compiled to direct intrinsic calls and dropped, 8-bit operands with an escape, static
   calls (report 2.2).
4. Design: sliced strings + result buffers + external results in snapshots (2.3, 2.8).
5. Run-table session images for codec-free hosts (2.4).
6. Seeded hashing for a VM's own atoms and Map/Set (2.5).
7. Owner decision: image compatibility keyed by tables and base, not the source hash (2.6).
8. Experiment: fuel at safepoints only (2.7).
9. Watch: the fresh-instance call_indirect deopt under node 26 / V8 14.6.
10. Wasm benchmarks on x86-64 too.

## Result
docs/lessons-from-pico.md: pico compared with djs area by area (values and cells,
memory, compiler and bytecode, interpreter and fuel, host calls, images, JIT, testing),
eight ranked lessons with effort and the djs code they touch, what does not transfer and
why, and a check of pico's benchmark numbers against current djs. No engine change.
