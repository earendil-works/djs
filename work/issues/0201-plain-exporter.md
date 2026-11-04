# 0201: Exporter: plain-JS continuations in Rust (behind a flag), djs run of plain dumps, node harness and chain check in the gate

status: open
agent: -
branch: issue/0201-plain-exporter
depends-on: 0200 (interface first)
guide: -
touches: crates/djs-core/src/vm/dump*.rs (a new plain writer next to format 2), crates/djs-difftest (--dump-every-idle writes plain dumps behind a flag), scripts/ (node harness), tests

## Goal
docs/plain-js-dumps.md section 8 "Recommendation and plan", step 2.

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
