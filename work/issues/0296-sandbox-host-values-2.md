# 0296: Sandbox for pi, part 2: cyclic plain values throw at the call, read-only host globals, Error message coercion, null-prototype namespaces

status: in-progress
agent: pns
branch: issue/0296-sandbox-host-values-2
depends-on: 0295
touches: js/src/sandbox.ts, js/src/reader.ts, js/src/wire.ts, js/test/, docs/guide/js-api.md, docs/decisions.md
guide: docs/guide/js-api.md

## Goal
pi on djs (agent pidjs) after 0295; repros with `jsonBoundary: { except: ["text"] }`,
`frozenBuiltins`, `missingMember` set:

1. **A cyclic value to a plain (`except`) sync host function** fails the whole run on the
   host side: `const c = {}; c.self = c; try { text(c) } catch (e) { return e.name }` ends
   the run with a `sandbox` error "cyclic value at $.self.self...". Wanted: a TypeError at
   the call that the script can catch, as the JSON boundary does (pi's `text()`/`store()`
   messages depend on it). Same for any other value the reader cannot read.
2. **Host globals are writable**: `globalThis.tools = null; return typeof tools` replaces
   the namespace. pi's prelude made tools/console/text/image/exit/store/load/ALL_TOOLS
   non-writable, non-configurable globals. Wanted: globals the sandbox binds (`globals`,
   `values`, namespaces) are non-writable and non-configurable (a strict assignment
   throws TypeError). Dumps: check that a dump and load keep them so.
3. **An Error whose message is not a string** arrives with message "":
   `const e = new Error('x'); e.message = 42; throw e;` -> `run.error.value.message === ""`
   (in the VM `String(e)` is "Error: 42"). Coerce name and message as `String()` would.
   Low priority.
4. **Namespaces with a null prototype**, as pi's were: with `missingMember`, `tools.toString`
   is Object.prototype's (inherited names never reach the hook). If cheap, namespace
   objects (and the Proxy's target) have a null prototype.

## Acceptance criteria
- [ ] 1-4 with tests (with and without the boundary; dump and load for 2 and 4)
- [ ] guide and decisions

## Log

## Follow-ups

## Result
