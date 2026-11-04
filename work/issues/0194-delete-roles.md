# 0194: Delete the role vocabulary, operand placement and gosub mapping; format {at, awaiting, this, args, slots, queue}

status: done
agent: sm
branch: issue/0194-delete-roles
depends-on: 0192, 0193
guide: -
touches: crates/djs-core/src/compiler/ (Role enum, unsupported markers, depth checks), vm/dump_resume.rs, vm/dump_flight.rs, docs/continuation-dumps.md, docs/functions-as-code.md

## Goal
docs/suspension-design-review.md, section 4 "Migration plan", step 6; decisions D2720
(state machines in the compiler, SM), D2721 (slot naming construct@span), D2722 (host-call
state with save/resume hooks). Read the whole report first: it describes this step's
design, what stays and what goes, the risks and the tests required at every step:
smap pins and golden dumps (rewritten only with a decision, D2619), DJS_E2E_MODE=dump,
difftest --dump-every-idle, the harvested/private idle-point counts at 100%
(vm::dump_harvest), test262 async/generator suites where relevant, benches before/after
where the report says so. Each step merges green on its own.

## Log
- Scope (after 0191-0193): the format was already `{at, awaiting, this, args, slots,
  queue}` (0191, D2731), gosub mapping gone (0193), the loader/exporter read slots by name
  (no role-specific edges left: `Holds::Temp(Option<Role>)` carried a role nobody read).
  What remained: the `Role` enum and `Role::slot_name`/`parts`/Display, `Site::operands`,
  `Role::Unsupported("class")` as a pseudo-role, `Role::Exception` as the last
  statement-level stack value, and docs.
- Done: parser hooks `names_push/names_pop/names_replace(at, &["construct"])`
  (`&'static str`, no index), `unsupported_begin/end` for class definitions; `Site::stack`
  (names, bottom first) replaces `operands` + `slots`; `Place::Temp(k)` renamed
  `Place::Stack(k)`; `Holds::Temp`. for-await's L_throw stores the exception in a
  temporary `for-await.exception@S` before closing (same slot name, now a local). Depth
  check kept as a consistency check (D2734). Test fixture `roles.*` renamed `operands.*`.
- Pins: only local numbering (async_generators, using, async-helpers: one more local per
  for-await) and the summary's construct lists. Goldens: none changed (regenerated
  for-await-close.* byte-identical). Snapshot async_generators.disasm.
- Checks: lib tests, e2e plain/dump, test262 for-await-of/class/AsyncGeneratorPrototype
  (ratchet OK), difftest --dump-every-idle --suspend-every 97 --gc-stress (OK), harvest
  1164/1164, private 1260 cases no refusals, nesting test, clippy, docs check. Size main
  441,254 (+27), slim 376,600 (+36). No hot path changed (for-await throw path only), no
  bench run.
- Docs: continuation-dumps.md (suspension maps section, slot names); functions-as-code.md
  had nothing about roles; guide pages: none affected.

## Follow-ups
- Class definitions stay refused (`unsupported class`); describing them would need a way
  to write a half-made class, not just names.
- (from 0193) for-of close block writes `completion.kind` even with a single exit.

## Result
The role vocabulary is gone: suspension maps name operand-stack values by construct and
offset only (`Site::stack`), statements keep nothing on the operand stack at any site
(for-await's pending exception is a temporary), class definitions are refused by a marker
(D2734). Slot names, dumps and the format are unchanged.
