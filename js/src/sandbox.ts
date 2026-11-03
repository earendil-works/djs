// The Sandbox API: loads djs-wasm (docs/host.md) and runs code in a djs VM. Values go in
// through the wire format (wire.ts) and come out through direct heap reads (reader.ts).

import { IN_PLACE_MIN, encode, stringInPlace } from "./wire.ts";
import { trapText } from "./trap.ts";
import { Opaque, readValue } from "./reader.ts";
import {
  APP_SANDBOX,
  baseReference,
  checkSnapshot,
  encodeSnapshot,
  provideBase,
  readSnapshotHeader,
  REFERENCE_BASE,
  rememberBase,
  type SnapshotCodec,
  SnapshotError,
  snapshotImage,
} from "./snapshot.ts";

export { DEFAULT_ZSTD_LEVEL, nodeZstd, type SnapshotCodec, SnapshotError } from "./snapshot.ts";

/** A host function callable from the sandbox. Arguments and the result are host values. */
export type HostFunction = (...args: unknown[]) => unknown;

export interface ConsoleLike {
  log(...args: unknown[]): void;
  info(...args: unknown[]): void;
  warn(...args: unknown[]): void;
  error(...args: unknown[]): void;
}

export interface SandboxOptions {
  /** The compiled djs-wasm module; default: `loadModule()`. */
  module?: WebAssembly.Module;
  /** Heap cap in bytes; default: the engine's (4 GiB). */
  memoryLimit?: number;
  /** Fuel per `vm_run` call: how often the host gets control back. Default 1,000,000. */
  fuelSlice?: number;
  /**
   * Fuel budget of one `run`, after which it fails with "out of fuel" (the call is
   * abandoned; the sandbox stays usable). Default: none.
   */
  maxFuel?: number;
  /** Global host functions, by name. A call parks the VM until the function returns. */
  functions?: Record<string, HostFunction>;
  /**
   * Global async host functions, by name (D1374): a call returns a promise in the VM at
   * once and the script goes on, so several calls can be outstanding (`Promise.all`
   * over tools). The function may return a value or a promise.
   */
  asyncFunctions?: Record<string, HostFunction>;
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
  /**
   * zstd for snapshots (`bytes()`, `fromBytes()`): node's built-in one by default
   * (`nodeZstd()`). `null` writes uncompressed snapshots; a browser can pass a zstd built
   * for wasm (D2260).
   */
  snapshotCodec?: SnapshotCodec | null;
}

/** Options of `Sandbox.bytes()`. */
export interface BytesOptions {
  /**
   * `"zstd"` (default when a codec is available): compressed against the module's base
   * image; `"none"`: the collected image as is.
   */
  compression?: "zstd" | "none";
  /** zstd level, default `DEFAULT_ZSTD_LEVEL` (3). */
  level?: number;
}

export type SandboxErrorKind = "threw" | "syntax" | "fuel" | "trap" | "memory" | "misuse" | "diverged";

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
}

/** A host call's outcome for `Run.settle`: a value, or (`ok` false) an error to reject with. */
export interface Settlement {
  ok: boolean;
  value: unknown;
}

/** What `Sandbox.load` recreated: the open host calls of a format 2 dump (0171). */
export interface LoadedDump {
  calls: OpenCall[];
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
const ABI_VERSION = 2;
/** `vm_new_with`'s flag: frozen builtins (D2304). */
const VM_FROZEN_BUILTINS = 1;
const CONSOLE_METHODS = ["log", "info", "warn", "error"] as const;

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
  vm_dump?(vm: number, len: number): number;
  vm_load_dump?(vm: number, namesLen: number, len: number): number;
  /** Since 0179. */
  vm_outcome?(vm: number): number;
  vm_from_bytes(len: number, memoryLimit: number): number;
  vm_free(vm: number): void;
  vm_arena_ptr(vm: number): number;
  vm_arena_len(vm: number): number;
  vm_collect(vm: number): number;
  vm_compile(vm: number, goal: number, paramsLen: number, srcLen: number): number;
  vm_global(vm: number): number;
  vm_new_object(vm: number): number;
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
  vm_set_weak_refs(vm: number, mode: number): number;
  vm_resolve_host_call_at(vm: number, callId: number, ok: number, value: number, now: number): number;
  vm_canonical_hash(vm: number): number;
  djs_build_hash(): number;
  /** The base snapshot (D2260); the inject exports only with the base coded (D2252). */
  djs_base_ptr(): number;
  djs_base_len(): number;
  djs_base_inject_ptr?(len: number): number;
  djs_base_inject?(): number;
  djs_base_coded_ptr?(len: number): number;
}

