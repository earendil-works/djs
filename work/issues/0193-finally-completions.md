# 0193: finally completions in slots (functions that can suspend); gosub index mapping and return-path refusals gone

status: in-progress
agent: sm
branch: issue/0193-finally-completions
depends-on: 0191
guide: -
touches: crates/djs-core/src/compiler/stmt.rs, generator.rs, vm/dump_*, tests (test262 try/generators/async suites), benches

## Goal
docs/suspension-design-review.md, section 4 "Migration plan", step 5; decisions D2720
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
