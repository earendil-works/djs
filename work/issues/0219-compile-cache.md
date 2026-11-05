# 0219: A compile cache across VMs: import a function's compiled cells instead of compiling again

status: open
agent: -
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

## Follow-ups

## Result
