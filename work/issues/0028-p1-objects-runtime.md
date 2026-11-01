# 0028: P1-2 runtime: call_intrinsic, prelude installation, ToPrimitive slow paths, Error stack

status: done
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
- [x] `call_intrinsic id argc` dispatch over the shared intrinsic table (format defined
      by the lexer agent in 0027's log; one source of truth). Leaf intrinsics in Rust for
      everything `builtins/intrinsics.d.ts` declares that the current builtins use.
      Reentrant ones (Get/Set/Call per D1173) push frames via continuations.
- [x] Prelude installation: the VM compiles `builtins/gen/prelude.js` in privileged mode
      at `Vm::new` (measure the cost; ARCH wants a base snapshot eventually) and installs
      the methods. `Array.prototype.{forEach,map,indexOf,join,includes}` from the
      prelude work end to end.
- [x] Operators on objects: `+`, comparisons, `==`, ToNumber/ToString/ToPropertyKey of
      objects go through the privileged ToPrimitive (valueOf/toString). There is no
      "not supported yet" left for these.
- [x] Error objects get `stack` (qjs format), and uncaught errors in `djs run` print it.
- [x] e2e cases for all of the above, in the three modes. Un-ignore 0027's `_0028` tests
      (none exist on main yet; 0027 part 2 adds them, see Follow-ups).
- [x] Startup cost of `Vm::new` with the prelude, recorded (wasm and native).

## Notes
- 0027 (lexer agent) is adding the compiler side right now. Start with the parts that do
  not need it (Error stack, slow-path plumbing, prelude installation design) and
  rebase when 0027 lands.

## Log
- 2026-11-01: Error `stack` first (no 0027 needed): frame walk from the faulting pc,
  `ret_pc - 1` for callers, the line table, qjs format, 10 frames. Compared with qjs on
  a program with nested calls, an anonymous function, a top-level fault and deep
  recursion. Everything matches except caller positions of `return f()`: qjs reports
  those at the `return` keyword (it compiles a tail call there), our line table at the
  `(` (a compiler question, see Follow-ups). `djs run` prints the stack after
  `Uncaught ...`. `Vm::error_stack(v)` is new, additive.
- Asked for the intrinsic table format (D1270: ids = index into
  `bytecode::intrinsics::INTRINSICS`, Builtin kind = installed JS). Rebased onto 0027
  part 1 (ac51f50).
- Implemented:
  - `interp/run/intrinsics.rs`: all 38 Rust intrinsics, Builtin dispatch through
    `Realm::INSTALLED` (new field, additive), `%InstallMethod` / `%InstallIntrinsic`.
  - `Continuation::Replace(slot)` and the `primitive_operands` slow path (D1280);
    `get_method` / `get_elem_method` through getters.
  - The 0027 ops `set_proto`, `set_function_name` (prefix none / get / set; symbols as
    `[desc]`), `call_spread`, `construct_spread`.
  - Prelude installation in `Vm::new` (D1281), `OBJ_BUILTIN` (a new header flag,
    additive) for `at map (native)` frames, and `Intrinsic::PreludeScript`.
- Checked against qjs: valueOf/toString precedence per hint, left-to-right conversion
  order (`a + b`, `a < b`, `b > a` give `ababba`, as qjs), TypeError `toPrimitive` when
  both return objects, exceptions out of valueOf, objects as property keys,
  `++` on an object, prelude methods with ToPrimitive-converted arguments
  (`indexOf(2, {valueOf})`), and builtin stack frames.
- Tests:
  - `vm/tests.rs`: stack traces; a method call through a getter (heap-defined
    accessor, since object-literal getters come with 0027 part 2); ToPrimitive through
    a *host* `valueOf`, including an image round trip while parked.
  - e2e: `to_primitive`, `array_builtins`, `builtin_errors`, `getters_in_methods`. The
    e2e test now starts every case from one shared base image. `loops_big` was cut to
    5k iterations: with the prelude's larger heap, GC stress and suspension (copy +
    verify at every stop) took 14 s and 8 s in debug; now 4 s and 7 s.
- Cost (D1281): `Vm::new` 92 µs native release (7 µs without the prelude), 146 µs in
  djs-wasm under node 25 (11 µs before), heap 36 KB (17 KB). `docs/host.md`'s startup
  table updated (outside `touches:`, one line).
- `js/src/layout.generated.ts` regenerated for the realm/flag changes.
- Merge: 0016 (djs-difftest) landed meanwhile and builds `VmOptions` with every field
  listed, so the new `prelude` field broke it. Added `..VmOptions::default()` there
  (crates/djs-difftest/src/djs.rs, one line, outside `touches:`). The difftest engine
  now runs with the prelude.
- 0030 note: symbol-keyed `%InstallMethod` needs `key: Str` in intrinsics.d.ts widened
  (bgen's file), so it stays with 0030.

## Follow-ups
- Builtins gap: `Object.prototype.toString` / `valueOf`, `Array.prototype.toString`,
  `push`, ... do not exist. So `({}) + 1` is `TypeError: toPrimitive` (qjs:
  `[object Object]1`), and `[[1,2]].join()` fails the same way. These belong in
  builtins/ (Object.prototype.toString needs a builtin-tag intrinsic).
- 0027 part 2's `_0028` tests: whoever lands second un-ignores them.
- Base snapshot (ARCH G2): `Vm::new` is at 92 µs native and 146 µs wasm with today's
  small prelude.
- Compiler: caller positions for `return f()` (qjs: at `return`).
- `Error.prototype.stack` as an accessor (qjs) rather than an own data property, if
  anything observes the difference. Error objects created by JS code (`new Error`)
  get a stack once the Error constructor exists.
- `ToObject` of primitives (wrapper objects) is "not supported yet".

## Result
- `call_intrinsic`: every intrinsic of `builtins/intrinsics.d.ts` (Rust, leaf and
  reentrant), and the installed `@intrinsic` builtins (D1281).
- The prelude is installed by `Vm::new`; `Array.prototype.{forEach,map,indexOf,join,
  includes}` work end to end. `VmOptions::prelude = false` gives a bare realm.
- Operators, template substitutions and property keys convert objects through
  `%ToPrimitive` (valueOf/toString/@@toPrimitive) via the `Replace` continuation (D1280).
  Method calls through getters work. No "not supported yet" is left for these.
- Errors carry `stack` in qjs format (D1282), and `djs run` prints it.
- New 0027 ops: `set_proto`, `set_function_name`, `call_spread`, `construct_spread`.
- Startup: 92 µs native, 146 µs wasm (D1281).
- Known gaps: see Follow-ups (missing Object/Array builtins, base snapshot, wrapper
  objects).
