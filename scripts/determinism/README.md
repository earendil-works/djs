# Determinism checks (issues 0055, 0072)

- `native-vs-wasm.sh <djs_wasm.wasm>` (the gate's "native vs wasm images" leg) runs the
  `*_body.js` cases and `weakref.js` of `js/` through djs-wasm's exports natively
  (`djs-abidrive`, in djs-difftest) and in the wasm build (`abidrive.mjs`, the same export
  calls) at two fuel slices, and requires the same build hash, raw and collected image
  hashes, stop count and result. `DJS_ABIDRIVE=<binary>` uses an already built driver.
- `djs-abidrive <body.js> <slice> [trace-every] [raw-out]` and
  `node abidrive.mjs <wasm> <body.js> <slice> ...` can also be run by hand, under
  `scripts/timeout.sh`.

Cases in `js/`: `floats_body.js` (libm, number formatting, NaN bytes, Date, sort),
`general_body.js` (a mixed workload with GC), `deep_body.js` (deep data structures and
promise chains), `weakref.js` (WeakRef/FinalizationRegistry under slicing and restore);
the tests of djs-core include `oom*.js` (out of memory near the cap, also through native
builtins and `@fast` entries) and `alloc_only.js` (pure interpreter allocation).
