// A thin handle-level client of djs-wasm for the pi codemode adapter (vm.ts). The Sandbox
// (src/sandbox.ts) runs whole function bodies; pi's VM contract instead keeps functions
// from its prelude (`settle`, `run`, `stalled`) alive across many calls, so this works on
// heap handles directly. Handles are GC roots and survive collections and snapshots; raw
// values do not (docs/host.md "The ABI").

import { encode } from "../src/wire.ts";
import { readValue } from "../src/reader.ts";

export const STATUS_DONE = 0;
export const STATUS_THREW = 1;
export const STATUS_OUT_OF_FUEL = 2;
export const STATUS_HOST_CALL = 3;
export const STATUS_IDLE = 4;
const ABI_VERSION = 1;

interface Exports {
  memory: WebAssembly.Memory;
  djs_abi_version(): number;
  djs_build_hash(): number;
  djs_staging(len: number): number;
  djs_out_ptr(): number;
  djs_out_len(): number;
  djs_panic_ptr(): number;
  djs_panic_len(): number;
  vm_new(memoryLimit: number): number;
  vm_from_bytes(len: number, memoryLimit: number): number;
  vm_free(vm: number): void;
  vm_arena_ptr(vm: number): number;
  vm_arena_len(vm: number): number;
  vm_compile_file(vm: number, goal: number, nameLen: number, paramsLen: number, srcLen: number): number;
  vm_new_host_function(vm: number, fnId: number, nameLen: number): number;
  vm_decode_wire(vm: number, len: number): number;
  vm_handle_bits(vm: number, h: number): void;
  vm_handle_free(vm: number, h: number): void;
  vm_display(vm: number, h: number): number;
  vm_call(vm: number, f: number, thisArg: number, argc: number): number;
  vm_run(vm: number, fuel: number): number;
  vm_result(vm: number): number;
  vm_host_call(vm: number): number;
  vm_promise_state(vm: number, h: number): number;
  vm_resolve_host_call(vm: number, callId: number, ok: number, value: number): number;
  vm_abort(vm: number): void;
  vm_set_now(vm: number, ms: number): void;
  vm_set_time_zone_offset(vm: number, minutes: number): void;
  vm_set_random_seed(vm: number, lo: number, hi: number): void;
  vm_set_weak_refs(vm: number, mode: number): number;
}

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

/** The module trapped (an engine bug); the instance is unusable. */
export class EngineTrap extends Error {}

/** One djs VM in its own wasm instance. */
export class Engine {
  private readonly x: Exports;
  readonly vm: number;
  private readonly atomCache: (string | undefined)[] = [];

  /** A new VM; throws if the module is not djs-wasm or `memoryLimit` is too small. */
  static create(module: WebAssembly.Module, memoryLimit: number | undefined): Engine {
    const x = instantiate(module);
    const vm = x.vm_new(memoryLimit ?? 0);
    if (vm === 0) throw new Error(`Cannot create a djs VM: ${outText(x) || "memory limit too small"}`);
    return new Engine(x, vm);
  }

  /** A VM restored from `image()` of the same build. */
  static restore(module: WebAssembly.Module, image: Uint8Array, memoryLimit: number | undefined): Engine {
    const x = instantiate(module);
    const at = x.djs_staging(image.length);
    new Uint8Array(x.memory.buffer).set(image, at);
    const vm = x.vm_from_bytes(image.length, memoryLimit ?? 0);
    if (vm === 0) throw new Error(outText(x));
    return new Engine(x, vm);
  }

  private constructor(x: Exports, vm: number) {
    this.x = x;
    this.vm = vm;
  }

  /** The engine's build hash, as 8 hex digits: identifies the engine build. */
  get buildHash(): string {
    return (this.x.djs_build_hash() >>> 0).toString(16).padStart(8, "0");
  }

  /** The VM image (a copy), for `Engine.restore`. */
  image(): Uint8Array {
    const { x, vm } = this;
    return new Uint8Array(x.memory.buffer, x.vm_arena_ptr(vm), x.vm_arena_len(vm)).slice();
  }

  get heapSize(): number {
    return this.trap(() => this.x.vm_arena_len(this.vm));
  }

  setClock(ms: number, timeZoneOffset: number): void {
    this.x.vm_set_now(this.vm, ms);
    this.x.vm_set_time_zone_offset(this.vm, timeZoneOffset);
  }

  /** What WeakRef and FinalizationRegistry do (D1680): 0 collected, 1 strong, 2 absent. */
  setWeakRefs(mode: 0 | 1 | 2): void {
    if (this.trap(() => this.x.vm_set_weak_refs(this.vm, mode)) !== 0) throw new Error("vm_set_weak_refs failed");
  }

  setRandomSeed(lo: number, hi: number): void {
    this.x.vm_set_random_seed(this.vm, lo >>> 0, hi >>> 0);
  }

  /** A handle to a copy of a host value (sent through the wire format); 0 for undefined. */
  value(v: unknown): number {
    if (v === undefined) return 0;
    const bytes = encode(v);
    this.stage(bytes);
    const h = this.trap(() => this.x.vm_decode_wire(this.vm, bytes.length));
    if (h === 0) throw new Error(`Cannot pass a value into the VM: ${outText(this.x)}`);
    return h;
  }

  hostFunction(fnId: number, name: string): number {
    const len = this.stageText(name);
    const h = this.trap(() => this.x.vm_new_host_function(this.vm, fnId, len));
    if (h === 0) throw new Error("Out of memory creating a host function");
    return h;
  }