const modules = new Map<string, Promise<WebAssembly.Module>>();
/** The default module once loaded, for the synchronous constructor. */
let loadedDefault: WebAssembly.Module | undefined;

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
/** The metadata and image of a sandbox snapshot container. */
function unpackSnapshot(
  snapshot: Uint8Array,
  module: WebAssembly.Module,
  x: Exports,
  codec: SnapshotCodec | null | undefined,
): { meta: { functions: string[]; clock?: number }; image: Uint8Array } {
  try {
    const header = readSnapshotHeader(snapshot);
    // The base completes the session image (D2066), and it is the only reference; its
    // roots hold the module's dependency descriptor (D2265).
    const base = baseReference(module, x);
    checkSnapshot(header, APP_SANDBOX, base);
    const reference = header.referenceKind === REFERENCE_BASE ? base : undefined;
    return { meta: header.meta as { functions: string[]; clock?: number }, image: snapshotImage(header, reference, codec, base) };
  } catch (e) {
    if (e instanceof SnapshotError) throw new SandboxError("misuse", e.message);
    throw e;
  }
}

/** What a run in progress needs from its sandbox. */
interface RunCore {
  /** The open async calls: the host settles them (the run never calls them itself). */
  open: Map<number, OpenCall>;
  /** A handle to the run's result promise once known (0: not yet), freed at the end. */
  result: number;
  advance(): Promise<RunEnd>;
  settle(id: number, s: Settlement): void;
  dump(): string;
}

type RunEnd = { state: "idle" } | { state: "done"; value: unknown } | { state: "threw"; error: SandboxError };

/**
 * A run the host drives itself (docs/continuation-dumps.md "Sandbox API"; the part 0179
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
   * The waiting run as a format 2 dump (docs/continuation-dumps.md): its suspended
   * activations, open calls and data as JavaScript, for `Sandbox.resume` in another
   * sandbox or process. Only while idle.
   */
  dump(): string {
    if (this.state !== "idle") throw new SandboxError("misuse", `the run is ${this.state}: only an idle run can be dumped`);
    return this.#core.dump();
  }

  /** Records how a step ended. */
  end(e: RunEnd): void {
    this.state = e.state;
    if (e.state === "done") this.value = e.value;
    if (e.state === "threw") this.error = e.error;
  }
}

