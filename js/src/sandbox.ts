// The Sandbox API: loads djs-wasm (docs/host.md) and runs code in a djs VM. Values go in
// through the wire format (wire.ts) and come out through direct heap reads (reader.ts).

import { IN_PLACE_MIN, encode, stringInPlace } from "./wire.ts";
import { trapText } from "./trap.ts";
import { Opaque, readValue } from "./reader.ts";
import { provideBase, rememberBase } from "./base.ts";
import { script as dumpRuntimeScript } from "./dump-runtime.js";

/** A host function callable from the sandbox. Arguments and the result are host values. */
export type HostFunction = (...args: unknown[]) => unknown;

/**
 * An async host call as an `AsyncHostFunction`'s hooks see it (D2771): what the script
 * called, and the host's state of the call, which a dump keeps.
 */
export interface HostCall<S = unknown> {
  /** The VM's call id; a dump keeps it. */
  readonly id: number;
  /** The host function's name, as given in `asyncFunctions`. */
  readonly fn: string;
  readonly args: unknown[];
  /** When this sandbox started (or, after a load, resumed) the call: ms, the host's `Date.now()`. */
  readonly startedAt: number;
  /**
   * The host's state of the call: JSON data that a dump writes with the call (as
   * `save`'s default) and a load gives back to `resume`. Set it at any time; a resumed
   * call starts with the state it was dumped with.
   */
  state: S | undefined;
  /**
   * Aborted when this sandbox stops waiting for the call: the task was suspended into a
   * dump (the call continues from there) or the sandbox was disposed.
   */
  readonly signal: AbortSignal;
}

/**
 * An async host function with hooks for continuation dumps (D2771): a task the sandbox
 * drives (`start`, `Sandbox.resume`) can be suspended while calls of it are open, and
 * continued from the dump in another sandbox or process. `S` is the type of the calls'
 * state.
 */
export interface AsyncHostFunction<S = unknown> {
  /** Starts the call: its result, or a promise of it. `this` is the call. */
  call(this: HostCall<S>, ...args: unknown[]): unknown;
  /** The call's state when a dump is taken: JSON data. Default: `call.state`. */
  save?(call: HostCall<S>): S | undefined;
  /**
   * Continues the call after a load, from the `state` it was dumped with: its result, or
   * a promise of it. Without `resume` the call is made again (`call`); a plain function
   * (no hooks) is not continued: its call rejects.
   */
  resume?(state: S, call: HostCall<S>): unknown;
}

export interface ConsoleLike {
  log(...args: unknown[]): void;
  info(...args: unknown[]): void;
  warn(...args: unknown[]): void;
  error(...args: unknown[]): void;
}

export interface SandboxOptions {
  /** The compiled djs-wasm module; default: `loadModule()`. */
  module?: WebAssembly.Module;
  /**
   * The dump module (`djs-dump.wasm`: djs-wasm with the `dump` feature) for a `module`
   * without the dumper (packaging B, D2635): `dump`, `Run.dump` and a task's `suspend`
   * and `checkpoint` copy the VM's image to an instance of it. Not needed when `module`
   * has the dumper itself, nor for loading (`load`, `loadRun`, `Sandbox.resume`): a dump
   * is a Script.
   */
  dumpModule?: WebAssembly.Module;
  /** Heap cap in bytes; default: the engine's (4 GiB). */
  memoryLimit?: number;
  /** Fuel per `vm_run` call: how often the host gets control back. Default 1,000,000. */
  fuelSlice?: number;
  /**
   * Fuel budget of one `run`, after which it fails with "out of fuel" (the call is
   * abandoned; the sandbox stays usable). Default: none.
   */
  maxFuel?: number;
  /**
   * Global host functions, by name. A call parks the VM until the function returns. A
   * dotted name (`tools.read`) is a property of a global object (`tools`) the sandbox
   * makes for its first such name; the host function's name is the last part (`read`).
   * Dumps name host functions by these names.
   */
  functions?: Record<string, HostFunction>;
  /**
   * Global async host functions, by name (D1374): a call returns a promise in the VM at
   * once and the script goes on, so several calls can be outstanding (`Promise.all`
   * over tools). The function may return a value or a promise. Names as in `functions`.
   * An `AsyncHostFunction` (`{ call, save, resume }`) adds the hooks that let a task
   * suspended into a dump continue its open calls after a load.
   */
  asyncFunctions?: Record<string, HostFunction | AsyncHostFunction>;
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
   * `"throw"` fails the run with a `threw` SandboxError, `"collect"` keeps them for
   * `takeUnhandledRejections()`.
   */
  unhandledRejections?: "warn" | "throw" | "collect";
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
   * What `WeakRef` and `FinalizationRegistry` do (D1680): `"collected"` (weak, as the
   * spec intends), `"strong"` (targets never die and cleanup callbacks never run, so no
   * collection is observable) or `"absent"` (the globals are deleted). Default
   * `"collected"`, with `deterministic` `"strong"`.
   */
  weakRefs?: "collected" | "strong" | "absent";
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
   * Run deterministically (D1680): with the same code, parameters and results in the same
   * order, a run does the same thing every time. `Math.random` is seeded from `seed`, the
   * time zone is UTC, weak references are strong (unless `weakRefs` says otherwise), and
   * the clock starts at `startedAt` and moves only when an async host function's result
   * is delivered: to `now` at that moment (never backwards). Sync host functions and
   * fuel slices do not move it. A fixed `now` makes the clock fully reproducible.
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
}

export type SandboxErrorKind = "threw" | "syntax" | "fuel" | "trap" | "memory" | "misuse" | "diverged" | "suspended";

/**
 * One entry of a run's journal (D1730, docs/host.md "Journals"): everything a
 * deterministic run took from outside, in order, plus checks. Values are host values
 * (JSON-serializable when the host's values are); a host error is `{ name, message }`.
 * - `run`: the start: the engine build, the code, the parameters and the canonical hash
 *   of the VM before the run.
 * - `sync`: a sync host function's call and outcome (console output is not journaled).
 * - `async`: an async host function's result, in delivery order, with the clock it
 *   carried.
 * - `check`: the VM was idle (quiescent) after `delivered` results, with `issued` host
 *   calls made and this canonical hash.
 * - `end`: the outcome (`ok` false: the error's display text), with the final counts and
 *   hash.
 */
export type JournalEntry =
  | { type: "run"; engine: string; code: string; params: Record<string, unknown>; hash: string }
  | { type: "sync"; fn: string; args: unknown[]; ok: boolean; value: unknown }
  | { type: "async"; callId: number; fn: string; args: unknown[]; ok: boolean; value: unknown; now: number }
  | { type: "check"; delivered: number; issued: number; hash: string }
  | { type: "end"; ok: boolean; value: unknown; delivered: number; issued: number; hash: string };

/** Options of one run. */
export interface RunOptions {
  /** Record the run (deterministic sandboxes only): entries are appended to this array. */
  journal?: JournalEntry[];
}

/** An async host call the script made that has no result yet (`Run.calls`). */
export interface OpenCall {
  /** The VM's call id: `settle` takes it, and a dump keeps it. */
  id: number;
  /** The host function's name, as given in `asyncFunctions` (`sleep`, `tools.read`). */
  fn: string;
  args: unknown[];
  /** A loaded call's state, as dumped (D2770); absent when the dump has none. */
  state?: unknown;
}

/** A host call's outcome for `Run.settle`: a value, or (`ok` false) an error to reject with. */
export interface Settlement {
  ok: boolean;
  value: unknown;
}

/** What `Sandbox.load` recreated: the open host calls of a dump (0171, D2890). */
export interface LoadedDump {
  calls: OpenCall[];
}

/**
 * Options of `Sandbox.begin`. A `journal` records the run while it stays in this sandbox:
 * the sync calls, every `settle` in order and a check at every idle point; `replay` runs
 * it like a `runAsync` journal.
 */
