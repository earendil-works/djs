# 0052: Code bloat investigation (report only)

status: in-progress
agent: bloat
branch: issue/0052-bloat-investigation
depends-on: -
touches: work/issues/0052-bloat-investigation.md (the report), work/scratch/0052/ (measurements), scripts/ (new analysis scripts only, no changes to existing ones). No changes to code or Cargo.toml.

## Goal
The owner wants to understand where djs's size comes from: "this comes out quite beefy
already". Investigate the wasm binary (and the native one, if useful) and write a clear
report: what takes the space, why, and what could realistically be done about it, with
numbers. This is an investigation: no code changes. Proposals go to the coordinator.

## Context
- ARCH G2 budget: < 400 KB release wasm after wasm-opt (quickjs.wasm is ~850 KB).
  wasm-opt/binaryen is not installed (the owner decides whether to install it).
- 0043 (heap agent) did a first pass: stripped module 1,200 → 948 KB, base image 502 →
  288 KB. Its tools are `scripts/wasm-size.py` (code per crate/module/function from the
  name section) and `cargo run -p djs-prelude --example image-size` (base image per cell
  kind). Read work/issues/0043-wasm-size.md for its attribution and the rejected ideas
  (compact image encoding: decoding is too slow; opt-level z: 15–60% slower; per-crate
  opt-levels: ignored under LTO).
- Since then BigInt (+35 KB code), RegExp, Proxy, classes, typed arrays and more
  landed. Build with `scripts/build-wasm.sh` (profile wasm, opt-level s).

## What to deliver
- [ ] The current numbers on main: code / data / names / total, stripped and gzipped,
      plus the base image per cell kind, and the top contributors by crate, module and
      function. How the size developed over the project history (sample a few commits
      along `git log` and build them in your own worktree: the 0019 bench `size` track
      in tests/scores/bench.tsv has some history too).
- [ ] Root causes, not just sizes. For example: monomorphization (generic code
      instantiated per string width or per type), inlining at opt-level s (the dispatch
      loop and `call_intrinsic`), big match statements, Unicode/case/property tables,
      libm, alloc/core generics, panic/format machinery, debug-only code that leaks into
      release, duplicated logic between the Rust leaves and the builtin-language
      prelude, the base image's content (bytecode vs constants vs property tables vs
      atoms vs strings), and anything else you find.
- [ ] A comparison point: quickjs-wasi's wasm in pi-private's node_modules (read-only),
      the same metrics for it, and where djs is bigger or smaller per area (parser,
      interpreter, builtins, regex, number formatting, unicode).
- [ ] Ranked opportunities: for each, the expected saving, the cost (performance,
      effort, risk), and whether it needs wasm-opt numbers first. Separate quick wins
      from structural changes.

## Notes
- Your own worktree (`.worktrees/0052-bloat-investigation`) for builds, so you don't
  disturb other agents. Builds are expensive on this shared machine: reuse your target
  dir and build historical commits sparingly.
- Use `scripts/timeout.sh` for long runs.

## Log

## Findings

## Opportunities

## Result
