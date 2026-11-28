# 0308: Docs and tooling cleanup after the dump removal

status: done
agent: pclean
branch: issue/0308-docs-tooling-cleanup
depends-on: 0307
touches: docs/ (non-guide), docs/guide, work/research, scripts/, crates/djs-cli, crates/djs-docs, scripts/gate.py, TESTING.md, ARCH.md
guide: docs/guide/cli.md, docs/guide/js-api.md, docs/guide/host.md, docs/guide/snapshots.md, docs/guide/determinism.md

## Goal
- Delete the dump research archive (docs/suspension-design-review.md, work/research/0188,
  0199, 0207, 0215, 0216); trim docs/heap-snapshots-revisited.md only where it misleads.
- Fold docs/cli.md and docs/host.md into the guide (cli.md, js-api.md, host.md): one place
  per topic; move any design reasoning only they hold into the guide's trade-off boxes or
  ARCH.md; update the code comments that point at them.
- Prune docs/decisions-index.md of superseded and removed-component entries; add D3640+.
- The guide pages that still describe moving a VM between fuel slices through an image, or
  mid-run images as a use, describe djs as it is; `djs run --suspend-every`, the REPL's
  `.suspend-every` and the docs runner slice in the same VM (difftest's `--suspend-every`
  through images stays: it is the heap-consistency stress, and byte-identical images stay a
  development check, owner).
- Gate: drop the stress variants from the unchecked release difftest leg (the checked leg
  runs them); record the CPU saved.
- Delete unreferenced research/perf/size scripts (scripts/wasm-areas-unnamed.py,
  wasm-leb-padding.py, unmerged scripts/size/patches, perf tools only the README names).

## Acceptance criteria
- [x] The above; djs-docs check clean; TESTING.md and ARCH.md follow.

## Log
- Deleted docs/suspension-design-review.md and work/research/0188, 0199, 0207, 0215, 0216
  (86 files, 8,418 lines; nothing outside them linked to them). docs/heap-snapshots-
  revisited.md: its status header now says the `snapshot` feature is merged (D3744), the
  fallback is the deterministic re-run (D3746), and "Reproducing" names what is not on
  main; the report itself stays as written.
- Folded docs/cli.md into guide/cli.md (identities, resume's exit code 2, the snapshot
  refusal, input rules, the top-level await rewrite and its gaps, output limits,
  completion through data properties, the line editor as a trade-off box, the heap
  report's paths and session part, `.disasm`/`.fuel`/`.step` details) and docs/host.md
  into the guide (limits.md: the wasm32 cap's 3x reasoning, the regexp progress record
  and host collects; everything else was already in host.md/js-api.md/determinism.md, the
  ABI contracts are lib.rs's doc comments, its old numbers superseded by benchmarks.md).
  Pointers updated: ARCH.md (also its stale outline: vm_new's args, the op count, js/pi,
  tests/ dirs), djs-cli run.rs/repl, djs-wasm lib.rs/Cargo.toml, js/src/sandbox.ts,
  js/test/run.test.ts, docs/wire.md. (djs-docs' render.rs test keeps `docs/host.md` as a
  sample of a forbidden link.)
- decisions-index.md: removed 122 entries (fully superseded per the log's "Supersedes"
  lines, D3730's ranges filtered to dump titles, the pi adapter section and pi-adapter
  entries elsewhere, the compile cache, D2616/D2618/D2838/D2839); added the 148 decisions
  from D3260 to D3755 it lacked (in "Not yet sorted", log order); "Replaced or refined"
  now names the removals.
- Slicing: `djs run/resume --suspend-every`, the REPL's `.suspend-every` and djs-docs'
  example runner go on in the same VM (D3754); outputs unchanged (djs-docs check clean,
  djs-cli tests pass). Guide: cli.md, snapshots.md (intro and trade-off: images' uses),
  suspension.md, values.md, README.md (the runner), determinism.md ("Why it matters"),
  testing.md (the unchecked leg).
- Gate (D3753): "unsafe-heap difftest" runs `--ratchet` only. From the 0307 gate's times
  file: suspend-every 22.3 CPU-s, gc-stress 14.7, plain 1.1: about 37 CPU-s saved per
  Rust gate. Checked: the release-less dev runner with `--ratchet` alone passes.
- Scripts deleted: scripts/wasm-areas-unnamed.py, wasm-leb-padding.py,
  scripts/size/patches/{nobounds,onepath,optsize,touch}.py, scripts/perf/{cpuprof.py,
  libsym.sh, trace.sh, trace.py, trace_report.py, wasm_arm.py, perelem.py, nobounds.py}
  (referenced only by scripts/perf/README.md or not at all); perf README, setup.sh (no
  all.s/wasm.dis) and size README follow; base-families.rs names the touch patch as
  history.
- TESTING.md: the stress-free unchecked leg, no dump history in the snapshot programs and
  the size budget paragraphs. D3753-D3755 recorded.

## Follow-ups
- docs/architecture-bloat-audit.md and docs/size-vs-quickjs.md still cite the deleted
  scripts/size/patches commands (research reports, left as written).
- decisions-index.md's "Not yet sorted" tail (now ~560 entries) wants sorting into topics.
- docs/codemode-feature-usage.md still lists TS legacy decorators (from 0307).
- scripts/determinism experiments (probe/, deep_search.py, sandbox_interleave.ts) were out
  of this issue's list; the survey suggests them too.

## Result
Landed: the dump research archive is gone; docs/cli.md and docs/host.md are folded into
the guide, which is the one place for the CLI, the Sandbox and the ABI; the decision
index has no superseded or removed-component entries and covers the log up to D3755; the
CLI, REPL and docs runner slice in the same VM, and the guide describes images by their
real uses; the unchecked release difftest is unstressed (~37 CPU-s less per Rust gate);
unreferenced research scripts are deleted. djs-docs check is clean.

What pi must change: nothing (no API or module change).
