# 0195: pi adapter: prelude-next and the adapter use host-call hooks for tool calls; pi suite with --dump-every-idle

status: done
agent: hostcalls
branch: issue/0195-pi-host-hooks
depends-on: 0189
guide: -
touches: tests/pi, js/pi, docs/host.md

## Goal
docs/suspension-design-review.md, section 4 "Migration plan", step 7; decisions D2720
(state machines in the compiler, SM), D2721 (slot naming construct@span), D2722 (host-call
state with save/resume hooks). Read the whole report first: it describes this step's
design, what stays and what goes, the risks and the tests required at every step:
smap pins and golden dumps (rewritten only with a decision, D2619), DJS_E2E_MODE=dump,
difftest --dump-every-idle, the harvested/private idle-point counts at 100%
(vm::dump_harvest), test262 async/generator suites where relevant, benches before/after
where the report says so. Each step merges green on its own.

## Log

- 2026-11-04 (hostcalls): the adapter's next mode passes each open call's pi id as its
  state to `vm_dump` (engine.ts `dump(..., states)`, from 0189) and rebuilds `ids` from
  `loadDump`'s per-call states on resume; `SessionDump.state.next` drops `ids`. D2772.
  prelude-next unchanged (its tool calls already are engine async host calls); pi's host
  is the resume policy (the adapter cannot run tools), as before.
- pi-vm.test.ts asserts the state in the dump and no `ids` in the saved state; 39/39.
- pi's suite on the release dump module: `--next` 215/226 and `--next --dump-every-idle`
  (scripts/pi-dump-every-idle.sh, the full-only gate leg) 215/226, the 11 expected
  failures, 0 new, 0 stale. Every dump there has open calls, so every resume goes through
  the call states.
- docs/host.md pi section updated (call states, what the adapter's state keeps; the stale
  "dump does not carry Math.random's state" sentence fixed: 0182 did it).

## Follow-ups

- None needed; if pi adopts the next shape, its host could keep per-tool state (a job id,
  a deadline) in the same call state, via an adapter API to set it (not built: pi's
  protocol has no message for it).

## Result

pi's next mode keeps its open calls' pi ids in the dump as call states (D2772); the
adapter's saved state has no call table. pi's suite passes the same with a dump and a
resume at every idle point (215/226, expected failures only).
