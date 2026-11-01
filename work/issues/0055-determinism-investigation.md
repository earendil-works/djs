# 0055: Determinism investigation (report only; becomes longer-term issues)

status: open
agent: -
branch: issue/0055-determinism
depends-on: -
touches: work/issues/0055-determinism-investigation.md (the report), work/scratch/0055/ (experiments), scripts/ or tests/ (new experiment scripts only). No changes to engine code.

## Goal
Codemode needs a high level of determinism. External effects (tool results, their
contents) cannot be deterministic, but everything else should be: given the same script,
the same engine build, and the same sequence of external inputs (tool results and the
order in which they arrive, the clock values and seeds the host supplies), djs must
produce the same observable behavior, the same outputs, and ideally byte-identical heap
images/snapshots. That should hold across runs, across machines and native vs wasm, and
across snapshot/restore and suspension at any point.

Investigate what is still needed in the design and the implementation to make that fully
true, with particular attention to promises and the job queue: the order of reactions,
and how the settlement order of concurrent async host calls interacts with the
microtask queue (pi's deterministic mode delivers results in a fixed order; see below).

## Context
- ARCH "Nondeterminism": `Math.random` seed, `Date.now`, `performance.now` come from host
  imports or heap state. Done so far: `Vm::set_now`, `set_time_zone_offset` (D1383),
  `set_random_seed` (D1314, D1491), and no clock or entropy of djs's own.
- pi's codemode has a deterministic mode: `~/Development/pi-private/packages/codemode/src/
  runtime/vm.ts` (read-only), top comment and `deterministicWasi`, `VirtualClock`,
  `DETERMINISTIC_VERSION`, and how results are applied in order ("the next result to
  apply, and results that arrived before it"). infra's adapter (js/pi/, D1490–D1495,
  docs/host.md "pi codemode") maps it onto djs.
- The GC is deterministic (D1090: collecting twice gives identical bytes), and the base
  image is deterministic (0032). The `gc_pending` threshold depends on heap size.
- WeakRef/FinalizationRegistry are being added (heap, 0051): they make GC timing
  observable.

## What to deliver
- [ ] An inventory of every source of nondeterminism or platform dependence, checked
      against the code, each classified (already deterministic / deterministic given host
      inputs / a gap). Candidates (not exhaustive):
  - the job queue and promise reaction order; async host call settlement order and
    timing relative to fuel slices (does resolving a call between two `run` slices give
    a different interleaving than resolving it inside the same slice?); `Idle`
    handling; host call ids;
  - fuel accounting: identical across native/wasm and opt-levels? Do fuel costs depend
    on anything but the bytecode (for example GC work, string flattening, rope depth,
    hash table probes)? Where the embedder suspends (the fuel slice size) must not
    change the observable behavior;
  - GC: when collections happen (thresholds based on heap size, the arena growth
    policy), and what is observable about them (WeakRef deref,
    FinalizationRegistry callbacks, memory-limit OOM points, performance);
  - heap images: byte-identical given the same history? Across native and wasm (0032
    showed the base matches)? Across a snapshot/restore in the middle of a run?
  - floating point: libm (pure Rust, so the same on every platform?), NaN
    canonicalization and NaN payloads observable through typed arrays / DataView, `-0`,
    fused multiply-add or x87-style differences in wasm vs native, `Math.*` results;
  - number/string conversions, `Array.prototype.sort` stability and comparator call
    order, `Object.keys` order, Map/Set order (spec-defined), hash seeds (are any hash
    functions seeded randomly? HashMap with RandomState anywhere in djs-core?), atom ids;
  - Date (time zone, DST: fixed offset), `toLocaleString`, `localeCompare`, `normalize`;
  - stack traces, error messages, `Function.prototype.toString`;
  - RegExp backtracking limits and fuel (input-size dependent but deterministic?);
  - memory limits: is the OOM point deterministic for a given input?
  - anything the host can observe differently (timing aside).
- [ ] Experiments where cheap: run the same scripts natively and in wasm, with different
      fuel slice sizes and suspension points, with and without GC stress, and compare
      outputs and heap images byte for byte (the e2e harness and the difftest stress
      modes do part of this; extend them in scratch).
- [ ] Design proposals for the gaps: for example a deterministic job/host-call delivery
      contract (results enqueued only at well-defined points, ordered by call id or by
      delivery order), deterministic GC triggering (allocation-count based instead of
      heap-size based?), a "deterministic mode" flag in the VM that turns off or pins
      anything else (WeakRef liveness only at defined points, and so on), a replay journal
      format, and a determinism test mode in the gate (run every corpus case twice with
      different slice sizes and compare the images).
- [ ] **A list of proposed issues**, each with a title, goal, acceptance criteria and
      dependencies, ready for the coordinator to turn into work/issues files. Don't
      create issue files yourself.

## Notes
- Your own worktree for experiments (`.worktrees/determinism`).
- Read AGENTS.md "Testing while iterating" first. The machine is shared: use the 6-job
  cap and filtered runs.

## Log

## Findings

## Proposed issues

## Result
