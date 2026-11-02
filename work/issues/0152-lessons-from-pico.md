# 0152: What can djs learn from pico? (research)

status: open
agent: -
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

## Follow-ups

## Result