export interface BeginOptions extends RunOptions {
  /**
   * The script's name, as stack traces show it (`sleep.js:4:13`); default `sandbox.js`.
   */
  filename?: string;
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
const ABI_VERSION = 4;
/** `vm_new_with`'s flag: frozen builtins (D2304). */
const VM_FROZEN_BUILTINS = 1;
const CONSOLE_METHODS = ["log", "info", "warn", "error"] as const;
/**
 * A plain dump's host hooks (D2832, D2890): the global object `$djs` with `resume(id,
 * name, args, state)` and `seed(lo, hi)`, defined while the dump's Script runs. Their
 * function ids are outside the host functions' (ids are places in `fnNames`).
 */
const PLAIN_HOOKS = "$djs";
const HOOK_RESUME = 0x7fff_fff0;
const HOOK_SEED = 0x7fff_fff1;

/** A plain dump (D2830): `"djs-dump";`, and whether its text is TypeScript. */
export function plainDump(text: string): { typescript: boolean } | undefined {
  const m = /^\s*"djs-dump( typescript)?";/.exec(text);
  return m ? { typescript: m[1] !== undefined } : undefined;
}

interface Exports {
  memory: WebAssembly.Memory;
  djs_abi_version(): number;
  djs_staging(len: number): number;
  /** Since 0094 (D1942); older modules lack it. */
  djs_image_staging?(len: number, memoryLimit: number): number;
  djs_out_ptr(): number;
  djs_out_len(): number;
  djs_panic_ptr(): number;
  djs_panic_len(): number;
  __stack_pointer?: WebAssembly.Global;
  djs_debug_panic(): void;
  vm_new(memoryLimit: number): number;
  /** Since 0146 (D2304); older modules lack it. */
  vm_new_with?(memoryLimit: number, live: number, flags: number): number;
  vm_frozen_builtins?(vm: number): number;
  vm_dump?(vm: number, len: number): number;
  vm_from_bytes(len: number, memoryLimit: number): number;
  /** Since 0177 (D2701). */
  vm_replace_image?(vm: number, len: number): number;
  vm_free(vm: number): void;
  vm_arena_ptr(vm: number): number;
  vm_arena_len(vm: number): number;
  vm_collect(vm: number): number;
  vm_compile(vm: number, goal: number, paramsLen: number, srcLen: number): number;
  vm_compile_file(vm: number, goal: number, nameLen: number, paramsLen: number, srcLen: number): number;
  vm_global(vm: number): number;
  vm_new_object(vm: number): number;
  /** Since 0207 (D2903): the plain dump runtime, `$djs`'s helpers. */
  vm_dump_runtime?(vm: number, len: number): number;
  vm_new_host_function(vm: number, fnId: number, nameLen: number): number;
  vm_new_async_host_function(vm: number, fnId: number, nameLen: number): number;
  vm_define(vm: number, obj: number, nameLen: number, value: number): number;
  vm_decode_wire(vm: number, len: number): number;
  /** Since 0162 (D2551); older modules lack it. */
  vm_string_reserve?(vm: number, len: number): number;
  vm_new_error(vm: number, kind: number, msgLen: number): number;
  vm_handle_bits(vm: number, h: number): void;
  vm_handle_free(vm: number, h: number): void;
  vm_display(vm: number, h: number): number;
  vm_call(vm: number, f: number, thisArg: number, argc: number): number;
  vm_run(vm: number, fuel: number): number;
  /** Since 0176 (D2653). */
  vm_fuel_used?(vm: number): number;
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
  vm_set_math_random_seed(vm: number, lo: number, hi: number): void;
  vm_outcome(vm: number): number;
  /** Since 0204 (D2890): loading plain dumps. */
  vm_open_call?(vm: number, parked: number, fnId: number): number;
  vm_set_next_call_id?(vm: number, id: number): void;
  vm_set_weak_refs(vm: number, mode: number): number;
  vm_resolve_host_call_at(vm: number, callId: number, ok: number, value: number, now: number): number;
  vm_canonical_hash(vm: number): number;
  djs_build_hash(): number;
  /** The base snapshot (D1321); the inject exports only with the base coded (D2252). */
  djs_base_ptr(): number;
  djs_base_len(): number;
  djs_base_inject_ptr?(len: number): number;
  djs_base_inject?(): number;
  djs_base_coded_ptr?(len: number): number;
}

const modules = new Map<string, Promise<WebAssembly.Module>>();
/** The default module once loaded, for the synchronous constructor. */
let loadedDefault: WebAssembly.Module | undefined;

/**
 * The dump module's sandboxes, one per module and memory limit in the process (D2802):
 * every sandbox with that `dumpModule` moves its image into the same instance and VM for
 * a dump or a load, which runs no host function (the loader refuses host calls), so the
 * VM is the only state they share, and it is replaced each time.
 */
const dumpers = new WeakMap<WebAssembly.Module, Map<number, Sandbox>>();

/** The default location of djs-wasm: `$DJS_WASM`, else the `wasm` profile build output. */
export function defaultWasmPath(): string | URL {
  const env = (globalThis as { process?: { env?: Record<string, string | undefined> } }).process?.env;
  return env?.DJS_WASM ?? new URL("../../target/wasm32-unknown-unknown/wasm/djs_wasm.wasm", import.meta.url);
}

/**
 * Compiles djs-wasm once per location and caches the `WebAssembly.Module` (ARCH: the
 * module is compiled once; every sandbox gets its own instance).
 */
export function loadModule(location: string | URL = defaultWasmPath()): Promise<WebAssembly.Module> {
  const key = String(location);
  let m = modules.get(key);
  if (m === undefined) {
    m = (async () => {
      let bytes: Uint8Array;
      if (typeof location === "string" || location.protocol === "file:") {
        // Not a literal specifier, so the package type-checks without node's types.
        const fsModule = "node:fs/promises";
        const fs = (await import(fsModule)) as { readFile(p: string | URL): Promise<Uint8Array> };
        bytes = await fs.readFile(location);
      } else {
        bytes = new Uint8Array(await (await fetch(location)).arrayBuffer());
      }
      return WebAssembly.compile(bytes as Uint8Array<ArrayBuffer>);
    })();
    modules.set(key, m);
    const isDefault = key === String(defaultWasmPath());
    m.then(
      (mod) => {
        if (isDefault) loadedDefault = mod;
      },
      () => modules.delete(key),
    );
  }
  return m;
}

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();
/** What a run in progress needs from its sandbox. */
interface RunCore {
  /** The open async calls, oldest first, until their results are delivered. */
  open: Map<number, OpenCall>;
  /**
   * A resumed run's result promise, the handle its dump named `run` (freed at the end);
   * 0 for a run this sandbox started, whose result is the entry call's outcome.
   */
  result: number;
  /** Fuel used so far: `maxFuel` is per run, across all its steps. */
  used: number;
  /** A new async call (`runAsync`: call the host function now). */
  onCall?(call: OpenCall, fnId: number): void;
  /** Before every fuel slice (`runAsync`: pass back the results that came in). */
  beforeSlice?(): void;
  advance(): Promise<RunEnd>;
  settle(id: number, s: Settlement): void;
  /** Passes a result back; `at`: a replayed result's clock. */
  deliver(id: number, ok: boolean, value: unknown, at?: number): void;
  /** `states`: the host's state of open calls, by id, as JSON text (D2770). */
  dump(states?: [number, string][]): string;
}

type RunEnd = { state: "idle" } | { state: "done"; value: unknown } | { state: "threw"; error: SandboxError };

/**
 * A run the host drives itself (docs/host.md "Host-driven runs"; the part 0179
 * needs of 0175): `Sandbox.begin` runs a body to its first idle point, the host settles
 * the open async calls in any order and advances it, and while it is idle it can be
 * dumped (`dump()`) and continued elsewhere (`Sandbox.resume`). The async host functions
 * are never called by the run: `calls` lists them and the host answers, so that a call
 * can outlive the process that issued it. Sync host functions are called as in `run`.
 */
export class Run {
  /** "idle": waiting on `calls`; "done": `value` is the result; "threw": `error`. */
  state: "idle" | "done" | "threw" = "idle";
  value: unknown = undefined;
  error: SandboxError | undefined = undefined;
  readonly #core: RunCore;

  /** Made by `Sandbox.begin` and `Sandbox.resume`. */
  constructor(core: RunCore) {
    this.#core = core;
  }

