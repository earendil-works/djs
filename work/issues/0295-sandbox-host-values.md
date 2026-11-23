# 0295: Sandbox: thrown errors and console values as data; namespace objects like pi's

status: in-progress
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

## Acceptance criteria
- [ ] a-g, each with tests in js/test (host-driven runs and `run`), with and without
      `jsonBoundary`, and through a dump and load where it applies (c, d, e)
- [ ] docs/guide/js-api.md describes them; decisions appended
- [ ] the shipped module's size: no change, or say what it costs

## Notes
- pi's old behaviour is in ~/Development/pi-mono-djs/packages/codemode/src/runtime/prelude-source.ts
  (read only): the exact messages and formats.
- Prefer JS-level solutions in sandbox.ts (the sandbox's helper script, defineOn flags,
  the reader) over engine changes; a missing-member hook may need a Proxy or an engine
  hook: measure and choose, and say why.

## Log

## Follow-ups

## Result
