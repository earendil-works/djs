// The Sandbox API: loads djs-wasm (docs/host.md) and runs code in a djs VM. Values go in
// through the wire format (wire.ts) and come out through direct heap reads (reader.ts).

import { IN_PLACE_MIN, encode, stringInPlace } from "./wire.ts";
import { elasticStack, trapText } from "./trap.ts";
import { djsExports } from "./abi.ts";
import { Opaque, readValue } from "./reader.ts";
import { provideBase, rememberBase } from "./base.ts";
import { envVar } from "./env.ts";
import { ImportSyntaxError, leadingImports, lineColumn } from "./imports.ts";

/** A VM value with no host form (a function, a symbol, a WeakMap, ...), as values out carry it. */
export { Opaque };

/** A host function callable from the sandbox. Arguments and the result are host values. */
export type HostFunction = (...args: unknown[]) => unknown;

/**
 * A host module (`SandboxOptions.modules`, D3597): what a codemode body's
 * `import ... from specifier` binds. Each export (`default` for a default import) names a
 * host function, namespace or value of the sandbox by its identity.
 */
export interface HostModule {
  exports: Record<string, string>;
}

/**
 * Where `console.*` goes. The arguments are host values as the reader reads them: an
 * Error as a host `Error` (the VM's `name`, `message` and `stack`), a Date, Map or Set as
 * one, and a value with no host form as the text `[Class]` (`[Function]`, `[WeakMap]`).
 */
export interface ConsoleLike {
  log(...args: unknown[]): void;
  info(...args: unknown[]): void;
  warn(...args: unknown[]): void;
  error(...args: unknown[]): void;
  /** `console.debug`; without it, `log` receives those calls. */
  debug?(...args: unknown[]): void;
}

export interface SandboxOptions {
  /** The compiled djs-wasm module; default: `loadModule()`. */
  module?: WebAssembly.Module;
  /** Heap cap in bytes; default: the engine's (4 GiB). */
  memoryLimit?: number;
  /** Fuel per `vm_run` call: how often the host gets control back. Default 1,000,000. */
  fuelSlice?: number;
  /**
   * Fuel budget of one run, after which it fails with "out of fuel" (the call is
   * abandoned; the sandbox stays usable): exact, and across all the steps of a
   * host-driven run (`Run.fuelUsed`, which a snapshot carries over to the loaded run).
   * Default: none.
   */
  maxFuel?: number;
  /**
   * Host functions, by identity (D2970): any string (`tools.read`, `@sdk/our-stuff#myTool`),
   * which calls, journals and snapshots name the function by. A call parks the VM until the
   * function returns. An identity `ns.member` of two identifiers makes the function member
   * `member` of the namespace object `ns`, which the sandbox makes for its first member and
   * freezes once it has them all. Where scripts find functions and namespaces is `globals`
   * and `modules`.
   */
  functions?: Record<string, HostFunction>;
  /**
   * Async host functions, by identity (D1374): a call returns a promise in the VM at
   * once and the script goes on, so several calls can be outstanding (`Promise.all`
   * over tools). `run` and `runAsync` call the function at the next idle point; it may
   * return a value or (`runAsync`) a promise. A host-driven run never calls it: the call
   * is listed in `Run.calls` and the host settles it. Identities as in `functions`.
   */
  asyncFunctions?: Record<string, HostFunction>;
  /**
   * The global bindings of host functions and namespaces: global name -> identity of a
   * function or namespace (`{ tools: "tools", console: "console", myTool:
   * "@sdk/our-stuff#myTool" }`, D2972). Without it, the one written rule: `console`, every
   * namespace and every function whose identity is an identifier are globals of their
   * own names (`tools.read` makes the global `tools`).
   */
  globals?: Record<string, string>;
  /**
   * Host data bound as globals: deep-frozen copies of values the host gives every run
   * (`ALL_TOOLS`), each a global of its name and a host value of that identity (for
   * `modules`). Snapshots name them and never write them, so a sandbox that loads such
   * a snapshot must be given them again. A name that is also a host
   * function's or namespace's identity, or a bound global, is a misuse. Default: none.
   */
  values?: Record<string, unknown>;
  /**
   * The JSON boundary: the arguments of every call of a host function given in `functions`
   * and `asyncFunctions`, and the result of every run, are converted inside the VM as
   * `JSON.parse(JSON.stringify(v))` (`toJSON` is called, a Date becomes its string, a Map
   * `{}`, `undefined` and functions are left out of objects; an `undefined` argument stays
   * `undefined`). A value JSON cannot carry (a cycle, a BigInt) fails at the call: a sync
   * function's call throws the TypeError, an async one's returns a promise rejected with
   * it, and a result fails the run. The functions scripts see are then the sandbox's
   * converting functions, one per identity, which snapshots name by identity like the
   * functions themselves. `console` is not converted, nor the functions whose
   * identities `{ except: [...] }` lists (`text`, `store`: they get plain wire values,
   * `NaN` and functions included). Default false: arguments and results cross as wire
   * values (a Date as a `Date`, a function as an `Opaque`).
   */
  jsonBoundary?: boolean | { except: string[] };
  /**
   * What reading a member a namespace object does not have does (`tools.Echo` where
   * `tools` has `echo`): called with the namespace's identity and the member's name, it
   * returns the message of the TypeError the read throws, or `undefined` for the read to
   * give `undefined` as usual. Not asked for symbols, members of `Object.prototype`,
   * `then` and `toJSON`, so `await`, `JSON.stringify` and `in` work as they do. The
   * namespaces are then Proxies over their frozen member objects; a snapshot still names
   * them by identity. Applies to the namespaces of dotted identities
   * (`console` included), not to `values` or module namespaces. Default: none.
   */
  missingMember?: (namespace: string, member: string) => string | undefined;
  /**
   * More member names of namespace members: identity `ns.member` -> names that are not
   * identifiers (`{ "tools.my_tool": ["my-tool"] }`), defined in `ns` right after
   * `member`, enumerable, holding the same function (`tools["my-tool"] === tools.my_tool`).
   * An alias is no identity of its own: calls, journals and snapshots name the function by
   * `ns.member`. Default: none.
   */
  aliases?: Record<string, string[]>;
  /**
   * The host modules codemode bodies import (D3597): `import { a } from "spec"`, `import *
   * as ns from "spec"` and `import d from "spec"` at the top of a body bind host values,
   * as parameters of the body after `params`. Called once per specifier
   * (lazily: for the modules scripts import, and for those a loaded snapshot names); an
   * unknown module (`undefined`) or export is a `syntax` error of the run. A namespace
   * import is one frozen object per specifier, a host value named by the specifier (as
   * snapshots name it); an export is the host value of its identity. Default: no modules.
   */
  modules?: (specifier: string) => HostModule | undefined;
  /** Receives `console.*` calls from the sandbox; default: the host's `console`. */
  console?: ConsoleLike;
  /**
   * The clock `Date.now()` and `new Date()` see, in ms since the epoch (the VM reads no
   * clock itself, D1383): a fixed time (replays) or a function; default `Date.now`.
   * Set before every fuel slice.
   */
  now?: number | (() => number);
  /**
   * The VM's local time zone, in minutes east of UTC (fixed, no DST); default: the
   * host's current offset. Set before every fuel slice.
   */
  timeZoneOffset?: number;
  /**
   * Promises rejected with no handler once the VM's job queue ran empty (as qjs reports
   * them, D1423; a rejected result promise does not count): `"warn"` (default) passes
   * each reason (its display form, `TypeError: x`) to `console.warn`,
   * `"throw"` fails the run with a `threw` SandboxError, `"ignore"` drops them.
   */
  unhandledRejections?: "warn" | "throw" | "ignore";
  /**
   * Parse the code as TypeScript (types are erased, enums and namespaces run; 0048).
   * Default false.
   */
  typescript?: boolean;
  /**
   * The 64-bit seed of `Math.random` for a new sandbox (a restored one continues its
   * sequence). Default: from `crypto.getRandomValues`. Ignored with `deterministic`.
   */
  randomSeed?: number | bigint;
  /**
   * Frozen builtins (D2304): the builtin objects (prototypes, constructors, `Math`, ...)
   * behave as frozen: writes to them fail (a TypeError in strict code), their properties
   * read as non-writable and non-configurable, and `Object.isFrozen` is true. Assigning
   * over an inherited builtin property (`obj.toString = f`, `this.name = ...` in an Error
   * subclass) defines an own property instead of failing, as after SES's lockdown.
   * `globalThis` stays ordinary. Default false (the spec's behavior); a restored sandbox
   * keeps the mode it was created with.
   */
  frozenBuiltins?: boolean;
  /**
   * Run deterministically (D1680): with the same code, parameters, sync host function
   * results and async results in the same order, a run makes the same calls and reaches
   * the same state at every idle point, however it is sliced, stepped or snapshotted.
   * `Math.random` is seeded from `seed`, the time zone is UTC, weak references are strong
   * (targets never die and cleanup callbacks never run, so no collection is observable),
   * and the clock starts at `startedAt` and moves only when an async result is applied: to
   * `now` at its `settle` (never backwards), which `Run.deliveries` records. Sync host
   * functions and fuel slices do not move it. A run re-run from its script with its
   * deliveries (`BeginOptions.deliveries`) is the fallback when a snapshot no longer loads.
   * Snapshots of such a run load only into a sandbox that is deterministic too.
   */
  deterministic?: { startedAt: number; seed: number | bigint };
  /**
   * With `typescript`: TypeScript's legacy `experimentalDecorators` (tsc's decorator
   * semantics and parameter decorators) instead of the standard decorators (0065).
   * Default false.
   */
  experimentalDecorators?: boolean;
  /**
   * With `typescript`: `emitDecoratorMetadata` (`design:type` and friends through
   * `Reflect.metadata`); implies `experimentalDecorators`. Default false.
   */
  emitDecoratorMetadata?: boolean;
  /**
   * Makes `debugger;` a pause (D3050): in an async function, the statement awaits a call
   * of the async host function with the identity `"debugger"`, bound as the global
   * `debugger`: this function, which `run` and `runAsync` call, or with `true` one that
   * returns at once. In a host-driven run the pause is an open call like any other: the
   * run is idle at the statement and can be snapshotted there, and the host settles it to
   * go on; a sandbox with this option that loads the snapshot pauses again at the next
   * `debugger;`. Outside async functions the statement does nothing, as without the option.
   */
  debugger?: HostFunction | boolean;
}

export type SandboxErrorKind = "threw" | "syntax" | "fuel" | "trap" | "memory" | "misuse" | "diverged" | "aborted";

/**
 * One entry of a run's journal (D1730, docs/host.md "Journals"): everything a
 * deterministic run took from outside, in order, plus checks. Values are host values
 * (JSON-serializable when the host's values are); a host error is `{ name, message }`.
 * - `run`: the start: the engine build, the code, the parameters, the script's name and
 *   the canonical hash of the VM before the run.
 * - `sync`: a sync host function's call and outcome (console output is not journaled).
 * - `async`: an async host function's result, in the order the run applied them, with
 *   the clock it carried.
 * - `check`: the VM was idle (quiescent) after `delivered` results, with `issued` host
 *   calls made and this canonical hash.
 * - `end`: the outcome (`ok` false: the error's display text), with the final counts and
 *   hash.
 */
export type JournalEntry =
  | { type: "run"; engine: string; code: string; params: Record<string, unknown>; filename?: string; hash: string }
  | { type: "sync"; fn: string; args: unknown[]; ok: boolean; value: unknown }
  | { type: "async"; callId: number; fn: string; args: unknown[]; ok: boolean; value: unknown; now: number }
  | { type: "check"; delivered: number; issued: number; hash: string }
  | { type: "end"; ok: boolean; value: unknown; delivered: number; issued: number; hash: string };

/**
 * A result a deterministic run applied (`Run.deliveries`): the call (id, function,
 * arguments), its outcome as settled, and the clock the run saw from then on. The
 * deliveries of a run, in order, are all a host needs to re-run it from its script
 * (`BeginOptions.deliveries`).
 */
export interface Delivery {
  id: number;
  fn: string;
  args: unknown[];
  ok: boolean;
  value: unknown;
  now: number;
}

/** A source position (`Sandbox.callSite`): the script's name, line and column from 1. */
export interface CallSite {
  file: string;
  line: number;
  column: number;
}

/** Options of one run. */
export interface RunOptions {
  /** Record the run (deterministic sandboxes only): entries are appended to this array. */
  journal?: JournalEntry[];
  /** The script's name, as stack traces show it (`sleep.js:4:13`); default `sandbox.js`. */
  filename?: string;
}

