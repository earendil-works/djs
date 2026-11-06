# 0238: Benchmarks page: graphs, and data passing through the host bridge

status: in-progress
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

## Follow-ups

## Result
