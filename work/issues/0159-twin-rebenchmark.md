# 0159: Re-benchmark every @native / @fast twin; keep only those that pay

status: open
agent: -
branch: issue/0159-twins
depends-on: 0137, 0154, 0155 (instr.py)
guide: docs/guide/builtins.md
touches: crates/djs-core/src/builtins/ (@native/@fast annotations, hand-written fast entries), crates/djs-bgen/ (only if needed), tests/scores/size.tsv

## Goal
docs/architecture-bloat-audit.md step 3: `@native` builtins are −10.6 KB gz and `@fast`
entries −8.3 KB gz of module code. Re-measure each twin on the workloads, pi-like, the
pi bench and the 310 codemode corpus cases (wasm instructions; instr.py's builtin split,
D2309) and drop those that don't pay their bytes, with an explicit rule (e.g. keep if it
saves >= X% on a workload or >= Y M instructions per KB). Keep run_hot. Record the rule and
the per-twin table as a decision; lower the size ratchet.

## Log

## Follow-ups

## Result
