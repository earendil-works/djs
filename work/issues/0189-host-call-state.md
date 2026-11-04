# 0189: Host calls as data: per-call state, save/resume hooks, run.suspend, sleep example rewrite

status: in-progress
agent: hostcalls
branch: issue/0189-host-call-state
depends-on: -
guide: docs/guide/host.md, suspension.md
touches: crates/djs-core/src/vm/dump*.rs (pending gets a state argument; Loaded.calls[].state), crates/djs-wasm (vm_dump/vm_load_dump carry per-call JSON), js/src/sandbox.ts (function descriptors {call, save, resume}, run.suspend(), Sandbox.resume(...).result), js/test, examples/sleep-dump (rewritten, ~40 lines), docs/host.md, guide host.md

## Goal
docs/suspension-design-review.md, section 4 "Migration plan", step 1; decisions D2720
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
