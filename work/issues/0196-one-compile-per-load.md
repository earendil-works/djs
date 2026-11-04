# 0196: Load performance: one compile per load (state tables kept or cached)

status: done
agent: sm
branch: issue/0196-one-compile-per-load
depends-on: 0194
guide: -
touches: crates/djs-core/src/compiler/, vm/dump_load.rs, scripts/dump-costs

## Goal
docs/suspension-design-review.md, section 4 "Migration plan", step 8; decisions D2720
(state machines in the compiler, SM), D2721 (slot naming construct@span), D2722 (host-call
state with save/resume hooks). Read the whole report first: it describes this step's
design, what stays and what goes, the risks and the tests required at every step:
smap pins and golden dumps (rewritten only with a decision, D2619), DJS_E2E_MODE=dump,
difftest --dump-every-idle, the harvested/private idle-point counts at 100%
(vm::dump_harvest), test262 async/generator suites where relevant, benches before/after
where the report says so. Each step merges green on its own.

## Log
- The load compiled the dump text to run it, then again (recording on) at the first
  `$dump.resume` for the maps. Recording leaves the code unchanged (pin test), so
  `load_dump` now compiles format 2 text once with `compiler::compile_with_maps` and keeps
  the maps in `Load` (`Load::text`/`typescript` and the lazy path gone). Format 1 keeps
  the plain compile.
- Profiling the recording compile (macOS `sample`): the extra cost was a second
  `stack_depths` walk per function, also for functions without sites, and an
  O(sites x insns) search for the sites' instructions. Now functions without sites skip
  the analyses (they only carry computed keys) and the suspending instructions are found
  in one pass. Recording overhead over a plain compile: ~10% on harvested dumps, 0 on a
  12 MB data-only dump, so no gating on the text having activations.
- Numbers (native release, 1,164 harvested idle-point dumps, 23 MB; temporary timing
  test, not committed): plain compile 343 ms; old load path compile + maps 711-741 ms;
  compile_with_maps 377-381 ms; load_dump total 450-463 ms vs ~800 ms before (0.56x).
  Harvest idle test 12.2 s -> 7.7 s.
- New unit test `dumping_a_loaded_dump_writes_the_same_text` (review section 7's fixed
  point) over the round-trip fixtures at every idle point: passes.
- Checks: lib tests, clippy, e2e dump, harvest 1164/1164, private 5257/5257 idle points,
  difftest --dump-every-idle --suspend-every 97 --gc-stress OK. Main/slim wasm sizes
  unchanged (no dumper in them). scripts/dump-costs not changed or rerun (wasm numbers in
  the guide, "loading about 1 ms", stay an upper bound).
- Docs: continuation-dumps.md (resume, suspension maps), D2735. Guide: none affected.

## Follow-ups
- Compiling large data dumps is ~100 ns/byte (12 MB literal dump: 1.2 s native): the
  compiler's literal path may be worth a look for big data dumps.
- The exporter still compiles each script again per dump (`script_maps`, plus the
  codemode wrapper in `body_site`); a per-VM map cache could cut dump time if it matters.

## Result
A load compiles the dump once: the compile that runs format 2 text also records the
suspension maps (D2735); about 0.56x the previous native load time on harvested dumps.
