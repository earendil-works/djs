# 0290: Stack traces name a frame by the function's own `name`; continuations take their origin's (GitHub PR #2)

status: done
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
- [x] The port landed with the PR's two tests (traces after a load name the original
      functions; a renamed function / a `name` getter), adapted to main.
- [x] Cost of the extra own-property lookup per frame on `new Error` measured (D3370's
      numbers, slope.py / wasm instructions); acceptable or made cheap (e.g. only when the
      function's name was redefined, or only for continuations).
- [x] Credit the PR author (rt2zz / Zack Story) in the commit message
      (`Co-authored-by: rt2zz <ztstory@gmail.com>`).
- [x] Decision recorded; guide pages checked.

## Log
- Applied the coordinator's port (work/scratch/pr2/ported.diff) and the PR's two tests,
  adapted: `traces_after_a_load_name_the_original_functions` (dump_resume_tests.rs) and
  `error_stack_traces_use_a_functions_own_name` (vm/tests.rs; plus an empty own name and
  computed-key / symbol methods).
- Cost (work/scratch/0290/opsins.py, 0266's script with trace-heavy ops; wasm
  instructions per op, node --no-liftoff, reps 1 vs 3, best of 3; load 9-13): the
  unconditional lookup cost ~+190 per frame (5 nested functions 37.6k → 38.5k). Gated on
  the closure's lazy-name flag (OBJ_LAZY_NAME, D2445: a closure whose `name` was never
  defined keeps it unstored and equal to the compiled name), the common frame pays one
  flag test: 37.6k, 19.5k in one function, 14.5k top level, all at main's within noise.
  A renamed function pays the lookup (19.9k → 20.6k). Shipped +54 B gz.
- Side effect: methods with computed or symbol keys now show their name (`ab`, `[s]`) as
  qjs does; 0266's first follow-up is fixed.
- Tests: djs-core --lib all pass; difftest errors 30/30; djs-docs check ok.
- Guide: interpreter.md (how a frame is named), continuations.md (`$djs.k` gives `K`
  the name), dumps.md (the helper table row).

## Follow-ups

## Result
A bytecode frame in a stack trace is named by its function's own `name` when that is a
string data property (else the compiled name); `$djs.k` gives a continuation its
origin's own string name, so traces after a plain dump's load match the uninterrupted
run. The lookup only runs for closures whose `name` is stored (lazy-name flag): `new
Error` costs the same as on main. D3580. Credit: rt2zz (GitHub PR #2).
