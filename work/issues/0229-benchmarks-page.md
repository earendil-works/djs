# 0229: A benchmarks page: djs vs QuickJS versions (and monty) on real codemode workloads; dumps, size, memory

status: in-progress
agent: benchpage
branch: issue/0229-benchmarks-page
depends-on: -
guide: new page (Using djs or Reference: "Benchmarks"); status.md / README links
touches: js/bench/ or a new bench/ harness, scripts/status.py + tests/scores/status.tsv (new keys), docs/guide/ (new page, SUMMARY.md), vendor/ (engine builds, gitignored), tests/bench/codemode/ (the workload set)

## Goal (owner)
A guide page with benchmarks a reader can trust and we can re-run:

1. **Workloads from real codemode use.** Mine `~/.pi/agent/sessions` (7.2 GB, ~3,700 session
   directories, JSONL) for the scripts agents actually ran in codemode. Cluster them into
   typical use cases (tool-call orchestration with Promise.all, JSON reshaping of tool results,
   string/regex processing of file contents, loops over search results, small computations,
   TypeScript bodies, …) and write a **representative, sanitized workload set** checked into the
   repo (tests/bench/codemode/ or similar): synthetic scripts that reproduce the shapes, sizes and
   operation mix, with stub tools returning realistic but invented data. **Never commit or publish
   anything from the sessions themselves** (paths, file contents, prompts, tool outputs, names):
   they are private. Record in work/research/0229/ how the set was derived (counts per cluster,
   size distributions), without quoting session content.
2. **Engines compared, all in wasm** (the fair comparison for an embedder): djs (shipped module),
   several popular QuickJS builds: quickjs-wasi (what pi used), quickjs-ng (current release),
   Bellard's QuickJS (latest release, e.g. via wasi-sdk or quickjs-emscripten), and
   quickjs-emscripten if it differs. Build or download them reproducibly into vendor/ (pinned
   versions and hashes, a script). Native numbers optional as a footnote.
3. **Monty, for fun:** pydantic's monty (a Python-subset interpreter for codemode). Translate the
   workload set into the Python subset monty supports (by hand or a small translator), mark which
   workloads translate, and run them the same way (its wasm or native build, say which).
4. **Measure:** per workload: startup (new instance/VM to first result), run time, peak memory
   (linear memory high-water mark / RSS where meaningful), and for djs also: dump time, dump size
   (raw/gzip), load + resume time. Plus module sizes (raw/gzip/brotli) for every engine, and memory
   of an idle VM. Use instruction counts where timing is noisy; record machine and load (bench
   only at load < ~4).
5. **Re-runnable:** one command (e.g. `scripts/bench-engines.sh`) that fetches/builds the engines
   and runs everything, writing results to tests/scores (new keys) so the page's numbers are
   generated (D3070 inline numbers / gen:status), not pasted. Reuse js/bench/pi.bench.ts,
   scripts/dump-costs/, scripts/status.py where possible.
6. **The page:** docs/guide/benchmarks.md: what is measured and why, the workload set
   described in general terms, tables per dimension, honest notes (where djs is slower and why),
   how to re-run. Follow the guide rules (no history, no decision IDs; name pi only on pi.md:
   describe the workloads as "agent codemode sessions").

## Log

## Follow-ups

## Result
