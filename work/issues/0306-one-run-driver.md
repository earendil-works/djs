# 0306: One way to drive a run; determinism kept as the fallback for refused snapshots

status: open
agent: pclean
branch: issue/0306-one-run-driver
depends-on: 0305
touches: js/src/sandbox.ts (and friends), js/test/, js/pkg-test, examples/sleep-snapshot, crates/djs-docs/assets/live.js, crates/djs-wasm (ops), crates/djs-core (only what the API needs), docs/guide (js-api, persistence, determinism, host, pi), docs/decisions.md
guide: docs/guide/js-api.md, docs/guide/persistence.md, docs/guide/determinism.md, docs/guide/host.md, docs/guide/pi.md, docs/guide/goals.md

## Goal
pi drives scripts as host-driven runs only (`begin`/`settle`/`advance`/`snapshot`/
`loadSnapshot`, see ~/Development/pi-mono-djs/packages/codemode/src/run.ts). Make that the one
driver of the Sandbox, delete what only duplicated it, and **keep determinism as a
first-class feature**: the owner: "pi does need to re-run with determinism; deterministic
re-runs are the recovery for when snapshots do not work any more (need a fallback). To the
extent possible, running a script a second time has to be as side-effect free as possible."

Delete:
- Tasks (`start`, `suspend`, `checkpoint`, `Sandbox.resume`, `sandbox.resume`) and the
  `AsyncHostFunction` save/resume hooks; `debugger` becomes an option for an async function
  the host settles like any other; rewrite the docs site's pauses (live.js), the npm smoke
  test and examples/sleep-snapshot on host-driven runs.
- `run`/`runAsync` keep their API but become thin loops over the host-driven driver (one
  driver: `auto()`, the RunCore hooks and the task loop go).
- `bindings` (imports replaced it), `setGlobal`/`defineFunction`/`defineAsyncFunction` (define
  everything through options; namespaces freeze in the constructor), object-form module
  exports (`HostModuleFunction`), scripts by hash on the JS side (`sources` option,
  `SnapshotOptions.sources: false`, the process-wide `ranScripts`), and make
  `unhandledRejections` `"warn" | "throw" | "ignore"` (pi's "collect" reads every reason and
  never takes them; check pi's use and tell the coordinator what pi must change).
- The snapshot ops' host-handle sections that the Sandbox never uses (ABI 12).

Keep and strengthen (the fallback):
- `deterministic`, journals, `replay`, canonical hashes, strong weak refs, per-result clocks.
- Determinism for **host-driven runs**: a deterministic run (seed, start time, the clock of
  each delivered result) begun again from its script, with the same results delivered in the
  same order, makes the same calls with the same arguments and reaches the same state at
  every idle point as the original and as a snapshot loaded there (compare by canonical hash,
  on the harvest corpus and the snapshot programs). A host can drive such a re-run without a
  journal of its own beyond (call id, result, clock) per delivery; provide what makes that
  easy (e.g. `Run.journal()` or a delivery log in host-driven runs, and a check that the
  re-run's calls match, reporting the first divergence). Decide and document what "as side-
  effect free as possible" means at the engine level: no host function runs during replay
  except the sync output/store functions the host provides (which it can stub), async calls
  come from the log.
- Snapshots of deterministic runs carry what a re-run needs to stay deterministic after a load
  (seed state, clock), and a run loaded from a snapshot stays deterministic.
- The guide's determinism page describes determinism as the fallback when a snapshot is
  refused (a newer build compiles a waiting function differently): re-run the script with
  the recorded results, with the divergence check; goals.md keeps "recorded and replayed".

## Acceptance criteria
- [ ] One driver; the deletions above; tests, examples, docs site and smoke test on it.
- [ ] Determinism tests for host-driven runs: re-run equivalence with snapshots on the corpus,
      divergence reported, re-run across a fuel-slice change.
- [ ] Guide pages rewritten; decisions recorded (superseding D2771, D2972, the hook parts of
      D2770/D3722, etc.); what pi must change listed in the Result.

## Log

## Follow-ups

## Result
