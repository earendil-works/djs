// The Sandbox API: loads djs-wasm (docs/host.md) and runs code in a djs VM. Values go in
// through the wire format (wire.ts) and come out through direct heap reads (reader.ts).

import { encode } from "./wire.ts";
import { Opaque, readValue } from "./reader.ts";

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
  /** Fuel budget of one `run`, after which it fails with "out of fuel". Default: none. */
  maxFuel?: number;
  /** Global host functions, by name. */
  functions?: Record<string, HostFunction>;
  /** Receives `console.*` calls from the sandbox; default: the host's `console`. */
  console?: ConsoleLike;
}

export type SandboxErrorKind = "threw" | "syntax" | "fuel" | "trap" | "memory" | "misuse";

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
const ABI_VERSION = 1;
const CONSOLE_METHODS = ["log", "info", "warn", "error"] as const;

interface Exports {
  memory: WebAssembly.Memory;
  djs_abi_version(): number;
  djs_staging(len: number): number;
  djs_out_ptr(): number;
  djs_out_len(): number;
  djs_panic_ptr(): number;
  djs_panic_len(): number;
  djs_debug_panic(): void;
  vm_new(memoryLimit: number): number;
  vm_from_bytes(len: number, memoryLimit: number): number;
  vm_free(vm: number): void;
  vm_arena_ptr(vm: number): number;
  vm_arena_len(vm: number): number;
  vm_collect(vm: number): number;
  vm_compile(vm: number, goal: number, paramsLen: number, srcLen: number): number;
  vm_global(vm: number): number;
  vm_new_object(vm: number): number;
  vm_new_host_function(vm: number, fnId: number, nameLen: number): number;
  vm_define(vm: number, obj: number, nameLen: number, value: number): number;
  vm_decode_wire(vm: number, len: number): number;
  vm_new_error(vm: number, kind: number, msgLen: number): number;
  vm_handle_bits(vm: number, h: number): void;
  vm_handle_free(vm: number, h: number): void;
  vm_display(vm: number, h: number): number;
  vm_call(vm: number, f: number, thisArg: number, argc: number): number;
  vm_run(vm: number, fuel: number): number;
  vm_result(vm: number): number;
  vm_host_call(vm: number): number;
  vm_resolve_host_call(vm: number, callId: number, ok: number, value: number): number;
}

const modules = new Map<string, Promise<WebAssembly.Module>>();

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
    m.catch(() => modules.delete(key));
  }
  return m;
}

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();
const SNAPSHOT_MAGIC = 0x53534a44; // "DJSS"

export class Sandbox {
  private readonly x: Exports;
  private readonly vm: number;
  private readonly opts: SandboxOptions;
  /** Host function names by id - 1. */
  private readonly fnNames: string[] = [];
  private readonly fns = new Map<number, HostFunction>();
  private readonly atomCache: (string | undefined)[] = [];
  private dead: string | null = null;

  /** A new sandbox (its own wasm instance and VM). */
  static async create(opts: SandboxOptions = {}): Promise<Sandbox> {
    const module = opts.module ?? (await loadModule());
    const instance = await WebAssembly.instantiate(module, {});
    const x = instance.exports as unknown as Exports;
    const vm = x.vm_new(opts.memoryLimit ?? 0);
    if (vm === 0) throw new SandboxError("memory", outText(x));
    return new Sandbox(x, vm, opts, []);
  }

