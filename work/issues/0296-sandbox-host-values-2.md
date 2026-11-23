# 0296: Sandbox for pi, part 2: cyclic plain values throw at the call, read-only host globals, Error message coercion, null-prototype namespaces

status: done
agent: pns
branch: issue/0296-sandbox-host-values-2
depends-on: 0295
touches: js/src/sandbox.ts, js/src/reader.ts, js/src/wire.ts, js/test/, docs/guide/js-api.md, docs/decisions.md
guide: docs/guide/js-api.md

## Goal
pi on djs (agent pidjs) after 0295; repros with `jsonBoundary: { except: ["text"] }`,
`frozenBuiltins`, `missingMember` set:

1. **A cyclic value to a plain (`except`) sync host function** fails the whole run on the
   host side: `const c = {}; c.self = c; try { text(c) } catch (e) { return e.name }` ends
   the run with a `sandbox` error "cyclic value at $.self.self...". Wanted: a TypeError at
   the call that the script can catch, as the JSON boundary does (pi's `text()`/`store()`
   messages depend on it). Same for any other value the reader cannot read.
2. **Host globals are writable**: `globalThis.tools = null; return typeof tools` replaces
   the namespace. pi's prelude made tools/console/text/image/exit/store/load/ALL_TOOLS
   non-writable, non-configurable globals. Wanted: globals the sandbox binds (`globals`,
   `values`, namespaces) are non-writable and non-configurable (a strict assignment
   throws TypeError). Dumps: check that a dump and load keep them so.
3. **An Error whose message is not a string** arrives with message "":
   `const e = new Error('x'); e.message = 42; throw e;` -> `run.error.value.message === ""`
   (in the VM `String(e)` is "Error: 42"). Coerce name and message as `String()` would.
   Low priority.
4. **Namespaces with a null prototype**, as pi's were: with `missingMember`, `tools.toString`
   is Object.prototype's (inherited names never reach the hook). If cheap, namespace
   objects (and the Proxy's target) have a null prototype.

## Acceptance criteria
- [x] 1-4 with tests (with and without the boundary; dump and load for 2 and 4)
- [x] guide and decisions

## Log
- 2026-11-23: (1) readCall catches the reader's TypeError/RangeError per argument: a sync
  call completes with `TypeError: argument N: <reason>` (function not called), an async
  call's promise rejects with it (never listed; the run loop runs on at once, else the
  run sat idle with the rejection in the inbox), console gets `[<reason>]` as text.
- (2) and (4) need engine bits (coordinator agreed, part of ABI 8, no bump): vm_define
  bit 2 = non-writable + non-configurable (`Vm::define_with` replaces
  `define_enumerable`), vm_new_object(vm, flags) bit 0 = null prototype
  (`Vm::new_null_proto_object`). Size: +83 bytes raw, +63 gzip (767,603 -> 767,666,
  ceiling 768,597). bindGlobal defines read-only; namespaces and module namespaces null
  prototype; the missing-member trap still skips Object.prototype's names (as pi's guard).
  The existing 0295 test expected `typeof tools.toString === "function"`: now "undefined".
- (3) the reader's textProperty: primitives as String() (number, boolean, null, BigInt);
  undefined/symbol/accessor/object give the defaults.
- Tests: js/test/host-values-2.test.ts (1-4, with/without boundary, dump/load for 2 and
  4); djs-wasm `define_read_only_and_null_prototype_objects`. Ran all js tests, djs-wasm
  tests, clippy (native and wasm), docs check.
- Guide: js-api.md (read-only globals, null prototype, unreadable arguments, coercion).

## Follow-ups
- A read-only global keeps the old object if a host redefines a function with another
  kind (sync <-> async): no host does it; could be a misuse.

## Result
Landed: (1) unreadable (cyclic, too large) arguments to host functions are a catchable
TypeError at the call (async: a rejected promise), console gets the reason as text;
(2) host-bound globals non-writable and non-configurable, also after a load; (3) Error
name/message primitives coerced like String(); (4) namespaces and module namespaces with
a null prototype. Engine: vm_define bit 2, vm_new_object flags (ABI 8). +63 bytes gzip.
Decisions D3620-D3623.
