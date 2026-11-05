# 0219: A compile cache across VMs: import a function's compiled cells instead of compiling again

status: done
agent: lazy
branch: issue/0219-compile-cache
depends-on: 0218
guide: compiler.md or host.md (when compiled code is reused)
touches: crates/djs-core (export/import of compiled cells; image.rs renumbering), crates/djs-wasm (exports), js/pi (cache per module), js/src/sandbox.ts (optional), tests

## Goal
0218's estimate (design A): pi loads repeat compiling the same texts at every idle point in a
fresh instance: the session's codemode script for `$djs.source` (~33-58 µs per load) and parts of
the dump. Compile once per process, export the compiled cells (Script, templates, info,
constants, line tables, names) as a relocatable blob keyed by (text hash, compile options); a
fresh VM imports it (append cells, patch refs, intern atoms by name and patch atom ids in
bytecode, like image.rs's renumbering) at ~10-20% of a compile. Hosts keep blobs per wasm module
(like pi's imageCache). Note: pjr (0215) moved the dump runtime and host values into pi's
prelude image, so the runtime part may already be gone; measure on main.

Acceptance: imported templates behave exactly like freshly compiled ones (tests compare them);
determinism checks (D1720) account for atom-id order differing between a cache hit and a miss (or
the import is made order-identical); pi load (pi-sessions, pjr's baseline method) measurably
faster; size ceilings respected (feature-gate like `lazy` if needed).

## Log

- 2026-11-06 (lazy): built. `vm/compile_cache.rs`: a compile is recorded (the heap logs
  internings with the arena length around them; the cells it appended in order) and
  replayed by an import (cells copied and rebased, the same texts interned at the same
  points, atom ids renumbered if needed), so a hit leaves the heap byte for byte as the
  compile, also from another state (tests over the corpus, before and after running;
  the difftest's `--determinism` checks every case). `Vm::compile_cached`,
  `vm_compile_cached`; pi's `Engine.compile` keeps blobs per module. On pjr's note, the
  suspension maps too: `compiler/smap_codec.rs`, `Vm::export_dump_maps`/`import_dump_maps`,
  `vm_dump_maps_export`/`import`, pi's `Engine.dump` passes them on (only after a dump
  that compiled maps). Feature `compile-cache` (djs-core default, djs-wasm `dump`); main
  and slim unchanged (442,608 / 378,134 gzip). D3040, D3041. After pjr's batch the dump
  runtime and host values are in the prelude image; the cache's load targets are the
  `$djs.source` wrapper and small snippets.
  Measured (pi-sessions, wasm release dump module, vs main 38d89766): instructions of the
  whole node process for 300 sessions 11,090 -> 10,642 M (-4.0%; maps alone -2.7%,
  compiles alone -0.9%), per session 26.6 -> 25.0 M (-6%). Timings on a quiet machine
  (medians, 4 alternating runs): l.sourceAndHost 22 -> 14 µs, loadPlain 81.5 -> 76 µs,
  resumeSession 121.5 -> 116 µs, d.vm_dump 137 -> 114 µs, dumpSession 143 -> 128 µs.
  Under machine load the timings were unstable (l.compile looked +10-25% with the cache on
  in some runs; instruction counts and quiet runs show no regression).

## Follow-ups

## Result

A compile cache across VMs of one process (D3040) and suspension maps carried from one
VM's dump to the next (D3041), in the dump module (feature `compile-cache`).

- `Vm::compile_cached(src, opts, blob)` returns the function and, on a miss, the blob;
  an import replays the compile's internings and copies its cells, leaving the VM byte for
  byte as the compile would (from the same or another state). `vm_compile_cached` in the
  dump module; pi's `Engine.compile` uses it for every text that is not a plain dump.
- `Vm::export_dump_maps`/`import_dump_maps` (`vm_dump_maps_export`/`import`): the dump
  cache's maps as bytes; pi's `Engine.dump` imports the module's last maps before a dump
  and keeps the new ones after.
- Determinism: no atom-order difference to allow for: the import interns in the compile's
  order. The difftest's `--determinism` compares the heaps of a hit and a miss for every
  case; `vm::compile_cache::tests` does so over the corpus.
- pi sessions: -6% of the instructions per session, dumpSession about -10% and
  resumeSession about -4% on a quiet machine.

Known gaps: the plain dump's own text is not cached (new at every idle point); maps of a
dump text's functions compiled on demand at a re-dump are not carried (a function-level
key would need its capture attributes); string hashes are cleared on import when the
hash keys differ (then the bytes differ in those words only).
