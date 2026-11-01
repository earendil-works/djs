# 0095: Documentation coherence pass and a project README

status: done
agent: infra
branch: issue/0095-docs-coherence
depends-on: 0091
touches: README.md (new), ARCH.md, TESTING.md, docs/*.md, builtins/README.md, work/README.md (process docs only), no code

## Goal
ARCH.md was written before any code, and ~200 decisions have refined or changed it since
(docs/decisions.md). Other docs grew piecemeal. Make the documentation a coherent,
current description of what djs is and how it works, so a new contributor (or a new agent
after a context reset) can get oriented without reading the whole decision log.

## Acceptance criteria
- [x] `README.md` (new): what djs is, status (test262/difftest numbers, perf vs
      quickjs-wasi, wasm size: current measured numbers), how to build/run/test
      (djs run/repl/disasm/strip-types, scripts/build-wasm.sh, the gate), the crate
      layout, the JS host package, and pointers to the other docs.
- [x] ARCH.md reviewed section by section against the code and decisions: correct what has
      changed (shapes and dictionaries, native frames and @fast, the host inbox, fuel debt,
      deterministic mode, journals, OOM reserve/host OOM, the base snapshot, the dev flow,
      D016 wasm-first), and keep the original rationale. Mark superseded plans clearly
      (for example the old-generation delta plan, pending the owner's decision on 0088).
      Don't change designs; describe them.
- [x] TESTING.md, docs/host.md, docs/compiler.md, docs/objects.md, docs/cli.md, docs/wire.md
      checked for stale statements (numbers, removed workarounds, renamed flags), fixed.
- [x] A short "where to find things" index of the decision log by topic (docs/decisions.md
      stays append-only; the index goes in a new docs/decisions-index.md).

## Log
- docs/decisions-index.md: all 265 entries grouped into 24 topics, plus the notable
  replacements (D1160→D1440, D1112→D1500, D1347→D1530, D1850's OOM rule→D1910, ...) and
  the duplicated D1783 heading.
- ARCH.md: every section checked against the code and the log. Status line rewritten;
  crate layout as it is (8 crates, js/, tests/, scripts/, docs/); values (int32, bigint
  cells, internal refs); heap (layout macro, append ropes, shapes/dictionaries, 16-byte
  entries, ephemerons, the GC's no-collect-on-allocation rule, thresholds in pages and in
  the image, the OOM reserve, collect-and-retry D1700/D1900/D1910, host OOM, no
  large-object space yet); compiler (instruction table, peepholes D1860, TS/decorators,
  semantics: no modules, `using`); interpreter (fast paths, ICs, iter_next, Retry,
  fuel debt, the inbox); builtins (`@fast`, protocol shortcuts, the Rust backend as
  built, try/finally, fuel per transition); host interface (the real ABI in outline, the
  Sandbox and pi adapter, determinism and journals); snapshots (build hash, base image,
  determinism; the delta/old-generation plan marked Superseded, pending 0088's owner
  decision); budget table with measured values; hill-climbing as practiced; phases and
  open questions with outcomes.
- TESTING.md: group counts, pass numbers, fuel-based timeouts (D1790), qjs cache,
  corpus sources as they are (no harvested pi sessions), messages verbatim, the gate's
  actual steps, known differences (0090's three fixes, 40 entries).
- docs/host.md: size and startup at 0095, pi bench at 0095 (djs now faster than
  quickjs-wasi on the tool-heavy script with a prelude image), stale gaps replaced,
  pi's former engine gaps marked closed. docs/wire.md (host refs not built, wasm numbers
  exist), docs/cli.md (`--backend`), docs/objects.md (status: shapes built),
  docs/compiler.md (status header, for-of and array patterns with iter_next, BigInt,
  toString), builtins/README.md (try/finally and constructors in @native; @fast at the
  cap), work/README.md (per-agent worktrees).
- README.md (new): what djs is, measured status (test262, corpus, wasm instruction table
  vs quickjs-wasi, pi bench, size, startup), build/run/test, the JS host package,
  layout, docs map.
- Measurements for the docs (load 7-12): wmeasure over all scripts/perf/w workloads
  (work/scratch/0080/wmeasure-all.txt), sandbox.bench and pi.bench, wasm size from the
  0091 build, difftest and test262 counts from the 0091 gate run and `djs-test262 list`.

## Follow-ups

## Result
Docs only. New: README.md, docs/decisions-index.md. Reviewed and corrected: ARCH.md (now
describes the engine as built, cites decisions, marks the delta/old-generation snapshot
plan, host refs, the large-object space and fuzzing as superseded or not built),
TESTING.md, docs/host.md, docs/wire.md, docs/cli.md, docs/objects.md, docs/compiler.md
(status and the stale for-of/pattern sequences; its section 10 stays historical),
builtins/README.md, work/README.md. No design changes.
