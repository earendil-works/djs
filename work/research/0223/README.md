# 0223: measurements refresh

Every number djs publishes, re-measured on 2026-11-05 and, where it sits in prose, made a
generated value (D3070).

## Machine and conditions

- Mac17,7, aarch64, 18 cores, macOS 26.6.2, node v26.10.0 (the speed track's quickjs-wasi
  3.6.2 from pi-private's node_modules).
- All other agents archived. One orphaned `djs-difftest` (from 0208's reducer in
  .worktrees/pjz, 15 h at 100% CPU, ppid 1) was killed before recording. A
  Virtualization.framework VM kept one core busy throughout.
- 1-minute load 2.3–4.7 during the recordings (`bench.sh --record` started at 2.7 and
  pushed it to 4.7 itself); 5-minute load 3.4–4.0. Instruction counts (the speed table,
  the bounds-check cost) do not depend on load; times do, and were taken as medians.
- Engine sources are those of 7167411f0f (the branch's base): nothing under crates/djs-core,
  djs-wasm, djs-prelude, builtins/, js/src, js/pi or tests/pi changed before recording, so
  every record says 7167411f0f (the branch's own hashes would not survive the rebase).

## How

| what | command | records |
| --- | --- | --- |
| module sizes | `scripts/status.py size` | `wasm/*`, `slim/*`, `dump/*` (bytes, gzip -9, brotli, code, data, coded base section, exports, slim's decoded base) |
| speed vs quickjs-wasi | `scripts/status.py speed` (wmeasure, n = 3) | `speed/*` |
| pi's tests | `scripts/status.py pi` (pi-tests.ts default, `--next`, `--next --dump-every-idle` on the dump module) | `pi/*/passed`, `pi/*/total` |
| pi's benchmark | same (js/bench/pi.bench.ts, median of 3 runs) | `pi/bench/*`, `fuel/slice-us` |
| startup | `scripts/status.py startup` (js/bench/sandbox.bench.ts, median of 3) | `startup/*` |
| dump costs | `scripts/status.py dumps` (scripts/dump-costs, packaging B, 300 harvested scripts) | `dumps/*` |
| bounds checks | `scripts/status.py bounds` (`unsafe-heap` build vs shipped: gzip, wmeasure instructions) | `bounds/*` |
| heap bytes, wasm memory | `scripts/status.py heap` (scripts/size/object-bytes.ts, a node snippet) | `heap/*` |
| engine benchmarks | `scripts/bench.sh --record` (all), then `--record compile/` (new lazy case) | tests/scores/bench.tsv |
| ratios quoted from bench.tsv | `scripts/status.py bench` | `compile/*`, `wire/*`, `native/*` |
| dump size score | difftest `--plain-out`, the v2/sleep tests, `dump-size.ts --score --update` | tests/scores/dump-size.tsv |
| conformance | computed by djs-docs (test262 grouping and ratchets, corpus) | none |

## What changed in the published numbers

Conformance: unchanged. test262 38,069 of 38,109 active (40 known failures), corpus 621
of 632, pi 221 of 226 in all three modes (the next mode and dump-every-idle were not
published before).

Speed in wasm (instructions, djs/qjs ratio; last record d780463b0d, 2026-11-01): better
on every workload. tight 0.42 → 0.38, prop1 0.49 → 0.45, strings 0.51 → 0.38, arrays
0.64 → 0.58, json 0.57 → 0.60 (qjs's count fell more than djs's), json-date 0.48 → 0.52
(same), forof 0.81 → 0.73, calls 0.81 → 0.75, push-pop 1.40 → 1.33, pi-like 1.38 → 0.84,
forEach 1.49 → 0.93, higher-order 2.92 → 1.73. djs's own counts fell on every workload
(pi-like 10.0G → 5.8G, higher-order 13.9G → 7.8G, forEach 9.0G → 5.4G).

Sizes: the old record (1,471 KB, 525 KB gzip, 566 KB data "of which 186 KB base") was
stale: the base moved into a coded custom section long ago. Now shipped 1,034 KB (443 KB
gzip, 372 KB brotli; 868 KB code, 116 KB data, base 186 KB coded to 46 KB), slim 906 KB
(378 KB gzip), dump module 1,966 KB (785 KB gzip, the dumper +342 KB gzip). Slim base
135 KB (prose said 132), full base 186 KB (prose said 182).

pi's benchmark (README said "at 0095": djs 4.2 ms vs qjs 5.3 ms tool-heavy, start 1.0 vs
3.9 ms): tool-heavy 2.4 ms vs 4.5 ms; start from the prelude image 0.14 ms vs 2.9 ms, 4.9
ms when djs runs the prelude (pi.md said 1 ms / 7 ms / 4 ms). Interrupt polls per second in
a loop that never ends: djs 77,000, quickjs-wasi 61,000 (pi.md said "about as often,
about 33,000": djs now polls 1.27x as often, so a pi interrupt budget is less CPU on djs;
pi.md says so now). A 10,000-fuel slice is 13 µs (limits.md said "a few tens").

Startup: `vm_new` 2.5 µs (README said 40 µs), `Sandbox.create` 113 µs, with a first run
142 µs (README said 60–140), instantiation 41 µs.

Dumps (dumps.md said median 6 KB, zstd 1.7 KB / p99 17 KB, write 0.4 ms / p99 3 ms):
median 2.6 KB, p99 12.4 KB; zstd 1.0 KB / 3.0 KB; write 0.26 ms / 1.7 ms; resume (new
sandbox included) 0.39 ms / 2.5 ms (the prose said "under a millisecond"; true at the
median, not at p99). The big-log case: 1.76 MB raw, 403 KB zstd (was 1.8 MB / 394 KB).
dump-size score: corpus 7,076,909 raw / 2,423,107 gzip, 34 gzip bytes below the ratchet.

Heap: a `{a, b, c, d}` literal 72 bytes per object with a shape, 112 as a dictionary (prose
said 82.5 / 122.5); five-key JSON record 104 / 152 (130.5 / 178.5). The old figures have
no recorded method; the new ones are arena bytes after a collection minus the same array
of numbers. Ten million appends: 40.5 MB of wasm memory (prose said 150 MB native RSS
against 574 MB with a rope per append, a design that no longer exists; the prose now
gives the arithmetic, 24 bytes per node). A 10 MiB ASCII parameter: 13.4 MiB of wasm
memory, 33.5 MiB through the staging buffer (prose said 11.4 vs 21.4 MiB). Fresh sandbox
640 KiB (as said).

Bounds checks: 7.9 KB gzip, instructions median +10.9%, max +16.8% (prose: 7 KB, up to 15%).
Lazy compile of typescript.js: 56% of the eager time (bench: 55 vs 98 MB/s; prose said
"about half").

Removed from the prose as not re-measurable without new tooling: gc.md's "1.1 to 1.6
times as long", "cuts the peak by up to half", "about 1% of the runs"; compiler.md's
per-function 1.45x / 0.4x; arena.md's "roughly 1,600 access sites". host.md's and
js-api.md's end-to-end wire/reader ratios (1.4x–9x, 3x–130x, from one-off runs of
js/bench/wasm.bench.ts with host-side JSON) are now VM-side ratios from bench.tsv's wasm
group: decode vs `JSON.parse` 1.9x (objects) / 16x (1 MiB string), reading vs the VM's
`JSON.stringify` 1.9x (objects) / 10x (non-ASCII text).

Left as written (they follow from the code, not from a measurement): fuel costs, stack and
reserve limits, the shape budget and its 160 KB / 2.3 MB arithmetic, the GC side-table
2.3%, 97/1009 slicing, pi's prelude freezing "about 700 objects". examples/sleep-dump's
688-byte dump matches the dump-size score. docs/host.md's dated benchmark tables are
design records, not published status, and were left alone.

## Regressions to look into (native engine benchmarks, vs the 2026-11-01 records)

Confirmed by a second run:

- `regexp/compile`: +26% to +83% (`/\bfox\b/i` 309 → 565 ns, `/https?:.../` 1150 → 1845 ns).
- `gc/records per MiB live` +60% (0.214 → 0.343 ms/MiB), `gc/linked list` +66%.
- `wire/stringify 10k-objects` +48% (2.29 → 3.38 ms).

Single run only:

- `regexp/scan` on UTF-16 literals: `/zebra/g` −27%, `/lazy dog/g` −19%, `/\bfox\b/gi` −14%
  (the Latin-1 variants are unchanged).
- `wasm/instantiate` 28.6 → 67.7 µs (the module is about twice the size of the one
  measured then).
- `wasm/json_stringify 1mib-string` +17%, `wasm/readValue 1mib-text-unicode` +11%.
- `engine/djs/empty` 0.005 → 0.131 ms: probably a change in what the case measures (it now
  matches `djs-bytecode/empty`), not a slowdown.

Much better: `regexp/scan` email pattern 5x, `\s+` +50%, JSON parse/stringify +20–56%, wire
decode −20 to −43%, `wasm/vm_new` 11.2 → 2.3 µs, the whole native engine track (djs÷qjs
arrays 12.8 → 1.6, higher-order 6.9 → 1.2, loop 3.8 → 1.5).

## Tooling fixed or added

- scripts/dump-costs/dump-costs.ts measured nothing (every dump refused: dumps need frozen
  builtins); it now sets `frozenBuiltins`.
- js/bench/pi.bench.ts measures interrupt polls per second.
- djs-bench: `compile/typescript.js lazy`.
- scripts/size/object-bytes.ts: heap bytes per object.
- status.py: `pi`, `startup`, `dumps`, `bounds`, `heap`, `bench`, `all`; `size` covers all
  three modules.
- djs-docs: inline numbers (`<!--n KEY FMT-->`), `gen:status recorded`, more computed counts.
