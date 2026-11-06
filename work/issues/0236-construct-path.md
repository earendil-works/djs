# 0236: Performance: the construct path (`new C()`, `new Map()`, `new Promise`)

status: done
agent: pnew
branch: issue/0236-construct-path
depends-on: -
guide: interpreter.md if the call/construct story changes
touches: interp (construct/new, OrdinaryCreateFromConstructor, prototype lookup), builtins constructors (Map, Set, Promise, Error, Array, user classes), compiler if `new` codegen changes

## Goal
pasync (0232b) measured, wasm instructions per op on the shipped module vs QuickJS 2026:
`new C()` of a user class 5.9k vs 2.5k (2.4x), `new Map()` 9.9k vs 2.1k (4.6x),
`new Promise(() => {})` 12.5k vs 5.1k (2.7x, after 0232b's promise fixes): the general construct
path is slow, not the classes. Codemode constructs Map/Set/Date/Error/Promise and small classes
often. Find the root cause (Get(newTarget, "prototype") lookups, OrdinaryCreateFromConstructor in
bytecode, derived-class this binding, builtin constructors' frames, string-key re-interning noted
in 0231) and fix it; target parity with QuickJS. Remember 0230/0231's finding: any edit to
run_hot's own code can cost plain loops 4-6% in V8; keep changes out of its body (out-of-line
helpers) and measure plain calls/loops each time. Exact semantics; dumps unaffected (fuzz regress
both modes). Size: the shipped module has ~5.9 KB to the 430 KB budget after 0232b; keep it small
and report.

## Log

2026-11-06 (pnew)
- Measured (wmicro, shipped-profile wasm with names, -n 20000, djs vs quickjs-wasi): new C()
  4.3k vs 3.1k, new C(i) 5.0k vs 4.1k, derived D 11.2k vs 5.5k, new F(i) 4.0k vs 3.9k,
  new Map() 10.1k vs 2.9k, new Promise 16.5k vs 8.7k, new Error 33k vs 18.8k, new Array(4)
  15.0k vs 3.5k, new Date(0) 17.2k vs 2.9k. New cases in scripts/perf/cases.py.
- Root cause: every instruction of the construct path left run_hot for `step` (construct,
  the return into a constructing frame, special_object new.target/active function,
  check_ctor/check_this/check_this_uninit/check_ctor_return, get_super, super_call, and the
  allocation intrinsics CollectionCreate/PromiseCreate/...). One such trip costs ~1000 wasm
  instructions (`call f(){return new.target}` 2.1k vs 1.0k for `return 1`), whatever the
  instruction does. GetPrototypeFromConstructor was a frame plus %Get with an interned key
  (2.7k of new Map()'s 10k).
- Fixes: (1) `decline_construct` (interp/run/construct.rs), called from `decline_fast`
  (out of line; run_hot's body unchanged): construct/super_call of bytecode functions and
  builtin constructors whose frame fits, the return into a ConstructResult frame, the
  cheap special_object kinds, the class checks, get_super. Each either completes as `step`
  would or changes nothing. A cheap decline then costs ~300. (2) construct_call pushes the
  frame with FLAG_CONSTRUCT and new.target in its header before the entry safepoint (no
  handle for an explicit new.target; `push_frame_head` takes it). (3) %ConstructorPrototype
  leaf: GetPrototypeFromConstructor in Rust unless a getter/proxy is on the way;
  GetPrototypeFromConstructor is now an inlined helper calling it, the old steps are
  GetPrototypeFromConstructorSlow. (4) CollectionCreate, PromiseCreate, ErrorCreate,
  DateCreate, NewBuiltinClosure are pure leaves (hot loop runs them). (5) `new` entries
  in fast.rs (FastFn::new) for Array, Map, Set, Date: `new X(...)` with new.target X
  answers in Rust where no user code can run; tried before `this` is inserted in both
  paths (else the stack garbage above sp differs and hot_loop_is_invisible fails).
- Tried and dropped: NEW_TARGET in run_hot's SPECIAL_OBJECT arm (~4% on new Map(), plain
  loops ±5% noise; not worth touching run_hot), a Latin-1 fast path in str_to_string for
  Error's stack (8% on new Error with a long file name, +340-530 B raw).
- Tests: hot_loop_is_invisible now constructs classes, derived classes, Map/Set/Date/
  Error/Promise and plain functions. lib, e2e plain, test262 (Array, Map, Set, Date,
  Promise, Error, NativeErrors, Reflect, Proxy, WeakMap, TypedArrayConstructors, new,
  super, new.target, class; native and bytecode backends) and difftest (stress on
  language/builtins/errors) pass.

- Later (same day): `rest` and `super(...args)` without step (derived default constructor
  10.0k -> 8.9k); decline_construct is reached only for its opcodes (`construct::takes`: an
  out-of-line call for every other decline cost props +2%); ErrorStack/IsPendingStack pure
  leaves; the compiler returns the checked `this` when a derived constructor falls off its
  body (epilogue only when a `return` uses it; E 8.2k -> 7.6k). Tried and dropped: the class
  check arms in run_hot's body (arrays +1.6%, tight +0.8% instructions), running on through
  trivial ops inside decline_construct (C and F got slower), a define-free builtin closure
  (0232b made them lazy meanwhile; dropped at the rebase).
- Rebased on 0232b: its @fast GetPrototypeFromConstructor entry is replaced by the inlined
  %ConstructorPrototype leaf (no decline). Decisions D3160-D3164; size approved (D3164).
- Guide: interpreter.md (the construct path's declines), builtins.md (constructors' `new`
  entries); docs/compiler.md (derived constructor epilogue); builtins/README.md (@fast new).
- Corpus case builtins/construct-path.js. Fuzz regress, plain and --lazy: only the
  pre-existing s13-s10173 refusal (same on main's gate build) and the known deliberate one.

## Follow-ups
- new Error: 30.1k vs 18.7k wasm instructions. ~60% is CaptureStack (trace_frames converts
  file and function names to Rust Strings with decode_utf16, malloc), and
  DefineDataPropertyOrThrow/CaptureStack go through step.
- Derived classes still 1.2-1.6x QuickJS: ~10 cheap declines (~300 each) per `new D()`; a
  `super_call` that reads the active function and new.target from its frame (a new op) would
  remove three of them.

## Result
The construct path no longer goes through `step`: `decline_construct` (interp/run/construct.rs)
finishes construct, super calls, constructing returns, new.target and the class checks;
GetPrototypeFromConstructor is the inlined leaf %ConstructorPrototype; constructor allocations
are pure leaves; Array/Map/Set/Date have `new` entries (FastFn::new). Wasm instructions per op,
main -> branch (quickjs-wasi): new C() 4.3k -> 3.2k (3.2k), new C(i) 5.0k -> 3.9k (4.2k),
new F(i) 4.0k -> 3.1k (3.8k), derived 11.2k -> 8.9k (5.5k), derived super(a) 10.7k -> 7.6k
(6.2k), new Map()/Set() 7.4k -> 2.1k (2.9k), new Promise 10.2k -> 8.5k (8.7k), new Array(4)
12.6k -> 1.7k (3.4k), new Date(0) 14.8k -> 1.7k (2.9k). Plain workloads within +0.6%
instructions; run_hot's body unchanged. Size +2,715 B gzip shipped (426,806), +2,680 slim
(D3164). Gaps: new Error (1.6x) and derived classes (1.2-1.6x), see Follow-ups.
