# 0303: Snapshots for hosts: Sandbox API, CLI, REPL, docs-site pauses, pi's setup

status: done
agent: psnap
branch: issue/0303-snapshot-hosts
depends-on: 0301
touches: js/src/sandbox.ts, js/test/, crates/djs-cli, crates/djs-docs (live examples' pauses), docs/guide (js-api, cli, host), examples/
guide: docs/guide/js-api.md, docs/guide/cli.md, docs/guide/host.md, docs/guide/pi.md

## Goal
Hosts use snapshots instead of dumps.

## Acceptance criteria
- [x] `Run.snapshot({ sources?: false })` -> bytes, `Sandbox.loadSnapshot(bytes, { sources })`
      -> Run with its open calls (same ids, states as dumps carried them), tasks'
      `suspend()`/`checkpoint()` and `Sandbox.resume` on snapshots; sources by hash with a
      host lookup; `describe(bytes)` for people.
- [x] **pi's setup through snapshots**: the Sandbox's namespaces, aliases, missingMember
      proxies, jsonBoundary converting functions, `values`, module namespaces (0292), RunExit,
      fuelUsed across a load, run as in pi's code mode; harvest cases with that setup.
- [x] CLI: `djs snapshot` (or `run --snapshot-after K`), `djs resume`, `djs describe`;
      the REPL's `.snapshot`/`.restore` (no `.dump` once 0304 lands).
- [x] The docs site's `debugger;` pauses continue from a snapshot; `debugger` no longer
      depends on the `dump` feature.
- [x] examples/sleep-dump becomes a snapshot example (one process suspends, another resumes).
- [x] Guide pages updated (a new persistence page may wait for 0304).

## Log

### Design (D3720)
- A host's run is snapshotted in *host mode* (`SnapshotOptions::host: Some(HostNames)`), as
  dumps named host values (D2971): host values (namespaces, `values`, converting functions,
  module namespaces) are written by name, and the objects reachable from them through own
  data properties as members (`ALL_TOOLS.0.schema`), never their content; host functions by
  identity wherever they are (also an open call's function: `HostCall.FN` has type `hostfn`);
  the handle table is not written, only the handles the host names (`named`), plus the host's
  data string (`hostData`: call states, fuelUsed, clock).
- Such a snapshot loads only *into the host's own VM* (`Vm::load_snapshot(base, bytes,
  sources, &HostValues)`): a fresh sandbox made with the same options; its host values and
  handles stay its own, the run's state comes from the snapshot (patches over its global,
  Interp state, host calls, realm per-VM values). A plain VM snapshot (handles by index)
  loads only with `from_snapshot`; each refuses the other.
- Why not the whole heap: pi's ALL_TOOLS (every tool's description) and the namespaces with
  their converting closures would be in every snapshot (KBs to tens of KBs), and the
  sandbox's JS-side tables (handles, fn ids) would have to travel too.
- `needs(bytes)`: the scripts stored by hash only (names) and the host names, so the host can
  find sources and make lazily-made module values before the load.
- The Vm record carries `frozenBuiltins` and `weakRefsStrong`; a load into a host VM with
  another mode refuses; from_snapshot sets them (vm_from_snapshot no longer forces frozen).
- FORMAT 2.

### Lazy functions (D3721)
The shipped module compiles functions lazily (D2990, D2993): every snapshot refused ("a lazy
function template"). Now: the export writes a lazy template like any (span + interface
hash); the import compiles every script eagerly, so functions a lazy parent made at its
first call exist; the interface hash reads only the capture names (a lazy template that
captures nothing has no names table) and drops synthetic bindings' numbers (`<fields 3>`:
counted per compiled unit). DJS_LAZY=1 harvest: 1164/1164 same, programs 197/197; the gate
leg "snapshots" runs both modes.

### Steps
- WIP commit 76128a88: export/import host mode, `load_snapshot`, `needs`, host_tests.rs.
- Host values named at every depth (dumps: two levels): `const t = ALL_TOOLS[0].schema.x`
  keeps its identity.
- Lazy functions (above); gate leg "snapshots" runs DJS_LAZY=1 too (scripts/gate.py, outside
  touches: one line).
- djs-wasm ABI 10: ops 65 `vm_snapshot(vm, len)` (flags + host names in staging), 66
  `vm_load_snapshot(vm, len)`, 67 `djs_snapshot_needs`, 68 `djs_snapshot_describe`;
  `vm_from_snapshot` gone (no users). `debugger_awaits`, `vm_call_site`, `Vm::call_site` no
  longer need the `dump` feature.
- BaseInfo in wasm: the instance's first VM took the base as its arena, so the sandbox hands
  the base over again (`provideBase`) before its first snapshot operation, and
  `BaseInfo::from_owned` keeps no copy (no image: host loads only).
- sandbox.ts: `Run.snapshot({ sources?, states? })`, `sandbox.loadSnapshot(bytes)` -> Run
  (fuelUsed and the deterministic clock travel in the host data), `resume(Uint8Array |
  dump)`, `Task.suspend/checkpoint(opts)` -> Uint8Array, `describeSnapshot(bytes)`. A Rust
  refusal leaves the sandbox dead; missing sources / host values are checked first (sandbox
  stays usable).
- Tests: task, debugger (callSite after a resume is in the body again: loop.js:6:3),
  bindings (node part dropped: it ran the dump in node), new js/test/snapshot.test.ts (pi's
  setup: aliases, missingMember, values identity, jsonBoundary, module imports, RunExit,
  fuel across loads; pi-like snapshots 3.0-4.2 KB with the TS source inside).
- Docs site: live.js suspends `debugger;` pauses into snapshots, shows `describeSnapshot`
  under the output and resumes from them; positions are always the example's (callSite
  after a load is in the same script). render.rs/run.rs: the build-time box is the
  snapshot at the first pause, described (`run::snapshot`); `gen:dump` stays for the dump
  pages until 0304. scripts/build-wasm.sh --docs: `docs,snapshot` (no dumper).
- Size: +9.8 KB gz (785,077 B), ceiling 786,101 (D3723); codemode-memory OK. Snapshot
  timings in pi's setup (shipped module): first of an instance 0.66 ms (base handed over
  again + paths), later 0.06 ms, load 0.84 ms; 2.3 KB with the source.
- Guide: js-api.md (tasks and host-driven runs on snapshots, a new "Snapshots" section,
  host data/JSON boundary wording), cli.md (snapshots on the command line, debugger
  pauses, REPL `.snapshot`), host.md (the snapshot operations, the builds), pi.md (the
  setup through snapshots), dumps.md made true again (the site's pauses and tasks are
  snapshots; writing a dump is a host-driven run's). Not rewritten: dumps.md,
  continuations.md, suspension.md stay dump pages for 0304.
- Harvest in pi's setup (js/test/snapshot-harvest.test.ts): all 300 cases, deterministic
  clock and seed, moved to a fresh sandbox at every idle point (one call answered per idle
  point, every other snapshot without sources): all continue the same; ~7 s on the dev
  module.
- examples/sleep-dump -> examples/sleep-snapshot (git mv; README rewritten: what the snapshot
  holds, read from `describeSnapshot`); js/test/sleep-snapshot.test.ts; npm-test.sh copies
  it and pi-setup.ts, skips snapshot-harvest (the corpus is the repository's).
- Host functions a host value holds are its members (console.log keeps its identity in the
  CLI, where print and console.log share an id).
- CLI (crates/djs-cli/src/snapshots.rs): host mode with `tools`/`console` as values, fixed ids
  + made tools as functions, data = tools' state (made, answered) + call origins. `djs run
  --snapshot-after K OUT`, `--snapshot-every-idle`, `--debugger OUT` writes a snapshot, `djs
  resume` takes snapshots (by magic) in a VM of the snapshot's builtins mode (`Needs::
  frozen_builtins`), `djs describe FILE`. REPL `.snapshot [file]`, `.restore` of snapshots.
  Tests: snapshot_after_every_idle_point, debugger_pauses_snapshot_and_resume,
  snapshot_a_waiting_session, debugger_pauses_resume_from_the_snapshot.

## Follow-ups
- 0304: the REPL's pause hint still says "(.dump shows the continuation, ...)"; `djs resume
  --explain` is dump-only (snapshots have `djs describe`, which lists no activations: a
  snapshot explain with activations and bindings, like the dump's, would need the
  activation reader on a loaded VM, which works, and is a small addition if wanted).
- The `types` section is most of a small snapshot (1.6-1.8 KB of 2.3-2.5 KB): the field
  lists of every record type, by name. A host keeping many snapshots of one build could
  share it (heap-snapshots-revisited 5.1: "the schema per build kept apart"), or the
  writer could drop the types a build's reader knows. Not done: an owner question on the
  format (D3700 keeps snapshots self-describing).
- The docs site's live.js changes are not covered by a test that runs the page (none exists
  for the pauses); checked by reading and with the Sandbox paths it calls.
- A refused load leaves the sandbox dead; pi disposes on failure anyway. A load that keeps
  the sandbox usable would need the import to check everything before it fills the
  host's objects (the global patches).

## Result
Hosts persist runs as snapshots (D3720-D3723), merged on top of 0302.
- Rust: `SnapshotOptions { sources, host: Some(HostNames { functions, values, handles,
  data }) }` writes a host's run with its host values named; `Vm::load_snapshot(base,
  bytes, sources, &HostValues)` loads it into the host's own fresh VM (`Loaded { handles,
  calls, data }`); `snapshot::needs(bytes)`; `BaseInfo::from_owned`; FORMAT 2. Lazily
  compiled VMs snapshot (D3721).
- djs-wasm ABI 10: `vm_snapshot(vm, len)`, `vm_load_snapshot(vm, len)`,
  `djs_snapshot_needs`, `djs_snapshot_describe`.
- Sandbox: `Run.snapshot`, `sandbox.loadSnapshot`, `resume`/`Sandbox.resume` and tasks'
  `suspend`/`checkpoint` on snapshots (Uint8Array), `describeSnapshot`; fuelUsed and the
  calls' states travel in the snapshot; pi's whole setup works through it (tests:
  snapshot.test.ts, snapshot-harvest.test.ts over all 300 harvest cases).
- CLI: `--snapshot-after`, `--snapshot-every-idle`, `--debugger` (now a snapshot), `djs
  resume` of snapshots, `djs describe`; REPL `.snapshot`, `.restore` of snapshots.
- Docs site pauses, examples/sleep-snapshot, the guide pages.
Known gaps: see Follow-ups. Dumps still work everywhere they did except tasks (which
suspend into snapshots) and the CLI's `--debugger`; 0304 removes them.
