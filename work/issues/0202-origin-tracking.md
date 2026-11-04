# 0202: Origin tracking: K.$djs read back; frames of continuations written as activations of their origin; corpus chain check

status: open
agent: -
branch: issue/0202-origin-tracking
depends-on: 0201
guide: -
touches: vm/dump*.rs, tests

## Goal
docs/plain-js-dumps.md section 8 "Recommendation and plan", step 3.

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