  /**
   * A sandbox restored from `bytes()`. Host functions are matched by name; `opts.functions`
   * must provide every function the snapshot's VM refers to.
   */
  static async fromBytes(snapshot: Uint8Array, opts: SandboxOptions = {}): Promise<Sandbox> {
    const view = new DataView(snapshot.buffer, snapshot.byteOffset, snapshot.byteLength);
    if (snapshot.length < 12 || view.getUint32(0, true) !== SNAPSHOT_MAGIC || view.getUint32(4, true) !== 1) {
      throw new SandboxError("misuse", "not a sandbox snapshot");
    }
    const metaLen = view.getUint32(8, true);
    const meta = JSON.parse(textDecoder.decode(snapshot.subarray(12, 12 + metaLen))) as { functions: string[] };
    const image = snapshot.subarray(12 + metaLen);
    const module = opts.module ?? (await loadModule());
    const instance = await WebAssembly.instantiate(module, {});
    const x = instance.exports as unknown as Exports;
    const at = x.djs_staging(image.length); // may grow memory: take the buffer afterwards
    new Uint8Array(x.memory.buffer).set(image, at);
    const vm = x.vm_from_bytes(image.length, opts.memoryLimit ?? 0);
    if (vm === 0) throw new SandboxError("misuse", outText(x));
    return new Sandbox(x, vm, opts, meta.functions);
  }

