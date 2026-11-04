# 0192: Constructs into slots: for-of/for-await records, switch discriminant, using, destructuring; `loops` key gone

status: done
agent: sm
branch: issue/0192-constructs-into-slots
depends-on: 0191
guide: -
touches: crates/djs-core/src/compiler/ (for_in_of, stmt, using, pattern), vm/dump_*, tests/smap, tests/dump

## Goal
docs/suspension-design-review.md, section 4 "Migration plan", step 4; decisions D2720
(state machines in the compiler, SM), D2721 (slot naming construct@span), D2722 (host-call
state with save/resume hooks). Read the whole report first: it describes this step's
design, what stays and what goes, the risks and the tests required at every step:
smap pins and golden dumps (rewritten only with a decision, D2619), DJS_E2E_MODE=dump,
difftest --dump-every-idle, the harvested/private idle-point counts at 100%
(vm::dump_harvest), test262 async/generator suites where relevant, benches before/after
where the report says so. Each step merges green on its own.

## Log
- 2026-11-04: Construct temporaries (D2732): `VarKind::Temp` locals named `<tN>`, slot
  names in `FuncState::temps` (`for-of.iterator@18`), parser helpers in compiler/temps.rs.
  Moved: for-of/for-in/for-await iterator records, yield*'s, switch discriminant (the
  case test's copy is `binary.left@case`), using Record (the disposal's stack copy is
  gone), array pattern iterator/next, object pattern source/excluded keys. Roles
  LoopIterator, LoopNext, SwitchDiscriminant, UsingRecord, Pattern{Source,Excluded,Next,
  Iterator} and the `Loop` enum deleted.
- Liveness pass over phase-1 code decides at which states a temporary is a slot (handler
  edges only from THROWS instructions; ret returns to its subroutine's gosubs). Exporter
  and loader dispatch on construct names (for-in iterator, from-sync `next`).
- Found: generator return path closing a pattern iterator inside the pattern's own close
  handler closed it twice when `return()` threw (13 test262 dstr tests); the return path
  now marks it `undefined` first; the for-of close handler skips an `undefined` iterator.
- Perf: `get it; get next; iter_next` would be +1 dispatch per for-of step (+2.9% on
  forof.js); new op `iter_next_loc` (CodeBuf::finish handles a jump with a prefix
  immediate) makes it 1 dispatch: forof -7% native/-3.6% wasm, dstr -7.8%/-7.4%, no
  workload above +0.5% (D2732 has the table). Instructions retired vs a build of 36a7f9ed
  in work/scratch/0192/main.
- Tests: djs-core lib, e2e all modes, workspace tests, test262 active (ratchet OK),
  difftest --ratchet --suspend-every 97 --gc-stress --dump-every-idle (identical),
  harvest 1164/1164, private 5257/5257 idle points.
- Rewritten under D2732: 14 golden dumps (the other 116 load unchanged), pins, compiler
  snapshots. Docs: continuation-dumps.md, compiler.md notes, guide interpreter.md.

## Follow-ups

## Result
Statement state lives in named locals (construct temporaries, D2732), listed in a state's
slots only where live; the `loops` key and the per-construct stack roles are gone. A
for-of step is one `iter_next_loc`. Dumps of loops keep their slot names.
