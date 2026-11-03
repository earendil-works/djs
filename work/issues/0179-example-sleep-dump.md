# 0179: Example: a host tool that sleeps 5 s; dump while waiting, restart, continue

status: open
agent: -
branch: issue/0179-example-sleep-dump
depends-on: 0171, 0172 (the exporter writes format 2 dumps of waiting sessions)
guide: maybe a short pointer in docs/guide/host.md
touches: examples/sleep-dump/ (new: a small TypeScript or JavaScript program, README), js/src/sandbox.ts and crates/djs-wasm only for the minimal plumbing the example needs (coordinate with 0175's Sandbox run API: build the smallest piece of it, not a parallel API), js/test/ (the example as a test, with a short sleep)

## Owner
"Add a little JavaScript (or TypeScript) example that registers a tool that just sleeps
for 5 seconds on the host side. We then suspend the VM by dumping and restarting it, and see
how the call continues."

## What the example shows
1. A host (node) creates a Sandbox with an async tool `sleep(ms)` (host side: a
   `setTimeout` that resolves after 5 s), and runs a script like
   ```js
   console.log("before");
   const t0 = Date.now();
   const r = await tools.sleep(5000);
   console.log("after", r);
   ```
2. While the call is open (the VM is idle), the host **dumps** the VM (format 2: the
   suspended activation + the open host call), prints/saves the dump text, disposes the
   sandbox (or exits the process: a second mode that really restarts, reading the dump
   from a file).
3. A **new** sandbox (or process) loads the dump, registers the same tool, and delivers
   the call's result when the sleep finishes (from the restarted side: either the
   remaining time or a fresh 5 s); the script continues and prints "after …".
4. README: how to run it, what the dump looks like (show it), what is and isn't
   preserved (the host's timer is host state: the host re-arms it from the open call list
   the load returns).

## Plumbing
Needs: `Sandbox.dump()` writing format 2 for an idle VM with open calls (0172's
exporter + the fn_id -> name lookup from the host), and `Sandbox.load()` passing a host
function lookup (`host_fn`) and returning the open calls (0171's `Loaded`) over wasm.
Build exactly what's needed, in the shape docs/continuation-dumps.md "Sandbox API" plans
(0175), and tell the coordinator so 0175 builds on it rather than duplicating it.

## Acceptance criteria
- [ ] `examples/sleep-dump` runs with one command (in-process mode and two-process mode).
- [ ] A js test runs the in-process mode with a short sleep in the gate.

## Log

## Result
