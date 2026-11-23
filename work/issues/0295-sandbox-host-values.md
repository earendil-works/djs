# 0295: Sandbox: thrown errors and console values as data; namespace objects like pi's

status: done
agent: pns
branch: issue/0295-sandbox-host-values
depends-on: - (0294 runs in parallel and also edits js/src/sandbox.ts: rebase on it when it lands)
touches: js/src/sandbox.ts, js/src/reader.ts, js/test/, docs/guide/js-api.md, docs/decisions.md (and djs-wasm/djs-core only if a JS-level solution is not enough)
guide: docs/guide/js-api.md, docs/guide/pi.md

## Goal
pi moves its code mode onto djs host-driven runs with no prelude in the VM (agent pidjs,
~/Development/pi-mono-djs). Probing the Sandbox (host-driven run, `jsonBoundary`,
`frozenBuiltins`) it found gaps where pi's QuickJS prelude used to produce what the model
sees. They belong in the Sandbox, not in a pi prelude:

a. **A thrown Error.** `run.error.value` is an opaque ref (`{type:"object",ref,className}`),
   `message` is "TypeError: boom 1", no stack. pi shows
   `TypeError: boom 1\n    at f (codemode.js:2:9)`; the VM has it (`e.stack`). The run's
   error must carry the thrown Error's `name`, `message` and `stack` (as the VM's `stack`
   string) as data; a non-Error throw stays a value (JSON with the boundary).
b. **console arguments.** `console.log(new Error("inner"))` reaches the host as the
   string "[Error]" (Map "[Map]", function "[Function]"). The host needs an Error as data
   (name, message, stack); Map/Set as data would be nice, not required.
c. **A missing namespace member.** `tools.Echo()` throws "TypeError: not a function".
   pi threw on the read: `tools.Echo does not exist. Did you mean tools.echo? ...`. Give
   the host a way to answer reads of a member a namespace does not have (a message the
   read throws as TypeError, e.g. `SandboxOptions.missingMember?(ns, member) => string`).
   Dumps must stay as they are (namespaces are host roots named by identity).
d. **Namespace objects.** `Object.keys(tools)` is `[]` and `tools`/`models` are not
   frozen (`models.extra = 1` sticks). Members must be enumerable, in definition order,
   and the namespace frozen once the sandbox made it (before the script runs).
e. **Aliases.** pi offers `tools["my-tool"]` next to `tools.my_tool` (the tool's own
   name). Identities are `ns.member` of identifiers. Let a namespace member have extra
   names that are not identifiers, bound to the same function (no new identity: dumps
   name the namespace, not its members).
f. **A host-driven run ends when the body's result settles**, not when every open async
   call is settled (pi: "Calls still running when the script ends are cancelled").
   `begin("tools.echo(1); return 'x'")` is idle with call 1 open today; wanted: `done`,
   value "x", the still-open calls reported so the host cancels them, and a later settle
   of such a call a no-op. runAsync and tasks consistent with it.
g. **The JSON boundary per function**: tools want it, pi's output and store functions
   (`text`, `image`, `store`, `load`) want plain values (`text(0/0)` arrives as null,
   `text(() => 1)` as undefined). A per-function opt-out (the 0291 follow-up).
h. **`maxFuel` across a host-driven run's steps** (coordinator, from pi): each step ended
   idle within a slice, so the budget was never checked; also decide what a dump and
   load do with it. Plus `console.debug`, a frozen `console`, Dates for console.

## Acceptance criteria
- [x] a-h, each with tests in js/test (host-driven runs and `run`), with and without
      `jsonBoundary`, and through a dump and load where it applies (c, d, e)
- [x] docs/guide/js-api.md describes them; decisions appended
- [x] the shipped module's size: no change, or say what it costs

## Notes
- pi's old behaviour is in ~/Development/pi-mono-djs/packages/codemode/src/runtime/prelude-source.ts
  (read only): the exact messages and formats.
- Prefer JS-level solutions in sandbox.ts (the sandbox's helper script, defineOn flags,
  the reader) over engine changes; a missing-member hook may need a Proxy or an engine
  hook: measure and choose, and say why.

## Log
- 2026-11-23: read the issue, js-api.md, sandbox.ts, D2632/D2970-2972/D3592-3596 and pi's
  prelude. Probed: `e.stack` is the Error's slot 0 (Error.prototype.stack is a getter), so
  an Error has no own `stack` to read as data; `console.log(new Error())` got "[Error]".
