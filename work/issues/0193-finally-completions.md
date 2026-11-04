# 0193: finally completions in slots (functions that can suspend); gosub index mapping and return-path refusals gone

status: done
agent: sm
branch: issue/0193-finally-completions
depends-on: 0191
guide: interpreter.md
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

- Design (D2733): finally blocks, `using` disposals and for-of closes on a jump out are
  code entered with a jump after storing the completion in `completion.kind@S` /
  `completion.value@S` temporaries (S: `try` keyword, `for`, first `using` binding).
  Kinds: 0 normal, 1 throw, 2 return, `3+2o`/`4+2o` break/continue to the statement at
  function-relative offset o (named by source, not emission order). Block end
  dispatches; chained exits enter the next block out with the same kind. New
  compiler/completion.rs (`push_block`, `emit_unwind(from, Exit)`, `end_completion`).
- Universal (all functions), not only suspendable ones: one lowering, and `gosub`/`ret`
  could go entirely (ops, GOSUB flag, stack checker and liveness subroutine handling).
  Measured below: no workload regresses beyond +0.6%.
- Deleted the D2627 machinery: roles FinallyValue/FinallyReturn, `Recorder::subs`,
  `subroutine()`, `Site::returns`, resolve's gosub list, loader/exporter index mapping.
  `OpenIter::ForOf` gone: the for-of's `Iter` env is pushed while its head target is
  stored, so a generator's return from a yield there leaves through the close block
  (which is outside the loop's close handler, so the L_throw "skip undefined" check went
  too). `emit_async_close_normal` lost its `describe` parameter.
- Refusals gone: "closing an async iterator"/"return from a generator" (the return path
  emits no sites now) and "a completion value" (Script/eval keep it in
  `completion.saved@S`). The "async generator awaiting a return() value" refusal is a
  heap-state one (not the compiler's), untouched.
- Statements keep nothing on the operand stack any more (D2732 + this), so
  `stmt_depth`, `BlockEnv::depth`, `UsingRegion::base` and the drop loops before jumps and
  returns are gone (also what made the size budget fit).
- Liveness: a write of completion.kind also defines completion.value (resolve.rs passes
  `also` to `temps::live_at`); the rethrow clears the value. So the value is listed only
  inside its block.
- Tests: smap_tests tables updated; pins (pcs, frame indexes, new slot names) and
  compiler snapshots rewritten; 19 golden dumps (finally/for-await-close/using) rewritten
  under D2733 (using.1/9/11 regenerated differently only by slot order: restored, they
  load); new golden program return-path (2 dumps, one at the formerly refused for-await
  close on a generator's return path); resume_refuses cases rewritten for the new names;
  bytecode stack test rewritten without gosub, vm finally_with_gosub test deleted.
- Ran: lib tests, e2e (all modes), difftest `--ratchet --dump-every-idle --suspend-every
  97 --gc-stress` (OK), harvest 100% / private extract 100% (no refusals, no load errors),
  test262 active group (ratchet OK, 40 expected; covers statements/try, generators, async).
- Perf (instructions retired, base f7bd3f3d in work/scratch/0193/main): native: fin.js
  (try/finally, return through finally, for-of exits) -4.46%; others -1.71%..+0.54%
  (loop +0.54%, tight +0.30%, empty +0.31%). wasm (wmeasure n=4): fin -3.8%; others
  -0.1%..+0.6% (prop1 +0.54%, asyncloop +0.52% spread 1.5%, call1 +0.39%).
- Size: main 441,240 (base 441,396), slim 376,525 (base 376,638) gzip.
- First merge attempt: the gate's js nesting test trapped (`while` nested 500 deep in
  wasm): BlockEnv grew (pos + completion, 32 -> 56 bytes) and every `self.blocks.pop()`
  in a recursive statement parser took a slot for the popped value; parse_try's new
  locals (inlined into parse_statement) too. Fixed: `pop_block()` truncates in place,
  `push_block(&mut labels)`, the finally tail (`end_try_finally`) and the Script
  completion save out of line, the normal-exit closure inlined by hand, the completion
  stays in the env (kind `Labeled`, unlabeled: inert) while the finally block is
  parsed. wasm frames now: parse_statement 112 (base 128), parse_while/do_while 32 (48),
  parse_labeled 32 (64); every construct reaches its SyntaxError at 550 in the release
  and dev modules.
- Docs: decisions D2733; continuation-dumps.md (completions, refusals, slot names);
  compiler.md 4.2/4.3/4.4/7.4/7.7/7.10/9.3; ARCH.md exceptions line; guide
  interpreter.md prose (finally) + regen.

## Follow-ups

- `Role::Exception` (`for-await.exception@S`, the body's exception under the close's
  await in a for-await's throw handler) is still an operand-stack role; 0194 can move it
  into a temporary (or the loop's completion) when the role vocabulary goes.
- The for-of close block writes `completion.kind` even when only one exit enters it (no
  test reads it then): two dead instructions on that path; harmless.
- tests/dump/v2/golden/for-await-close.src.js's comment still says "close subroutine"
  (the program text is part of its goldens; left as is).

## Result

Finally completions live in temporaries in every function (D2733): `completion.kind@S`
and `completion.value@S` replace the gosub return address, so dumps name a pending exit
by its kind code (statement offsets for break/continue) and the loader/exporter have no
return-address mapping. `gosub`/`ret` are gone from the instruction set. The refusals
for a for-await close on a generator's return path and for Script completion values are
gone. Perf neutral to slightly better; size slightly smaller.
