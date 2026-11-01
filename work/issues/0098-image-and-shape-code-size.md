# 0098: Base image and shapes code size (0083 part 4 + D016 follow-ups)

status: done
agent: heap
branch: issue/0098-image-and-shape-code-size
depends-on: 0083
touches: crates/djs-core/src/heap/ (compact_for_image, shapes, the atom index), crates/djs-core/src/bytecode/template.rs (slim privileged templates), crates/djs-prelude/, crates/djs-core/src/interp/run/ic.rs (code size only)

## Goal
0083's leftovers plus wasm-first size (D016): shapes added 12.2 KB of wasm code (data −8.6
KB), and part 4 was left optional: slimmer privileged (prelude) function templates
(~80 → 48 B each), the atom index load factor, and compact_for_image shrinking descriptor
and transition arrays (~1 KB). The base image is 398 KB of the 1.5 MB module.

## Acceptance criteria
- [ ] Part 4 items, each measured (base image bytes; wasm data and code; gzip).
- [ ] The shapes/IC code size reviewed with scripts/wasm-funcs.py: out-of-line cold paths,
      fewer monomorphized copies; no speed loss (wasm instructions, engine track).
- [ ] Determinism checks and image cross tests unchanged in behavior; images may change
      (build hash).

## Log

- Baseline (main 4301f893): base.img 398,216 B (gzip 110,014); wasm 1,505,773 (gzip
  530,198), code 924,028, data 577,451. Sizes from work/scratch/0098/measure.sh.
- Slim privileged templates (D1980). `cells!` got an optional `min N`: fields from N on
  are optional, and the walker skips the ones a cell lacks. FuncTemplate's debug fields
  (LINES, SCRIPT, SRC_*, NAMES) moved to the end; NATIVE/FAST are u16. A privileged
  template without lines/names is 56 B instead of 80. The IC field stays: builtins need
  their caches. Readers use `template::debug_info`, and traces know prelude frames by the
  PRIVILEGED flag. Base image −20,448 B (gzip −6.9 KB); code +0.2 KB.
- compact_for_image shrinks shape descriptor arrays (shared ones copied once) and
  transition arrays: −744 B.
- Atom index at 3/4 load: built (any length, `hash * len >> 32`; image index at 2/3 load):
  −5.1 KB. Computed-key lookups got ~10% slower and new atoms 8% slower, so it is not
  adopted (D1981); the patch is in work/scratch/0098/atom-load-34.diff.
- Code size of shapes/ICs (scripts/wasm-funcs.py on --keep-names builds, compared with
  pre-shapes 32b9f05):
  - the shapes code is about 10 KB, with no monomorphized duplicates: prop_insert,
    prop_remove, shape_child, become_dictionary, cached_add, verify's checks;
  - define sites now take an addition-only path (`ic_define`/`cached_define`) instead of
    inlining all of cached_put again (−0.5 KB);
  - `data_at`/`set_data_at` are `#[inline]` instead of `inline(always)` (−2.9 KB). Wasm
    got faster with it: global accesses −10%, proto method calls −5%, dictionary reads
    −9%. Native was unchanged.
- Tooling: `scripts/build-wasm.sh --keep-names` failed to link (wasm-ld rejects
  --compress-relocations with a name section); crates/djs-wasm/build.rs now skips that
  flag when CARGO_PROFILE_WASM_STRIP=false.
- Rebased onto 0093's run_hot: its define_field hit uses `ic_define` too, and its raw NATIVE read masks to the u16 (hot.rs, two
  line).

- Follow-up after the first merge (coordinator, from num's 0104): the heap's formatted
  invariant panics moved out of line, into heap/fail.rs (cold, inline(never), same
  messages). Covers cell_kind, the GC's kind check, object_class, object_slot and
  set_object_slot, the string-kind matches, the atom and handle checks, the hash index
  probe and the cell walker. Code 920,619 -> 918,782 B (-1.8 KB, gzip -0.9 KB). Wasm
  engine track unchanged (±3%, p-w +5% within noise).

## Follow-ups

## Result

Measured against main 4724fa20 (byte counts; timing at load 12–20, interleaved):

| | main | 0098 |
| --- | ---: | ---: |
| base.img | 403,248 B | 381,792 B (−21.5 KB, −5.3%) |
| base.img gzip | 111,244 | 104,201 (−7.0 KB) |
| wasm module | 1,510,342 | 1,486,565 (−23.8 KB) |
| wasm gzip | 533,094 | 524,931 (−8.2 KB) |
| code section | 923,278 | 920,955 (−2.3 KB) |
| data section | 582,729 | 561,273 (−21.5 KB) |

- Speed: native engine track within ±3% (LTO builds). Wasm engine track within ±3%;
  wasm microbenchmarks faster (globals −13%, method calls −6%, own reads −6%, own writes
  −4%, pi-like −9%).
- Checks: lib, e2e ×3, filtered test262, difftest ratchet (both backends), stress and
  determinism difftest over builtins/errors/language/codemode, native-vs-wasm, node tests,
  reader cross tests, clippy native and wasm32 release.
- Not adopted: the atom index at 3/4 load (D1981).
