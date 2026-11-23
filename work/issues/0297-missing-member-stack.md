# 0297: missingMember's TypeError stack shows the Sandbox's Proxy trap frame

status: done
agent: pns
branch: issue/0297-missing-member-stack
depends-on: 0295
touches: js/src/sandbox.ts (the helper's namespace Proxy), js/test/, crates/djs-core only if a frame-hiding hook is needed
guide: docs/guide/js-api.md (if it describes the stack)

## Goal
D3611 says the TypeError of a missing namespace member has a stack that starts at the
script's read. On 0.2.0 it starts with the helper's frame:

    TypeError: tools.Echo does not exist
        at get (sandbox.js:12:54)
        at f (sandbox.js:1:28)

(repro: `missingMember: (ns, m) => ...`, script `function f() { return tools.Echo(1) }
return f()`). pi prints every frame to the model, so the trap's line leaks. Make the
first frame the script's read (hide the trap frame: a privileged/hidden helper function,
or build the error so its stack omits it), with a test that pins the whole stack.

## Log
- 2026-11-23: the VM's `e.stack` was right already (the trap's `Error.captureStackTrace`
  defines an own `stack`); the reader read the error's slot, the trace recorded by `new
  TypeError` inside the trap. Fix in the reader: an own string `stack` first (D3624). No
  engine change, no size change. Tests pin the whole stack at the host, in the VM and on
  `console`, with and without the boundary, plus a script-set `e.stack`.
- Guide: js-api.md's sentence on `stack` says "or the string a script set".

## Follow-ups
- The helper compiles under the default name, so its frames would read `sandbox.js:12:..`
  if one ever leaked again; a name of its own would make that obvious.

## Result
The host's Error (`run.error.value`, console arguments) takes the VM error's own `stack`
when it has one, so the missing-member TypeError starts at the script's read:
`    at f (codemode.js:1:28)\n    at <eval> (codemode.js:2:9)\n`. D3624.