  /**
   * Compiles `src` named `file` in stack traces: goal 0 a Script, 1 an async function body
   * with `params`. Returns the closure's handle, or the compile error's text
   * (`SyntaxError: message at line:col`).
   */
  compile(goal: 0 | 1, file: string, params: string[], src: string): { handle: number } | { error: string } {
    const name = textEncoder.encode(file);
    const head = textEncoder.encode(params.join("\n"));
    const body = textEncoder.encode(src);
    const all = new Uint8Array(name.length + head.length + body.length);
    all.set(name);
    all.set(head, name.length);
    all.set(body, name.length + head.length);
    this.stage(all);
    const h = this.trap(() => this.x.vm_compile_file(this.vm, goal, name.length, head.length, body.length));
    return h === 0 ? { error: outText(this.x) } : { handle: h };
  }

  /** Starts calling `f` with `thisArg` and argument handles (0 = undefined). */
  call(f: number, thisArg: number, args: number[]): void {
    const bytes = new Uint8Array(4 * args.length);
    const view = new DataView(bytes.buffer);
    args.forEach((h, i) => view.setUint32(4 * i, h, true));
    this.stage(bytes);
    if (this.trap(() => this.x.vm_call(this.vm, f, thisArg, args.length)) !== 0) {
      throw new Error("The VM is already in a call, or out of memory");
    }
  }

  run(fuel: number): number {
    return this.trap(() => this.x.vm_run(this.vm, fuel));
  }

  /** The DONE/THREW value's handle (the caller frees it). */
  result(): number {
    return this.x.vm_result(this.vm);
  }

  /** The parked host call: its id, function id and arguments as host values. */
  hostCall(): { callId: number; fnId: number; args: unknown[] } {
    return this.trap(() => {
      const argc = this.x.vm_host_call(this.vm);
      const w = outWords(this.x, 4 + 2 * argc);
      const arena = this.arena();
      const args: unknown[] = [];
      for (let i = 0; i < argc; i++) {
        args.push(readValue(arena, [w[4 + 2 * i], w[5 + 2 * i]], { atomCache: this.atomCache }));
      }
      return { callId: w[0], fnId: w[1], args };
    });
  }

  /** Returns `value` (a handle, 0 = undefined) from the parked host call. */
  resolveHostCall(callId: number, value: number): void {
    this.trap(() => this.x.vm_resolve_host_call(this.vm, callId, 1, value));
  }

  /** A promise's state (0 none, 1 pending, 2 fulfilled, 3 rejected) and, if settled, its result handle. */
  promiseState(h: number): { state: number; result: number } {
    return this.trap(() => {
      const state = this.x.vm_promise_state(this.vm, h);
      return { state, result: state >= 2 ? outWords(this.x, 1)[0] : 0 };
    });
  }

  /** A handle's value as a host value. */
  read(h: number): unknown {
    return this.trap(() => {
      this.x.vm_handle_bits(this.vm, h);
      const w = outWords(this.x, 2);
      return readValue(this.arena(), [w[0], w[1]], { atomCache: this.atomCache });
    });
  }

  /** A handle's value as `console.log` shows it (Errors as `Name: message`). */
  display(h: number): string {
    return this.trap(() => {
      const n = this.x.vm_display(this.vm, h);
      return textDecoder.decode(new Uint8Array(this.x.memory.buffer, this.x.djs_out_ptr(), n));
    });
  }

  free(h: number): void {
    if (h !== 0) this.x.vm_handle_free(this.vm, h);
  }

  abort(): void {
    this.x.vm_abort(this.vm);
  }

  dispose(): void {
    this.x.vm_free(this.vm);
  }

  private arena(): Uint8Array {
    const { x, vm } = this;
    return new Uint8Array(x.memory.buffer, x.vm_arena_ptr(vm), x.vm_arena_len(vm));
  }

  private stage(bytes: Uint8Array): void {
    const at = this.x.djs_staging(bytes.length);
    new Uint8Array(this.x.memory.buffer).set(bytes, at);
  }

  private stageText(s: string): number {
    const b = textEncoder.encode(s);
    this.stage(b);
    return b.length;
  }

  /** Runs `f`, turning a wasm trap into an EngineTrap with the panic message. */
  private trap<T>(f: () => T): T {
    try {
      return f();
    } catch (e) {
      if (e instanceof WebAssembly.RuntimeError) {
        const x = this.x;
        const msg = textDecoder.decode(new Uint8Array(x.memory.buffer, x.djs_panic_ptr(), x.djs_panic_len()));
        throw new EngineTrap(`djs-wasm trapped: ${msg || e.message}`);
      }
      throw e;
    }
  }
}

function instantiate(module: WebAssembly.Module): Exports {
  const x = new WebAssembly.Instance(module, {}).exports as unknown as Exports;
  if (typeof x.djs_abi_version !== "function" || x.djs_abi_version() !== ABI_VERSION) {
    throw new Error("The module is not djs-wasm of ABI version 1");
  }
  if (typeof x.vm_compile_file !== "function" || typeof x.vm_set_weak_refs !== "function") {
    throw new Error("The djs-wasm module is too old for pi codemode (no vm_compile_file / vm_set_weak_refs)");
  }
  return x;
}

function outWords(x: Exports, n: number): number[] {
  const dv = new DataView(x.memory.buffer, x.djs_out_ptr(), 4 * n);
  return Array.from({ length: n }, (_, i) => dv.getUint32(4 * i, true));
}

function outText(x: Exports): string {
  return textDecoder.decode(new Uint8Array(x.memory.buffer, x.djs_out_ptr(), x.djs_out_len()));
}
