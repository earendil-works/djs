# 0016: djs in the difftest runner and the gate

status: open
agent: -
branch: issue/0016-djs-difftest-gate
depends-on: 0010, 0022
touches: crates/djs-difftest/, crates/djs-bench/ (engine track: register djs), tests/corpus/expected-failures.txt, scripts/gate.sh

## Goal
docs/compiler.md section 10, row 0006d. Run 0010's corpus on djs in-process (through
`Vm::compile` in async-body mode with `tools`/`console` parameters, D008, after running
the shared prelude), check in the ratchet file, and put difftest (djs, plus
`--suspend-every` and `--gc-stress` variants) into the gate. Also register djs in
djs-bench's engine track (0019 follow-up) and report the djs/qjs ratios.

## Acceptance criteria
- [ ] A `djs` engine in djs-difftest: in-process, a fresh `Vm` per case, the prelude run
      first (it will mostly fail today because builtins are missing: decide whether
      the prelude must be split into a minimal part djs can run now, or whether the djs
      engine provides `console`/`text`/`exit` natively as host functions for now;
      record the decision), the fuel limit from the case metadata, host calls for
      console/text/store/load/tools, the return value and uncaught error formatted
      exactly like the qjs side.
- [ ] Stress variants: `--suspend-every N` (heap-image round trips) and `--gc-stress`.
      Outputs must be identical to the plain djs run (that is a hard failure, separate
      from the oracle comparison).
- [ ] `tests/corpus/expected-failures.txt` generated (most cases fail now: that is
      expected), and the ratchet in `scripts/gate.sh` for the plain run plus a
      consistency-only check for the stress variants. Keep the gate's added time small
      (parallelism; GC stress maybe on a subset if slow; record the timings).
- [ ] The djs engine in djs-bench's engine track, with a djs/qjs ratio column (most
      benchmarks will not run yet; report what runs).
- [ ] A Log entry with the pass count and the biggest failure clusters (grouped by first
      error line), so the coordinator can prioritize.

## Log

## Follow-ups

## Result
