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

## Follow-ups

## Result
