# 0204: Host hooks for plain dumps: resume(id, name, args, state), seed(lo, hi); Sandbox tasks, pi adapter, CLI/REPL

status: open
agent: -
branch: issue/0204-host-hooks-plain
depends-on: -
guide: docs/guide/snapshots.md, suspension.md, host.md
touches: js/src/sandbox.ts, js/pi/, crates/djs-cli (run/resume/REPL), crates/djs-wasm (exports for loading a plain dump = running a script with hooks), docs/host.md, examples/sleep-dump

## Goal
docs/plain-js-dumps.md section 8 "Recommendation and plan", step 5.

## Context
docs/plain-js-dumps.md (the design, results and plan: read all of it), the prototype in
work/research/0199/ (plain.mjs, normalize.mjs, freevars.mjs, the harness and chain checks),
decisions D2830-D2833, and today's format 2 exporter (vm/dump*.rs, D2610-D2628, D2730-D2735,
D2800-D2802). Acceptance at every merge: dumps run as plain Scripts in djs **and node** and
continue identically to the uninterrupted run over every corpus idle point they cover,
re-dumps stay bounded (cap chains at 50 links), and the existing gate checks stay green.
djs is unreleased (D024): no compatibility code for format 2 once it is retired.

## Log

## Follow-ups

## Result
