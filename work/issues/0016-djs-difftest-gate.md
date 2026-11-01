# 0016: djs in the difftest runner and the gate

status: open
agent: -
branch: issue/0016-djs-difftest-gate
depends-on: 0010, 0014, 0015
touches: crates/djs-difftest/, tests/corpus/, scripts/gate.sh

## Goal
docs/compiler.md section 10, row 0006d. Run 0010's corpus on djs in-process, check in
the ratchet file, and add difftest (djs side, plus `--suspend-every` and `--gc-stress`
variants) to the gate. Most cases fail at this point (no functions). That is expected;
the ratchet records it.

(The coordinator refines the criteria once 0010, 0014 and 0015 have landed.)