  private constructor(x: Exports, vm: number, opts: SandboxOptions, restoredFns: string[]) {
    if (x.djs_abi_version() !== ABI_VERSION) {
      throw new SandboxError("misuse", `djs-wasm ABI ${x.djs_abi_version()}, expected ${ABI_VERSION}`);
    }
    this.x = x;
    this.vm = vm;
    this.opts = opts;
    const cons = opts.console ?? (globalThis.console as ConsoleLike);
    const builtins: Record<string, HostFunction> = {};
    for (const m of CONSOLE_METHODS) builtins[`console.${m}`] = (...a) => cons[m](...a);
    if (restoredFns.length > 0) {
      // The VM already has its host functions; bind the ids to the given implementations.
      restoredFns.forEach((name, i) => {
        const f = builtins[name] ?? opts.functions?.[name];
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
    }
  }

  /** Defines a global host function. */
  defineFunction(name: string, f: HostFunction): void {
    this.guard(() => {
      const h = this.hostFunction(name, f);
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
   * `params` as its parameters (sent through the wire format). `console` is a global. Host functions must return synchronously; use
   * `runAsync` for ones that return promises.
   */
  run(code: string, params: Record<string, unknown> = {}): unknown {
    this.start(code, params);
    // In sync mode the loop never yields: a promise from a host function is an error.
    const r = this.drive(false).next();
    return r.value;
  }

  /** Like `run`, but awaits host functions that return promises (the VM stays parked). */
  async runAsync(code: string, params: Record<string, unknown> = {}): Promise<unknown> {
    this.start(code, params);
    const it = this.drive(true);
    let r = it.next();
    while (!r.done) r = it.next(await r.value);
    return r.value;
  }

  /** The VM image plus the host function table, for `Sandbox.fromBytes`. */
  bytes(): Uint8Array {
    this.alive();
    const meta = textEncoder.encode(JSON.stringify({ functions: this.fnNames }));
    const image = this.arena();
    const out = new Uint8Array(12 + meta.length + image.length);
    const view = new DataView(out.buffer);
    view.setUint32(0, SNAPSHOT_MAGIC, true);
    view.setUint32(4, 1, true);
    view.setUint32(8, meta.length, true);
    out.set(meta, 12);
    out.set(image, 12 + meta.length);
    return out;
  }

  /** The arena's current size in bytes. */
  get heapSize(): number {
    this.alive();
    return this.x.vm_arena_len(this.vm);
  }

  /** Releases the VM. The sandbox cannot be used afterwards. */
  dispose(): void {
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
        const x = this.x;
        const msg = textDecoder.decode(new Uint8Array(x.memory.buffer, x.djs_panic_ptr(), x.djs_panic_len()));
        this.dead = `trapped (${msg || e.message})`;
        throw new SandboxError("trap", `djs-wasm trapped: ${msg || e.message}`);
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

  private hostFunction(name: string, f: HostFunction): number {
    let id = this.fnNames.indexOf(name) + 1;
    if (id === 0) {
      this.fnNames.push(name);
      id = this.fnNames.length;
    }
    this.fns.set(id, f);
    const shortName = name.slice(name.lastIndexOf(".") + 1);
    return this.check(this.x.vm_new_host_function(this.vm, id, this.stageText(shortName)), "memory");
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
    const w = new Uint32Array(this.x.memory.buffer, this.x.djs_out_ptr(), 2);
    return this.read(w[0], w[1]);
  }

  private display(h: number): string {
    const n = this.x.vm_display(this.vm, h);
    return textDecoder.decode(new Uint8Array(this.x.memory.buffer, this.x.djs_out_ptr(), n));
  }

  /** Compiles `code` and starts the call; returns nothing (the VM holds the call). */
  private start(code: string, params: Record<string, unknown>): void {
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
      const f = x.vm_compile(vm, 1, head.length, src.length);
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
   * The run loop, as a generator: yields a promise when an async host function must be
   * awaited (only when `async`), returns the result.
   */
  private *drive(async: boolean): Generator<Promise<unknown>, unknown, unknown> {
    const x = this.x;
    const vm = this.vm;
    const slice = this.opts.fuelSlice ?? 1_000_000;
    const maxFuel = this.opts.maxFuel ?? Infinity;
    let used = 0;
    for (;;) {
      const st = this.guard(() => x.vm_run(vm, slice));
      if (st === STATUS_DONE || st === STATUS_THREW) {
        const { value, message } = this.guard(() => {
          const h = x.vm_result(vm);
          const value = this.readHandle(h);
          const message = st === STATUS_THREW ? this.display(h) : "";
          x.vm_handle_free(vm, h);
          return { value, message };
        });
        if (st === STATUS_THREW) throw new SandboxError("threw", message, value);
        return value;
      }
      if (st === STATUS_OUT_OF_FUEL) {
        used += slice;
        if (used >= maxFuel) {
          // The call cannot be abandoned yet: the VM stays mid-call.
          this.dead = "out of fuel";
          throw new SandboxError("fuel", `out of fuel after ${used} units`);
        }
        continue;
      }
      if (st !== STATUS_HOST_CALL) return undefined; // idle
      const { callId, fnId, args } = this.guard(() => {
        const argc = x.vm_host_call(vm);
        const w = new Uint32Array(x.memory.buffer, x.djs_out_ptr(), 4 + 2 * argc).slice();
        const args: unknown[] = [];
        for (let i = 0; i < argc; i++) {
          const v = this.read(w[4 + 2 * i], w[5 + 2 * i]);
          args.push(v);
        }
        return { callId: w[0], fnId: w[1], args };
      });
      const f = this.fns.get(fnId);
      let ok = true;
      let h = 0;
      try {
        if (f === undefined) throw new TypeError(`unknown host function ${fnId}`);
        const isConsole = this.fnNames[fnId - 1]?.startsWith("console.");
        let result = f(...(isConsole ? args.map(consoleArg) : args));
        if (result instanceof Promise) {
          if (!async) throw new TypeError(`host function ${this.fnNames[fnId - 1]} returned a promise; use runAsync`);
          result = yield result;
        }
        h = this.guard(() => this.toHandle(result));
      } catch (e) {
        if (e instanceof SandboxError && e.kind === "trap") throw e;
        // Thrown at the call in the VM, as an Error with the host error's message.
        ok = false;
        h = this.guard(() => {
          const msg = this.stageText(e instanceof Error ? e.message : String(e));
          const kind = e instanceof TypeError ? 1 : e instanceof RangeError ? 2 : 0;
          return this.check(x.vm_new_error(vm, kind, msg), "memory");
        });
      }
      this.guard(() => {
        x.vm_resolve_host_call(vm, callId, ok ? 1 : 0, h);
        if (h !== 0) x.vm_handle_free(vm, h);
      });
    }
  }
}

/** Console arguments: values without a host form are shown by their class. */
function consoleArg(a: unknown): unknown {
  return a instanceof Opaque ? `[${a.className ?? a.type}]` : a;
}

function outText(x: Exports): string {
  return textDecoder.decode(new Uint8Array(x.memory.buffer, x.djs_out_ptr(), x.djs_out_len()));
}