  /** The async host calls waiting for a result, oldest first. */
  get calls(): OpenCall[] {
    return [...this.#core.open.values()];
  }

  /**
   * Delivers the result of open call `id`. The VM applies it at the next `advance`, after
   * the results settled before it (D1670).
   */
  settle(id: number, s: Settlement): void {
    this.#core.settle(id, s);
  }

  /**
   * Runs to the next idle point (or the end): applies the settled results, and calls the
   * sync host functions the script calls meanwhile (awaiting them if they return
   * promises). New async calls show up in `calls`.
   */
  async advance(): Promise<void> {
    if (this.state !== "idle") throw new SandboxError("misuse", `the run is ${this.state}`);
    this.end(await this.#core.advance());
  }

  /**
   * The waiting run as a dump (a plain Script, D2830; docs/plain-js-dumps.md): its
   * suspended activations, open calls and data as JavaScript, for `loadRun` in another
   * sandbox or process. Only while idle, and only with `frozenBuiltins` (D2836). `states` are the host's states of open calls, by id (JSON
   * data, D2770): the dump writes each with its call, and the load gives it back
   * (`OpenCall.state`).
   */
  dump(states: Record<number, unknown> = {}): string {
    if (this.state !== "idle") throw new SandboxError("misuse", `the run is ${this.state}: only an idle run can be dumped`);
    return this.#core.dump(Object.entries(states).map(([id, v]) => [Number(id), stateJson(Number(id), v)]));
  }

  /** Records how a step ended. */
  end(e: RunEnd): void {
    this.state = e.state;
    if (e.state === "done") this.value = e.value;
    if (e.state === "threw") this.error = e.error;
  }
}

/** A call a task waits for: the hooks' view of it, and how to stop waiting. */
interface LiveCall {
  call: HostCall;
  abort: AbortController;
}

/** A dump a task was asked for: `suspend` (the task stops) or `checkpoint`. */
interface DumpRequest {
  stop: boolean;
  resolve(text: string): void;
  reject(e: unknown): void;
}

/**
 * A run the sandbox drives (D2771): `Sandbox.start` begins one, `Sandbox.resume`
 * continues one from a dump. It calls the async host functions as the script calls them
 * and passes their results back, like `runAsync`; `result` is the outcome. While it waits
 * on host calls it can be suspended (`suspend()`): the sandbox writes it as a dump, with
 * each open call's state from its function's `save` hook, and stops. `Sandbox.resume`
 * continues the dump in a new sandbox, any time later and in any process: each open call
 * is continued by its function's `resume` hook (or made again, or rejected).
 */
export class Task {
  /** The sandbox the task runs in (dispose it when done). */
  readonly sandbox: Sandbox;
  /**
   * The run's result. Rejects with a SandboxError: `threw` for an exception, `suspended`
   * when the task was suspended into a dump (this rejection counts as handled).
   */
  readonly result: Promise<unknown>;
  readonly #calls: Map<number, LiveCall>;
  readonly #request: (r: DumpRequest) => void;

  /** Made by `Sandbox.start` and `Sandbox.resume`. */
  constructor(sandbox: Sandbox, result: Promise<unknown>, calls: Map<number, LiveCall>, request: (r: DumpRequest) => void) {
    this.sandbox = sandbox;
    this.result = result;
    this.#calls = calls;
    this.#request = request;
  }

  /** The async host calls the task waits for, oldest first. */
  get calls(): HostCall[] {
    return [...this.#calls.values()].map((c) => c.call);
  }

  /**
   * Suspends the task at its next idle point (no script work left, only host calls to
   * wait for): calls `save` for every open call, dumps the run with those states and
   * stops. The open calls' signals are aborted and their results ignored; `result`
   * rejects with kind `suspended`. Resolves to the dump (a Script, for
   * `Sandbox.resume`). Rejects, and the task goes on, when the run cannot be dumped (or a
   * `save` throws); rejects when the run ends first.
   */
  suspend(): Promise<string> {
    return new Promise((resolve, reject) => this.#request({ stop: true, resolve, reject }));
  }

  /** `suspend` without stopping: a dump at the next idle point, and the task goes on. */
  checkpoint(): Promise<string> {
    return new Promise((resolve, reject) => this.#request({ stop: false, resolve, reject }));
  }
}

/**
 * A VM image moving between the main module and the dump module (packaging B, D2635):
 * the arena as is, with what the sandbox keeps beside it.
 */
interface RawImage {
  image: Uint8Array;
  functions: string[];
  clock?: number;
  namespaces: [string, number][];
}

/** `drive`'s value at an idle point of a host-driven run. */
const IDLE = Symbol("idle");

export class Sandbox {
  private readonly x: Exports;
  private readonly module: WebAssembly.Module;
  /** The VM; replaced when the dump module's sandbox takes another image (`takeImage`). */
  private vm: number;
  private readonly opts: SandboxOptions;
  /** Host function names by id - 1. */
  private readonly fnNames: string[] = [];
  private readonly fns = new Map<number, HostFunction>();
  /** Async host functions with hooks (D2771), by id. */
  private readonly hooks = new Map<number, AsyncHostFunction>();
  private readonly atomCache: (string | undefined)[] = [];
  private readonly rejections: unknown[] = [];
  private readonly cons: ConsoleLike;
  private dead: string | null = null;
  /** Deterministic mode: the VM's clock, moved only by async results (never backwards). */
  private clock: number | undefined;
  /** The journal of the current run, being recorded or replayed (D1730). */
  private journal: Journal | null = null;
  /** Namespace objects this sandbox made for dotted function names (`tools`), as handles. */
  private readonly namespaces = new Map<string, number>();
  /** The host-driven run in progress (`begin`, `resume`), until it ends. */
  private active: RunCore | null = null;
  /** The task in progress (`start`, `resume`): ends it when the sandbox is disposed. */
  private cancelTask: (() => void) | null = null;

  /** A new sandbox; loads (and caches) the default module unless `opts.module` is given. */
  static async create(opts: SandboxOptions = {}): Promise<Sandbox> {
    return new Sandbox({ ...opts, module: opts.module ?? (await loadModule()) });
  }

  /**
   * Continues a task from its dump (`Task.suspend()`, or a host-driven `Run.dump()`) in
   * a new sandbox made from `opts` (the same host functions, by name, as the one that
   * dumped): `(await Sandbox.resume(dump, opts)).result`. See `resume`. A dump is code:
   * resume only dumps you trust.
   */
  static async resume(dump: string, opts: SandboxOptions = {}): Promise<Task> {
    const sandbox = await Sandbox.create(opts);
    try {
      return sandbox.resume(dump);
    } catch (e) {
      sandbox.dispose();
      throw e;
    }
  }

  /**
   * Loads a dumped run into this sandbox (a fresh one) and drives it (D2771): each open
   * call is continued by its function's `resume` hook with the state it was dumped with,
   * made again (`call`) when the function has no `resume`, and rejected when it is a
   * plain function (the sandbox cannot know whether calling it again is safe).
   */
  resume(dump: string): Task {
    const { calls, result } = this.loadRunDump(dump);
    return this.task(this.startRun(calls, result), calls);
  }

  /**
   * Loads a dumped run into this sandbox (a fresh one) for the host to drive: the run is
   * idle, waiting on the same open calls with the same ids (`run.calls`, with their
   * states); the host settles them, issues them again or rejects them. No hooks run.
   */
  loadRun(dump: string): Run {
    const { calls, result } = this.loadRunDump(dump);
    return this.startRun(calls, result).run;
  }

  /** Loads a dump that holds a run: its open calls and the run's result promise. */
  private loadRunDump(dump: string): { calls: OpenCall[]; result: number } {
    const { calls, handles } = this.loadDump(dump);
    const result = handles.get("run") ?? 0;
    this.guard(() => {
      for (const [name, h] of handles) if (name !== "run") this.x.vm_handle_free(this.vm, h);
    });
    if (result === 0) throw new SandboxError("misuse", "the dump holds no run (no handle named run)");
    return { calls, result };
  }

  /**
   * A new sandbox (its own wasm instance and VM), synchronously: needs a compiled module,
   * `opts.module` or one `loadModule()` has finished loading. (`raw`: internal, a VM image
   * of this process moving to the dump module, D2635.)
   */
  constructor(opts: SandboxOptions = {}, raw?: RawImage) {
    const module = opts.module ?? loadedDefault;
    if (module === undefined) {
      throw new SandboxError("misuse", "no compiled module: pass opts.module or await loadModule() first");
    }
    const x = new WebAssembly.Instance(module, {}).exports as unknown as Exports;
    if (x.djs_abi_version() !== ABI_VERSION) {
      throw new SandboxError("misuse", `djs-wasm ABI ${x.djs_abi_version()}, expected ${ABI_VERSION}`);
    }
    let vm: number;
    let restoredFns: string[] = [];
    if (raw === undefined) {
      // The module's base, decoded once by its first instance, not by each (D2252), and
      // kept for later instances before `vm_new` takes this instance's bytes as its arena
      // (D2060).
      provideBase(module, x);
      rememberBase(module, x);
      if (opts.frozenBuiltins) {
        if (!x.vm_new_with) throw new SandboxError("misuse", "the djs-wasm module has no frozenBuiltins (vm_new_with)");
        vm = x.vm_new_with(opts.memoryLimit ?? 0, 0, VM_FROZEN_BUILTINS);
      } else vm = x.vm_new(opts.memoryLimit ?? 0);
      if (vm === 0) throw new SandboxError("memory", "memory limit too small for the realm");
    } else {
      const image = raw.image;
      restoredFns = raw.functions;
      if (opts.deterministic) this.clock = raw.clock ?? opts.deterministic.startedAt;
      for (const [ns, h] of raw.namespaces) this.namespaces.set(ns, h);
      // May grow memory: take the buffer afterwards. The image's buffer becomes the arena,
      // so it is staged with the arena's capacity (D1942).
      const limit = opts.memoryLimit ?? 0;
      const at = x.djs_image_staging ? x.djs_image_staging(image.length, limit) : x.djs_staging(image.length);
      new Uint8Array(x.memory.buffer).set(image, at);
      vm = x.vm_from_bytes(image.length, opts.memoryLimit ?? 0);
      if (vm === 0) throw new SandboxError("misuse", outText(x));
    }
    this.x = x;
    this.module = module;
    this.vm = vm;
    this.opts = opts;
    const cons = opts.console ?? (globalThis.console as ConsoleLike);
    this.cons = cons;
    const builtins: Record<string, HostFunction> = {};
    for (const m of CONSOLE_METHODS) builtins[`console.${m}`] = (...a) => cons[m](...a);
    if (raw !== undefined) {
      // The VM already has its host functions; bind the ids to the given implementations.
      restoredFns.forEach((name, i) => {
        const a = opts.asyncFunctions?.[name];
        const f = builtins[name] ?? opts.functions?.[name] ?? (typeof a === "object" ? a.call : a);
        if (f === undefined) throw new SandboxError("misuse", `the image needs host function ${name}`);
        this.fnNames.push(name);
        this.fns.set(i + 1, f);
        if (typeof a === "object") this.hooks.set(i + 1, a);
      });
    } else {
      this.guard(() => {
        const console = this.check(x.vm_new_object(vm), "memory");
        for (const m of CONSOLE_METHODS) {
          const f = this.hostFunction(`console.${m}`, builtins[`console.${m}`]);
          this.defineOn(console, m, f);
          x.vm_handle_free(vm, f);
        }
        const g = x.vm_global(vm);
        this.defineOn(g, "console", console);
        x.vm_handle_free(vm, g);
        x.vm_handle_free(vm, console);
      });
      for (const [name, f] of Object.entries(opts.functions ?? {})) this.defineFunction(name, f);
      for (const [name, f] of Object.entries(opts.asyncFunctions ?? {})) this.defineAsyncFunction(name, f);
      // What a new VM starts with; an image keeps its own (D1680).
      const det = opts.deterministic;
      const [lo, hi] = det ? seedWords(det.seed) : opts.randomSeed !== undefined ? seedWords(opts.randomSeed) : entropy();
      x.vm_set_random_seed(vm, lo, hi);
      const weak = opts.weakRefs ?? (det ? "strong" : "collected");
      if (this.guard(() => x.vm_set_weak_refs(vm, WEAK_REF_MODES[weak])) !== 0) {
        throw new SandboxError("misuse", `bad weakRefs mode ${String(weak)}`);
      }
      if (det) {
        this.clock = det.startedAt;
        x.vm_set_now(vm, det.startedAt);
        x.vm_set_time_zone_offset(vm, 0);
      }
    }
  }

  /** Defines a global host function. */
  defineFunction(name: string, f: HostFunction): void {
    this.defineGlobalFunction(name, f, false);
  }

  /** Defines a global async host function (see `SandboxOptions.asyncFunctions`). */
  defineAsyncFunction(name: string, f: HostFunction | AsyncHostFunction): void {
    this.defineGlobalFunction(name, f, true);
  }

  private defineGlobalFunction(name: string, f: HostFunction | AsyncHostFunction, async: boolean): void {
    this.guard(() => {
      const x = this.x;
      const h = this.hostFunction(name, f, async);
      const dot = name.indexOf(".");
      if (dot > 0) {
        // `tools.read`: a property of the namespace object `tools`, made on first use.
        const ns = name.slice(0, dot);
        let o = this.namespaces.get(ns);
        if (o === undefined) {
          o = this.check(x.vm_new_object(this.vm), "memory");
          const g = x.vm_global(this.vm);
          this.defineOn(g, ns, o);
          x.vm_handle_free(this.vm, g);
          this.namespaces.set(ns, o);
        }
        this.defineOn(o, name.slice(dot + 1), h);
      } else {
        const g = x.vm_global(this.vm);
        this.defineOn(g, name, h);
        x.vm_handle_free(this.vm, g);
      }
      x.vm_handle_free(this.vm, h);
    });
  }

  /** Defines a global variable holding a copy of `value` (sent through the wire format). */
  setGlobal(name: string, value: unknown): void {
    this.guard(() => {
      const h = this.toHandle(value);
      const g = this.x.vm_global(this.vm);
      this.defineOn(g, name, h);
      this.x.vm_handle_free(this.vm, g);
      if (h !== 0) this.x.vm_handle_free(this.vm, h);
    });
  }

  /**
   * Runs `code` as a codemode function body (`return` gives the result) with the
   * `params` as its parameters (sent through the wire format). `console` is a global. Host
   * functions must return synchronously; use `runAsync` for ones that return promises. A
   * promise as the result is unwrapped (fulfilled: its value; rejected: thrown).
   */
  run(code: string, params: Record<string, unknown> = {}, opts: RunOptions = {}): unknown {
    this.beginJournal(code, params, opts);
    this.enter(code, params);
    try {
      // In sync mode the loop never yields: a promise from a host function is an error.
      return this.auto(false).next().value;
    } catch (e) {
      this.abandon(e);
      throw e;
    } finally {
      this.endRun();
      this.journal = null;
    }
  }

  /**
   * Starts `code` as a codemode function body (as `run`) and runs it to its first idle
   * point or its end, for the host to drive (`Run`): async host functions are not called
   * but listed in `run.calls`. Sync host functions must return synchronously here (as in
   * `run`); `advance` awaits them. A compile error throws; an exception in the script is
   * `run.state === "threw"`. One run at a time: the sandbox runs nothing else until it
   * ends.
   */
  begin(code: string, params: Record<string, unknown> = {}, opts: BeginOptions = {}): Run {
    this.beginJournal(code, params, opts);
    this.enter(code, params, false, opts.filename);
    const core = this.startRun([], 0);
    try {
      core.run.end(this.step(core, false).next().value as RunEnd);
    } catch (e) {
      this.endRun();
      this.abandon(e);
      throw e;
    }
    if (core.run.state !== "idle") this.endRun();
    return core.run;
  }

  /**
   * Starts `code` as a codemode function body (as `runAsync`) in a task the sandbox
   * drives (D2771): it calls the async host functions as the script calls them, and
   * `task.result` is the outcome. Unlike `runAsync`, the task can be suspended into a
   * dump while it waits (`task.suspend()`) and continued later (`Sandbox.resume`). The
   * first step runs now: a compile error throws. One task or run at a time.
   */
  start(code: string, params: Record<string, unknown> = {}, opts: BeginOptions = {}): Task {
    this.beginJournal(code, params, opts);
    this.enter(code, params, false, opts.filename);
    return this.task(this.startRun([], 0), []);
  }

  /**
   * Drives `core` as a task: continues the `loaded` calls of a resumed run through their
   * hooks, calls new async host functions as the script calls them, delivers results at
   * the next slice, and serves `suspend`/`checkpoint` at idle points.
   */
  private task(core: RunCore & { run: Run }, loaded: OpenCall[]): Task {
    const calls = new Map<number, LiveCall>();
    const finished: { id: number; ok: boolean; value: unknown }[] = [];
    const requests: DumpRequest[] = [];
    let ended = false;
    let wake: (() => void) | null = null;
    const poke = () => {
      const w = wake;
      wake = null;
      w?.();
    };
    const done = (id: number, ok: boolean, value: unknown) => {
      if (ended || !calls.has(id)) return;
      finished.push({ id, ok, value });
      poke();
    };
    // Starts (or, after a load, continues) a call; its outcome arrives through `done`.
    const begin = (open: OpenCall, loaded: boolean) => {
      const fnId = this.fnNames.indexOf(open.fn) + 1;
      const live = newCall(open.id, open.fn, open.args, open.state);
      calls.set(open.id, live);
      let r: unknown;
      try {
        const hooks = this.hooks.get(fnId);
        if (!loaded) r = this.invoke(fnId, open.args, live.call);
        else if (hooks === undefined) {
          throw new Error(`${open.fn}: the call was open when the run was dumped, and the host function has no hooks to continue it`);
        } else if (hooks.resume) r = hooks.resume(open.state, live.call);
        else r = this.invoke(fnId, open.args, live.call);
      } catch (e) {
        done(open.id, false, e);
        return;
      }
      if (r instanceof Promise) r.then((v) => done(open.id, true, v), (e) => done(open.id, false, e));
      else done(open.id, true, r);
    };
    core.onCall = (call) => begin(call, false);
    core.beforeSlice = () => {
      for (const { id, ok, value } of finished.splice(0)) {
        calls.delete(id);
        core.deliver(id, ok, value);
      }
    };
    const stopWaiting = () => {
      ended = true;
      this.cancelTask = null;
      for (const c of calls.values()) c.abort.abort();
      for (const r of requests.splice(0)) r.reject(new SandboxError("misuse", "the task ended before an idle point"));
    };
    let settle!: { resolve(v: unknown): void; reject(e: unknown): void };
    const result = new Promise<unknown>((resolve, reject) => (settle = { resolve, reject }));
    const stop = (e: SandboxError) => {
      if (ended) return;
      stopWaiting();
      if (this.active === core) this.endRun();
      if (e.kind === "suspended") result.catch(() => {});
      settle.reject(e);
      poke();
    };
    this.cancelTask = () => stop(new SandboxError("misuse", "the sandbox was disposed"));
    // The dump at an idle point: each open call's state, from `save` or `call.state`.
    const dump = (): string => {
      const states: [number, string][] = [];
      for (const { call } of calls.values()) {
        const save = this.hooks.get(this.fnNames.indexOf(call.fn) + 1)?.save;
        const state = save ? save(call) : call.state;
        if (state !== undefined) states.push([call.id, stateJson(call.id, state)]);
      }
      return this.dumpRun(core, states);
    };
    const loop = async () => {
      for (;;) {
        const end = await core.advance();
        if (ended) return;
        if (end.state !== "idle") {
          stopWaiting();
          if (end.state === "done") settle.resolve(end.value);
          else settle.reject(end.error);
          return;
        }
        // Idle: deliver what came in meanwhile, serve dump requests, or wait.
        while (finished.length === 0 && !ended) {
          const asked = requests.splice(0);
          if (asked.length > 0) {
            let text: string;
            try {
              text = dump();
            } catch (e) {
              for (const r of asked) r.reject(e);
              continue;
            }
            for (const r of asked) r.resolve(text);
            if (asked.some((r) => r.stop)) {
              stop(new SandboxError("suspended", "the task was suspended into a dump"));
              return;
            }
            continue;
          }
          if (calls.size === 0) {
            // Nothing to wait for (as in `runAsync`): the run is over without a result.
            stopWaiting();
            this.endRun();
            settle.resolve(undefined);
            return;
          }
          await new Promise<void>((w) => (wake = w));
        }
        if (ended) return;
      }
    };
    for (const c of loaded) begin(c, true);
    const task = new Task(this, result, calls, (r) => {
      if (ended) r.reject(new SandboxError("misuse", "the task has ended"));
      else {
        requests.push(r);
        poke();
      }
    });
    loop().catch((e) => {
      if (ended) return;
      stopWaiting();
      settle.reject(e);
    });
    return task;
  }

  /** Makes a run, waiting on `calls`, with its result promise `result` (0: the entry call's). */
  private startRun(calls: OpenCall[], result: number): RunCore & { run: Run } {
    const core: RunCore & { run: Run } = {
      open: new Map(calls.map((c) => [c.id, c])),
      result,
      used: 0,
      run: undefined as unknown as Run,
      advance: async () => {
        try {
          const it = this.step(core, true);
          let r = it.next();
          while (!r.done) r = it.next(await r.value);
          const end = r.value;
          if (end.state !== "idle") this.endRun();
          return end;
        } catch (e) {
          this.endRun();
          this.abandon(e);
          throw e;
        }
      },
      settle: (id, s) => {
        if (this.active !== core) throw new SandboxError("misuse", "the run has ended");
        if (!core.open.has(id)) throw new SandboxError("misuse", `no open call ${id}`);
        core.deliver(id, s.ok, s.value);
      },
      deliver: (id, ok, value, at) => {
        const call = core.open.get(id);
        core.open.delete(id);
        const now = this.complete(id, ok, value, true, at);
        const j = this.journal;
        if (j) j.delivered++;
        if (j?.mode === "record" && call) {
          j.push({ type: "async", callId: id, fn: call.fn, args: call.args, ok, value: errorJson(ok, value), now: now! });
        }
      },
      dump: (states) => {
        if (this.active !== core) throw new SandboxError("misuse", "the run has ended");
        return this.dumpRun(core, states);
      },
    };
    core.run = new Run(core);
    this.active = core;
    return core;
  }

  /** One step of a host-driven run: `drive` until idle (recording a check) or the end. */
  private *step(core: RunCore, async: boolean): Generator<Promise<unknown>, RunEnd, unknown> {
    try {
      const r = yield* this.drive(async, core);
      if (r !== IDLE) return { state: "done", value: r };
      this.recordCheck();
      return { state: "idle" };
    } catch (e) {
      if (e instanceof SandboxError && e.kind === "threw") return { state: "threw", error: e };
      throw e;
    }
  }

  /**
   * The run loop of `run`, `runAsync` and `replay`, on top of the host-driven run (`drive`
   * to each idle point): calls the async host functions as soon as the script calls them
   * (or, replaying, takes their results from the journal), passes their results back
   * before the next fuel slice, and at an idle point waits for the next one. Yields a
   * promise when the host must wait (only when `async`), returns the result.
   */
  private *auto(async: boolean): Generator<Promise<unknown>, unknown, unknown> {
    const j = this.journal;
    // Async host calls being served, and those that finished but are not passed back yet.
    const inflight = new Map<number, Promise<void>>();
    const finished: { callId: number; ok: boolean; value: unknown }[] = [];
    const core = this.startRun([], 0);
    core.beforeSlice = () => {
      for (const { callId, ok, value } of finished.splice(0)) {
        inflight.delete(callId);
        core.deliver(callId, ok, value);
      }
    };
    core.onCall = (call, fnId) => {
      // Replay: the result comes from the journal, at an idle point.
      if (j?.mode === "replay") return;
      const callId = call.id;
      let p: Promise<unknown>;
      try {
        const r = this.invoke(fnId, call.args);
        if (!(r instanceof Promise)) {
          finished.push({ callId, ok: true, value: r });
          return;
        }
        if (!async) throw new TypeError(`host function ${call.fn} returned a promise; use runAsync`);
        p = r;
      } catch (e) {
        finished.push({ callId, ok: false, value: e });
        return;
      }
      inflight.set(
        callId,
        p.then(
          (value) => void finished.push({ callId, ok: true, value }),
          (value) => void finished.push({ callId, ok: false, value }),
        ),
      );
    };
    for (;;) {
      const r = yield* this.drive(async, core);
      if (r !== IDLE) return r;
      if (j?.mode === "record") this.recordCheck();
      else if (j && core.open.size > 0) {
        // Replay: the checks recorded at this point, then the next result.
        j.checkIdle(this.canonicalHash());
        const e = j.take("async");
        const call = core.open.get(e.callId);
        j.expect(call !== undefined, () => `result ${j.delivered + 1} is for call ${e.callId}, which the script has not made`);
        j.expectCall(e.fn, e.args, call!.fn, call!.args);
        core.deliver(e.callId, e.ok, e.ok ? e.value : errorFromJson(e.value), e.now);
        continue;
      }
      if (finished.length > 0) continue;
      if (inflight.size === 0) return undefined; // nothing was called
      // Only async host calls are left: wait for one of them.
      yield Promise.race(inflight.values());
    }
  }

  /** Records a `check` entry at an idle point, when recording. */
  private recordCheck(): void {
    const j = this.journal;
    if (j?.mode === "record") j.push({ type: "check", delivered: j.delivered, issued: j.issued, hash: this.canonicalHash() });
  }

  /** The run has ended: frees its result handle; the sandbox can run again. */
  private endRun(): void {
    const a = this.active;
    this.active = null;
    this.journal = null;
    if (a !== null && a.result !== 0 && this.dead === null) {
      this.x.vm_handle_free(this.vm, a.result);
      a.result = 0;
    }
  }

  /**
   * `Run.dump()`: the waiting run as a dump, its result promise named "run" (handle
   * 0: the entry call's outcome, which takes no handle, so a dump leaves the heap as it
   * was and a journal's later checks still replay).
   */
  private dumpRun(core: RunCore, states: [number, string][] = []): string {
    return this.dumpText([], [["run", core.result]], states);
  }

  /**
   * The VM's global bindings (or the ones in `names`) as JavaScript that recreates them
   * when `load`ed into another sandbox, also of a later djs (docs/functions-as-code.md):
   * functions as their source text, closures and data as code, builtins by name. The
   * host's functions and `console` are not written: the loading sandbox provides them
   * under the same names. Needs `frozenBuiltins` (D2836). Throws a "misuse" error listing
   * every value that cannot be written, by its path (a weak collection, a Proxy, ...).
   */
  dump(names: string[] = []): string {
    return this.dumpText(names, [], []);
  }

  /**
   * `vm_dump`. staging: the globals the host defines and the names to dump (comma
   * separated, a line each), then this sandbox's host function names (tab separated, id
   * order: an open call or a host function held as a value is written by name), then
   * the values to name for the host (`handle:name`, tab separated), then the open calls'
   * states (`id:bytes`, tab separated) followed by their JSON texts (D2770).
   */
  private dumpText(names: string[], handles: [string, number][], states: [number, string][]): string {
    // Dumps need frozen builtins (D2836): the VM's mode, which a restored sandbox keeps
    // from its image.
    if (this.x.vm_frozen_builtins?.(this.vm) !== 1) {
      throw new SandboxError("misuse", "cannot dump: dumps need frozen builtins (SandboxOptions.frozenBuiltins, D2836)");
    }
    return this.viaDumper((sb) => sb.dumpHere(names, handles, states));
  }

  /**
   * Runs `f` on a sandbox with the dumper: this one, or (with `opts.dumpModule`, D2635)
   * a transient sandbox in the dump module made from this VM's image. The image travels
   * as is, so a dump leaves this VM untouched (D2633).
   */
  private viaDumper<T>(f: (sb: Sandbox) => T): T {
    const dm = this.opts.dumpModule;
    if (dm === undefined) return f(this);
    this.alive();
    const raw = (sb: Sandbox): RawImage => ({
      image: sb.arena().slice(),
      functions: [...sb.fnNames],
      clock: sb.clock,
      namespaces: [...sb.namespaces],
    });
    // One dump-module sandbox per module and memory limit, kept between dumps (D2701,
    // D2802): an instance costs a base, and its VM keeps what the dumper derives from the
    // base on the first dump.
    const limit = this.opts.memoryLimit ?? 0;
    let byLimit = dumpers.get(dm);
    if (byLimit === undefined) dumpers.set(dm, (byLimit = new Map()));
    let d = byLimit.get(limit);
    if (d === undefined || d.dead !== null) {
      d = new Sandbox({ ...this.opts, module: dm, dumpModule: undefined }, raw(this));
      byLimit.set(limit, d);
    } else d.takeImage(raw(this));
    return f(d);
  }

  /** The dump module's sandbox: takes the main sandbox's image into the same VM. */
  private takeImage(raw: RawImage): void {
    const x = this.x;
    this.fnNames.splice(0, this.fnNames.length, ...raw.functions);
    this.namespaces.clear();
    for (const [ns, h] of raw.namespaces) this.namespaces.set(ns, h);
    if (!x.vm_replace_image) {
      this.replaceVm(raw);
      return;
    }
    this.guard(() => {
      const limit = this.opts.memoryLimit ?? 0;
      const at = x.djs_image_staging ? x.djs_image_staging(raw.image.length, limit) : x.djs_staging(raw.image.length);
      new Uint8Array(x.memory.buffer).set(raw.image, at);
      if (x.vm_replace_image!(this.vm, raw.image.length) !== 0) {
        throw new SandboxError("misuse", `the image does not load in the dump module: ${outText(x)}`);
      }
    });
    this.clock = raw.clock;
    this.atomCache.length = 0;
  }

  /** Replaces this sandbox's VM with an image (from the dump module). */
  private replaceVm(raw: RawImage): void {
    const x = this.x;
    this.guard(() => {
      x.vm_free(this.vm);
      const limit = this.opts.memoryLimit ?? 0;
      const at = x.djs_image_staging ? x.djs_image_staging(raw.image.length, limit) : x.djs_staging(raw.image.length);
      new Uint8Array(x.memory.buffer).set(raw.image, at);
      const vm = x.vm_from_bytes(raw.image.length, limit);
      if (vm === 0) {
        this.dead = `lost its VM: ${outText(x)}`;
        throw new SandboxError("misuse", `the dump module's image does not load here: ${outText(x)}`);
      }
      this.vm = vm;
    });
    this.clock = raw.clock;
    this.atomCache.length = 0;
  }

  /** `vm_dump` in this sandbox's own module. */
  private dumpHere(names: string[], handles: [string, number][], states: [number, string][]): string {
    const x = this.x;
    if (!x.vm_dump) throw new SandboxError("misuse", "the djs-wasm module has no dump (vm_dump): build it with the `dump` feature, or pass `dumpModule`");
    const host = new Set(["console"]);
    for (const n of this.fnNames) host.add(n.split(".")[0]);
    const text = [
      [...host].join(","),
      names.join(","),
      this.fnNames.join("\t"),
      handles.map(([name, h]) => `${h}:${name}`).join("\t"),
      states.map(([id, json]) => `${id}:${textEncoder.encode(json).length}`).join("\t"),
      states.map(([, json]) => json).join(""),
    ].join("\n");
    let r = this.guard(() => x.vm_dump!(this.vm, this.stageText(text)));
    if (r === 2) {
      provideBase(this.module, x);
      r = this.guard(() => x.vm_dump!(this.vm, this.stageText(text)));
    }
    if (r !== 0) throw new SandboxError("misuse", r === 1 ? `cannot dump:\n${outText(x)}` : "no base image");
    return outText(x);
  }

  /**
   * Loads a dump (the text of `dump()`): runs it as a Script (D2890), so its bindings
   * become globals of this sandbox. A dump is code: load only dumps you trust, into a
   * fresh sandbox. Not recorded in a journal. Returns the open host calls the dump
   * recreated (its open calls name this sandbox's host functions); `Sandbox.resume`
   * continues a dumped run. The dumper is not needed.
   */
  load(dump: string): LoadedDump {
    const { calls, handles } = this.loadDump(dump);
    this.guard(() => {
      for (const h of handles.values()) this.x.vm_handle_free(this.vm, h);
    });
    return { calls };
  }

  /** Loads a dump: the open calls it recreated and the handles it named (the caller frees them). */
  private loadDump(dump: string): { calls: OpenCall[]; handles: Map<string, number> } {
    if (this.active !== null) throw new SandboxError("misuse", "a run is in progress");
    const plain = plainDump(dump);
    if (!plain) throw new SandboxError("misuse", 'cannot load the dump: not a djs dump (no "djs-dump"; header)');
    return this.loadPlain(dump, plain.typescript);
  }

  /**
   * Loads a plain dump (D2830): runs it as a Script with the hooks in `$djs` (D2890), to
   * its first stop that is not a sync host call (idle, or done); the hooks are served only
   * meanwhile. The dump deletes `$djs` at the end of its Script.
   * `$djs.resume(id, name, args, state)` opens the dump's call `id` of host function
   * `name` again under its id (`vm_open_call`): one of the calls returned, with its state;
   * `$djs.seed(lo, hi, next)` sets `Math.random`'s state and, once the dump has loaded,
   * the id of the next call (D2897). Other sync host functions are called
   * (they must return synchronously); async calls the Script makes are new calls, which
   * the run that follows reports. The Script's completion value, when a promise, is the
   * handle "run". The dumper is not needed: a plain dump is code.
   */
  private loadPlain(dump: string, typescript: boolean): { calls: OpenCall[]; handles: Map<string, number> } {
    const x = this.x;
    const vm = this.vm;
    if (!x.vm_open_call || !x.vm_dump_runtime) {
      throw new SandboxError("misuse", "the djs-wasm module cannot load plain dumps (no vm_open_call or vm_dump_runtime): rebuild it");
    }
    const dumpRuntime = x.vm_dump_runtime;
    const calls: OpenCall[] = [];
    this.guard(() => {
      // `$djs`: the dump runtime's helpers (D2903) with the hooks added.
      const hooks = this.check(dumpRuntime(vm, this.stageText(dumpRuntimeScript)), "memory");
      for (const [name, id] of [["resume", HOOK_RESUME], ["seed", HOOK_SEED]] as const) {
        const f = this.check(x.vm_new_host_function(vm, id, this.stageText(name)), "memory");
        this.defineOn(hooks, name, f);
        x.vm_handle_free(vm, f);
      }
      const g = x.vm_global(vm);
      this.defineOn(g, PLAIN_HOOKS, hooks);
      x.vm_handle_free(vm, g);
      x.vm_handle_free(vm, hooks);
    });
    let st = STATUS_DONE;
    // `$djs.seed`'s next call id: the counter goes on from the dumped VM's (D2897).
    let next: number | undefined;
    this.enter(dump, {}, true, "dump.js", typescript);
    const slice = this.opts.fuelSlice ?? 1_000_000;
    const maxFuel = this.opts.maxFuel ?? Infinity;
    let used = 0;
    for (;;) {
      st = this.guard(() => {
        this.setClock();
        return x.vm_run(vm, slice);
      });
      used += x.vm_fuel_used ? x.vm_fuel_used(vm) : st === STATUS_OUT_OF_FUEL ? slice : 0;
      if (used >= maxFuel) {
        this.guard(() => x.vm_abort(vm));
        throw new SandboxError("fuel", `out of fuel after ${used} units while loading the dump`);
      }
      if (st === STATUS_OUT_OF_FUEL) continue;
      if (st !== STATUS_HOST_CALL) break;
      const { callId, fnId, args } = this.guard(() => this.readCall(x.vm_host_call(vm)));
      if (fnId === HOOK_RESUME) {
        const [id, name, callArgs, state] = args;
        const f = typeof name === "string" ? this.fnNames.indexOf(name) + 1 : 0;
        if (f === 0) {
          this.complete(callId, false, new Error(`resume: unknown host function ${String(name)}`), false);
          continue;
        }
        if (this.guard(() => x.vm_open_call!(vm, callId, f)) !== 0) {
          // (Out of memory may come after the parked call returned: then this is a no-op.)
          this.complete(callId, false, new Error(outText(x)), false);
          continue;
        }
        const call: OpenCall = { id: id as number, fn: name as string, args: Array.isArray(callArgs) ? callArgs : [] };
        if (state !== undefined) call.state = state;
        calls.push(call);
        continue;
      }
      let ok = true;
      let result: unknown;
      if (fnId === HOOK_SEED) {
        const [lo, hi, n] = args;
        if (typeof lo === "number" && typeof hi === "number") this.guard(() => x.vm_set_math_random_seed(vm, lo >>> 0, hi >>> 0));
        else [ok, result] = [false, new TypeError("seed: not two int32 halves")];
        if (typeof n === "number") next = n;
      } else {
        try {
          result = this.invoke(fnId, args);
          if (result instanceof Promise) throw new TypeError(`host function ${this.fnNames[fnId - 1]} returned a promise while a dump loads`);
        } catch (e) {
          if (e instanceof SandboxError && e.kind === "trap") throw e;
          [ok, result] = [false, e];
        }
      }
      this.complete(callId, ok, result, false);
    }
    if (next !== undefined && x.vm_set_next_call_id) {
      const last = Math.max(0, ...calls.map((c) => c.id));
      this.guard(() => x.vm_set_next_call_id!(vm, Math.max(next!, last + 1)));
    }
    return this.guard(() => {
      if (st === STATUS_THREW || st === STATUS_HOST_OUT_OF_MEMORY) {
        const h = x.vm_result(vm);
        const why = st === STATUS_THREW ? `the dump threw: ${this.display(h)}` : "out of host memory";
        x.vm_handle_free(vm, h);
        throw new SandboxError("misuse", `cannot load the dump: ${why}`);
      }
      // The completion value: vm_result once done, else (idle) the entry call's outcome.
      const h = st === STATUS_DONE ? x.vm_result(vm) : x.vm_outcome(vm);
      const handles = new Map<string, number>();
      const ps = h === 0 ? 0 : x.vm_promise_state(vm, h);
      if (ps >= 2) x.vm_handle_free(vm, outWords(x, 1)[0]);
      if (ps !== 0) handles.set("run", h);
      else if (h !== 0) x.vm_handle_free(vm, h);
      return { calls, handles };
    });
  }

  /**
   * Like `run`, but awaits host functions that return promises: a sync one keeps the VM
   * parked meanwhile, async ones run concurrently while the script goes on.
   */
  async runAsync(code: string, params: Record<string, unknown> = {}, opts: RunOptions = {}): Promise<unknown> {
    this.beginJournal(code, params, opts);
    this.enter(code, params);
    try {
      const it = this.auto(true);
      let r = it.next();
      while (!r.done) r = it.next(await r.value);
      return r.value;
    } catch (e) {
      this.abandon(e);
      throw e;
    } finally {
      this.endRun();
      this.journal = null;
    }
  }

  /**
   * Replays a recorded run (D1730): runs the journal's code and parameters, answers every
   * host call from the journal instead of calling the host functions (console output
   * still goes to the console), delivers the async results in the recorded order with
   * their clocks, and checks the calls the script makes and the canonical hashes at the
   * start, at every recorded idle point and at the end. The sandbox must be deterministic
   * and in the state the recording one was in (a fresh one created the same way, given the
   * same runs before); the fuel slice may differ.
   * Returns the run's result; a mismatch is a `diverged` SandboxError.
   */
  async replay(journal: JournalEntry[]): Promise<unknown> {
    const head = journal[0];
    if (head?.type !== "run") throw new SandboxError("misuse", "a journal starts with a run entry");
    if (this.opts.deterministic === undefined) throw new SandboxError("misuse", "replay needs a deterministic sandbox");
    const j = new Journal("replay", journal);
    j.expect(head.engine === this.buildHash(), () => `engine ${this.buildHash()}, the journal was made by ${head.engine}`);
    j.expect(head.hash === this.canonicalHash(), () => "the sandbox's state before the run differs from the recording's");
    j.next = 1;
    this.enter(head.code, head.params);
    this.journal = j;
    try {
      const it = this.auto(true);
      let r = it.next();
      while (!r.done) r = it.next(await r.value);
      return r.value;
    } catch (e) {
      this.abandon(e);
      throw e;
    } finally {
      this.endRun();
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
    opts.journal.push({ type: "run", engine: this.buildHash(), code, params, hash: this.canonicalHash() });
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

  /** Collects garbage now. False if the heap could not reserve the to-space. */
  collect(): boolean {
    return this.guard(() => this.x.vm_collect(this.vm) === 0);
  }

  /** The arena's current size in bytes. */
  get heapSize(): number {
    this.alive();
    return this.x.vm_arena_len(this.vm);
  }

  /** Releases the VM. The sandbox cannot be used afterwards. A task in progress ends (`misuse`). */
  dispose(): void {
    this.cancelTask?.();
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

  private stageText(s: string): number {
    const b = textEncoder.encode(s);
    this.stage(b);
    return b.length;
  }

  private hostFunction(name: string, f: HostFunction | AsyncHostFunction, async = false): number {
    let id = this.fnNames.indexOf(name) + 1;
    if (id === 0) {
      this.fnNames.push(name);
      id = this.fnNames.length;
    }
    if (typeof f === "function") {
      this.fns.set(id, f);
      this.hooks.delete(id);
    } else {
      if (!async || typeof f?.call !== "function") throw new SandboxError("misuse", `host function ${name}: not a function or { call, save, resume }`);
      this.fns.set(id, f.call);
      this.hooks.set(id, f);
    }
    const shortName = name.slice(name.lastIndexOf(".") + 1);
    const len = this.stageText(shortName);
    const h = async
      ? this.x.vm_new_async_host_function(this.vm, id, len)
      : this.x.vm_new_host_function(this.vm, id, len);
    return this.check(h, "memory");
  }

  private defineOn(obj: number, name: string, value: number): void {
    if (this.x.vm_define(this.vm, obj, this.stageText(name), value) !== 0) {
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
  private enter(code: string, params: Record<string, unknown>, script = false, filename?: string, typescript = false): void {
    if (this.active !== null) throw new SandboxError("misuse", "a run is in progress");
    this.guard(() => {
      const x = this.x;
      const vm = this.vm;
      const names = Object.keys(params);
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
      const goal = script
        ? typescript ? 2 : 0
        : 1 |
        (o.typescript ? 2 : 0) |
        (o.experimentalDecorators || o.emitDecoratorMetadata ? 4 : 0) |
        (o.emitDecoratorMetadata ? 8 : 0);
      const f = x.vm_compile_file(vm, goal, name.length, head.length, src.length);
      if (f === 0) {
        const msg = outText(x);
        for (const h of handles) if (h !== 0) x.vm_handle_free(vm, h);
        throw new SandboxError(msg.startsWith("SyntaxError") ? "syntax" : "memory", msg);
      }
      const args = new Uint8Array(4 * handles.length);
      const dv = new DataView(args.buffer);
      handles.forEach((h, i) => dv.setUint32(4 * i, h, true));
      this.stage(args);
      const r = x.vm_call(vm, f, 0, handles.length);
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
    for (;;) {
      run.beforeSlice?.();
      const st = this.guard(() => {
        this.setClock();
        return x.vm_run(vm, slice);
      });
      // Counted across sync host calls too, which end a slice early (D2653, D2702).
      run.used += x.vm_fuel_used ? x.vm_fuel_used(vm) : st === STATUS_OUT_OF_FUEL ? slice : 0;
      for (const callId of this.newHostCalls()) {
        const { fnId, args } = this.callInfo(callId);
        const call = { id: callId, fn: this.fnNames[fnId - 1], args };
        run.open.set(callId, call);
        if (j) j.issued++;
        run.onCall?.(call, fnId);
      }
      if (st === STATUS_DONE || st === STATUS_THREW) {
        const outcome = this.guard(() => {
          let h = x.vm_result(vm);
          if (run.result !== 0) {
            // A resumed run's result promise: its entry call was the dump's Script.
            x.vm_handle_free(vm, h);
            h = run.result;
            run.result = 0;
          }
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
          const value = this.readHandle(h);
          const message = threw ? this.display(h) : "";
          x.vm_handle_free(vm, h);
          return { value, message, threw };
        });
        const { value, message, threw } = outcome;
        if (j) {
          const end = { type: "end" as const, ok: !threw, value: threw ? message : value };
          j.finish(end, this.canonicalHash());
        }
        if (threw) throw new SandboxError("threw", message, value);
        this.checkRejections();
        return value;
      }
      if (st === STATUS_OUT_OF_FUEL) {
        if (run.used >= maxFuel) {
          // `run` abandons the call (vm_abort), so the sandbox stays usable.
          throw new SandboxError("fuel", `out of fuel after ${run.used} units`);
        }
        continue;
      }
      if (st === STATUS_HOST_OUT_OF_MEMORY) {
        // The host (wasm memory) could not back the heap below its cap: not a script
        // error. The VM abandoned the call and is usable again (D1690).
        throw new SandboxError("memory", "out of host memory");
      }
      if (st === STATUS_IDLE) {
        this.checkRejections();
        return IDLE;
      }
      if (st !== STATUS_HOST_CALL) return undefined;
      if (run.used >= maxFuel) throw new SandboxError("fuel", `out of fuel after ${run.used} units`);
      const { callId, fnId, args } = this.guard(() => {
        const argc = x.vm_host_call(vm);
        return this.readCall(argc);
      });
      let ok = true;
      let result: unknown;
      const fn = this.fnNames[fnId - 1];
      const journaled = j !== null && !fn?.startsWith("console.");
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
      this.complete(callId, ok, result, false);
    }
  }

  /**
   * The unhandled rejections collected so far (`unhandledRejections: "collect"`), as host
   * values; forgets them.
   */
  takeUnhandledRejections(): unknown[] {
    return this.rejections.splice(0);
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
    for (const r of reasons) {
      if (mode === "collect") this.rejections.push(r.value);
      else this.cons.warn("Possibly unhandled promise rejection:", r.message);
    }
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

  /** Calls host function `fnId` with arguments read from the VM; `call` is `this` for hooks. */
  private invoke(fnId: number, args: unknown[], call?: HostCall): unknown {
    const f = this.fns.get(fnId);
    if (f === undefined) throw new TypeError(`unknown host function ${fnId}`);
    if (this.hooks.has(fnId)) return f.apply(call ?? newCall(fnId, this.fnNames[fnId - 1], args).call, args);
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
  private callInfo(callId: number): { fnId: number; args: unknown[] } {
    return this.guard(() => this.readCall(this.x.vm_host_call_info(this.vm, callId)));
  }

  /** Reads `[call_id, fn_id, this, args...]` from the out buffer. */
  private readCall(argc: number): { callId: number; fnId: number; args: unknown[] } {
    const x = this.x;
    const w = outWords(x, 4 + 2 * argc);
    const args: unknown[] = [];
    for (let i = 0; i < argc; i++) args.push(this.read(w[4 + 2 * i], w[5 + 2 * i]));
    return { callId: w[0], fnId: w[1], args };
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
        return this.check(x.vm_new_error(vm, kind, msg), "memory");
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

const WEAK_REF_MODES = { collected: 0, strong: 1, absent: 2 } as const;

/** A call's hook view (`this` of `call`, the argument of `save` and `resume`) and its abort. */
function newCall(id: number, fn: string, args: unknown[], state?: unknown): LiveCall {
  const abort = new AbortController();
  return { call: { id, fn, args, startedAt: Date.now(), state, signal: abort.signal }, abort };
}

/** A call's state as the JSON text a dump carries (D2770). */
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
  return v instanceof Error ? { name: v.name, message: v.message } : { name: "Error", message: String(v) };
}

/** The host error a journal entry stands for, of the same kind. */
function errorFromJson(v: unknown): Error {
  const { name, message } = (v ?? {}) as { name?: string; message?: string };
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

/** A seed from the host's entropy (never 0, which would mean the VM's fixed default). */
function entropy(): [number, number] {
  const words = new Uint32Array(2);
  globalThis.crypto.getRandomValues(words);
  return [words[0], words[1] | 1];
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
