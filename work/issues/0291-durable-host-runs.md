# 0291: Host-driven runs for durable code mode: a JSON boundary, exit, bounded steps, host data, dump compatibility

status: done
agent: pdurable
branch: issue/0291-durable-host-runs
depends-on: -
touches: js/src/sandbox.ts, js/src/wire.ts, js/src/reader.ts, js/test/, crates/djs-core/src/vm/ (host calls, abort), crates/djs-wasm (exports, if needed), docs/decisions.md, docs/guide
guide: docs/guide/js-api.md, docs/guide/dumps.md, docs/guide/pi.md, docs/guide/host.md

## Goal
pi's durable code mode will run scripts as djs host-driven runs (`Sandbox.begin` /
`loadRun` / `Run.settle` / `advance` / `dump`) with no prelude: every capability is a host
function named by an identity (`tools.read`, `text`, `store`, `console.log`, later
`@mcp/github#search_issues`). A script that waits on a tool that parks (a person's
approval, for days) is saved as a dump and resumed in another process, possibly weeks
later after djs, pi and the tool set changed. No journal, no replay.

A prototype of the whole stack works (owner's session, 2026-11-22):
- djs: branch `proto/pi-durable`, commit 111eaa64 (cherry-picked onto this branch):
  host errors carry their own data properties into the script's Error (a tool's
  `structuredOutput`), and a `values` option binds host data globals (`ALL_TOOLS`) that
  dumps name as `$djs.host(name)` instead of writing them.
- pi: worktree `~/Development/pi-mono-djs`, branch `djs-durable`:
  `packages/codemode/src/djs.ts` (the code mode VM over `Sandbox`),
  `packages/durable/src/codemode/djs-call.ts` (save before launch, park, resume),
  `packages/durable/test/djs-demo.ts` (two processes) and `test/codemode-djs.test.ts`.
  Read-only for you; it imports djs from this worktree's `js/src/sandbox.ts`, so keep the
  `Sandbox` API compatible or tell the coordinator what changed.

A review of the prototype (subagent "critic") found engine-side problems, which this
issue fixes. Without pi's prelude nothing serializes values at the host boundary:
`tools.ask({ when: new Date(0) })` reaches the host as a wire `Opaque` ref
(`{type:"object", ref:<heap address>, className:"Date"}`) whose address changes across a
dump and load, and `store("k", new Map())` stores one.

## Acceptance criteria
- [x] The prototype commit lands with tests: error data properties (own enumerable,
      not name/message/stack, wire values), `values` (deep-frozen, named in dumps, given
      again on load, a collision with a function or namespace identity is a misuse error).
- [x] A JSON host boundary, opt-in per sandbox (or per host function): arguments of host
      calls are converted with `JSON.stringify` semantics inside the VM (toJSON, Date ->
      string, Map -> {}, undefined/functions dropped from objects, `undefined` argument
      stays undefined); a value JSON cannot carry (cycle, BigInt) fails at the call site
      the way pi's prelude does: a sync call throws a TypeError the script can catch, an
      async call returns a rejected promise. Same rule for what sync functions such as
      `store`/`text` receive and for the run's result. Decide where it lives (engine
      native vs Sandbox) by cost; record the decision.
- [x] A host function can end the run so the script cannot catch it and no more script
      code runs (`exit()`): e.g. a sentinel the host function throws or a `Run` method;
      the run ends with a distinct state or `done` with a value, and its effects up to that
      point are what the host saw. Record the decision.
