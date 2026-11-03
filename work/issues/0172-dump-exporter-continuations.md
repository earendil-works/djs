# 0172: Exporter: continuations, re-enactment, and the dump-every-idle stress mode

status: in-progress
agent: portable
branch: issue/0172-dump-exporter
depends-on: 0171, 0173
touches: crates/djs-core/src/vm/dump*.rs, crates/djs-difftest (--dump-every-idle), crates/djs-core/tests/e2e.rs (dump mode), scripts/gate.sh (the new stress leg), vm/dump_harvest.rs, TESTING.md, docs/continuation-dumps.md, docs/functions-as-code.md
guide: -

## Goal
`Vm::dump` writes format 2 when the VM is idle with something in flight
(docs/continuation-dumps.md), and the gate proves it: at every idle point, dump, load into a
fresh VM, continue, same transcript.

## Acceptance criteria
- [ ] Dumping requires an idle VM (D2561) and refuses otherwise with the reason.
- [ ] Roots: the host's handles and open host calls, plus the selected bindings; activations
      found through promise reactions and generator objects, written in dependency order with
      the sites and roles from 0169.
- [ ] Re-enactment (D2562): promise combinators with settled elements, then/catch/finally
      reactions in order, resolving functions, array/Map/Set/string/regexp-string iterators,
      for-await and async-from-sync iterator state. Anything else in flight is refused with its
      path.
- [ ] Unit round trips per construct (each role, each loop kind, each re-enactment) and
      refusal tests.
- [ ] difftest `--dump-every-idle` over codemode and codemode-harvest (needs 0173's idle
      points): a transcript that differs from the plain djs run is a hard failure. Added to
      the gate next to `--suspend-every 97`; TESTING.md updated.
- [ ] e2e: `DJS_E2E_MODE=dump`.
- [ ] Coverage: vm::dump_harvest at every idle point of the harvested cases and of the private
      scripts (counts by refusal reason) in the result. Target: every harvested idle point.
- [ ] Measure the dumper's wasm size (feature `dump`) before and after.

## Log

## Follow-ups

## Result
