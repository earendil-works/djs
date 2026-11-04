# 0196: Load performance: one compile per load (state tables kept or cached)

status: in-progress
agent: sm
branch: issue/0196-one-compile-per-load
depends-on: 0194
guide: -
touches: crates/djs-core/src/compiler/, vm/dump_load.rs, scripts/dump-costs

## Goal
docs/suspension-design-review.md, section 4 "Migration plan", step 8; decisions D2720
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
