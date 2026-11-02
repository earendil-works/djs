# 0156: Architecture audit: are we carrying bloat from design missteps? (research)

status: done
agent: audit
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
- Built shipped and name/link-map/DWARF variants of main 40ad9290: 454.8 KB gz = code 343.5,
  coded image 51.5, Unicode 32.3, other data 21. Areas (areas.py), data by symbol (new
  scripts/size/datamap.py), lines (wasm-dwarf-lines.py: 34.7% inlined library code;
  Vec/RawVec 100 KB raw, Result 56 KB, arena accessors 43 KB).
- Variants (scripts/size/variant.sh): no @native −10.6 KB gz, no @fast −8.3, no run_hot
  −6.7, all three −26.8 (new patches/onepath.py); no bounds −11.8; opt z −27.4;
  #[optimize(size)] on all of djs-core −25.8 but on the cold modules only −4.2 (new
  patches/optsize.py); Unicode off −34.5; 0149's slim A on today's main −52.2; stacked
  slim + one path + no bounds 363.3, + opt z 342.2 (−24.7%).
- Memory: a fresh Sandbox is 19 pages = stack 4 + data 3 + arena 11 (295 KiB base +
  409 KiB reserved headroom: 2x base threshold + 64 KiB) + dlmalloc 1; RSS ~700–720 KiB,
  same as quickjs-wasi (21 pages). Prototype (reserve base + 1 page, 128 KiB stack): 12
  pages at start, but growing runs end 1–4 pages higher because dlmalloc pages land after
  the arena (needs the allocator to keep the arena on top). New
  scripts/size/sandbox-memory.ts.
- Wrote docs/architecture-bloat-audit.md. Guide: no pages affected. No decisions.

## Follow-ups
- Codemode module (0149's slim A as a cargo feature): −52 KB gz measured on today's main.
- Unchecked arena accessors behind the image verifier (needs a decision on `unsafe`):
  −11.8 KB gz.
- Re-benchmark each `@native`/`@fast` twin on the codemode corpus; drop those under
  ~1.3x (ceiling −18.9 KB gz). Keep `run_hot`.
- `Fault` as a zero-sized type with the pending error in the VM, and a niche in
  `Value`/refs, so `Result`s return in one register (est. −5–10 KB gz; prototype one
  module first to measure).
- Memory: reserve the base + a page at `vm_new` with the page allocator keeping the
  arena at the end of memory (dlmalloc pages below it); 128 KiB shadow stack with a lower
  parser nesting cap; the coded base out of linear memory (passive segment or custom
  section). 19 → ~11 pages per fresh Sandbox.

## Result
docs/architecture-bloat-audit.md. djs is not a reduced engine (same surface as QuickJS
plus TypeScript, suspension, snapshots, speed tiers); no large architectural misstep:
every duplicate path together is −26.8 KB gz (5.9%), each with measured speed. The gap to
QuickJS is Rust density (~90 KB gz, unchanged since 0123), builtins in two languages
(~42) and features (~34 − 8). Half (227 KB) is 0.79x quickjs-wasi and not reachable by
trimming; all measured structural cuts stacked reach 342 KB. Plan: codemode module,
unchecked accessors, twins only where they pay, niche Results, memory reservation →
~365–375 KB gz at little speed cost and ~0.69 MiB per fresh Sandbox. New analysis
scripts: scripts/size/datamap.py, sandbox-memory.ts, patches/onepath.py, patches/optsize.py.
