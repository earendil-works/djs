# 0191: Spill at suspend: the operand stack is empty at every await/yield; dumps write slots

status: open
agent: -
branch: issue/0191-spill-at-suspend
depends-on: 0190
guide: docs/guide/suspension.md, snapshots.md
touches: crates/djs-core/src/compiler/ (spill/unspill moves from the role stack), interp/run/suspend.rs, vm/dump_flight.rs, vm/dump_resume.rs, tests/smap, tests/dump (goldens rewritten under a decision), COMPAT_VERSION bump

## Goal
docs/suspension-design-review.md, section 4 "Migration plan", step 3; decisions D2720
(state machines in the compiler, SM), D2721 (slot naming construct@span), D2722 (host-call
state with save/resume hooks). Read the whole report first: it describes this step's
design, what stays and what goes, the risks and the tests required at every step:
smap pins and golden dumps (rewritten only with a decision, D2619), DJS_E2E_MODE=dump,
difftest --dump-every-idle, the harvested/private idle-point counts at 100%
(vm::dump_harvest), test262 async/generator suites where relevant, benches before/after
where the report says so. Each step merges green on its own.

## Log

## Follow-ups

## Result
