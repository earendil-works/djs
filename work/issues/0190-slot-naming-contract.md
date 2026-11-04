# 0190: Slot naming contract (construct@span) and pins listing slots per state

status: done
agent: sm
branch: issue/0190-slot-naming-contract
depends-on: -
guide: -
touches: docs/continuation-dumps.md, crates/djs-core/src/compiler/smap.rs (report named slots per site, additive), tests/smap (pin format lists slots), no behaviour change

## Goal
docs/suspension-design-review.md, section 4 "Migration plan", step 2; decisions D2720
(state machines in the compiler, SM), D2721 (slot naming construct@span), D2722 (host-call
state with save/resume hooks). Read the whole report first: it describes this step's
design, what stays and what goes, the risks and the tests required at every step:
smap pins and golden dumps (rewritten only with a decision, D2619), DJS_E2E_MODE=dump,
difftest --dump-every-idle, the harvested/private idle-point counts at 100%
(vm::dump_harvest), test262 async/generator suites where relevant, benches before/after
where the report says so. Each step merges green on its own.

## Log
- 2026-11-04: Recorder roles carry a source offset (`roles_push(at, rs)`, `roles_put`,
  `roles_replace(n, at, rs)`); `Site::slots` gives each operand's name `construct@S`
  (`Role::slot_name`), relative to the function start. Offsets per D2730: a list
  element's own start, else the construct's start. Copies of one value (switch case
  discriminant, await-using disposal values) get the `case`'s/site's offset so names stay
  unique. Pins list names instead of roles (same order); summary.txt unchanged (role
  sequences). Unit tests: names per construct, uniqueness over the construct tables; the
  pin test also fails on a duplicate name. Contract written in continuation-dumps.md
  "Slot names". Guide: none affected (the format does not change until 0191).

## Follow-ups

## Result
The slot naming contract (D2721, made exact by D2730) is in docs/continuation-dumps.md
"Slot names". `compiler::smap::Site::slots` names each operand below the awaited value
(`call.callee@41`, `for-of.iterator@18`), bottom first, beside `operands`; the pins in
tests/smap list those names per site. No dump format, code or behaviour change (the pin
test still checks that recording leaves the code alone).
