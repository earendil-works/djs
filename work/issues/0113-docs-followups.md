# 0113: Docs follow-ups: stale reference docs, status numbers, search, live disassembly

status: in-progress
agent: docs
branch: issue/0113-docs-followups
depends-on: 0103
guide: docs/guide/ (status numbers, search), ARCH.md, docs/compiler.md
touches: ARCH.md, docs/*.md, docs/guide/, crates/djs-docs/, crates/djs-wasm/ (a docs-only feature export for disassembly)

## Goal
0103's follow-ups:
1. Stale reference docs: ARCH "Interpreter" still names fast_step/fast_more (D1930
   replaced them with run_hot); docs/compiler.md 7.1 gives the frame header as 5 words (it
   is 6, prev_hp); ARCH's table says vm_new is "a few µs" (D1990 measured 39–40 µs). Do a
   pass for similar drift.
2. Generated status numbers (test262/difftest counts from the ratchet files, perf vs
   quickjs-wasi from the bench scores, wasm size) as gen regions, used by README.md and
   the guide, so prose numbers stop going stale.
3. Search on the site (client-side, small).
4. Live disassembly for edited examples (a `djs_disasm` export behind a docs-only djs-wasm
   feature, so the shipping module doesn't grow).

## Acceptance criteria
- [ ] Each item merged separately; `djs-docs check` green; the shipping wasm unchanged in
      size.

## Log

- Part 1 (stale reference docs, drift pass):
  - ARCH "Interpreter": the fast paths are `run_hot` (D1930/D1931), `fast_step`/
    `fast_more` only in tests; the `@fast` entry runs after `run_hot` declines a call.
    ARCH's performance table: `vm_new` about 40 us (D1990).
  - docs/compiler.md 7.1: the 6-word frame header with `prev_hp` (D1151, frame.rs),
    `6 + LOCALS + STACK`; 2.1/2.2 and 3.1/3.7 note the as-built names
    (`FuncState` fields, `scan::is_arrow_head`, `Parser::lvalue` + `truncate_last`).
  - Drift pass, mechanical: every repository path in ARCH, README, TESTING, AGENTS,
    work/README, builtins/README, docs/*.md and the guide exists (fixed TESTING's bgen
    golden path); every backticked Rust-like identifier there was grepped in the code.
    The leftovers are QuickJS names, the original design sketches in compiler.md (now
    marked as such) and ARCH's illustrative `array_map`/`vm_restore`.
  - Numbers in prose (README status, ARCH tables, corpus and test262 counts) are left
    for part 2, which generates them.

- Part 2 (status numbers, D2101): `gen:status conformance|speed|size|inventory` in
  djs-docs (crates/djs-docs/src/status.rs). Counted on every regen: test262 active/total
  (djs-test262's grouping over vendor/test262; no checkout: the region keeps its text,
  `check` warns), both test262 ratchets, corpus cases/skip-djs/ratchet, the base
  snapshot, builtin tags, opcodes/cells/intrinsics. Recorded by `scripts/status.py`
  (size, speed, `set --at`) in `tests/scores/status.tsv`, shown with their commit.
  README.md is now a djs-docs file (regions only, not rendered): its Status section is
  generated. New guide page "Status in numbers" (Reference). ARCH points to README for
  current counts. Recorded now: size and speed at d780463b0d (wmeasure n=3), pi 221/226
  at 0066. `check` takes ~2 s more (the test262 listing).
  The README's speed rows now name their workload files; two rows changed meaning
  (push/pop is push-pop.js with string methods; JSON with Dates is json-date.js).

- Part 3 (search): `build` writes `search.json` (crates/djs-docs/src/search.rs: one
  entry per h1/h2/h3 section of every page and the decision log, plain text, diagrams
  and the live bytecode pane left out; 428 sections, 530 KB, 190 KB gzipped, fetched on
  the first focus of the box). assets/search.js: all query words must occur, headings
  and page titles rank first, snippets with highlights, arrows/Enter/Escape, `/` to
  focus, `#search=words` in the URL opens with results. Checked in headless Chrome,
  light and dark, wide and narrow.

- Part 4 (live disassembly): djs-wasm feature `docs` exports `djs_disasm(goal,
  params_len, src_len)` (compiles into a scratch heap, the listing or the compile error
  in the out buffer). `scripts/build-wasm.sh --docs` builds it into target/docs-wasm so
  it never replaces the shipping module (touches scripts/build-wasm.sh: one option);
  `djs-docs build` prefers it; the Pages workflow builds it. live.js: Run also replaces
  the Bytecode tab with the listing of the code that ran; with the shipping module the
  tab keeps the build-time listing (feature detection on the module's exports).
  Shipping module unchanged: 1,471,134 bytes, the same as recorded at d780463b0d, no
  `djs_disasm` in it; the docs module is 1,516,619 bytes (+45 KB, the disassembler).

## Follow-ups

- The gate never compiles djs-wasm's `docs` feature; a break would show first in the
  Pages workflow. If that matters: add `--features docs` to the gate's wasm32 clippy step.

## Result
