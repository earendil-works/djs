# 0200: Compiler: record each site's statement path, block declarations and expression constructs with spans

status: in-progress
agent: pjc
branch: issue/0200-site-structure
depends-on: -
guide: -
touches: crates/djs-core/src/compiler/ (recording next to the suspension maps; no behaviour change), tests/smap pins (extended), a written interface for the exporter (docs/plain-js-dumps.md or a doc comment)

## Goal
docs/plain-js-dumps.md section 8 "Recommendation and plan", step 1.

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
