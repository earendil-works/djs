# 0071: GC points and raw images independent of fuel slice size

status: done
agent: num
branch: issue/0071-gc-points-slice-independent
depends-on: -
touches: crates/djs-core/src/vm/ (run start), crates/djs-core/src/interp/ + regexp/builtins (progress records), crates/djs-core/src/heap/ (threshold accounting)

## Goal
From the determinism investigation (0055): see work/issues/0055-determinism-investigation.md,
"Proposed issues", for the full goal and acceptance criteria. Experiment tools are in
scripts/determinism/.

## Acceptance criteria
- [x] Remove the collection at the start of `run`. Progress records of suspended regexps/builtins don't count towards the GC threshold. Raw images are identical at slices max/997/101/1.
- [x] The criteria in 0055's report for this item.
- [x] Ratchets unchanged or better, stress modes identical, decisions recorded.

## Log
- Baseline (detprobe, slices max/997/101/1): alloc_only differed at slice 1, regexp.js had 4
  raw images, regexp_props 4; iteration_runtime was already stable on this main.
- Removed the run-start safepoint; suspension records (regexp exec/split/replace) are
  allocated unbudgeted (no cap, no gc_pending) and truncated off the arena top when the
  intrinsic resumes (`Heap::begin_unbudgeted`/`end_unbudgeted`/`release_top`,
  `Arena::truncate`). D1710.
- Sweep (release detprobe, slices max/997/101/13, restore on/off): all 67 e2e +
  determinism cases have one raw image. New `crates/djs-core/tests/slices.rs` (raw image,
  output and total fuel at max/997/101/1 + restores; ignored sweep over all e2e cases).
- determinism's notes (via coordinator): (1) fuel vs collection points: total fuel was
  already equal at all slices in my append/JSON/Array.from workloads even with the run-start
  collection put back (asked determinism for their workload); tests added anyway. (2)
  regexp: SCAN/STAR were additive already; the actual leak was BACKREF's saturating_sub.
  Regexp fuel is now i64 and overdraws (D1711); unit test over chunk sizes.
- wasm: +387 B raw / +158 B gzip; regexp-scan +0.7% instructions in node (noise ~0.5%),
  regexp/loop/strbuild within noise.

## Follow-ups
- 0072 part 2 (determinism): compare raw images in e2e (plain vs suspend) and difftest
  `--determinism`; `tests/slices.rs` already does it for selected cases.
- scripts/determinism/probe (detprobe) no longer builds on main (no arm for
  `Status::HostOutOfMemory`); left to determinism (0072 touches scripts/determinism/).

## Result
GC points and raw heap images no longer depend on the fuel slice (D1710): `run` does not
collect at its start, and progress records of suspended regexp intrinsics are allocated
outside the cap and the GC threshold and freed off the arena top when the intrinsic
resumes. Regexp fuel is an i64 that a backreference may overdraw, so its total no longer
depends on the slice (D1711). Raw images, outputs and total fuel are identical at slices
max/997/101/13/1 with and without restores for every e2e and determinism case;
`crates/djs-core/tests/slices.rs` keeps that checked (plus an ignored all-e2e sweep).
wasm: +387 B raw / +158 B gzip, regexp-scan ~+0.7% instructions.