/** An async host call the script made that has no result yet (`Run.calls`). */
export interface OpenCall {
  /** The VM's call id: `settle` takes it, and a snapshot keeps it. */
  id: number;
  /** The host function's name, as given in `asyncFunctions` (`sleep`, `tools.read`). */
  fn: string;
  args: unknown[];
  /** A loaded call's state, as snapshotted (D2770); absent when the snapshot has none. */
  state?: unknown;
}

/** A host call's outcome for `Run.settle`: a value, or (`ok` false) an error to reject with. */
export interface Settlement {
  ok: boolean;
  value: unknown;
}

/**
 * Options of `Sandbox.begin`. A `journal` records the run while it stays in this sandbox:
 * the sync calls, every applied result in order and a check at every idle point; `replay`
 * checks a re-run against it.
 */
export interface BeginOptions extends RunOptions, StepOptions {
  /**
   * Run the first step now (default), with the step options given here (but `yieldMs`:
   * `begin` returns synchronously). `false`: compile and start the call, run nothing, and
   * return the run in state `"ready"`; its first `advance` runs the first step, which can
   * then yield.
   */
  start?: boolean;
  /**
   * Re-run (deterministic sandboxes only): the deliveries of an earlier run of the same
   * script, parameters and sandbox options (`Run.deliveries`), applied in order, each
   * once its call is open, before `begin` returns. The run is then where the earlier one
   * was after its last delivery, and goes on as it would have. A call the re-run does not
   * make, or makes with another function or other arguments, is a `diverged` SandboxError
   * naming the first difference. No async host function runs; sync host functions run as
   * they did (the host stubs those whose effects it already has: output, stores).
   */
  deliveries?: Delivery[];
}

/**
 * Bounds of one step of a host-driven run (`Run.advance`, `begin`'s first step): the step
 * stops between fuel slices when one is reached. A step that stops this way ends the run
 * (as `maxFuel` does): it rejects with a SandboxError, and the run's state is `"threw"`.
 */
export interface StepOptions {
  /** Checked before every fuel slice: once aborted, the step fails with kind `aborted`. */
  signal?: AbortSignal;
  /** The step's fuel budget: past it, the step fails with kind `fuel`. Default: none. */
  fuel?: number;
  /**
   * Yield to the host's event loop (a macrotask) between fuel slices once the step has run
   * this many milliseconds since it started or last yielded, so an in-thread host stays
   * responsive while a script computes (and a `signal` can be aborted meanwhile). Only in
   * `advance`. Default: never (the step runs to its end in one go, but for sync host
   * functions that return promises).
   */
  yieldMs?: number;
}

/**
 * Thrown by a host function to end the run at once (`exit()`): the script cannot catch
 * it, and no more script code runs. The run ends as done with `value` (`Run.exited` is
 * true; `run` and `runAsync` return it); its open calls are dropped. A sync host function
 * throws it from its call; an async one in `runAsync` throws it or rejects with it, and a
 * host-driven run's host can settle a call with it (`{ ok: false, value: new RunExit() }`):
 * the run exits when the result is applied. What the script did before is what the host
 * saw.
 */
export class RunExit extends Error {
  readonly value: unknown;
  constructor(value?: unknown) {
    super("the run exited");
    this.name = "RunExit";
    this.value = value;
  }
}

/** Why a sandbox operation failed. `value` is the thrown VM value (kind "threw"). */
export class SandboxError extends Error {
  readonly kind: SandboxErrorKind;
  readonly value?: unknown;
  constructor(kind: SandboxErrorKind, message: string, value?: unknown) {
    super(message);
    this.name = "SandboxError";
    this.kind = kind;
    this.value = value;
  }
}

const STATUS_DONE = 0;
const STATUS_THREW = 1;
const STATUS_OUT_OF_FUEL = 2;
const STATUS_HOST_CALL = 3;
const STATUS_IDLE = 4;
const STATUS_HOST_OUT_OF_MEMORY = 5;
const ABI_VERSION = 13;
/** `vm_new`'s flag: frozen builtins (D2304). */
const VM_FROZEN_BUILTINS = 1;
const CONSOLE_METHODS = ["log", "info", "warn", "error", "debug"] as const;
/**
 * `SandboxOptions.missingMember`, as the namespaces' Proxies call it; not a host function
 * of an identity. Its function id is outside the host functions' (ids are places in
 * `fnNames`).
 */
const HOOK_MISSING = 0x7fff_fff4;
/**
 * `vm_define`'s flags: bit 0 enumerable (D3592), bit 1 freeze the object afterwards
 * (D3611), bit 2 non-writable and non-configurable (D3621).
 */
const DEFINE_ENUMERABLE = 1;
const DEFINE_FREEZE = 2;
const DEFINE_READONLY = 4;
/** `vm_new_object`'s flag: a null prototype (D3623). */
const NULL_PROTO = 1;
/**
 * The sandbox's own helper in the VM, made only for `values`, `jsonBoundary` and
 * `missingMember`: one function `(op, a, b, c)`. op 0: `a` through JSON (the boundary's
 * conversion); 1: deep-freezes `a` (a host value); 2 and 3: the converting function of
 * sync (2) or async (3) host function `a`, named `b`; 4: `a` through JSON, or undefined
 * when JSON cannot carry it (a thrown value); 5: a Proxy over the namespace object `a`
 * named `b` whose reads of missing members ask `c` (`missingMember`). It takes the builtins it uses when it is made, before any
 * script runs, so a script that replaces `JSON.stringify` does not change the boundary.
 * Nothing a snapshot writes holds it: the converting functions are host values (D3592, D3593).
 */
const HELPER = `"use strict";
(() => {
  const apply = Reflect.apply, json = JSON, stringify = JSON.stringify, parse = JSON.parse, def = Object.defineProperty;
  const freezeOne = Object.freeze, isFrozen = Object.isFrozen, keys = Object.keys, isView = ArrayBuffer.isView;
  const P = Promise, reject = Promise.reject, Px = Proxy, get = Reflect.get, TE = TypeError, capture = Error.captureStackTrace, OP = Object.prototype;
  const clone = (v) => { const j = apply(stringify, json, [v]); return j === undefined ? undefined : apply(parse, json, [j]); };
  const cloneOr = (v) => { try { return clone(v); } catch { return undefined; } };
  const guard = (o, ns, missing) => {
    const h = { get(t, p, r) {
      if (typeof p === "string" && !(p in t) && !(p in OP) && p !== "then" && p !== "toJSON") {
        const m = apply(missing, undefined, [ns, p]);
        if (typeof m === "string") { const e = new TE(m); apply(capture, undefined, [e, h.get]); throw e; }
      }
      return apply(get, undefined, [t, p, r]);
    } };
    return new Px(o, h);
  };
  const each = (a) => { const n = a.length, out = []; for (let i = 0; i < n; i++) def(out, i, { value: clone(a[i]), writable: true, enumerable: true, configurable: true }); return out; };
  const freeze = (o) => { if (o !== null && typeof o === "object" && !isView(o) && !isFrozen(o)) { freezeOne(o); const ks = keys(o); for (let i = 0; i < ks.length; i++) freeze(o[ks[i]]); } };
  const wrap = (f, name, async) => {
    const w = async
      ? (...a) => { let c; try { c = each(a); } catch (e) { return apply(reject, P, [e]); } return apply(f, undefined, c); }
      : (...a) => apply(f, undefined, each(a));
    def(w, "name", { value: name });
    return w;
  };
  return (op, a, b, c) => op === 0 ? clone(a) : op === 1 ? freeze(a) : op === 4 ? cloneOr(a) : op === 5 ? guard(a, b, c) : wrap(a, b, op === 3);
})()`;
/** An identity `ns.member` of two identifiers: member `member` of the namespace `ns` (D2632). */
const NAMESPACE_MEMBER = /^([A-Za-z_$][\w$]*)\.([A-Za-z_$][\w$]*)$/;
const IDENTIFIER = /^[A-Za-z_$][\w$]*$/;

interface Exports {
  memory: WebAssembly.Memory;
  djs_abi_version(): number;
  djs_staging(len: number): number;
  djs_out_ptr(): number;
  djs_out_len(): number;
  djs_panic_ptr(): number;
  djs_panic_len(): number;
  __stack_pointer: WebAssembly.Global;
  djs_debug_panic(): void;
  vm_new(memoryLimit: number, flags: number): number;
  /** `line:col:file` of the activation waiting on a call, in the out buffer (D3054). */
  vm_call_site(vm: number, callId: number): number;
  vm_free(vm: number): void;
  vm_arena_ptr(vm: number): number;
  vm_arena_len(vm: number): number;
  vm_collect(vm: number): number;
  vm_compile_file(vm: number, goal: number, nameLen: number, paramsLen: number, srcLen: number): number;
  vm_global(vm: number): number;
  /** `flags` bit 0 (D3623): a null prototype. */
  vm_new_object(vm: number, flags?: number): number;
  vm_new_host_function(vm: number, fnId: number, nameLen: number): number;
  vm_new_async_host_function(vm: number, fnId: number, nameLen: number): number;
  /** `flags` bit 0 (D3592): enumerable; bit 1 (D3611): freeze `obj` afterwards. */
  vm_define(vm: number, obj: number, nameLen: number, value: number, flags?: number): number;
  vm_decode_wire(vm: number, len: number): number;
  vm_string_reserve(vm: number, len: number): number;
  vm_new_error(vm: number, kind: number, msgLen: number): number;
  vm_handle_bits(vm: number, h: number): void;
  vm_handle_free(vm: number, h: number): void;
  vm_display(vm: number, h: number): number;
  vm_call(vm: number, f: number, thisArg: number, argc: number): number;
  vm_run(vm: number, fuel: number): number;
  vm_fuel_used(vm: number): number;
  vm_result(vm: number): number;
  vm_host_call(vm: number): number;
  vm_host_call_info(vm: number, callId: number): number;
  vm_new_host_calls(vm: number): number;
  vm_promise_state(vm: number, h: number): number;
  vm_take_unhandled_rejections(vm: number): number;
  vm_resolve_host_call(vm: number, callId: number, ok: number, value: number): number;
  vm_abort(vm: number): void;
  vm_set_now(vm: number, ms: number): void;
  vm_set_time_zone_offset(vm: number, minutes: number): void;
  vm_set_random_seed(vm: number, lo: number, hi: number): void;
  vm_outcome(vm: number): number;
  vm_set_weak_refs(vm: number, mode: number): number;
  vm_resolve_host_call_at(vm: number, callId: number, ok: number, value: number, now: number): number;
  vm_canonical_hash(vm: number): number;
  djs_build_hash(): number;
  vm_tables_need(vm: number): number;
  /** A logical snapshot (D3700, D3720): staging holds the host's names and data. */
  vm_snapshot(vm: number, len: number): number;
  vm_load_snapshot(vm: number, len: number): number;
  djs_snapshot_needs(len: number): number;
  djs_snapshot_describe(len: number): number;
  djs_elastic_stack(on: number): void;
  djs_stack(len: number): number;
  /** The base snapshot (D1321), coded in the module (D2252). */
  djs_base_ptr(): number;
  djs_base_len(): number;
  djs_base_inject_ptr(len: number): number;
  djs_base_inject(): number;
  djs_base_coded_ptr(len: number): number;
}

/** The modules being compiled or compiled, by location. */
const modules = new Map<string, Promise<WebAssembly.Module>>();
/** The compiled ones, for the synchronous constructor. */
const compiled = new Map<string, WebAssembly.Module>();

/**
 * The default location of djs-wasm: `$DJS_WASM`, else, running from the repository's
 * sources (js/src/*.ts), the `wasm` profile build. The package's entry point has its own
 * default (`Sandbox.defaultModule`), the module shipped in the package (D3106, D3602).
 */
export function defaultWasmPath(): string | URL {
  const env = envVar("DJS_WASM");
  if (env !== undefined) return env;
  // Not `new URL("…", import.meta.url)` literally: bundlers take that for an asset of the
  // package, and the built package has no repository around it.
  const here = import.meta.url;
  if (here.endsWith(".ts")) return new URL("../../target/wasm32-unknown-unknown/wasm/djs_wasm.wasm", here);
  throw new SandboxError("misuse", "no default djs-wasm module: pass a location or set $DJS_WASM");
}

/**
 * Compiles djs-wasm once per location and caches the `WebAssembly.Module` (ARCH: the
 * module is compiled once; every sandbox gets its own instance). A `file:` URL, or a
 * string that is no URL in node, Bun or Deno, is read from the file system; anything else
 * is fetched (in a browser, a relative string is relative to the page).
 */