/** `drive`'s value at an idle point of a host-driven run. */
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
  private readonly rejections: unknown[] = [];
  private readonly cons: ConsoleLike;
  private dead: string | null = null;
  /** Deterministic mode: the VM's clock, moved only by async results (never backwards). */
  private clock: number | undefined;
  /** The journal of the current run, being recorded or replayed (D1730). */
  private journal: Journal | null = null;
  /** The host-driven run in progress (`begin`, `resume`), until it ends. */
  private active: { run: Run; core: RunCore } | null = null;

  /** A new sandbox; loads (and caches) the default module unless `opts.module` is given. */
  static async create(opts: SandboxOptions = {}): Promise<Sandbox> {
    return new Sandbox({ ...opts, module: opts.module ?? (await loadModule()) });
  }

  /**
   * A sandbox restored from `bytes()`. Host functions are matched by name; `opts.functions`
   * must provide every function the snapshot's VM refers to.
   */
  static async fromBytes(snapshot: Uint8Array, opts: SandboxOptions = {}): Promise<Sandbox> {
    return new Sandbox({ ...opts, module: opts.module ?? (await loadModule()) }, snapshot);
  }

  /**
   * Continues a run from its format 2 dump (`Run.dump()`) in a new sandbox: the sandbox
   * is made from `opts` (the same host functions, by name, as the one that dumped) and
   * loads the dump. The run is idle, waiting on the same open calls with the same ids:
   * the host settles them, issues them again or rejects them. A dump is code: resume only
   * dumps you trust.
   */
  static async resume(dump: string, opts: SandboxOptions = {}): Promise<{ sandbox: Sandbox; run: Run }> {
    const sandbox = await Sandbox.create(opts);
    try {
      const { calls, handles } = sandbox.loadDump(dump);
      const result = handles.get("run") ?? 0;
      for (const [name, h] of handles) if (name !== "run") sandbox.x.vm_handle_free(sandbox.vm, h);
      if (result === 0) throw new SandboxError("misuse", "the dump holds no run (no handle named run)");
      const run = sandbox.startRun(calls, result);
      return { sandbox, run };
    } catch (e) {
      sandbox.dispose();
      throw e;
    }
  }

  /**
   * A new sandbox (its own wasm instance and VM), synchronously: needs a compiled module,
   * `opts.module` or one `loadModule()` has finished loading. With `snapshot`, restores
   * it like `fromBytes`.
   */
  constructor(opts: SandboxOptions = {}, snapshot?: Uint8Array) {
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
    if (snapshot === undefined) {
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
      const { meta, image } = unpackSnapshot(snapshot, module, x, opts.snapshotCodec);
      restoredFns = meta.functions;
      if (opts.deterministic) this.clock = meta.clock ?? opts.deterministic.startedAt;
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
    if (snapshot !== undefined) {
      // The VM already has its host functions; bind the ids to the given implementations.
      restoredFns.forEach((name, i) => {
        const f = builtins[name] ?? opts.functions?.[name] ?? opts.asyncFunctions?.[name];
        if (f === undefined) throw new SandboxError("misuse", `snapshot needs host function ${name}`);
        this.fnNames.push(name);
        this.fns.set(i + 1, f);
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
  defineAsyncFunction(name: string, f: HostFunction): void {
    this.defineGlobalFunction(name, f, true);
  }

  private defineGlobalFunction(name: string, f: HostFunction, async: boolean): void {
    this.guard(() => {
      const h = this.hostFunction(name, f, async);
      const g = this.x.vm_global(this.vm);
      this.defineOn(g, name, h);
      this.x.vm_handle_free(this.vm, g);
      this.x.vm_handle_free(this.vm, h);
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
    this.start(code, params);
    try {
      // In sync mode the loop never yields: a promise from a host function is an error.
      return this.drive(false).next().value;
    } catch (e) {
      this.abandon(e);
      throw e;
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
  begin(code: string, params: Record<string, unknown> = {}): Run {
    this.journal = null;
    this.start(code, params);
    const run = this.startRun([], 0);
    try {
      run.end(this.step(this.active!.core, false).next().value as RunEnd);
    } catch (e) {
      this.endRun();
      this.abandon(e);
      throw e;
    }
    if (run.state !== "idle") this.endRun();
    return run;
  }

  /** Makes the host-driven run, waiting on `calls`, with its result promise `result`. */
  private startRun(calls: OpenCall[], result: number): Run {
    const core: RunCore = {
      open: new Map(calls.map((c) => [c.id, c])),
      result,
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
        if (this.active?.core !== core) throw new SandboxError("misuse", "the run has ended");
        if (!core.open.delete(id)) throw new SandboxError("misuse", `no open call ${id}`);
        this.complete(id, s.ok, s.value, true);
      },
      dump: () => {
        if (this.active?.core !== core) throw new SandboxError("misuse", "the run has ended");
        return this.dumpRun(core);
      },
    };
    const run = new Run(core);
    this.active = { run, core };
    return run;
  }

  /** One step of a host-driven run: `drive` until idle or the end. */
  private *step(core: RunCore, async: boolean): Generator<Promise<unknown>, RunEnd, unknown> {
    try {
      const r = yield* this.drive(async, core);
      return r === IDLE ? { state: "idle" } : { state: "done", value: r };
    } catch (e) {
      if (e instanceof SandboxError && e.kind === "threw") return { state: "threw", error: e };
      throw e;
    }
  }

  /** The run has ended: frees its result handle; the sandbox can run again. */
  private endRun(): void {
    const a = this.active;
    this.active = null;
    if (a !== null && a.core.result !== 0 && this.dead === null) {
      this.x.vm_handle_free(this.vm, a.core.result);
      a.core.result = 0;
    }
  }

  /** `Run.dump()`: the waiting run as a format 2 dump, its result promise named "run". */
  private dumpRun(core: RunCore): string {
    return this.dumpText([], [["run", core.result]]);
  }

  /**
   * The VM's global bindings (or the ones in `names`) as JavaScript that recreates them
   * when `load`ed into another sandbox, also of a later djs (docs/functions-as-code.md):
   * functions as their source text, closures and data as code, builtins by name. The
   * host's functions and `console` are not written: the loading sandbox provides them
   * under the same names. Throws a "misuse" error listing every value that cannot be
   * written, by its path (a pending promise, a Proxy, a class, ...).
   */
  dump(names: string[] = []): string {
    return this.dumpText(names, []);
  }

  /**
   * `vm_dump`. staging: the globals the host defines and the names to dump (comma
   * separated, a line each), then this sandbox's host function names (tab separated, id
   * order: an open call or a host function held as a value is written by name), then
   * the values to name for the host (`handle:name`, tab separated).
   */
  private dumpText(names: string[], handles: [string, number][]): string {
    const x = this.x;
    if (!x.vm_dump) throw new SandboxError("misuse", "the djs-wasm module has no dump (vm_dump)");
    const host = new Set(["console"]);
    for (const n of this.fnNames) host.add(n.split(".")[0]);
    const text = [
      [...host].join(","),
      names.join(","),
      this.fnNames.join("\t"),
      handles.map(([name, h]) => `${h}:${name}`).join("\t"),
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
   * Loads a dump (the text of `dump()`) with the module's loader (`vm_load_dump`,
   * `Vm::load_dump`): its bindings become globals of this sandbox. Format 2 dumps get the
   * loader's privileged `$dump`; version 1 dumps run as plain Scripts. A dump is code:
   * load only dumps you trust, into a fresh sandbox. Not recorded in a journal. Returns
   * the open host calls a format 2 dump recreated (its open calls name this sandbox's
   * host functions); `Sandbox.resume` continues a dumped run.
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
    const x = this.x;
    // Format 2 starts with the directive `"djs-dump 2";` (D2640), version 1 with a comment.
    if (!dump.startsWith('"djs-dump ') && !dump.startsWith("// djs-dump ")) {
      throw new SandboxError("misuse", "not a djs dump");
    }
    if (!x.vm_load_dump) {
      throw new SandboxError("misuse", "the djs-wasm module has no dump loader (vm_load_dump)");
    }
    if (this.active !== null) throw new SandboxError("misuse", "a run is in progress");
    // The loader names open calls' functions by the host's names (`$dump.pending(1,
    // "sleep", [5000])`); this sandbox's ids are their places in the table.
    const names = textEncoder.encode(this.fnNames.join("\n"));
    const text = textEncoder.encode(dump);
    const both = new Uint8Array(names.length + text.length);
    both.set(names);
    both.set(text, names.length);
    this.stage(both);
    const r = this.guard(() => x.vm_load_dump!(this.vm, names.length, text.length));
    if (r !== 0) throw new SandboxError("misuse", `cannot load the dump: ${outText(x)}`);
    const [nCalls, nHandles] = outWords(x, 2);
    const w = outWords(x, 2 + 2 * nCalls + 2 * nHandles);
    const bytes = new Uint8Array(x.memory.buffer, x.djs_out_ptr(), x.djs_out_len());
    let at = 4 * w.length;
    const handles = new Map<string, number>();
    for (let i = 0; i < nHandles; i++) {
      const [h, len] = [w[2 + 2 * nCalls + 2 * i], w[3 + 2 * nCalls + 2 * i]];
      handles.set(textDecoder.decode(bytes.subarray(at, at + len)), h);
      at += len;
    }
    const ids = Array.from({ length: nCalls }, (_, i) => w[2 + 2 * i]);
    const calls = ids.map((id) => {
      const { fnId, args } = this.callInfo(id);
      return { id, fn: this.fnNames[fnId - 1], args };
    });
    return { calls, handles };
  }

  /**
   * Like `run`, but awaits host functions that return promises: a sync one keeps the VM
   * parked meanwhile, async ones run concurrently while the script goes on.
   */
  async runAsync(code: string, params: Record<string, unknown> = {}, opts: RunOptions = {}): Promise<unknown> {
    this.beginJournal(code, params, opts);
    this.start(code, params);
    try {
      const it = this.drive(true);
      let r = it.next();
      while (!r.done) r = it.next(await r.value);
      return r.value;
    } catch (e) {
      this.abandon(e);
      throw e;
    }
  }

  /**
   * Replays a recorded run (D1730): runs the journal's code and parameters, answers every
   * host call from the journal instead of calling the host functions (console output
   * still goes to the console), delivers the async results in the recorded order with
   * their clocks, and checks the calls the script makes and the canonical hashes at the
   * start, at every recorded idle point and at the end. The sandbox must be deterministic
   * and in the state the recording one was in (a fresh one created the same way, or one
   * restored from a snapshot taken right before the run); the fuel slice may differ.
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
    this.journal = j;
    this.start(head.code, head.params);
    try {
      const it = this.drive(true);
      let r = it.next();
      while (!r.done) r = it.next(await r.value);
      return r.value;
    } catch (e) {
      this.abandon(e);
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
   * The sandbox as bytes, for `Sandbox.fromBytes` (D2260): the VM image, collected first,
   * zstd-compressed against the module's base image, in a container that names what it
   * depends on (D2265), plus the host function table. Restores into a djs module with the
   * same dependency descriptor: the same cell layout, features, tables and builtins.
   */
  bytes(opts: BytesOptions = {}): Uint8Array {
    this.alive();
    // Only live cells (as pi's adapter, D1890): the sandbox itself collects, so going on
    // with it and restoring the bytes continue from the same heap. A collection that cannot
    // get memory leaves the heap as it was; the snapshot is then just larger.
    this.guard(() => this.x.vm_collect(this.vm));
    return encodeSnapshot({
      app: APP_SANDBOX,
      meta: { functions: this.fnNames, clock: this.clock },
      image: this.arena(),
      reference: baseReference(this.module, this.x),
      // Only the sandbox's own part of the heap (D2066).
      base: baseReference(this.module, this.x),
      codec: opts.compression === "none" ? null : this.opts.snapshotCodec,
      level: opts.level,
    });
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

  private stageText(s: string): number {
    const b = textEncoder.encode(s);
    this.stage(b);
    return b.length;
  }

  private hostFunction(name: string, f: HostFunction, async = false): number {
    let id = this.fnNames.indexOf(name) + 1;
    if (id === 0) {
      this.fnNames.push(name);
      id = this.fnNames.length;
    }
    this.fns.set(id, f);
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
  private start(code: string, params: Record<string, unknown>, script = false): void {
    if (this.active !== null) throw new SandboxError("misuse", "a run is in progress (Sandbox.begin)");
    this.guard(() => {
      const x = this.x;
      const vm = this.vm;
      const names = Object.keys(params);
      const handles = Object.values(params).map((v) => this.toHandle(v));
      const head = textEncoder.encode(names.join("\n"));
      const src = textEncoder.encode(code);
      const both = new Uint8Array(head.length + src.length);
      both.set(head);
      both.set(src, head.length);
      this.stage(both);
      const o = this.opts;
      const goal = script
        ? 0
        : 1 |
        (o.typescript ? 2 : 0) |
        (o.experimentalDecorators || o.emitDecoratorMetadata ? 4 : 0) |
        (o.emitDecoratorMetadata ? 8 : 0);
      const f = x.vm_compile(vm, goal, head.length, src.length);
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
   * The run loop, as a generator: yields a promise when the host must wait (a sync host
   * function's promise, or until one of the outstanding async calls settles; only when
   * `async`), returns the result.
   */
  private *drive(async: boolean, run?: RunCore): Generator<Promise<unknown>, unknown, unknown> {
    const x = this.x;
    const vm = this.vm;
    const slice = this.opts.fuelSlice ?? 1_000_000;
    const maxFuel = this.opts.maxFuel ?? Infinity;
    let used = 0;
    // Async host calls being served, and those that finished but are not passed back yet.
    const inflight = new Map<number, Promise<void>>();
    const finished: { callId: number; ok: boolean; value: unknown }[] = [];
    const j = this.journal;
    // Async calls made and not resolved yet, for the journal.
    const calls = new Map<number, { fn: string; args: unknown[] }>();
    for (;;) {
      for (const { callId, ok, value } of finished.splice(0)) {
        inflight.delete(callId);
        const call = calls.get(callId);
        calls.delete(callId);
        const now = this.complete(callId, ok, value, true);
        if (j) j.delivered++;
        if (j && call) j.push({ type: "async", callId, fn: call.fn, args: call.args, ok, value: errorJson(ok, value), now: now! });
      }
      const st = this.guard(() => {
        this.setClock();
        return x.vm_run(vm, slice);
      });
      for (const callId of this.newHostCalls()) {
        const { fnId, args } = this.callInfo(callId);
        const fn = this.fnNames[fnId - 1];
        if (run) {
          // A host-driven run: the host answers it (`Run.settle`).
          run.open.set(callId, { id: callId, fn, args });
          continue;
        }
        if (j) {
          j.issued++;
          calls.set(callId, { fn, args });
          // Replay: the result comes from the journal, at an idle point.
          if (j.mode === "replay") continue;
        }
        let p: Promise<unknown>;
        try {
          const r = this.invoke(fnId, args);
          if (!(r instanceof Promise)) {
            finished.push({ callId, ok: true, value: r });
            continue;
          }
          if (!async) throw new TypeError(`host function ${this.fnNames[fnId - 1]} returned a promise; use runAsync`);
          p = r;
        } catch (e) {
          finished.push({ callId, ok: false, value: e });
          continue;
        }
        inflight.set(
          callId,
          p.then(
            (value) => void finished.push({ callId, ok: true, value }),
            (value) => void finished.push({ callId, ok: false, value }),
          ),
        );
      }
      if (st === STATUS_DONE || st === STATUS_THREW) {
        const outcome = this.guard(() => {
          let h = x.vm_result(vm);
          if (run && run.result !== 0) {
            // The run's result promise, taken at an idle point (or named by its dump): a
            // loaded run's entry call was the dump's Script.
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
        used += slice;
        if (used >= maxFuel) {
          // `run` abandons the call (vm_abort), so the sandbox stays usable.
          throw new SandboxError("fuel", `out of fuel after ${used} units`);
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
        if (run) {
          if (run.result === 0) run.result = this.guard(() => this.outcomeHandle());
          return IDLE;
        }
        if (j?.mode === "record") {
          j.push({ type: "check", delivered: j.delivered, issued: j.issued, hash: this.canonicalHash() });
        } else if (j && calls.size > 0) {
          // Replay: the checks recorded at this point, then the next result.
          j.checkIdle(this.canonicalHash());
          const e = j.take("async");
          const call = calls.get(e.callId);
          j.expect(call !== undefined, () => `result ${j.delivered + 1} is for call ${e.callId}, which the script has not made`);
          j.expectCall(e.fn, e.args, call!.fn, call!.args);
          calls.delete(e.callId);
          this.complete(e.callId, e.ok, e.ok ? e.value : errorFromJson(e.value), true, e.now);
          j.delivered++;
          continue;
        }
        if (finished.length > 0) continue;
        if (inflight.size === 0) return undefined; // nothing was called
        // Only async host calls are left: wait for one of them.
        yield Promise.race(inflight.values());
        continue;
      }
      if (st !== STATUS_HOST_CALL) return undefined;
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

  /** A handle to the entry call's outcome (an async body's promise) at an idle point. */
  private outcomeHandle(): number {
    const x = this.x;
    if (!x.vm_outcome) throw new SandboxError("misuse", "the djs-wasm module has no vm_outcome");
    return this.check(x.vm_outcome(this.vm), "memory");
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