- (a, b) In the reader, not the helper: console arguments are read during a sync host
  call, where the VM cannot run JS. VM Errors become host `Error`s (name/message from data
  properties up the prototype chain, stack from slot 0 or a pending trace's default
  string, cause, own enumerable data), Dates `Date`s, Maps/Sets `Map`/`Set`. With the
  boundary a thrown non-Error goes through JSON (helper op 4, undefined on failure); a
  thrown value the reader cannot read is undefined. `Opaque` exported. reader-cross.ts's
  error case updated (the Rust image case is unchanged).
- (d) Freezing in JS costs a compile per sandbox (shipped module, node: create 125 µs,
  +108 µs with HELPER, ~+35 µs for a minimal script). Asked the coordinator; chose
  `vm_define` flag bit 1 = freeze afterwards (`Vm::freeze` over the existing
  `set_integrity_level`), +90 bytes raw, +39 gzip (999,705 -> 999,795 on the module
  without the dumper, before 0294). Namespaces freeze lazily before every script
  (`enter`, which loads use too), so `defineFunction` still works until the first run;
  after it, a member for a frozen namespace is a misuse. Module namespaces use the flag
  too (no helper). Note: a define on a frozen object silently does nothing (the Vm's
  `define`/`define_enumerable` ignore define_own_property's false); the Sandbox checks first.
- (c) `missingMember` as a Proxy made by the helper (op 5) over the frozen member
  object, calling a hidden sync function (`HOOK_MISSING`, no identity, not journaled).
  Proxy invariants: present members go through Reflect.get, so a frozen target is fine
  (tested). The TypeError's stack starts at the script (captureStackTrace below the trap).
- (e) `aliases: { "tools.my_tool": ["my-tool"] }`; identifier aliases are refused.
- (f) At IDLE the run loop checks the body's result (vm_outcome / a loaded run's handle);
  settled -> vm_abort, open calls to `Run.dropped`, settle of a dropped call a no-op.
  runAsync and tasks changed with it: they used to wait for every open call (an unawaited
  `tool()` kept the run alive and its result was delivered); now they end, ignore late
  results and abort the hook functions' signals (runAsync now passes hook functions a
  call with a signal). sandbox.test.ts's slice-order test now awaits its call.
- (g) `jsonBoundary: { except: [...] }`.
- (h) The slice is cut to what is left of maxFuel; `Run.fuelUsed`; `loadRun(dump, {
  fuelUsed })` carries the count over a dump (default fresh), as the coordinator leaned.
- Rebased onto 0294 (ABI 8; one module): the flag is documented as part of ABI 8. 0294
  took D3600-D3602, so mine are D3610-D3614.
- Tests: js/test/host-values.test.ts (a-h, with/without boundary, dump/load for c, d, e,
  f, g, h), djs-wasm `define_flags_enumerable_and_freeze`. Ran: js tests, reader cross,
  pi tests (next mode, shipped module: 0 new failures), codemode workloads, docs check.
- Guide: js-api.md (options, namespaces section, boundary, values out, errors, host-driven
  runs, limits), limits.md (maxFuel exact, across steps), host.md (reader: slot meanings,
  errors/dates/maps), pi.md (durable code mode bullets).

## Follow-ups
- `Vm::define`/`define_enumerable` ignore a refused define (a frozen object): vm_define
  returns 0. Harmless for the Sandbox (it checks first), but a host-facing ABI would want -1.
- `error_summary`'s DOMException branch checks 2 slots, a DOMException has 3 (stack, name,
  message), so its display falls back to "Error: ..."? Not checked further.
- Tasks have no `fuelUsed` carry-over (`Sandbox.resume`): only host-driven runs.
- pi's "stalled" message (a body waiting on a promise that can never settle) is still the
  host's: a run idle with no open calls and a pending result.

## Result
Landed (JS layer plus one vm_define flag):
- Errors, Dates, Maps and Sets are read as host values: `run.error.value` is an `Error`
  with the VM's `name`, `message`, `stack` (frames only, as `e.stack`), `cause` and own
  enumerable data; `console.*` gets them too (and `[Class]` text for the rest);
  `console.debug`; `Opaque` exported.
- Namespaces: enumerable members in definition order, frozen before the first script
  (`console` included), `aliases`, `missingMember(ns, member) => message | undefined`.
- Runs end when the body's result settles: `Run.dropped`, settle of a dropped call is a
  no-op; runAsync/tasks consistent (late results ignored, hook signals aborted).
- `jsonBoundary: { except: identities }`.
- `maxFuel` exact and across a host-driven run's steps; `Run.fuelUsed`, `loadRun(dump,
  { fuelUsed })`.
- Size: +90 bytes raw / +39 gzip (shipped module); after the rebase 767,603 gzip, under
  the 768,597 ceiling. Decisions D3610-D3614.
