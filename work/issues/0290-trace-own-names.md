# 0290: Stack traces name a frame by the function's own `name`; continuations take their origin's (GitHub PR #2)

status: in-progress
agent: pcall
branch: issue/0290-trace-own-names
depends-on: -
touches: crates/djs-core/src/interp/run/stack.rs, crates/djs-core/src/vm/dump_runtime.rs, js/src/dump-runtime.js, crates/djs-core/src/vm/{tests,dump_resume_tests}.rs
guide: docs/guide/dumps.md / continuations.md if they mention trace names, else -

## Goal
https://github.com/earendil-works/djs/pull/2 (by rt2zz) found that after loading a plain
dump, every waiting frame shows as `at <anonymous> (dump.js:...)`: a continuation is an
anonymous function expression. Still true on main c34b2a59 (`check,<anonymous>` instead
of `check,main`). The PR is ~928 commits stale and does not apply: stack.rs was rewritten
by D3370 (heap-string `Name`s), and `$djs.k` is now `(K, f, at, r, h, p)` with the origin
in `f` (the PR reads `r.f`, which would throw on every load now).

The coordinator ported the idea onto main: work/scratch/pr2/ported.diff (own `name` data
property string names the frame, else the compiled name; `$djs.k` copies `f`'s own string
`name` onto K). With it: the PR's test passes, `djs-core vm::` passes, the plain-dump
corpus in djs and node passes (1550 links, 0 differ), re-dumps write no extra `name`.

Engines: renamed function -> qjs and V8 show the new name; empty own name -> both show
anonymous; a `name` getter -> V8 shows the compiled name, qjs `<anonymous>` (the port
follows V8; record it).

## Acceptance criteria
- [ ] The port landed with the PR's two tests (traces after a load name the original
      functions; a renamed function / a `name` getter), adapted to main.
- [ ] Cost of the extra own-property lookup per frame on `new Error` measured (D3370's
      numbers, slope.py / wasm instructions); acceptable or made cheap (e.g. only when the
      function's name was redefined, or only for continuations).
- [ ] Credit the PR author (rt2zz / Zack Story) in the commit message
      (`Co-authored-by: rt2zz <ztstory@gmail.com>`).
- [ ] Decision recorded; guide pages checked.

## Log

## Follow-ups

## Result
