# 0238: Benchmarks page: graphs, and data passing through the host bridge

status: done
agent: bridge
branch: issue/0238-bench-graphs-bridge
depends-on: 0229
guide: docs/guide/benchmarks.md (and the JS interface / wasm boundary pages where they describe passing values)
touches: crates/djs-docs (chart generation: svg.rs, status.rs, gen regions), js/bench/engines/ (a bridge benchmark), scripts/status.py, tests/scores/status.tsv, docs/guide/benchmarks.md

## Goal (owner)
1. **Graphs.** The benchmarks page needs charts, not only tables: e.g. run time per workload
   per engine (grouped bars, log scale where ranges differ), startup, memory, module size, the
   per-operation ratios (djs / best QuickJS, with a parity line), dump size/time per workload.
   Generate them as SVG from status.tsv (like the existing `gen/*.svg` diagrams: djs-docs writes
   them, `regen` keeps them current, `check` fails when stale), in the site's Elanor style, readable
   in light and dark mode, accessible (title/desc, the table stays next to the chart).
2. **Data passing through the bridge.** A dedicated benchmark of moving data between host and
   sandbox, the core of the "cheap data passing" goal: djs's wire encoding in and its reader that
   reads values directly out of linear memory, against what each QuickJS embedding has to do
   (stringify/copy/parse, or its handle API), and monty's.
   - Directions: host -> VM (a tool result arriving) and VM -> host (a run's result / a tool call's
     arguments being read by the host).
   - Payload shapes: small objects (call args), JSON-like tool results of 1 KB / 100 KB / 1 MB / 10 MB,
     large strings (ASCII and non-ASCII), arrays of numbers, binary (Uint8Array), deeply nested.
   - Measures: latency per call, throughput MB/s, allocations/copies, peak memory; plus the
     end-to-end "tool call with a 1 MB result, script reads one field" case, where djs's direct
     reads should shine.
   - Fair embeddings: describe for each engine exactly what the host does (e.g. QuickJS: JS_NewString
     + JSON.parse in the VM, or JS_ParseJSON; reading: JSON.stringify + copy out, or walking
     handles), and use the best available path for each.
   - Results to status.tsv (re-runnable via scripts/status.py), charts and a section on the page
     explaining why djs's design (wire format in, reader out of the arena) is cheaper or where it
     is not.
Bench at load < ~4. Merge in at most two batches.

## Log

- Plan sent to the coordinator (charts batch, then bridge batch).
- Charts: crates/djs-docs/src/chart.rs (columns, paneled bars, ratios around parity; log axes
  with 1-2-5 ticks thinned to fit the space), svg::style_with for the extra series rules
  (Elanor data-viz palette, same in light and dark). Every `gen:status engines-*` region now
  writes its chart above its table (status::chart, regions.rs). D3166.
- Bridge: js/bench/engines/bridge.ts; `BridgeVm` (put/get/call) per engine in engines.ts;
  driver.c gains bench_put/bench_get (JS_ParseJSON / JS_NewStringLen / ArrayBuffer copy in;
  JS_JSONStringify + ToCStringLen in place out) and a sync C host function `tool` through the
  import env.bench_host. Bellard's JS_NewTypedArray needs a new.target, so `new Uint8Array(buf)`
  goes through the constructor in both releases. Alternatives measured per engine
  (quickjs-wasi json/handles, emscripten json/handles, monty json/values); each number is the
  faster path's (D3165). Monty's json.loads hits its recursion limit at ~200 levels, so the
  nested payload is 100 deep.
- First look (load ~2.7): djs leads in and out for JSON and strings (out: 7x at 1 MB JSON, the
  reader does no VM work), edit-1m 0.4 ms vs 3.8 (QuickJS); behind on bytes in (0.19 vs 0.13 ms:
  wire encode copies the bytes twice before the VM's copy) and on 1,000 small sync calls
  (3.4 ms vs 1.8 for QuickJS).
- scripts/gate.py: the codemode-workloads leg also runs `bridge.ts --quick --engines djs`
  (outside `touches:`, one line).
- A first `status.py engines` run hit load 7 during its ops phase; ops and bridge re-ran
  quiet. Then the driver's host strings (malloc'd at len*3 for encodeInto) inflated QuickJS's
  workload memory (heavy 2.4 -> 4.1 MiB): fixed to the exact UTF-8 size and everything
  re-recorded at 1268c57e, load 2.0-2.3. The `-dirty` suffix status.py wrote (only status.tsv
  itself was modified) was stripped.
- Page: new section "Passing data through the bridge" (paths table, in/out/rate/memory/e2e
  regions, prose on why and where not); "In short", re-running and where-to-look updated;
  host.md's two trade-offs link to it. Other guide pages unaffected.
- driver.c's change rebuilt vendor/engines/build/quickjs*.wasm (shared; the new exports are
  additive), so the engines-size numbers are re-recorded with them.

## Follow-ups

- djs: a sync host call costs ~3.4 µs round trip in the Sandbox vs ~1.8 µs for a minimal
  QuickJS C embedding (calls-1000); worth profiling (vm_run re-entry, record read, toHandle).
- djs: Uint8Array in goes encode -> staging -> Bytes cell (two host-side copies); could be
  copied in place like long ASCII strings.

## Result

- Charts: crates/djs-docs/src/chart.rs; every `gen:status engines-*` and `bridge-*` region
  writes `gen/chart-*.svg` above its table (9 charts), regen/check keep them current (D3166).
- Bridge benchmark: `node js/bench/engines/bridge.ts` (or `scripts/status.py bridge`, also run
  by `status.py engines`), 11 payloads x in/out x 7 engines plus 3 end-to-end cases, each
  engine's best path, alternatives measured (D3165). Gate: the codemode-workloads leg runs it
  on djs with --quick.
- Findings (load 2.0): in, djs ~2x faster than the best QuickJS path (1 MB JSON 3.1 vs 6.4 ms;
  10 MB ASCII 1.0 vs 3.8); out ~7-10x (1 MB JSON 1.2 vs 8.5 ms, strings at memcpy speed);
  edit-1m 0.30 vs 4.0 ms. Behind: Uint8Array in (0.19 vs 0.08-0.14 ms, three copies) and 1,000
  small sync calls (2.9 vs 1.7 ms for the C embedding).
- Known gaps: memory is the linear-memory high-water mark, so emscripten's 16 MiB initial heap
  hides small payloads; copies are described, not counted.
