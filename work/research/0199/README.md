# 0199 research prototype: plain-JavaScript dumps (not engine code)

docs/plain-js-dumps.md is the report. The prototype turns today's format 2 dumps into plain
JavaScript Scripts (every suspended activation a continuation function), maps re-dumps back
to the original functions (origin tracking), and checks the results in djs and node. Nothing
builds or tests it; it is kept for reproducibility.

Run from a copy in work/scratch/0199/ (scripts find the repo three levels up; `npm install`
brings acorn; a debug `target/debug/djs` must exist):

- `harvest.mjs [filter]`: progs/ (tests/dump/v2, golden, the harvested corpus with a one-line
  `text`/`store`/`load` shim, extra/), today's dumps at every idle point (`djs run
  --dump-after K`, at most MAX=40) in today/, with the output before each dump and the
  whole run's output.
- `plain.mjs`: `toPlain(dumpText, {prune, pieces})` -> `{text, stats}`, the transform
  (continuations, placement of closures by what they capture, callbacks, placeholders,
  plain helpers, origin records). `freevars.mjs`: free variables of a function's text.
- `normalize.mjs`: `normalize(dumpText)`: a dump of a run that resumed continuations, mapped
  back to activations of the original functions (and dead code dropped).
- `t.mjs`: transforms today/ into plain/, counts refusals.
- `check.mjs [filter] [--node]`: runs every plain dump with `djs run` (and in node through
  `harness.mjs`), compares with the uninterrupted run.
- `chain.mjs [filter] [--node]`: dump, plain, run to idle point 2, dump, normalize, plain, ...
  (at most 50 links; the last runs to the end); sizes per link against today's dump of
  the same state.
- `sizes.mjs`: raw and zstd -3 sizes, with and without pruning; transform time.
- `timing/`: a standalone release binary (its own `[workspace]`) timing `Vm::load_dump` of
  today's dump against compile + run of the plain dump to the first idle point, best of
  REPS fresh VMs: `REPS=10 timing/target/release/plain-timing pairs.txt` with lines
  "today/X plain/X".
- `tdz.mjs`: counts shared variables in their TDZ.
- `examples/`: today's and the plain dump of golden loop-nested and v2 cells; a re-dump of
  a plain run (`loops.link2.redump.js`) and its next plain dump.
- `*.log`: the numbers the report quotes.