export function loadModule(location: string | URL = defaultWasmPath()): Promise<WebAssembly.Module> {
  const key = String(location);
  let m = modules.get(key);
  if (m === undefined) {
    m = (async () => {
      let bytes: Uint8Array;
      if (isFile(location)) {
        // Not a literal specifier, so the package type-checks without node's types, and
        // bundlers for the browser leave it alone.
        const fsModule = "node:fs/promises";
        const fs = (await import(/* @vite-ignore */ /* webpackIgnore: true */ fsModule)) as {
          readFile(p: string | URL): Promise<Uint8Array>;
        };
        bytes = await fs.readFile(typeof location === "string" && /^file:/i.test(location) ? new URL(location) : location);
      } else {
        const r = await fetch(location);
        if (!r.ok) throw new SandboxError("misuse", `cannot fetch djs-wasm from ${key}: HTTP ${r.status}`);
        bytes = new Uint8Array(await r.arrayBuffer());
      }
      return WebAssembly.compile(bytes as Uint8Array<ArrayBuffer>);
    })();
    modules.set(key, m);
    m.then(
      (mod) => compiled.set(key, mod),
      () => modules.delete(key),
    );
  }
  return m;
}

/** Whether `loadModule` reads `location` from the file system (else it fetches it). */
function isFile(location: string | URL): boolean {
  // A scheme of two letters or more: `C:\x` is a path.
  const url = typeof location === "string" ? /^([a-z][a-z0-9+.-]+):/i.exec(location)?.[1] : location.protocol.slice(0, -1);
  if (url !== undefined) return url.toLowerCase() === "file";
  const g = globalThis as { process?: { versions?: { node?: string } }; Deno?: unknown; Bun?: unknown };
  return g.process?.versions?.node !== undefined || g.Deno !== undefined || g.Bun !== undefined;
}

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();
/** A run in progress, as its sandbox keeps it. */
interface RunCore {
  run: Run;
  /** The open async calls, oldest first, until their results are applied. */
  open: Map<number, OpenCall>;
  /** Fuel used so far: `maxFuel` is per run, across all its steps. */
  used: number;
  /**
   * With `jsonBoundary`: the result (or a thrown value that is no Error) went through the
   * conversion, which runs once; `threw` and `message`: it was thrown, with this display text.
   */
  converted?: { threw: boolean; message: string };
  /** The calls still open when the run ended (its result settled, or `RunExit`), by id. */
  dropped: Map<number, OpenCall>;
  /** A host function ended the run (`RunExit`): applied before the next fuel slice. */
  exit?: RunExit;
  /** The bounds of the step in progress. */
  step?: StepOptions;
  /** A step is running: settlements wait in `pending` until its next fuel slice. */
  busy: boolean;
  pending: { call: OpenCall; ok: boolean; value: unknown }[];
  /** Deterministic sandboxes: the results applied, in order. */
  deliveries: Delivery[];
}

/** What a `Run` asks of its sandbox. */
interface RunOps {
  advance(core: RunCore, step: StepOptions): Promise<RunEnd>;
  settle(core: RunCore, id: number, s: Settlement): void;
  snapshot(core: RunCore, states: [number, unknown][]): Uint8Array;
}

type RunEnd = { state: "idle" } | { state: "done"; value: unknown; exited?: boolean } | { state: "threw"; error: SandboxError };

/**
 * A run the host drives (docs/host.md "Host-driven runs"): `Sandbox.begin` starts a body,
 * `advance` runs it to its next idle point, the host settles the open async calls in any
 * order, and while it is idle it can be snapshotted (`snapshot()`) and continued elsewhere
 * (`Sandbox.loadSnapshot`). The async host functions are never called by the run: `calls`
 * lists them and the host answers, so that a call can outlive the process that issued it.
 * Sync host functions are called as the script calls them. `run` and `runAsync` are loops
 * over such a run.
 */
export class Run {
  /**
   * "ready": started with `start: false`, nothing ran yet (`advance` runs the first step);
   * "idle": waiting on `calls`; "done": `value` is the result; "threw": `error`.
   */
  state: "ready" | "idle" | "done" | "threw" = "idle";
  value: unknown = undefined;
  error: SandboxError | undefined = undefined;
  /** Done because a host function ended the run (`RunExit`); `value` is the exit's. */
  exited = false;
  readonly #sandbox: Sandbox;
  readonly #core: RunCore;
  readonly #ops: RunOps;

  /** Made by `Sandbox.begin` and `Sandbox.loadSnapshot`. */
  constructor(sandbox: Sandbox, core: RunCore, ops: RunOps) {
    this.#sandbox = sandbox;
    this.#core = core;
    this.#ops = ops;
  }

  /** The sandbox the run is in. */
  get sandbox(): Sandbox {
    return this.#sandbox;
  }

