# 0007: Run test262 on djs and put the ratchet in the gate

status: open
agent: -
branch: issue/0007-test262-djs-engine
depends-on: 0002, 0014, 0015, P1-8
touches: crates/djs-test262/, tests/test262/, scripts/gate.sh

## Goal
Plug djs into the test262 runner as an in-process engine (Script mode, `$262`, `print`),
check in `tests/test262/expected-failures.txt` and make the ratchet part of the gate.
First milestone: "the harness loads" (TESTING.md).

(Detailed acceptance criteria are written by the coordinator once 0006 has landed.)
