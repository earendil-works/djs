# Determinism experiments (issue 0055)

Experiment tools from the determinism investigation. They are not part of the workspace or
the gate. The findings are in `work/issues/0055-determinism-investigation.md`.

Run everything from this directory and under `scripts/timeout.sh`.

```sh
# Native probe (detprobe) and the native twin of the wasm ABI driver (abidrive), built into
# the repo's target dir:
(cd probe && CARGO_TARGET_DIR=../../../target cargo build -q && rm -f Cargo.lock)
../../scripts/build-wasm.sh
W=../../target/wasm32-unknown-unknown/wasm/djs_wasm.wasm
```

- `detprobe <script.js> [--body] [--limit BYTES] [--async-policy eager|idle] [--slices a,b,max]
  [--no-restore] [--no-stress]` runs one script under several embedder schedules (fuel
  slice, restore from the image at every stop, GC stress) and prints for each: the output
  hash (`same`/`DIFF` against the first), the raw image hash and length, the image hash
  after an explicit collect, and the number of stops. `tool(x)` is an async host function
  that resolves with `x * 2`, either at the first stop after the call (`eager`) or only at
  IDLE (`idle`, the default).
- `abidrive <body.js> <slice> [trace-every] [raw-out]` (native, `target/debug/abidrive`) and
  `node probe/abidrive.mjs $W <body.js> <slice> ...` (wasm) make exactly the same export
  calls and print the raw and collected image hashes and the stop count. Build abidrive
  with `--target x86_64-apple-darwin` (a separate `CARGO_TARGET_DIR`) to compare x86-64.
- `node sandbox_interleave.ts $PWD/$W`: the Sandbox's output depends on `fuelSlice`.
- `node pi_polls.ts $PWD/$W`: pi adapter interrupt polls, cold start vs prelude image.
- `node pi_resume.ts $PWD/$W`: pi adapter, uninterrupted vs `resumeVm` at every tool call.
- `python3 deep_search.py`: the deepest source each compile-time nesting limit accepts
  natively, then the same source in wasm.

Cases in `js/`: `async_interleave.js` (delivery position), `oom.js` (`--limit 1500000` /
`2000000`: OOM depends on restores), `alloc_only.js` (pure interpreter allocation: raw
images vs slice size and restore), `general.js` / `general_body.js` (mixed workload with
GC), `floats_body.js` (libm, number formatting, NaN bytes, Date, sort), `deep_body.js`
(deep data structures and promise chains), `weakref.js` (WeakRef/FinalizationRegistry under slicing and restore).
