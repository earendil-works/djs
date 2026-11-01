# 0028: P1-2 runtime: call_intrinsic, prelude installation, ToPrimitive slow paths, Error stack

status: in-progress
agent: heap
branch: issue/0028-p1-objects-runtime
depends-on: 0022, 0027 (intrinsic id table format)
touches: crates/djs-core/src/interp/, crates/djs-core/src/vm/, builtins/ (intrinsics needed by the slow paths; coordinate with bgen's rules), crates/djs-core/src/heap/ (additive)

## Goal
docs/compiler.md section 10, row P1-2, runtime half, plus 7.3: the leaf intrinsics
behind `call_intrinsic`, installation of the bgen prelude (D1172:
`%InstallMethod`/`%InstallIntrinsic`) at VM creation (compiled at startup for now; base
snapshot later), the slow paths that call privileged JS (ToPrimitive for operators on
objects, ToPropertyKey of objects, method calls through getters, …), and Error `stack`
from the line table (the 0025 contract: the current pc on top, `ret_pc - 1` for callers,
`at name (file:line:col)`, qjs format).

## Acceptance criteria
- [ ] `call_intrinsic id argc` dispatch over the shared intrinsic table (format defined
      by the lexer agent in 0027's log; one source of truth). Leaf intrinsics in Rust for
      everything `builtins/intrinsics.d.ts` declares that the current builtins use.
      Reentrant ones (Get/Set/Call per D1173) push frames via continuations.
- [ ] Prelude installation: the VM compiles `builtins/gen/prelude.js` in privileged mode
      at `Vm::new` (measure the cost; ARCH wants a base snapshot eventually) and installs
      the methods. `Array.prototype.{forEach,map,indexOf,join,includes}` from the
      prelude work end to end.
- [ ] Operators on objects: `+`, comparisons, `==`, ToNumber/ToString/ToPropertyKey of
      objects go through the privileged ToPrimitive (valueOf/toString). There is no
      "not supported yet" left for these.
- [ ] Error objects get `stack` (qjs format), and uncaught errors in `djs run` print it.
- [ ] e2e cases for all of the above, in the three modes. Un-ignore 0027's `_0028` tests.
- [ ] Startup cost of `Vm::new` with the prelude, recorded (wasm and native).

## Notes
- 0027 (lexer agent) is adding the compiler side right now. Start with the parts that do
  not need it (Error stack, slow-path plumbing, prelude installation design) and
  rebase when 0027 lands.

## Log

## Follow-ups

## Result
