# 0156: Architecture audit: are we carrying bloat from design missteps? (research)

status: open
agent: -
branch: issue/0156-bloat-audit
depends-on: -
guide: -
touches: docs/architecture-bloat-audit.md (report), scripts/size/ (analysis scripts, if new), work/scratch/0156/ (variant builds, experiments); no engine changes on main

## Owner
"I'm generally quite puzzled about the overall memory and binary size. A meaningful part
comes from Rust compared to C, but I would have assumed you can get it to half the binary
size trivially, considering the generally reduced footprint of this implementation
compared to a full JS one. I wonder if we're missing some meaningful bloat from
architectural missteps. Investigate just that."

## Stance
Fresh eyes. Earlier analyses (docs/size-vs-quickjs.md: "~half Rust density, ~half the
bytecode image"; docs/empty-heap.md; docs/slim-codemode.md; 0122/0126/0127/0131) explained
the size *as built*. This audit questions the **architecture** itself: what structural
choices multiply code or memory, and what a leaner design would look like. Don't accept
earlier conclusions without re-checking them.

## Questions (non-exhaustive; follow the evidence)
- **Is djs actually reduced?** Compare its feature surface (test262 99.9% of the active
  group, TS stripping, decorators, Intl-ish?, regexp with \p, normalization, BigInt,
  typed arrays, Proxy, determinism, snapshots, two builtin backends...) with QuickJS's and
  with what codemode needs. Where is djs a full engine after all?
- **Duplicated machinery**: several interpreter loops (step, fast_step, run_hot, and the
  paths between them) implementing the same opcodes; two builtin backends (bytecode +
  bgen `@native` Rust state machines + hand-written `@fast` entries + leaf intrinsics)
  for the same builtins; two collectors (Cheney for canonical images + mark-compact);
  JSON in three input widths; string representations (Latin-1/UTF-16/ropes/slices) with
  per-representation code paths; the wire format vs JSON vs images; heap readers in Rust
  and JS (js/src/reader.ts). Measure each: code bytes per duplicate, what a single path
  would cost in speed.
- **Rust-specific multipliers**: generics/monomorphization per cell kind or string width,
  `Result`/error plumbing, bounds-checked heap access patterns (every read a checked slice
  op?), inlining (opt-level, inline thresholds), panics/fmt leftovers, trait objects vs
  enums, big `match` dispatch duplicated across loops. Use twiggy/name sections
  (scripts/wasm-funcs.py, wasm-size.py, scripts/size/) to attribute code to these causes,
  and small experiments (variant builds) to confirm.
- **Data**: the base image (now ~250 KB raw after 0151 L4), Unicode tables, other
  tables; module data vs code.
- **Memory per VM**: arena sizing, the base copy, stacks, side tables, Rust heap (dlmalloc)
  allocations per VM, wasm stack size (D1941: 256 KiB), page allocator overheads; what a
  fresh Sandbox's 1.19 MiB actually consists of.
- **What a leaner architecture would be**: concretely, which 2–5 structural changes would
  cut the module (and per-VM memory) the most, e.g. one interpreter loop, one builtin
  path, one collector, generic-free heap access, with estimated savings, speed cost,
  effort and risk. Target framing: what would it take to reach ~half today's module size?

## Deliverable
docs/architecture-bloat-audit.md: findings with numbers (attributions and experiments),
the missteps ranked by bytes, and a plan. bloat (0151) is shrinking the base heap and
editing compiler/interp; perf (0155) fixes the profiler and image stack slots: don't change
code on main; experiments in scratch builds only.

## Log

## Follow-ups

## Result
