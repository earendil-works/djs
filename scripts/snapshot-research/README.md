# Snapshot design measurements (issues 0088, 0117, 0133, 0140)

Two tools on the same six cases (`cases/`), neither part of the workspace or the gate (like
`scripts/determinism/probe`):

- `snaplab` (Rust, native) measures what alternatives to raw VM images would cost on
  realistic pi codemode sessions. The findings and the designs it compares are in
  `work/issues/0088-snapshot-design-research.md`.
- `wasm.ts` (node, djs-wasm) measures the snapshot container that was built (D2260) through
  the real pi adapter: below.

Two more for snapshots across djs versions (docs/snapshot-versioning.md):

- `churn.py [OLD NEW]`: how many merged issues touched each input of a snapshot (build
  hash, layouts, opcodes, intrinsics, builtins, base image layout, pi adapter, container);
  with two commits, also which intrinsic ids moved between them.
- `upgrade.ts OLD [OUT]`: a Sandbox and a pi session suspended on an older build (OLD, a
  `git archive` of an older commit with its djs-wasm built) and restored on this one:
  refused; restored with the old build (an engine registry); replayed on this build.

```sh
scripts/snapshot-research/run.sh [out-dir] [case.js ...]                # snaplab
scripts/build-wasm.sh && node scripts/snapshot-research/wasm.ts [out-dir] [case.js ...]
```

## wasm.ts

Runs each case through `js/pi/vm.ts` on djs-wasm (`$DJS_WASM`, default the `wasm` profile
build) in deterministic mode, with tools like snaplab's (real source files and prose). At
every idle point it calls `snapshot()`, restores it with `resumeVm()` into a new instance and
continues there, and also restores the same image from an uncompressed container. It writes
`wasm-report.md` to the out dir (default `work/scratch/snapshot-research`):

- per case: idle points, collected image and container sizes, `snapshot()` and `resumeVm()`
  times, and the uncompressed restore time (median and max);
- for the Sandbox: a new one and one holding 1000 records, `bytes()` and `fromBytes()`;
- at the first, the largest and the last idle point of each case: zstd at levels 1 to 19,
  with the prelude image, the base image or no dictionary, and a window that covers the
  dictionary and the image or the default one; compressed size, compression and
  decompression time.

It also writes the prelude image (`prelude.img`), each case's last image (for comparisons
with other compressors; 0117 measured zstd-rs compiled to wasm on them), the snapshot
containers of the first, the largest and the last idle point (`CASE-N.djsn`) and the
cases' start data (`start.json`). `djs heap` explains a container's size (0133):

```sh
node scripts/heap/reference.ts pi-prelude --data OUT/start.json -o OUT/prelude.img   # or use OUT/prelude.img
cargo run -q -p djs-cli -- heap --reference OUT/prelude.img --only diff,session OUT/ask-1.djsn
```

The search tool steps through the source files with a stride coprime with their count, so
its hits are distinct files (with a fixed 7, every file came three times whenever djs-core
had 147 source files, which halved tool-heavy's snapshots).

Defaults: `work/scratch/snapshot-research`, all of `cases/`. The script extracts pi's prelude
from `tests/pi/prelude-source.ts` (needs node), builds snaplab into the repo's `target/`
(djs-core at opt-level 1) and runs it under `scripts/timeout.sh`. It needs `zstd`, `gzip`,
`brotli` and `xz` on the PATH. All six cases take about a minute, mostly compression.

## What it does

`src/pi.rs` is a Rust copy of `js/pi/vm.ts`'s session logic in deterministic mode: a VM from
the base image, the adapter's host globals, pi's prelude with a fixed tool table, then the
case as the codemode body. Tool calls are answered in call order through the prelude's
`settle` and `stalled`, as the adapter does. Tool results come from this repository's own
source files and docs (`src/main.rs`, `Tools`), so their compression ratios are realistic.

At every idle point (the script waits on tool results) it takes the image and measures:

- the raw image (as `VmSession.snapshot()` takes it, without a collection) and the
  collected image; zstd -3/-19, gzip -9, brotli -q 11 and xz -9 of the collected image;
- byte deltas: `zstd -19 --patch-from` against the image right after pi's prelude
  (`post-prelude.img`), against the base, and against the previous idle image;
- a simulated old generation (ARCH "Deltas"): `analyze.rs` matches the image's cells to a
  reference image's by a parallel BFS from the roots, then counts new cells and the 4 KiB
  reference pages that changed cells dirty;
- a prototype logical snapshot (`logical.rs`, encoder only): the object graph as records,
  with reference-world objects named by path, templates as (script, source span), strings
  deduplicated, bytecode, inline caches, hash indexes, atoms and compiled regexps left out;
  against the post-prelude world and against the base;
- the journal (code plus delivered results) a replay needs;
- restore times: `Vm::from_bytes` vs replaying the journal from scratch or from the
  prelude image (native, dev build: use them as ratios);
- suspended activations (function, resume op, operand stack) and builtin-internal objects
  (builtin closures by intrinsic name, records, promises).

Outputs in the out dir: `report.txt` (details for the first, the largest and the last idle
point of every case), `points.csv` (one row per idle point), the reference images and the
selected images.

## Cases

| case | what it exercises |
| --- | --- |
| `ask.js` | a script waiting on a human (the smallest state) |
| `tool-heavy.js` | `js/bench/pi.bench.ts`'s TOOL_HEAVY: search, 60 reads in batches of 10 |
| `sequential.js` | an await inside a for-of; the script keeps only summaries |
| `big-log.js` | a 1.5 MB tool result kept in a variable, a Map of error lines |
| `records.js` | 1500 records, a class, Maps, `Promise.all` over async helpers |
| `agent-loop.js` | a retry wrapper with try/catch/finally, an async generator, `for await`, awaits inside expressions |
