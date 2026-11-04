# 0188 research prototype (not engine code)

Variant R of docs/suspension-design-review.md: rewrite a format 2 dump's `$dump.resume`
activations as residual continuations in plain JavaScript, and compare with today's dumps.
Kept for reproducibility; nothing builds or tests it.

Run from a copy in work/scratch/0188/ (paths are relative to the repo root three levels up;
`npm install` brings acorn and typescript):

- `gen1.sh prog.js outdir` — today's dumps at every idle point (`djs run --dump-after K`, at most 40);
  `progs/` held the tests/dump/v2 programs, `hprogs/` the harvested cases with a `text` shim.
- `residualize.mjs in.dump.js > out.dump.js` — the residualizer (~600 lines).
- `DIR=today|htoday drive.sh` — residualize every dump, resume both, compare the continued
  output (stack lines stripped).
- `chain.sh prog.js` — dump, residualize, resume, dump again, ... (at most 50 links; idle point 1 of
  a resumed run is the loaded state itself, so links dump at idle point 2).
- `measure.py` — sizes (raw, zstd -3) and load times; needs a release `djs` with a local,
  uncommitted `DJS_TIME_LOAD=N` hook in `resume_dump` (N fresh-VM loads, best and mean).
- `ts-downlevel-*.js` — the await-heavy loop and job-order probe compiled with tsc to es2015/es5.