  /** The async host calls waiting for a result, oldest first. */
  get calls(): OpenCall[] {
    return [...this.#core.open.values()];
  }

  /**
   * The fuel the run used so far, across its steps (and, after `loadSnapshot`, the
   * snapshot's): what `maxFuel` bounds (D3614). A snapshot carries it.
   */
  get fuelUsed(): number {
    return this.#core.used;
  }

  /**
   * The calls that were still open when the run ended (D3612): its result settled while
   * they were running, or a host function exited it. The run no longer waits for them;
   * the host cancels them, and settling one is a no-op.
   */
  get dropped(): OpenCall[] {
    return [...this.#core.dropped.values()];
  }

  /**
   * In a deterministic sandbox: the results this run applied since `begin` (re-run ones
   * included) or `loadSnapshot`, in order, with their clocks. With the deliveries before a
   * snapshot, the host can re-run the run from its script (`BeginOptions.deliveries`)
   * when the snapshot no longer loads. Empty otherwise.
   */
  get deliveries(): readonly Delivery[] {
    return this.#core.deliveries;
  }

  /**
   * Delivers the result of open call `id`. The VM applies results one at a time in the
   * order they were settled, each once the script has nothing else to run (D1670), so
   * where a result lands does not depend on when it was settled.
   */
  settle(id: number, s: Settlement): void {
    this.#ops.settle(this.#core, id, s);
  }

  /**
   * Runs to the next idle point (or the end): applies the settled results, and calls the
   * sync host functions the script calls meanwhile (awaiting them if they return
   * promises). New async calls show up in `calls`.
   */
  async advance(step: StepOptions = {}): Promise<void> {
    if (this.state !== "idle" && this.state !== "ready") throw new SandboxError("misuse", `the run is ${this.state}`);
    this.state = "idle";
    try {
      this.end(await this.#ops.advance(this.#core, step));
    } catch (e) {
      // The run is over (out of fuel, aborted, a host-side failure).
      if (e instanceof SandboxError) this.end({ state: "threw", error: e });
      throw e;
    }
  }

  /**
   * The waiting run as a logical snapshot (D3700, D3720): its state as data, scripts
   * included, for `loadSnapshot` in a fresh sandbox made with the same options, in this or
   * another process. The sandbox's host functions, namespaces, `values` and module
   * namespaces are named, not written: the loading sandbox has its own. Only while idle.
   * `states` are the host's states of open calls, by id (JSON data): the load gives each
   * back with its call (`OpenCall.state`); `fuelUsed` and a deterministic run's clock
   * travel with the run.
   */
  snapshot(opts: { states?: Record<number, unknown> } = {}): Uint8Array {
    if (this.state !== "idle") throw new SandboxError("misuse", `the run is ${this.state}: only an idle run can be snapshotted`);
    return this.#ops.snapshot(this.#core, Object.entries(opts.states ?? {}).map(([id, v]) => [Number(id), v]));
  }

  /** Records how a step ended. */
  end(e: RunEnd): void {
    this.state = e.state;
    if (e.state === "done") {
      this.value = e.value;
      this.exited = e.exited === true;
    }
    if (e.state === "threw") this.error = e.error;
  }
}

/** `drive`'s value at an idle point of a run. */
const IDLE = Symbol("idle");

export class Sandbox {
  private readonly x: Exports;
  private readonly module: WebAssembly.Module;
  private readonly vm: number;
  private readonly opts: SandboxOptions;
  /** Host function names by id - 1. */
  private readonly fnNames: string[] = [];

  private readonly fns = new Map<number, HostFunction>();
  private readonly atomCache: (string | undefined)[] = [];
  private readonly cons: ConsoleLike;
  private dead: string | null = null;
  /** Deterministic mode: the VM's clock, moved only by async results (never backwards). */
  private clock: number | undefined;
  /** The journal of the current run, being recorded or replayed (D1730). */
  private journal: Journal | null = null;
  /**
   * The host values that are namespaces, by identity, as handles: the objects this
   * sandbox made for dotted function names (`tools`; with `missingMember` the Proxies
   * over them), module namespaces and `values`.
   */
  private readonly namespaces = new Map<string, number>();
  /** The member objects of the namespaces of dotted function names, by identity. */
  private readonly nsTargets = new Map<string, number>();
  /** While the constructor defines them: each namespace's last member (name and handle). */
  private readonly nsLast = new Map<string, [string, number]>();
  /** `missingMember` as a function in the VM (`HOOK_MISSING`), as a handle; 0 until needed. */
  private missingFn = 0;
  /** The host function objects by id, as handles: one per identity (D2970). */
  private readonly fnHandles = new Map<number, number>();
  /**
   * With `jsonBoundary`: the converting functions scripts see instead of the host
   * functions, by id, as handles; host values of the functions' identities.
   */
  private readonly wrappers = new Map<number, number>();
  /** The sandbox's helper in the VM (`HELPER`), as a handle; 0 until needed. */
  private helper = 0;
  /** `opts.modules`'s answers, by specifier (null: no such module). */
  private readonly hostModules = new Map<string, HostModule | null>();
  /** The specifiers whose namespace objects this sandbox made (in `namespaces`). */
  private readonly moduleNamespaces = new Set<string>();
  /** The globals the sandbox bound to host functions and namespaces. */
  private readonly globalNames = new Set<string>();
  /** Whether the instance has what snapshots need of the base (`snapshotBase`). */
  private snapshotsReady = false;
  /** The run in progress (`begin`, `loadSnapshot`, `run`, `runAsync`), until it ends. */
  private active: RunCore | null = null;
  /** What the runs of this sandbox ask of it. */
  private readonly ops: RunOps = {
    advance: (core, step) => this.advanceRun(core, step),
    settle: (core, id, s) => this.settleRun(core, id, s),
    snapshot: (core, states) => {
      if (this.active !== core) throw new SandboxError("misuse", "the run has ended");
      return this.snapshotRun(core, states);
    },
  };

  /**
   * Where `create` and the constructor find the module when `opts.module` is not given:
   * here `defaultWasmPath()`; the package's entry point overrides it with the package's
   * module (D3106, D3602).
   */
  static defaultModule(): string | URL {
    return defaultWasmPath();
  }

  /** A new sandbox; loads (and caches) the default module unless `opts.module` is given. */
  static async create(opts: SandboxOptions = {}): Promise<Sandbox> {
    return new this({ ...opts, module: opts.module ?? (await loadModule(this.defaultModule())) });
  }

  /**
   * Loads a snapshot of a run (`Run.snapshot`, D3720) into this sandbox, a fresh one made
   * with the same options as the one that took it, for the host to drive: the run is
   * idle, waiting on the same open calls with the same ids (`run.calls`, with their
   * states), and its `fuelUsed` goes on from the snapshot's. The sandbox's host
   * functions, namespaces and values are its own (by identity); the snapshot refuses to
   * load when it names one this sandbox does not have, when a function waiting in it
   * compiles differently in this build, or when the sandboxes differ in `deterministic`
   * or `frozenBuiltins`. A refusal leaves the sandbox unusable.
   */
  loadSnapshot(snapshot: Uint8Array): Run {
    return this.loadSnapshotRun(snapshot).run;
  }

  /**
   * A snapshot described for people: its sections and sizes, record types, builtins,
   * functions and the run's state. Any sandbox of the module that wrote it can read it.
   */
  describeSnapshot(snapshot: Uint8Array): string {
    const x = this.x;
    this.stage(snapshot);
    if (x.djs_snapshot_describe(snapshot.length) !== 0) throw new SandboxError("misuse", `not a snapshot: ${outText(x)}`);
    return outText(x);
  }

  /**
   * A new sandbox (its own wasm instance and VM), synchronously: needs a compiled module,
   * `opts.module` or one `loadModule()` has finished loading. Everything the sandbox
   * defines comes from its options; its namespaces are frozen before any script runs.
   */
  constructor(opts: SandboxOptions = {}) {
    let module = opts.module;
    if (module === undefined) {
      try {
        module = compiled.get(String((new.target as typeof Sandbox).defaultModule()));
      } catch {
        // No default location: the error below.
      }
    }
    if (module === undefined) {
      throw new SandboxError("misuse", "no compiled module: pass opts.module or await loadModule() first");
    }
    let x: Exports;
    try {
      x = elasticStack(djsExports(module) as unknown as Exports, module);
    } catch (e) {
      throw new SandboxError("misuse", `djs-wasm: ${(e as Error).message}`);
    }
    if (x.djs_abi_version() !== ABI_VERSION) {
      throw new SandboxError("misuse", `djs-wasm ABI ${x.djs_abi_version()}, expected ${ABI_VERSION}`);
    }
    // The module's base, decoded once by its first instance, not by each (D2252), and
    // kept for later instances before `vm_new` takes this instance's bytes as its arena
    // (D2060).
    provideBase(module, x);
    rememberBase(module, x);
    const vm = x.vm_new(opts.memoryLimit ?? 0, opts.frozenBuiltins ? VM_FROZEN_BUILTINS : 0);
    if (vm === 0) throw new SandboxError("memory", "memory limit too small for the realm");
    this.x = x;
    this.module = module;
    this.vm = vm;
    this.opts = opts;
    const cons = opts.console ?? (globalThis.console as ConsoleLike);
    this.cons = cons;
    // `console` is a namespace like `tools` (a host value snapshots name, D2971).
    for (const m of CONSOLE_METHODS) {
      this.addFunction(`console.${m}`, (...a) => (cons[m] ?? cons.log).apply(cons, a), false, false);
    }
    for (const [name, f] of Object.entries(opts.functions ?? {})) this.addFunction(name, f, false);
    for (const [name, f] of Object.entries(opts.asyncFunctions ?? {})) this.addFunction(name, f, true);
    if (opts.debugger !== undefined && opts.debugger !== false) {
      this.addFunction("debugger", typeof opts.debugger === "function" ? opts.debugger : () => undefined, true, false);
    }
    // The namespaces of dotted names have all their members: frozen (D3611), by defining
    // the last member again with `DEFINE_FREEZE`, so no script code runs.
    this.guard(() => {
      for (const [ns, [name, h]] of this.nsLast) this.defineOn(this.nsTargets.get(ns)!, name, h, DEFINE_ENUMERABLE | DEFINE_FREEZE);
    });
    const globals = { ...(opts.globals ?? this.defaultGlobals()) };
    if (opts.debugger !== undefined && opts.debugger !== false) globals.debugger = "debugger";
    for (const [name, identity] of Object.entries(globals)) this.bindGlobal(name, identity);
    // Host data (`values`): deep-frozen copies, host values like the namespaces, so a
    // snapshot names them instead of writing them.
    for (const [name, value] of Object.entries(opts.values ?? {})) {
      if (this.hostValue(name) !== undefined || this.globalNames.has(name)) {
        throw new SandboxError("misuse", `value ${name}: the name of a host function, namespace or global already`);
      }
      let h: number;
      try {
        h = this.guard(() => this.toHandle(value));
      } catch (e) {
        if (e instanceof SandboxError) throw e;
        throw new SandboxError("misuse", `value ${name}: ${(e as Error).message}`);
      }
      if (h === 0) continue;
      this.guard(() => this.x.vm_handle_free(vm, this.callHelper(1, h)));
      this.namespaces.set(name, h);
      this.bindGlobal(name, name);
    }
    // The boundary converts results with the helper, also with no host function.
    if (opts.jsonBoundary && this.helper === 0) this.guard(() => this.x.vm_handle_free(vm, this.callHelper(1, 0)));
    // What a new VM starts with (D1680).
    const det = opts.deterministic;
    const [lo, hi] = det ? seedWords(det.seed) : opts.randomSeed !== undefined ? seedWords(opts.randomSeed) : entropy();
    x.vm_set_random_seed(vm, lo, hi);
    if (det) {
      // Strong weak references: no collection is observable.
      x.vm_set_weak_refs(vm, 1);
      this.clock = det.startedAt;
      x.vm_set_now(vm, det.startedAt);
      x.vm_set_time_zone_offset(vm, 0);
    }
  }

  /** Makes host function `name` and, for `ns.member`, adds it to the namespace `ns`. */
  private addFunction(name: string, f: HostFunction, async: boolean, convert = this.converts(name)): void {
    if (typeof f !== "function") throw new SandboxError("misuse", `host function ${name}: not a function`);
    if (this.fnNames.includes(name)) throw new SandboxError("misuse", `host function ${name}: defined twice`);
    const m = NAMESPACE_MEMBER.exec(name);
    for (const alias of this.opts.aliases?.[name] ?? []) {
      if (!m) throw new SandboxError("misuse", `aliases of ${name}: not a namespace member (ns.member)`);
      if (IDENTIFIER.test(alias)) throw new SandboxError("misuse", `alias ${alias} of ${name}: an identifier (it would be the member of an identity ${m[1]}.${alias})`);
    }
    this.guard(() => {
      this.fnNames.push(name);
      const id = this.fnNames.length;
      this.fns.set(id, f);
      // One function object per identity (D2970): a snapshot names the one the sandbox bound.
      const len = this.stageText(name.slice(name.lastIndexOf(".") + 1));
      let h = this.check(async ? this.x.vm_new_async_host_function(this.vm, id, len) : this.x.vm_new_host_function(this.vm, id, len), "memory");
      this.fnHandles.set(id, h);
      if (convert) {
        // The JSON boundary: scripts get the converting function, never the host function.
        const n = this.toHandle(name.slice(name.lastIndexOf(".") + 1));
        h = this.callHelper(async ? 3 : 2, h, n);
        this.x.vm_handle_free(this.vm, n);
        this.wrappers.set(id, h);
      }
      if (m) {
        // Enumerable, in definition order, each alias right after its member (D3611).
        const target = this.namespace(m[1]);
        let last = m[2];
        this.defineOn(target, m[2], h, DEFINE_ENUMERABLE);
        for (const alias of this.opts.aliases?.[name] ?? []) {
          this.defineOn(target, alias, h, DEFINE_ENUMERABLE);
          last = alias;
        }
        this.nsLast.set(m[1], [last, h]);
      }
    });
  }

  /** Whether the JSON boundary converts host function `identity`'s arguments (D3613). */
  private converts(identity: string): boolean {
    const b = this.opts.jsonBoundary;
    return b === true || (typeof b === "object" && b !== null && !b.except.includes(identity));
  }

  /**
   * Calls the sandbox's helper (`HELPER`, made on first use) with `op` and the handles
   * `args`, to its end: no script runs meanwhile, and
   * it calls no host function. Returns the result's handle; an exception is a misuse.
   */
  private callHelper(op: number, ...args: number[]): number {
    const x = this.x;
    const vm = this.vm;
    const finish = (): number => {
      let st = x.vm_run(vm, 1_000_000);
      while (st === STATUS_OUT_OF_FUEL) st = x.vm_run(vm, 1_000_000);
      const h = x.vm_result(vm);
      if (st === STATUS_DONE) return h;
      const why = st === STATUS_THREW ? this.display(h) : `status ${st}`;
      if (h !== 0) x.vm_handle_free(vm, h);
      x.vm_abort(vm);
      throw new SandboxError("misuse", `the sandbox's helper failed: ${why}`);
    };
    if (this.helper === 0) {
      const src = textEncoder.encode(HELPER);
      this.stage(src);
      const f = this.check(x.vm_compile_file(vm, 0, 0, 0, src.length), "memory");
      this.stage(new Uint8Array(0));
      const r = x.vm_call(vm, f, 0, 0);
      x.vm_handle_free(vm, f);
      if (r !== 0) throw new SandboxError("misuse", "a call is already in progress");
      this.helper = finish();
    }
    this.startHelper(op, args);
    return finish();
  }

  /** Starts the call of the helper (made already) with `op` and `args`; the caller runs it. */
  private startHelper(op: number, args: number[]): void {
    const x = this.x;
    const argv = [this.toHandle(op), ...args];
    const bytes = new Uint8Array(4 * argv.length);
    const dv = new DataView(bytes.buffer);
    argv.forEach((h, i) => dv.setUint32(4 * i, h, true));
    this.stage(bytes);
    const r = x.vm_call(this.vm, this.helper, 0, argv.length);
    x.vm_handle_free(this.vm, argv[0]);
    if (r !== 0) throw new SandboxError("misuse", "a call is already in progress");
  }

  /**
   * The member object of namespace `ns`, made on first use (D2632); bound nowhere by
   * itself. With `missingMember`, the namespace (the host value) is a Proxy over it.
   */
  private namespace(ns: string): number {
    let o = this.nsTargets.get(ns);
    if (o === undefined) {
      o = this.check(this.x.vm_new_object(this.vm, NULL_PROTO), "memory");
      this.nsTargets.set(ns, o);
      let value = o;
      const missing = this.opts.missingMember;
      if (missing !== undefined) {
        if (this.missingFn === 0) {
          this.fns.set(HOOK_MISSING, (namespace, member) => missing(String(namespace), String(member)));
          this.missingFn = this.check(this.x.vm_new_host_function(this.vm, HOOK_MISSING, this.stageText("missingMember")), "memory");
        }
        const n = this.toHandle(ns);
        value = this.callHelper(5, o, n, this.missingFn);
        this.x.vm_handle_free(this.vm, n);
      }
      this.namespaces.set(ns, value);
    }
    return o;
  }

  /** The written rule: the globals the sandbox binds without `SandboxOptions.globals`. */
  private defaultGlobals(): Record<string, string> {
    const globals: Record<string, string> = {};
    for (const name of this.fnNames) {
      const m = NAMESPACE_MEMBER.exec(name);
      if (m) globals[m[1]] = m[1];
      else if (IDENTIFIER.test(name)) globals[name] = name;
    }
    return globals;
  }

  /** The object of a namespace or host function by identity (D2970), as a handle the sandbox owns. */
  private hostValue(identity: string): number | undefined {
    const id = this.fnNames.indexOf(identity) + 1;
    return this.namespaces.get(identity) ?? this.wrappers.get(id) ?? this.fnHandles.get(id);
  }

  /**
   * The imports of codemode body `code` (D3597): its leading import declarations, each
   * name with the host value it binds. `taken`: a name the body has as a parameter already.
   */
  private bodyImports(code: string, taken: (local: string) => boolean): [string, number][] {
    if (!code.includes("import")) return [];
    const fail = (why: string, at: number) => {
      const [line, col] = lineColumn(code, at);
      return new SandboxError("syntax", `SyntaxError: ${why} at ${line}:${col}`);
    };
    let decls;
    try {
      decls = leadingImports(code, this.opts.typescript === true);
    } catch (e) {
      if (!(e instanceof ImportSyntaxError)) throw e;
      // Before the first import: the compiler reports it in its own words.
      if (e.at < code.indexOf("import")) return [];
      throw fail(e.message, e.at);
    }
    const out: [string, number][] = [];
    for (const d of decls) {
      const mod = this.hostModule(d.specifier);
      if (mod === undefined) throw fail(`Cannot find module '${d.specifier}'`, d.at);
      for (const b of d.bindings) {
        const h = b.name === "*" ? this.moduleNamespace(d.specifier, mod) : this.moduleExport(d.specifier, mod, b.name);
        if (h === undefined) {
          throw fail(b.name === "*" ? `The module '${d.specifier}' cannot be imported as a namespace: a host value has that name` : `The requested module '${d.specifier}' does not provide an export named '${b.name}'`, b.at);
        }
        if (taken(b.local) || out.some(([l]) => l === b.local)) throw fail(`Identifier '${b.local}' has already been declared`, b.at);
        out.push([b.local, h]);
      }
    }
    return out;
  }

  /** `opts.modules(specifier)`, asked once. */
  private hostModule(specifier: string): HostModule | undefined {
    let m = this.hostModules.get(specifier);
    if (m === undefined) {
      const r = this.opts.modules?.(specifier);
      m = r !== undefined && r !== null && typeof r.exports === "object" && r.exports !== null ? r : null;
      this.hostModules.set(specifier, m);
    }
    return m ?? undefined;
  }

  /** The host value export `name` of module `specifier` binds; undefined if none. */
  private moduleExport(specifier: string, mod: HostModule, name: string): number | undefined {
    if (!Object.hasOwn(mod.exports, name)) return undefined;
    const e = mod.exports[name];
    const h = typeof e === "string" ? this.hostValue(e) : undefined;
    if (h === undefined) throw new SandboxError("misuse", `module ${specifier}: export ${name} names no host function, namespace or value ${String(e)}`);
    return h;
  }

  /**
   * The namespace object of module `specifier` (D3597): made on first use with every
   * export as an enumerable member, and frozen; a host value named by the specifier.
   * Undefined when another host value has that name.
   */
  private moduleNamespace(specifier: string, mod: HostModule): number | undefined {
    if (this.moduleNamespaces.has(specifier)) return this.namespaces.get(specifier);
    if (this.hostValue(specifier) !== undefined) return undefined;
    const members = Object.keys(mod.exports).map((name): [string, number] => {
      const h = this.moduleExport(specifier, mod, name);
      return [name, h!];
    });
    return this.guard(() => {
      const ns = this.check(this.x.vm_new_object(this.vm, NULL_PROTO), "memory");
      members.forEach(([name, h], i) => this.defineOn(ns, name, h, DEFINE_ENUMERABLE | (i === members.length - 1 ? DEFINE_FREEZE : 0)));
      if (members.length === 0) this.x.vm_handle_free(this.vm, this.callHelper(1, ns));
      this.namespaces.set(specifier, ns);
      this.moduleNamespaces.add(specifier);
      return ns;
    });
  }

  private bindGlobal(name: string, identity: string): void {
    const h = this.hostValue(identity);
    if (h === undefined) throw new SandboxError("misuse", `global ${name}: no host function or namespace ${identity}`);
    this.guard(() => {
      // Read-only and non-configurable, as pi's prelude bound its globals (D3621).
      const g = this.x.vm_global(this.vm);
      this.defineOn(g, name, h, DEFINE_READONLY);
      this.x.vm_handle_free(this.vm, g);
    });
    this.globalNames.add(name);
  }

  /**
   * Runs `code` as a codemode function body (`return` gives the result) with the
   * `params` as its parameters (sent through the wire format), to its end: a loop over a
   * host-driven run that, at each idle point, calls the async host functions the script
   * called and settles each call with what it returned. Host functions must return
   * synchronously; use `runAsync` for ones that return promises. A promise as the result
   * is unwrapped (fulfilled: its value; rejected: thrown).
   */
  run(code: string, params: Record<string, unknown> = {}, opts: RunOptions = {}): unknown {
    const core = this.open(code, params, opts);
    for (;;) {
      const end = this.stepSync(core, {});
      core.run.end(end);
      if (end.state === "done") return end.value;
      if (end.state === "threw") throw end.error;
      if (core.open.size === 0) {
        // Waiting on nothing the host can answer: over, without a result.
        this.endRun();
        return undefined;
      }
      for (const call of [...core.open.values()]) {
        let s: Settlement;
        try {
          const r = this.invoke(this.fnNames.indexOf(call.fn) + 1, call.args);
          s = r instanceof Promise ? { ok: false, value: new TypeError(`host function ${call.fn} returned a promise; use runAsync`) } : { ok: true, value: r };
        } catch (e) {
          s = { ok: false, value: e };
        }
        this.settleRun(core, call.id, s);
      }
    }
  }

  /**
   * Like `run`, but awaits host functions that return promises: a sync one keeps the VM
   * parked meanwhile, async ones run concurrently, each settled when it finishes, and the
   * run goes on whenever a result came in.
   */
  async runAsync(code: string, params: Record<string, unknown> = {}, opts: RunOptions = {}): Promise<unknown> {
    const core = this.open(code, params, opts);
    const inflight = new Map<number, Promise<void>>();
    // A result that comes in after the run ended is the host's to drop.
    const settle = (id: number, s: Settlement) => {
      if (this.active === core) this.settleRun(core, id, s);
    };
    for (;;) {
      const end = await this.advanceRun(core, {});
      core.run.end(end);
      if (end.state === "done") return end.value;
      if (end.state === "threw") throw end.error;
      let settled = false;
      for (const call of [...core.open.values()]) {
        if (inflight.has(call.id)) continue;
        let r: unknown;
        try {
          r = this.invoke(this.fnNames.indexOf(call.fn) + 1, call.args);
        } catch (e) {
          settle(call.id, { ok: false, value: e });
          settled = true;
          continue;
        }
        if (!(r instanceof Promise)) {
          settle(call.id, { ok: true, value: r });
          settled = true;
          continue;
        }
        inflight.set(
          call.id,
          r.then(
            (value) => settle(call.id, { ok: true, value }),
            (value) => settle(call.id, { ok: false, value }),
          ).finally(() => inflight.delete(call.id)),
        );
      }
      if (settled) continue;
      if (inflight.size === 0) {
        // Waiting on nothing the host can answer: over, without a result.
        this.endRun();
        return undefined;
      }
      await Promise.race(inflight.values());
    }
  }

  /**
   * Starts `code` as a codemode function body (as `run`) and runs it to its first idle
   * point or its end, for the host to drive (`Run`): async host functions are not called
   * but listed in `run.calls`. Sync host functions must return synchronously here (as in
   * `run`); `advance` awaits them. A compile error throws; an exception in the script is
   * `run.state === "threw"`. One run at a time: the sandbox runs nothing else until it
   * ends. With `deliveries`, a re-run: see `BeginOptions.deliveries`.
   */
  begin(code: string, params: Record<string, unknown> = {}, opts: BeginOptions = {}): Run {
    if (opts.deliveries !== undefined && (this.clock === undefined || opts.start === false)) {
      throw new SandboxError("misuse", "a re-run (`deliveries`) needs a deterministic sandbox and its first step at once");
    }
    const core = this.open(code, params, opts);
    if (opts.start === false) {
      core.run.state = "ready";
      return core.run;
    }
    core.run.end(this.stepSync(core, { signal: opts.signal, fuel: opts.fuel }));
    if (opts.deliveries !== undefined) this.rerun(core, opts.deliveries);
    return core.run;
  }

  /**
   * Re-runs `deliveries` into `core` (`BeginOptions.deliveries`): each when its call is
   * open, checked against it, then a step to the next idle point.
   */
  private rerun(core: RunCore, deliveries: Delivery[]): void {
    const run = core.run;
    const diverged = (why: string) => {
      if (this.active === core) {
        this.endRun();
        this.abandon(undefined);
      }
      return new SandboxError("diverged", `re-run diverged: ${why}`);
    };
    deliveries.forEach((d, i) => {
      const at = `delivery ${i + 1} of ${deliveries.length} (call ${d.id}, ${d.fn})`;
      if (run.state !== "idle") throw diverged(`the run ${run.state === "done" ? "ended" : "threw"} before ${at}`);
      const call = core.open.get(d.id);
      if (call === undefined) {
        const open = [...core.open.values()].map((c) => `${c.id} ${c.fn}`).join(", ") || "none";
        throw diverged(`${at} is for a call the re-run has not made (open: ${open})`);
      }
      if (call.fn !== d.fn || !sameValue(call.args, d.args)) {
        throw diverged(`call ${d.id} is ${call.fn}(${stable(call.args)}), the recording's ${d.fn}(${stable(d.args)})`);
      }
      core.open.delete(d.id);
      this.deliver(core, call, d.ok, d.value, d.now);
      run.end(this.stepSync(core, {}));
    });
  }

  /** Compiles `code`, starts its call and makes its run (recording into `opts.journal`). */
  private open(code: string, params: Record<string, unknown>, opts: RunOptions): RunCore {
    this.beginJournal(code, params, opts);
    this.enter(code, params, opts.filename);
    return this.startRun([]);
  }

  /** Makes a run, waiting on `calls`; its result is the entry call's outcome. */
  private startRun(calls: OpenCall[]): RunCore {
    const core: RunCore = {
      run: undefined as unknown as Run,
      open: new Map(calls.map((c) => [c.id, c])),
      dropped: new Map(),
      used: 0,
      busy: false,
      pending: [],
      deliveries: [],
    };
    core.run = new Run(this, core, this.ops);
    this.active = core;
    return core;
  }

  /** One step of a run, to its next idle point or its end, without awaiting anything. */
  private stepSync(core: RunCore, step: StepOptions): RunEnd {
    if (this.active !== core) throw new SandboxError("misuse", "the run has ended");
    core.step = step;
    try {
      const r = this.step(core, false).next();
      if (!r.done) throw new Error("a sync step waited");
      if (r.value.state !== "idle") this.endRun();
      return r.value;
    } catch (e) {
      this.endRun();
      this.abandon(e);
      throw e;
    }
  }

  /** `Run.advance`: one step, awaiting sync host functions' promises and `yieldMs` breaks. */
  private async advanceRun(core: RunCore, step: StepOptions): Promise<RunEnd> {
    if (this.active !== core) throw new SandboxError("misuse", "the run has ended");
    core.step = step;
    try {
      const it = this.step(core, true);
      let r = it.next();
      while (!r.done) r = it.next(await r.value);
      if (r.value.state !== "idle") this.endRun();
      return r.value;
    } catch (e) {
      this.endRun();
      this.abandon(e);
      throw e;
    }
  }

  /** `Run.settle`: the result goes to the VM now, or while a step runs at its next slice. */
  private settleRun(core: RunCore, id: number, s: Settlement): void {
    if (core.dropped.has(id)) return; // the run ended without it (D3612)
    if (this.active !== core) throw new SandboxError("misuse", "the run has ended");
    const call = core.open.get(id);
    if (call === undefined) throw new SandboxError("misuse", `no open call ${id}`);
    core.open.delete(id);
    if (core.busy) core.pending.push({ call, ok: s.ok, value: s.value });
    else this.deliver(core, call, s.ok, s.value);
  }

  /**
   * Passes call `call`'s result to the VM (`at`: a re-run's or replay's clock), and records
   * it: in the journal, and in a deterministic sandbox in the run's deliveries.
   */
  private deliver(core: RunCore, call: OpenCall, ok: boolean, value: unknown, at?: number): void {
    let now: number | undefined;
    if (!ok && value instanceof RunExit) {
      // The run exits before its next slice; the result is never applied.
      core.exit ??= value;
      now = this.clock;
    } else {
      now = this.complete(call.id, ok, value, true, at);
    }
    const j = this.journal;
    if (j) j.delivered++;
    if (j?.mode === "record") j.push({ type: "async", callId: call.id, fn: call.fn, args: call.args, ok, value: errorJson(ok, value), now: now ?? 0 });
    if (now !== undefined) core.deliveries.push({ id: call.id, fn: call.fn, args: call.args, ok, value, now });
  }

  /** One step of a run: `drive` until idle (recording a check) or the end. */
  private *step(core: RunCore, async: boolean): Generator<Promise<unknown>, RunEnd, unknown> {
    core.busy = true;
    try {
      const r = yield* this.drive(async, core);
      if (r !== IDLE) return { state: "done", value: r, exited: core.exit !== undefined };
      this.recordCheck();
      return { state: "idle" };
    } catch (e) {
      if (e instanceof SandboxError && e.kind === "threw") return { state: "threw", error: e };
      throw e;
    } finally {
      core.busy = false;
    }
  }

  /**
   * A host function ended the run (`RunExit`, D3594): the VM drops its call, its jobs and the
   * open calls, and the run is done with the exit's value.
   */
  private exitRun(run: RunCore): unknown {
    const e = run.exit!;
    this.dropOpen(run);
    const j = this.journal;
    if (j) j.finish({ type: "end", ok: true, value: e.value }, this.canonicalHash());
    return e.value;
  }

  /**
   * The run is over while async calls are open: the VM drops its call, jobs and open
   * calls (`vm_abort`), and the calls move to `run.dropped` (D3612).
   */
  private dropOpen(run: RunCore): void {
    this.guard(() => this.x.vm_abort(this.vm));
    for (const [id, c] of run.open) run.dropped.set(id, c);
    run.open.clear();
  }

  /**
   * At an idle point: the run's result if it has settled (the body returned or threw
   * while async calls are still open), as `{ h, threw }` with a handle the caller frees;
   * undefined while it is pending.
   */
  private settledResult(): { h: number; threw: boolean } | undefined {
    const x = this.x;
    const vm = this.vm;
    const p = x.vm_outcome(vm);
    if (p === 0) return undefined;
    const ps = x.vm_promise_state(vm, p);
    const h = ps < 2 ? 0 : outWords(x, 1)[0];
    x.vm_handle_free(vm, p);
    return ps < 2 ? undefined : { h, threw: ps === 3 };
  }

  /**
   * The run's outcome from the handle `h` of its result or thrown value (freed here), or
   * undefined when the JSON boundary converts it first: a call of the helper the run loop
   * runs, whose result comes back as the run's (D3593). A thrown Error is read as data
   * (D3610); another thrown value is converted too, to undefined if JSON cannot carry it.
   */
  private outcome(run: RunCore, h: number, threw: boolean): { value: unknown; message: string; threw: boolean } | undefined {
    const x = this.x;
    const c = run.converted;
    if (this.opts.jsonBoundary && c === undefined) {
      if (!threw) {
        run.converted = { threw: false, message: "" };
        this.startHelper(0, [h]);
        x.vm_handle_free(this.vm, h);
        return undefined;
      }
      if (!(this.readThrown(h) instanceof Error)) {
        run.converted = { threw: true, message: this.display(h) };
        this.startHelper(4, [h]);
        x.vm_handle_free(this.vm, h);
        return undefined;
      }
    }
    if (c?.threw) threw = true;
    const value = threw ? this.readThrown(h) : this.readHandle(h);
    const message = c?.threw ? c.message : threw ? this.display(h) : "";
    x.vm_handle_free(this.vm, h);
    return { value, message, threw };
  }

  /** A thrown value as a host value; undefined when it cannot be read (a cycle). */
  private readThrown(h: number): unknown {
    try {
      return this.readHandle(h);
    } catch (e) {
      if (e instanceof TypeError || e instanceof RangeError) return undefined;
      throw e;
    }
  }

  /** Records a `check` entry at an idle point, when recording. */
  private recordCheck(): void {
    const j = this.journal;
    if (j?.mode === "record") j.push({ type: "check", delivered: j.delivered, issued: j.issued, hash: this.canonicalHash() });
  }

  /** The run has ended: the sandbox can run again. */
  private endRun(): void {
    this.active = null;
    this.journal = null;
  }

  /**
   * `Run.snapshot()`: `vm_snapshot` of a host's run (D3720). The host names its functions
   * by identity and its host values (namespaces, `values`, module namespaces, converting
   * functions) by name; its data are the calls' states, the fuel used and the clock.
   */
  private snapshotRun(core: RunCore, states: [number, unknown][]): Uint8Array {
    const x = this.x;
    for (const [id, v] of states) {
      if (!core.open.has(id)) throw new SandboxError("misuse", `a state for a call that is not open: ${id}`);
      stateJson(id, v);
    }
    const data: SnapshotData = { states, fuelUsed: core.used };
    if (this.clock !== undefined) data.clock = this.clock;
    const w = new StagingWriter();
    w.u32(this.fnNames.length);
    this.fnNames.forEach((name, i) => {
      w.u32(i + 1);
      w.text(name);
    });
    const values = this.hostValues();
    w.u32(values.length);
    for (const [name, h] of values) {
      w.u32(h);
      w.text(name);
    }
    w.text(JSON.stringify(data));
    this.snapshotBase();
    const n = this.guard(() => {
      this.stage(w.bytes());
      return x.vm_snapshot(this.vm, w.length);
    });
    if (n < 0) throw new SandboxError("misuse", `cannot snapshot the run:\n${outText(x)}`);
    return new Uint8Array(x.memory.buffer, x.djs_out_ptr(), n).slice();
  }

  /**
   * Snapshots name builtins by their paths in the base, which the instance reads once,
   * at its first snapshot operation: its first VM took the base as its arena (D2060), so
   * it gets the base again (D2252; the instance keeps the paths, not the bytes).
   */
  private snapshotBase(): void {
    if (this.snapshotsReady) return;
    this.guard(() => provideBase(this.module, this.x));
    this.snapshotsReady = true;
  }

  /** The host values by name a snapshot names (D3720): the converting functions, the namespaces, `values` and module namespaces. */
  private hostValues(): [string, number][] {
    return [...[...this.wrappers].map(([id, h]): [string, number] => [this.fnNames[id - 1], h]), ...this.namespaces];
  }

  /** Loads a snapshot of a run into this sandbox (D3720): the run, idle, with its open calls. */
  private loadSnapshotRun(snapshot: Uint8Array): RunCore {
    const x = this.x;
    if (this.active !== null) throw new SandboxError("misuse", "a run is in progress");
    // What it needs: the host values it names, some made lazily (module namespaces).
    this.stage(snapshot);
    if (x.djs_snapshot_needs(snapshot.length) !== 0) throw new SandboxError("misuse", `cannot load the snapshot: ${outText(x)}`);
    const names = new OutReader(x).list();
    if (this.opts.modules !== undefined) {
      for (const name of names) {
        const mod = this.hostValue(name) === undefined ? this.hostModule(name) : undefined;
        if (mod !== undefined) this.moduleNamespace(name, mod);
      }
    }
    const missing = names.filter((n) => this.hostValue(n) === undefined);
    if (missing.length > 0) {
      throw new SandboxError("misuse", `cannot load the snapshot: this sandbox has no host value or function ${missing.join(", ")}`);
    }
    const w = new StagingWriter();
    w.bytes_(snapshot);
    w.u32(this.fnNames.length);
    this.fnNames.forEach((name, i) => {
      w.u32(i + 1);
      w.u32(this.fnHandles.get(i + 1) ?? 0);
      w.text(name);
    });
    const values = this.hostValues();
    w.u32(values.length);
    for (const [name, h] of values) {
      w.u32(h);
      w.text(name);
    }
    this.snapshotBase();
    const st = this.guard(() => {
      this.stage(w.bytes());
      return x.vm_load_snapshot(this.vm, w.length);
    });
    if (st !== 0) {
      const why = outText(x);
      // The VM may be half loaded: nothing runs in it again.
      this.dead = `a snapshot failed to load into this sandbox: ${why}`;
      throw new SandboxError("misuse", `cannot load the snapshot:\n${why}`);
    }
    const out = new OutReader(x);
    const ids: number[] = [];
    for (let n = out.u32(); n > 0; n--) ids.push(out.u32());
    const data = JSON.parse(out.text() || "{}") as SnapshotData;
    const states = new Map(data.states ?? []);
    const calls = ids.map((id): OpenCall => {
      const { fnId, args } = this.callInfo(id);
      const call: OpenCall = { id, fn: this.fnNames[fnId - 1] ?? `#${fnId}`, args };
      if (states.has(id)) call.state = states.get(id);
      return call;
    });
    // A deterministic run's clock (the VM's strong weak references make a load into a
    // sandbox that differs in `deterministic` refuse).
    if (data.clock !== undefined && this.clock !== undefined) this.clock = data.clock;
    const core = this.startRun(calls);
    core.used = Math.max(0, data.fuelUsed ?? 0);
    return core;
  }

  /**
   * Replays a recorded run (D1730): runs the journal's code and parameters, answers every
   * host call from the journal instead of calling the host functions (console output
   * still goes to the console), applies the async results in the recorded order with
   * their clocks, and checks the calls the script makes and the canonical hashes at the
   * start, at every recorded idle point and at the end. The sandbox must be deterministic
   * and in the state the recording one was in (a fresh one created the same way); the
   * fuel slice may differ. Returns the run's result; a mismatch is a `diverged`
   * SandboxError.
   */
  async replay(journal: JournalEntry[]): Promise<unknown> {
    const head = journal[0];
    if (head?.type !== "run") throw new SandboxError("misuse", "a journal starts with a run entry");
    if (this.opts.deterministic === undefined) throw new SandboxError("misuse", "replay needs a deterministic sandbox");
    const j = new Journal("replay", journal);
    j.expect(head.engine === this.buildHash(), () => `engine ${this.buildHash()}, the journal was made by ${head.engine}`);
    j.expect(head.hash === this.canonicalHash(), () => "the sandbox's state before the run differs from the recording's");
    j.next = 1;
    const core = this.open(head.code, head.params, { filename: head.filename });
    this.journal = j;
    try {
      for (;;) {
        const end = await this.advanceRun(core, {});
        core.run.end(end);
        if (end.state === "done") return end.value;
        if (end.state === "threw") throw end.error;
        if (core.open.size === 0) {
          this.endRun();
          return undefined;
        }
        // Idle: the checks recorded at this point, then the next result.
        j.checkIdle(this.canonicalHash());
        const e = j.take("async");
        const call = core.open.get(e.callId);
        j.expect(call !== undefined, () => `result ${j.delivered + 1} is for call ${e.callId}, which the script has not made`);
        j.expectCall(e.fn, e.args, call!.fn, call!.args);
        core.open.delete(e.callId);
        this.deliver(core, call!, e.ok, e.ok ? e.value : errorFromJson(e.value), e.now);
      }
    } catch (e) {
      if (this.active === core) {
        this.endRun();
        this.abandon(e);
      }
      throw e;
    } finally {
      this.journal = null;
    }
  }

  /** The VM's canonical hash (D1730) as 16 hex digits: its state, independent of collections. */
  canonicalHash(): string {
    return this.guard(() => {
      if (this.x.vm_canonical_hash(this.vm) !== 0) throw new SandboxError("memory", "no memory for the canonical hash");
      const [lo, hi] = outWords(this.x, 2);
      return hi.toString(16).padStart(8, "0") + lo.toString(16).padStart(8, "0");
    });
  }

  private buildHash(): string {
    return (this.x.djs_build_hash() >>> 0).toString(16).padStart(8, "0");
  }

  /** Starts recording a run into `opts.journal`, if given. */
  private beginJournal(code: string, params: Record<string, unknown>, opts: RunOptions): void {
    if (this.active !== null) throw new SandboxError("misuse", "a run is in progress");
    this.journal = null;
    if (opts.journal === undefined) return;
    if (this.opts.deterministic === undefined) throw new SandboxError("misuse", "recording needs a deterministic sandbox");
    const head: JournalEntry = { type: "run", engine: this.buildHash(), code, params, hash: this.canonicalHash() };
    if (opts.filename !== undefined) head.filename = opts.filename;
    opts.journal.push(head);
    this.journal = new Journal("record", opts.journal);
  }

  /**
   * After a failed run: drops the VM's call if it is still in one (out of fuel, or a host
   * side error escaped the loop), so the next run starts clean. Not after a trap: the
   * instance is gone then.
   */
  private abandon(e: unknown): void {
    if (this.dead === null && !(e instanceof SandboxError && e.kind === "trap")) {
      this.guard(() => this.x.vm_abort(this.vm));
    }
  }

  /**
   * The VM's arena: a view into wasm memory, valid until the next call into the sandbox
   * (the arena may move when it grows). For inspecting the heap with `LAYOUT` (the docs'
   * live examples walk it cell by cell).
   */
  heapView(): Uint8Array {
    this.alive();
    return this.arena();
  }

  /**
   * Where the run waits on open host call `id` (D3054): the `await` (or `yield`) of the
   * innermost suspended activation waiting on the call, directly or through promise
   * combinators. `file` is the name the code was compiled as: the run's `filename` for
   * the code it started with. Undefined when the call is not open or nothing waits on it
   * at a known position.
   */
  callSite(id: number): CallSite | undefined {
    this.alive();
    const x = this.x;
    const n = this.guard(() => x.vm_call_site(this.vm, id));
    if (n === 0) return undefined;
    const m = /^(\d+):(\d+):(.*)$/s.exec(outText(x));
    return m ? { file: m[3], line: Number(m[1]), column: Number(m[2]) } : undefined;
  }

  /** Collects garbage now. False if the heap could not reserve the to-space. */
  collect(): boolean {
    return this.guard(() => this.x.vm_collect(this.vm) === 0);
  }

  /** The arena's current size in bytes. */
  get heapSize(): number {
    this.alive();
    return this.x.vm_arena_len(this.vm);
  }

  /** Releases the VM. The sandbox cannot be used afterwards. */
  dispose(): void {
    this.active = null;
    if (this.dead === null) {
      this.x.vm_free(this.vm);
      this.dead = "disposed";
    }
  }

  /** Panics inside the module on purpose (tests the trap path). */
  debugTrap(): void {
    this.guard(() => this.x.djs_debug_panic());
  }

  // ---- internals ---------------------------------------------------------------------

  private alive(): void {
    if (this.dead !== null) throw new SandboxError("misuse", `sandbox is unusable: ${this.dead}`);
  }

  /** Runs `f` and turns a wasm trap into a SandboxError; the sandbox is dead afterwards. */
  private guard<T>(f: () => T): T {
    this.alive();
    try {
      return f();
    } catch (e) {
      if (e instanceof WebAssembly.RuntimeError) {
        const msg = trapText(this.x, e);
        this.dead = `trapped (${msg})`;
        throw new SandboxError("trap", `djs-wasm trapped: ${msg}`);
      }
      throw e;
    }
  }

  private check(h: number, kind: SandboxErrorKind): number {
    if (h === 0) throw new SandboxError(kind, outText(this.x) || "out of memory");
    return h;
  }

  /** Copies bytes into the staging buffer. */
  private stage(bytes: Uint8Array): void {
    const at = this.x.djs_staging(bytes.length);
    new Uint8Array(this.x.memory.buffer).set(bytes, at);
  }

  /** Stages `s` as UTF-8: a short one (names) encoded in place, without a copy. */
  private stageText(s: string): number {
    if (s.length > 1024) {
      const b = textEncoder.encode(s);
      this.stage(b);
      return b.length;
    }
    // At most 3 bytes per UTF-16 unit.
    const at = this.x.djs_staging(3 * s.length);
    return textEncoder.encodeInto(s, new Uint8Array(this.x.memory.buffer, at, 3 * s.length)).written;
  }

  /** Defines `name` on `obj`; `flags`: `DEFINE_ENUMERABLE`, `DEFINE_FREEZE`. */
  private defineOn(obj: number, name: string, value: number, flags = 0): void {
    if (this.x.vm_define(this.vm, obj, this.stageText(name), value, flags) !== 0) {
      throw new SandboxError("memory", `cannot define ${name}`);
    }
  }

  /** A handle for a host value (0 for undefined). */
  private toHandle(v: unknown): number {
    if (v === undefined) return 0;
    const bytes = encode(v);
    if (typeof v === "string" && v.length >= IN_PLACE_MIN) {
      const h = stringInPlace(this.x, this.vm, v, bytes);
      if (h !== 0) return h;
    }
    this.stage(bytes);
    return this.check(this.x.vm_decode_wire(this.vm, bytes.length), "memory");
  }

  private arena(): Uint8Array {
    const x = this.x;
    return new Uint8Array(x.memory.buffer, x.vm_arena_ptr(this.vm), x.vm_arena_len(this.vm));
  }

  private read(lo: number, hi: number): unknown {
    return readValue(this.arena(), [lo, hi], { atomCache: this.atomCache });
  }

  private readHandle(h: number): unknown {
    this.x.vm_handle_bits(this.vm, h);
    const w = outWords(this.x, 2);
    return this.read(w[0], w[1]);
  }

  private display(h: number): string {
    const n = this.x.vm_display(this.vm, h);
    return textDecoder.decode(new Uint8Array(this.x.memory.buffer, this.x.djs_out_ptr(), n));
  }

  /** Compiles `code` and starts the call; returns nothing (the VM holds the call). */
  private enter(code: string, params: Record<string, unknown>, filename?: string): void {
    if (this.active !== null) throw new SandboxError("misuse", "a run is in progress");
    this.guard(() => {
      const x = this.x;
      const vm = this.vm;
      // A body's parameters: the run's data, then its imports (D3597), host values the
      // sandbox owns.
      const imported = this.bodyImports(code, (local) => Object.hasOwn(params, local));
      const names = [...Object.keys(params), ...imported.map(([p]) => p)];
      const handles = Object.values(params).map((v) => this.toHandle(v));
      const name = textEncoder.encode(filename ?? "");
      const head = textEncoder.encode(names.join("\n"));
      const src = textEncoder.encode(code);
      const all = new Uint8Array(name.length + head.length + src.length);
      all.set(name);
      all.set(head, name.length);
      all.set(src, name.length + head.length);
      this.stage(all);
      const o = this.opts;
      const goal = (o.debugger ? 16 : 0) |
        1 |
        (o.typescript ? 2 : 0) |
        (o.experimentalDecorators || o.emitDecoratorMetadata ? 4 : 0) |
        (o.emitDecoratorMetadata ? 8 : 0);
      const f = x.vm_compile_file(vm, goal, name.length, head.length, src.length);
      if (f === 0) {
        const msg = outText(x);
        for (const h of handles) if (h !== 0) x.vm_handle_free(vm, h);
        throw new SandboxError(msg.startsWith("SyntaxError") ? "syntax" : "memory", msg);
      }
      const argv = [...handles, ...imported.map(([, h]) => h)];
      const args = new Uint8Array(4 * argv.length);
      const dv = new DataView(args.buffer);
      argv.forEach((h, i) => dv.setUint32(4 * i, h, true));
      this.stage(args);
      const r = x.vm_call(vm, f, 0, argv.length);
      x.vm_handle_free(vm, f);
      for (const h of handles) if (h !== 0) x.vm_handle_free(vm, h);
      if (r !== 0) throw new SandboxError("misuse", "a call is already in progress");
    });
  }

  /**
   * The run loop to the next idle point or the end, as a generator: yields a promise when
   * the host must wait for a sync host function (only when `async`), returns the result,
   * or `IDLE` when only async host calls are left (they are in `run.open`).
   */
  private *drive(async: boolean, run: RunCore): Generator<Promise<unknown>, unknown, unknown> {
    const x = this.x;
    const vm = this.vm;
    const slice = this.opts.fuelSlice ?? 1_000_000;
    const maxFuel = this.opts.maxFuel ?? Infinity;
    const j = this.journal;
    // The step's bounds (D3595).
    const step = run.step ?? {};
    const budget = step.fuel ?? Infinity;
    const start = run.used;
    let resumed = Date.now();
    for (;;) {
      // Results settled while this step ran (it awaited a sync host function's promise or
      // yielded): to the VM in the order they were settled.
      for (const p of run.pending.splice(0)) this.deliver(run, p.call, p.ok, p.value);
      if (run.exit !== undefined) return this.exitRun(run);
      if (step.signal?.aborted) {
        const why = step.signal.reason;
        throw new SandboxError("aborted", `the step was aborted${why === undefined ? "" : `: ${why instanceof Error ? why.message : String(why)}`}`);
      }
      // The slice ends where the step's budget or the run's `maxFuel` does: the run's
      // budget spans its steps, which may each end idle before a full slice (D3614).
      const left = Math.min(budget - (run.used - start), maxFuel - run.used);
      const st = this.guard(() => {
        this.setClock();
        return x.vm_run(vm, Math.max(1, Math.min(slice, left)));
      });
      // Counted across sync host calls too, which end a slice early (D2653, D2702).
      run.used += x.vm_fuel_used(vm);
      let rejected = false;
      for (const callId of this.newHostCalls()) {
        const { fnId, args, unreadable } = this.callInfo(callId);
        if (unreadable) {
          // The call's promise rejects with it; the host never sees the call (D3620).
          this.complete(callId, false, unreadable, false);
          rejected = true;
          continue;
        }
        const call = { id: callId, fn: this.fnNames[fnId - 1], args };
        run.open.set(callId, call);
        if (j) j.issued++;
      }
      if (st === STATUS_DONE || st === STATUS_THREW) {
        const outcome = this.guard(() => {
          let h = x.vm_result(vm);
          let threw = st === STATUS_THREW;
          // A promise result (an async body): its outcome.
          const ps = x.vm_promise_state(vm, h);
          if (ps === 1) {
            x.vm_handle_free(vm, h);
            throw new SandboxError("threw", "the result is a promise that never settles");
          }
          if (ps >= 2) {
            const r = outWords(x, 1)[0];
            x.vm_handle_free(vm, h);
            h = r;
            threw = ps === 3;
          }
          // (The JSON boundary converts it first, in a call of its own that this loop
          // runs: toJSON may call host functions.)
          return this.outcome(run, h, threw);
        });
        if (outcome === undefined) continue;
        return this.finishRun(outcome);
      }
      if (st === STATUS_OUT_OF_FUEL) {
        if (run.used >= maxFuel) {
          // `run` abandons the call (vm_abort), so the sandbox stays usable.
          throw new SandboxError("fuel", `out of fuel after ${run.used} units`);
        }
        if (run.used - start >= budget) throw new SandboxError("fuel", `the step ran out of its fuel (${budget} units)`);
        if (async && step.yieldMs !== undefined && Date.now() - resumed >= step.yieldMs) {
          // Between slices: the host's other tasks (timers, I/O, an abort) run now.
          yield macrotask();
          resumed = Date.now();
        }
        continue;
      }
      if (st === STATUS_HOST_OUT_OF_MEMORY) {
        // The host (wasm memory) could not back the heap below its cap: not a script
        // error. The VM abandoned the call and is usable again (D1690).
        throw new SandboxError("memory", "out of host memory");
      }
      if (st === STATUS_IDLE) {
        // (A rejection just delivered: the script goes on with it first.)
        if (rejected) continue;
        this.checkRejections();
        // The body's result settled while async calls are open: the run is over, and
        // the calls are dropped (D3612), as when no call was open.
        const settled = this.guard(() => this.settledResult());
        if (settled === undefined) return IDLE;
        this.dropOpen(run);
        const outcome = this.guard(() => this.outcome(run, settled.h, settled.threw));
        if (outcome === undefined) continue;
        return this.finishRun(outcome);
      }
      if (st !== STATUS_HOST_CALL) return undefined;
      if (run.used >= maxFuel) throw new SandboxError("fuel", `out of fuel after ${run.used} units`);
      const { callId, fnId, args, unreadable } = this.guard(() => {
        const argc = x.vm_host_call(vm);
        return this.readCall(argc);
      });
      if (unreadable) {
        // A TypeError at the call, which the script can catch, as the JSON boundary's (D3620).
        this.complete(callId, false, unreadable, false);
        continue;
      }
      let ok = true;
      let result: unknown;
      const fn = this.fnNames[fnId - 1];
      // (Console output and `missingMember`, which has no identity, are not journaled.)
      const journaled = j !== null && fn !== undefined && !fn.startsWith("console.");
      try {
        if (journaled && j.mode === "replay") {
          j.issued++;
          const e = j.take("sync");
          j.expectCall(e.fn, e.args, fn, args);
          if (!e.ok) throw errorFromJson(e.value);
          result = e.value;
        } else {
          result = this.invoke(fnId, args);
          if (result instanceof Promise) {
            if (!async) throw new TypeError(`host function ${fn} returned a promise; use runAsync`);
            result = yield result;
          }
        }
      } catch (e) {
        if (e instanceof SandboxError && (e.kind === "trap" || e.kind === "diverged")) throw e;
        ok = false;
        result = e;
      }
      if (journaled && j.mode === "record") {
        j.issued++;
        j.push({ type: "sync", fn, args, ok, value: errorJson(ok, result) });
      }
      if (!ok && result instanceof RunExit) {
        // The host ends the run: the call never returns (the next slice aborts the VM's call).
        run.exit = result;
        continue;
      }
      this.complete(callId, ok, result, false);
    }
  }

  /** The end of a run with `outcome`: the journal's end, then the value or the thrown error. */
  private finishRun({ value, message, threw }: { value: unknown; message: string; threw: boolean }): unknown {
    const j = this.journal;
    if (j) j.finish({ type: "end", ok: !threw, value: threw ? message : value }, this.canonicalHash());
    if (threw) throw new SandboxError("threw", message, value);
    this.checkRejections();
    return value;
  }

  /** Reports the VM's unhandled rejections per `opts.unhandledRejections` (D1423). */
  private checkRejections(): void {
    const reasons = this.guard(() => {
      const x = this.x;
      const n = x.vm_take_unhandled_rejections(this.vm);
      const handles = outWords(x, n);
      return handles.map((h) => {
        const value = this.readHandle(h);
        const message = this.display(h);
        x.vm_handle_free(this.vm, h);
        return { value, message };
      });
    });
    if (reasons.length === 0) return;
    const mode = this.opts.unhandledRejections ?? "warn";
    if (mode === "throw") {
      const first = reasons[0];
      throw new SandboxError("threw", `Possibly unhandled promise rejection: ${first.message}`, first.value);
    }
    if (mode === "ignore") return;
    for (const r of reasons) this.cons.warn("Possibly unhandled promise rejection:", r.message);
  }

  /**
   * Passes the host clock and time zone to the VM (before every slice). Not in
   * deterministic mode: there the clock moves only with async results (`complete`).
   */
  private setClock(): void {
    if (this.clock !== undefined) return;
    const { now, timeZoneOffset } = this.opts;
    const ms = typeof now === "number" ? now : (now ?? Date.now)();
    this.x.vm_set_now(this.vm, ms);
    this.x.vm_set_time_zone_offset(this.vm, timeZoneOffset ?? -new Date(ms).getTimezoneOffset());
  }

  /** Calls host function `fnId` with arguments read from the VM. */
  private invoke(fnId: number, args: unknown[]): unknown {
    const f = this.fns.get(fnId);
    if (f === undefined) throw new TypeError(`unknown host function ${fnId}`);
    const isConsole = this.fnNames[fnId - 1]?.startsWith("console.");
    return f(...(isConsole ? args.map(consoleArg) : args));
  }

  /** The async host calls made since the last check (`vm_new_host_calls`). */
  private newHostCalls(): number[] {
    return this.guard(() => {
      const n = this.x.vm_new_host_calls(this.vm);
      return outWords(this.x, n);
    });
  }

  /** The function and arguments of pending host call `callId`. */
  private callInfo(callId: number): { fnId: number; args: unknown[]; unreadable?: TypeError } {
    return this.guard(() => this.readCall(this.x.vm_host_call_info(this.vm, callId)));
  }

  /**
   * Reads `[call_id, fn_id, this, args...]` from the out buffer. An argument the reader
   * cannot read (a cycle, a blow-up) is `unreadable`, the TypeError the call fails with
   * (D3620); `console` gets its reason as text instead, since output must not throw.
   */
  private readCall(argc: number): { callId: number; fnId: number; args: unknown[]; unreadable?: TypeError } {
    const x = this.x;
    const w = outWords(x, 4 + 2 * argc);
    const fnId = w[1];
    const args: unknown[] = [];
    let unreadable: TypeError | undefined;
    for (let i = 0; i < argc; i++) {
      try {
        args.push(this.read(w[4 + 2 * i], w[5 + 2 * i]));
      } catch (e) {
        if (!(e instanceof TypeError || e instanceof RangeError)) throw e;
        if (this.fnNames[fnId - 1]?.startsWith("console.")) args.push(`[${e.message}]`);
        else {
          unreadable ??= new TypeError(`argument ${i + 1}: ${e.message}`);
          args.push(undefined);
        }
      }
    }
    return { callId: w[0], fnId, args, unreadable };
  }

  /**
   * Passes a host call's outcome back: a value (through the wire format), or an error
   * thrown at the call (sync) or rejecting its promise (async), as an Error of the same
   * kind with the host error's message. A value the wire format cannot carry becomes
   * such an error too.
   */
  private complete(callId: number, ok: boolean, value: unknown, async: boolean, at?: number): number | undefined {
    const x = this.x;
    const vm = this.vm;
    let h = 0;
    if (ok) {
      try {
        h = this.guard(() => this.toHandle(value));
      } catch (e) {
        if (e instanceof SandboxError && e.kind === "trap") throw e;
        ok = false;
        value = e;
      }
    }
    if (!ok) {
      const e = value;
      h = this.guard(() => {
        const msg = this.stageText(e instanceof Error ? e.message : String(e));
        const kind = e instanceof TypeError ? 1 : e instanceof RangeError ? 2 : 0;
        const error = this.check(x.vm_new_error(vm, kind, msg), "memory");
        // The host error's own enumerable properties (a tool's `structuredOutput`, a
        // `code`) go with it, so the script's catch sees them (D3592): as wire values, not
        // name/message/stack, and not the ones the wire cannot carry (a function).
        if (typeof e === "object" && e !== null) {
          for (const [key, v] of Object.entries(e)) {
            if (key === "name" || key === "message" || key === "stack" || v === undefined) continue;
            let p: number;
            try {
              p = this.toHandle(v);
            } catch (err) {
              // (The wire's refusal is a TypeError; a trap or out of memory goes on up.)
              if (!(err instanceof TypeError)) throw err;
              continue;
            }
            this.defineOn(error, key, p, DEFINE_ENUMERABLE);
            if (p !== 0) x.vm_handle_free(vm, p);
          }
        }
        return error;
      });
    }
    return this.guard(() => {
      let clock: number | undefined;
      if (async && this.clock !== undefined) {
        // Deterministic mode: the result's time (a replay's: the recorded one), applied
        // when the VM takes the result (D1670).
        const { now } = this.opts;
        this.clock = at ?? Math.max(this.clock, typeof now === "number" ? now : (now ?? Date.now)());
        clock = this.clock;
        x.vm_resolve_host_call_at(vm, callId, ok ? 1 : 0, h, this.clock);
      } else {
        x.vm_resolve_host_call(vm, callId, ok ? 1 : 0, h);
      }
      if (h !== 0) x.vm_handle_free(vm, h);
      return clock;
    });
  }
}

/** A macrotask: `setImmediate` where there is one (node, Bun), else a zero timeout. */
function macrotask(): Promise<void> {
  const g = globalThis as { setImmediate?: (f: () => void) => void };
  return new Promise((resolve) => (g.setImmediate ? g.setImmediate(resolve) : setTimeout(resolve, 0)));
}

/** A call's state as JSON text (D2770): checks that a snapshot can carry it. */
function stateJson(id: number, state: unknown): string {
  let json: string | undefined;
  try {
    json = JSON.stringify(state);
  } catch (e) {
    throw new SandboxError("misuse", `the state of call ${id} is not JSON data: ${String(e)}`);
  }
  if (json === undefined) throw new SandboxError("misuse", `the state of call ${id} is not JSON data`);
  return json;
}

/** The journal of a run being recorded or replayed (D1730). */
class Journal {
  /** Replay: the index of the next entry. */
  next = 0;
  /** Async results delivered, non-console host calls made. */
  delivered = 0;
  issued = 0;
  readonly mode: "record" | "replay";
  readonly entries: JournalEntry[];
  constructor(mode: "record" | "replay", entries: JournalEntry[]) {
    this.mode = mode;
    this.entries = entries;
  }

  push(e: JournalEntry): void {
    this.entries.push(e);
  }

  expect(ok: boolean, why: () => string): void {
    if (!ok) throw new SandboxError("diverged", `replay diverged: ${why()}`);
  }

  /** Replay: takes the next entry, which must be of `type`. */
  take<T extends JournalEntry["type"]>(type: T): Extract<JournalEntry, { type: T }> {
    const e = this.entries[this.next];
    this.expect(e?.type === type, () => `the script needs ${type === "sync" ? "a host call" : "a result"} here, the journal has ${e ? `a ${e.type} entry` : "nothing more"} (entry ${this.next})`);
    this.next++;
    return e as Extract<JournalEntry, { type: T }>;
  }

  expectCall(fn: string, args: unknown[], gotFn: string, gotArgs: unknown[]): void {
    this.expect(fn === gotFn && sameValue(args, gotArgs), () => `the script called ${gotFn}(${stable(gotArgs)}), the recording ${fn}(${stable(args)})`);
  }

  /** Replay at an idle point: the recorded checks for this many delivered results. */
  checkIdle(hash: string): void {
    for (let e = this.entries[this.next]; e?.type === "check"; e = this.entries[this.next]) {
      this.expect(e.delivered === this.delivered, () => `the recording was idle after ${e.delivered} results, the replay after ${this.delivered}`);
      this.expect(e.issued === this.issued, () => `${this.issued} host calls made, the recording made ${e.issued} (after ${e.delivered} results)`);
      this.expect(e.hash === hash, () => `the state after ${e.delivered} results differs`);
      this.next++;
    }
  }

  /** At the end: records or checks the `end` entry. */
  finish(end: { type: "end"; ok: boolean; value: unknown }, hash: string): void {
    const full: JournalEntry = { ...end, delivered: this.delivered, issued: this.issued, hash };
    if (this.mode === "record") {
      this.push(full);
      return;
    }
    while (this.entries[this.next]?.type === "check") this.next++;
    const e = this.take("end");
    this.expect(e.ok === full.ok && sameValue(e.value, full.value), () => `the run ended ${full.ok ? "with" : "throwing"} ${stable(full.value)}, the recording ${e.ok ? "with" : "throwing"} ${stable(e.value)}`);
    this.expect(e.issued === full.issued && e.delivered === full.delivered, () => "a different number of host calls or results");
    this.expect(e.hash === full.hash, () => "the final state differs");
  }
}

/** A host value as a journal records it: errors as `{ name, message }`. */
function errorJson(ok: boolean, v: unknown): unknown {
  if (ok) return v;
  if (v instanceof RunExit) return { name: "RunExit", message: v.message, value: v.value };
  return v instanceof Error ? { name: v.name, message: v.message } : { name: "Error", message: String(v) };
}

/** The host error a journal entry stands for, of the same kind. */
function errorFromJson(v: unknown): Error {
  const { name, message, value } = (v ?? {}) as { name?: string; message?: string; value?: unknown };
  if (name === "RunExit") return new RunExit(value);
  const E = name === "TypeError" ? TypeError : name === "RangeError" ? RangeError : Error;
  return new E(message ?? "");
}

/** A stable text form of a host value, for comparing and messages. */
function stable(v: unknown): string {
  return JSON.stringify(v, (_k, x) =>
    typeof x === "bigint" ? `${x}n` : x === undefined ? "<undefined>" : x instanceof Uint8Array ? Array.from(x) : x,
  ) ?? "<undefined>";
}

function sameValue(a: unknown, b: unknown): boolean {
  return stable(a) === stable(b);
}

/** A 64-bit seed as the two u32 words `vm_set_random_seed` takes. */
function seedWords(seed: number | bigint): [number, number] {
  const s = BigInt.asUintN(64, BigInt(seed));
  return [Number(s & 0xffffffffn), Number(s >> 32n)];
}

/**
 * The host's entropy, drawn in blocks: one `getRandomValues` call costs a few µs in node,
 * a few percent of a small run with its new sandbox (D3273). Each word is used once.
 */
const entropyPool = new Uint32Array(256);
let entropyAt = entropyPool.length;

/** A seed from the host's entropy (never 0, which would mean the VM's fixed default). */
function entropy(): [number, number] {
  if (entropyAt === entropyPool.length) {
    globalThis.crypto.getRandomValues(entropyPool);
    entropyAt = 0;
  }
  const lo = entropyPool[entropyAt];
  const hi = entropyPool[entropyAt + 1] | 1;
  entropyAt += 2;
  return [lo, hi];
}

/** Console arguments: values without a host form are shown by their class. */
function consoleArg(a: unknown): unknown {
  return a instanceof Opaque ? `[${a.className ?? a.type}]` : a;
}

/** The first `n` u32 (LE) words of the out buffer, which need not be aligned. */
function outWords(x: Exports, n: number): number[] {
  const dv = new DataView(x.memory.buffer, x.djs_out_ptr(), 4 * n);
  return Array.from({ length: n }, (_, i) => dv.getUint32(4 * i, true));
}

function outText(x: Exports): string {
  return textDecoder.decode(new Uint8Array(x.memory.buffer, x.djs_out_ptr(), x.djs_out_len()));
}

/** The host's data in a snapshot of a run (D3720): the calls' states, the fuel used, the clock. */
interface SnapshotData {
  states?: [number, unknown][];
  fuelUsed?: number;
  clock?: number;
}

/** Little-endian words and length-prefixed byte strings for the snapshot operations' staging. */
class StagingWriter {
  private parts: Uint8Array[] = [];
  length = 0;
  u32(n: number): void {
    const b = new Uint8Array(4);
    new DataView(b.buffer).setUint32(0, n, true);
    this.push(b);
  }
  bytes_(b: Uint8Array): void {
    this.u32(b.length);
    this.push(b);
  }
  text(s: string): void {
    this.bytes_(textEncoder.encode(s));
  }
  private push(b: Uint8Array): void {
    this.parts.push(b);
    this.length += b.length;
  }
  bytes(): Uint8Array {
    const out = new Uint8Array(this.length);
    let at = 0;
    for (const p of this.parts) {
      out.set(p, at);
      at += p.length;
    }
    return out;
  }
}

/** Reads the out buffer's words and length-prefixed strings (a copy: later calls reuse it). */
class OutReader {
  private readonly b: Uint8Array;
  private at = 0;
  constructor(x: Exports) {
    this.b = new Uint8Array(x.memory.buffer, x.djs_out_ptr(), x.djs_out_len()).slice();
  }
  u32(): number {
    const v = new DataView(this.b.buffer, this.at, 4).getUint32(0, true);
    this.at += 4;
    return v;
  }
  text(): string {
    const n = this.u32();
    const t = textDecoder.decode(this.b.subarray(this.at, this.at + n));
    this.at += n;
    return t;
  }
  list(): string[] {
    const out: string[] = [];
    for (let n = this.u32(); n > 0; n--) out.push(this.text());
    return out;
  }
}
