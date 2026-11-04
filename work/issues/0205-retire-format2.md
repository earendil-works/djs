# 0205: Retire format 2: delete the loader, $dump.resume, primitives, recording compile; plain goldens; docs and guide

status: in-progress
agent: pjf
branch: issue/0205-retire-format2
depends-on: 0203 (closing), 0204
guide: docs/guide/snapshots.md, suspension.md, host.md
touches: vm/dump_load.rs, dump_resume.rs, dump_prims.rs, compiler recording at load, tests/dump, docs/continuation-dumps.md, guide snapshots/suspension/host

## Goal
docs/plain-js-dumps.md section 8 "Recommendation and plan", step 6.

## Context
docs/plain-js-dumps.md (the design, results and plan: read all of it), the prototype in
work/research/0199/ (plain.mjs, normalize.mjs, freevars.mjs, the harness and chain checks),
decisions D2830-D2833, and today's format 2 exporter (vm/dump*.rs, D2610-D2628, D2730-D2735,
D2800-D2802). Acceptance at every merge: dumps run as plain Scripts in djs **and node** and
continue identically to the uninterrupted run over every corpus idle point they cover,
re-dumps stay bounded (cap chains at 50 links), and the existing gate checks stay green.
djs is unreleased (D024): no compatibility code for format 2 once it is retired.

Notes from 0207 (coordinator): plain dumps require frozen builtins (D2836, D2904); the
Sandbox's `plainDumps` default follows the VM's mode, so once format 2 is gone a sandbox
without frozen builtins cannot dump at all: make that a clear error (and say so in the
host docs), and drop `plainDumps`/`--plain`/`.plain`. The header is `"djs-dump";` (D2835);
`$djs` carries the runtime helpers and `source(ref)` (D2834, D2903).

## Log

Plan (pjf): M1 hosts off format 2; M2 core deletions (loader, primitives, recording
compile, format 1, `DumpOptions::plain`, goldens → plain, core tests); M3 the
`jump_if_loading` op and `heap.loading_dump`; M4 docs and guide.

- M1 (hosts). CLI: `--plain`/`.plain` gone; `run --dump-after/--dump-every-idle` and
  every `resume` make frozen VMs; `djs repl --frozen` (was `--plain`), `.dump` and
  `.dump-every` without it print "dumps need frozen builtins (D2836): start the REPL with
  --frozen"; `resume`/`--explain`/`.restore` read only `"djs-dump";`; `--explain`'s
  header drops "format X" (` (TypeScript)` when it is); a dump naming an unknown host
  function no longer loads for `--explain` (its code calls it while loading). Fixtures:
  tests/run/reads.dump.js and inputs.dump.js are the former .plain.js, sleep.dump.js
  hand-written plain; the generator and plan explain/restore cases dump on the fly.
  wasm: `vm_load_dump` removed, `vm_dump`'s options line removed, ABI 3 → 4. Sandbox:
  `plainDumps` gone, `dump`/`Run.dump`/tasks throw misuse "dumps need frozen builtins"
  without `frozenBuiltins`; `load`/`loadRun`/`resume` take only plain dumps (the
  dump-module round trip for loads is gone, `viaDumper` lost `back`). pi adapter:
  `plainDumps` gone, `dumpSession` always plain (`PLAIN_FN_NAMES`), `Engine.loadDump`
  gone; pi-tests `--plain` gone. difftest: `--dump-every-idle` (format 2) gone, the Djs
  engine's `plain` folded into `dump_every_idle` (label `--plain-dumps`); the gate's
  difftest leg no longer passes it (the plain dumps leg covers dumps). JS tests converted
  (dumpload, task, dumpresume, run, pi-vm, sandbox). docs/cli.md, docs/host.md,
  TESTING.md (difftest part), ARCH.md's export list, guide snapshots.md's one sentence.
- Found while converting (reported to the coordinator, not fixed here): plain dumps refuse
  any class with a static private member ("an instance of class … with private name #o",
  contradicting D2878), and a class in a factory scope that names itself in a method or
  constructor gives "plain dump: cyclic dependencies".
- M1's first gate run failed on rustc incremental corruption that the gate's detector
  (D2896) missed: the `_anon.*.llvm.*` symbol came ~50 lines after "Undefined symbols",
  past its 40-line window. Widened to 400 lines in scripts/gate.py (outside touches, one
  number).

## Follow-ups

## Result