- [x] Bounded steps: `advance()` (and `begin`/`loadRun`'s first step) can be bounded and
      interrupted: an AbortSignal checked between fuel slices, a fuel budget per step,
      and yielding to the host's event loop between slices so an in-thread host stays
      responsive (`while (true) i++` must not freeze node). Tests.
- [x] Dump compatibility as an explicit exception to D024, recorded: what a stored dump
      may rely on forever (the `$djs.*` vocabulary: `k`, `resume`, `host`, `source`, `seed`
      and the helpers; identities are opaque host strings; `inlineSources` dumps need no
      script), and how that is guarded (tests/dump golden dumps; add golden dumps of
      host-driven code mode runs: open calls by identity, a captured host function, a
      `values` root, inlined source).
- [x] Guide pages updated (js-api.md host-driven runs, dumps.md, host.md, pi.md: how pi's
      durable code mode uses host-driven runs; the next-mode adapter's status).

## Notes
- Read the prototype's `packages/codemode/src/djs.ts` first: it is the client these APIs
  are for. Its gaps on the pi side (stable identities, inlineSources, the saved format,
  routing every async call through keyed tasks) are the coordinator's, not yours.
- `inlineSources` already exists; the prototype will use it, so check that a dump with an
  inlined body and `bindings` loads with no `sources`.
- Keep `js/pi/` (the next-mode adapter) working; do not extend it.
- Static imports in code mode bodies are the next issue, 0292 (same agent, after this).
- Merge with scripts/merge-to-main.sh as usual.

## Log
- Read the prototype client (pi-mono-djs `packages/codemode/src/djs.ts`,
  `packages/durable/src/codemode/djs-call.ts`) and pi's prelude for the semantics to match.
- Prototype landed with tests (js/test/durable.test.ts): host error properties are now
  defined *enumerable* (the prototype's `vm_define` made them hidden, so `Object.keys(e)`
  was empty, unlike the prelude): `vm_define` got a fifth argument `flags` (bit 0),
  `Vm::define_enumerable` in djs-core; unencodable properties are skipped. `values`:
  collision checks (identity, namespace, bound global) and the deep freeze moved into a
  sandbox helper function in the VM (no script compiled under `sandbox.js`). D3592.
- JSON boundary (`jsonBoundary`): Sandbox-made converting functions in the VM, host values
  by identity (dump roots first, before namespaces), the result converted by a second
  helper call in the run loop. Decided against an engine-native conversion by cost
  (JSON.stringify's full algorithm is JS; wasm size). D3593.
- `RunExit` (exported): thrown by a sync host function, thrown/rejected by an async one
  in sandbox-driven runs, or settled by a host-driven host; vm_abort before the next
  slice; done with `exited`. Journaled; replays. D3594.
- Bounded steps: `StepOptions { signal, fuel, yieldMs }` on `advance` and begin's first
  step, `begin(..., { start: false })` (state `ready`), new error kind `aborted`. A
  stopped step ends the run (`threw`). D3595.
- Dump compatibility: D3596 (plain dumps are an exception to D024; the vocabulary and the
  guard). Golden dumps of host-driven code mode runs: tests/dump/v2/golden-host/ (tools:
  4 idle points with source refs; inline: 3 with inlined body and bindings, loaded with no
  `sources`), js/test/golden-host.test.ts (both packagings). TESTING.md updated.
- Guide: js-api.md (options, errors with data, RunExit, host data, the JSON boundary,
  error kinds, bounded steps), dumps.md (`$djs.host` names, "What later builds keep"),
  host.md (`vm_define` flags, ending a run with vm_abort), pi.md (durable code mode on
  host-driven runs; the next mode's status). docs/host.md's export table.
- Checked: js tests (171), djs-wasm tests, clippy, djs-docs check, pi tests next mode,
  and the client's `test/codemode-djs.test.ts` with modules built from this branch.

## Follow-ups
- `runAsync`/tasks take no `StepOptions` (only host-driven runs do); add if a host needs
  them.
- The JSON boundary is per sandbox; per-function opt-in if a host needs it.

## Result
Landed in js/src/sandbox.ts (+ `vm_define` flags in djs-wasm, `Vm::define_enumerable`):
- `SandboxOptions.values` (host data; misuse on collisions), host error properties in the
  script's catch (enumerable).
- `SandboxOptions.jsonBoundary`: JSON conversion of host-call arguments and run results in
  the VM; failures at the call site (sync throws, async rejects).
- `RunExit` (export): `throw new RunExit(value)` in a host function ends the run
  uncatchably; `run.exited`; `run.settle(id, { ok: false, value: new RunExit() })`.
- `StepOptions { signal, fuel, yieldMs }`: `run.advance(step)`, `begin(code, params,
  { fuel, signal, start })`; `start: false` gives state `ready`; kind `aborted`.
- D3592-D3596; golden host dumps in tests/dump/v2/golden-host/.
Known gaps: see Follow-ups. A module built before this branch still works with the new
Sandbox, but defines error properties non-enumerable.
