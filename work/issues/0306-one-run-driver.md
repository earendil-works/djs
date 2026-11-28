# 0306: One way to drive a run; determinism kept as the fallback for refused snapshots

status: done
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
- [x] One driver; the deletions above; tests, examples, docs site and smoke test on it.
- [x] Determinism tests for host-driven runs: re-run equivalence with snapshots on the corpus,
      divergence reported, re-run across a fuel-slice change.
- [x] Guide pages rewritten; decisions recorded (D3745-D3747); what pi must change listed in
      the Result.

## Notes
Consumers to check every change against (all in ~/Development/pi-mono-djs, branch
djs-durable; only packages/codemode imports djs, but it serves all of them):
- packages/codemode: src/run.ts (Sandbox options, host-driven runs, snapshot/loadSnapshot,
  fuelUsed, dropped, RunExit, SandboxError kinds), script-values.ts (Opaque), wasm.ts
  (loadModule), cloudflare.ts (`@earendil-works/djs/wasm/djs.wasm`), runtime/execution.ts
  (`execute()` on the same runs), runtime/remote.ts (Cloudflare/Durable Objects: a
  stateless RPC per step that loads and saves a snapshot every step), examples/cloudflare.
- packages/durable: src/codemode/call.ts (saves before launches and parks, resumes after
  restarts and wakes), the remote stepper for Durable Objects.
- packages/coding-agent: extensions/codemode/execute.ts (in-thread execute()), the Bun
  binary's wasm path (src/bun/runtime-setup.ts, config.ts).
- apps/p2: plugins/codemode (durable code mode on the session worker thread; resume after a
  worker kill, test/p2/codemode.test.ts), plugins/rollups (execute() with a custom global).
- apps/p3: code mode planned ("replay after restart"), on durable: relies on the
  determinism fallback.

## Log
- ABI 13 (djs-wasm): `vm_snapshot` staging = functions, values, data (no flags, no handles;
  sources always stored); `vm_load_snapshot` takes no candidate sources and returns calls
  and data only; `djs_snapshot_needs` returns the host names only and refuses a snapshot
  that stores a script by hash.
- sandbox.ts: one driver. `run`/`runAsync` loop over the host-driven run (async host
  functions are called at the next idle point now, not at the call); `auto()`, RunCore
  hooks, Tasks, `AsyncHostFunction`/`HostCall`, `Sandbox.resume`/`sandbox.resume`, the
  `suspended` kind, `bindings`, `setGlobal`/`defineFunction`/`defineAsyncFunction`,
  `HostModuleFunction`, `sources`/`SnapshotOptions.sources`/`ranScripts`, `collect`/
  `takeUnhandledRejections` gone; `unhandledRejections: "ignore"`; `debugger: true | fn`.
  Namespaces freeze in the constructor (last member defined again with DEFINE_FREEZE). A
  settle during a step waits for its next slice (`pending`), so the VM is never touched
  re-entrantly.
- Determinism: checked first that the VM applies inbox results one at a time after the job
  queue drains (promise.rs `inbox_apply`, D1670), so batching does not matter and a log of
  (id, fn, args, outcome, clock) per result suffices. Added `Run.deliveries` (deterministic
  sandboxes), `BeginOptions.deliveries` (re-run, `diverged` at the first difference),
  `Settlement.now`, `filename` in the journal's run entry; `replay` is a loop over the
  driver too. Snapshot of a deterministic run: clock and random state travel (verified),
  and a non-deterministic sandbox refuses it (strong weak references).
- Found: the canonical hash of a VM loaded from a snapshot differs from the original's (fresh
  layout), but the snapshot bytes are equal; and snapshotting a loaded VM again is not
  always byte-identical (h002: one extra string in the re-snapshot). So the corpus test
  compares re-run vs original by hash and snapshot bytes, and loaded runs by calls/output.
- js/test/determinism.test.ts (new, ~15 s): every harvest case, re-run to every idle point
  (1164) from its deliveries on fuel slice 997: output, calls, canonical hash and snapshot
  bytes equal; whole re-runs end the same (slice 1009); a snapshot loaded in the middle and
  continued with recorded results and clocks makes the same calls/output to the same end;
  divergence messages; deterministic-only loads.
- Tests rewritten: task.test.ts deleted; debugger, bindings (globals + imports), host-values,
  imports (string exports), snapshot, run, durable, sandbox, sleep-snapshot, pi-setup
  ("ignore"). examples/sleep-snapshot rewritten on host-driven runs (states carry the
  deadline), README too; npm smoke test on begin/snapshot/loadSnapshot; the docs site's
  `debugger;` pauses (live.js) on host-driven runs (checked with a node harness with a stub
  DOM: three pauses, three loads, the transcript as before); djs-docs' native snapshot
  examples store sources, as the Sandbox does (persistence.md/suspension.md regenerated).
- Guide: js-api (tasks section gone, "Runs the host drives", "Deterministic runs" with the
  fallback), persistence, determinism (new "When a snapshot is refused", engine-level
  side-effect rule), host (ABI rows, builds), pi, goals, language, typescript; docs/host.md,
  js/README.md, TESTING.md, ARCH.md follow.

## Follow-ups
- Snapshot of a loaded run is not always byte-identical to the snapshot it was loaded from
  (h002-bash-read.js: one more string, 5 bytes): load + snapshot should be idempotent; find
  which string is duplicated (likely a rope/slice or an atom that becomes a separate string).
- `Run.deliveries` keeps every result's value for the run's life in deterministic
  sandboxes; a host with huge tool results that keeps its own log may want to turn it off.

## Result
Landed: the Sandbox has one driver, the host-driven run; `run`/`runAsync`/`replay` are
loops over it. Tasks, hooks, bindings, post-construction definitions, object-form module
exports, hashed scripts on the JS side and `collect` are gone (D3745). Deterministic
re-runs are the documented fallback for refused snapshots: `Run.deliveries`,
`begin(code, params, { deliveries })` with divergence reporting, `Settlement.now` (D3746).
ABI 13 drops the snapshot ops' handle and hashed-script sections (D3747).

What pi must change (pi-mono-djs packages/codemode/src/run.ts):
- `unhandledRejections: "collect"` -> `"ignore"` (the only API change pi hits; the type
  rejects "collect").
- Ship the new module with the new package (ABI 13).
- To use the fallback: create the sandbox with `deterministic: { startedAt, seed }` (and keep
  `now` as the host clock), keep `run.deliveries` with each saved snapshot (concatenating
  across loads: a loaded run's `deliveries` start empty), and when `loadSnapshot` refuses,
  `begin(code, {}, { filename, deliveries })` in a fresh sandbox, stubbing `text`/`image`/
  `store` (their effects are already in the saved output and writes) and answering `load`
  from the saved store; then go on with `run.calls` as after a load. Not adopted by pi yet.
Nothing else: pi uses only begin/settle/advance/snapshot/loadSnapshot, string module
exports and plain async functions.
