# 0297: missingMember's TypeError stack shows the Sandbox's Proxy trap frame

status: open
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

## Follow-ups

## Result
