// A thin handle-level client of djs-wasm for the pi codemode adapter (vm.ts). The Sandbox
// (src/sandbox.ts) runs whole function bodies; pi's VM contract instead keeps functions
// from its prelude (`settle`, `run`, `stalled`) alive across many calls, so this works on
// heap handles directly. Handles are GC roots and survive collections and snapshots; raw
// values do not (docs/host.md "The ABI").

import { encode } from "../src/wire.ts";
import { readValue } from "../src/reader.ts";
import { baseReference, provideBase, type Reference, rememberBase } from "../src/snapshot.ts";

export const STATUS_DONE = 0;
export const STATUS_THREW = 1;
export const STATUS_OUT_OF_FUEL = 2;
export const STATUS_HOST_CALL = 3;
export const STATUS_IDLE = 4;
/** The host could not provide memory below the cap (D1690); the call is abandoned. */
export const STATUS_HOST_OUT_OF_MEMORY = 5;
const ABI_VERSION = 1;
/** `vm_new_with`'s flag: frozen builtins (D2304). */
const VM_FROZEN_BUILTINS = 1;

interface Exports {
  memory: WebAssembly.Memory;
  djs_abi_version(): number;
  djs_build_hash(): number;
  djs_staging(len: number): number;
  /** Since 0094 (D1942); older modules lack it. */
  djs_image_staging?(len: number, memoryLimit: number): number;
  djs_out_ptr(): number;
  djs_out_len(): number;
  djs_panic_ptr(): number;
  djs_panic_len(): number;
  vm_new(memoryLimit: number): number;
  vm_from_bytes(len: number, memoryLimit: number): number;
  vm_free(vm: number): void;
  vm_arena_ptr(vm: number): number;
  vm_arena_len(vm: number): number;
  vm_collect(vm: number): number;
  vm_seal(vm: number): number;
  vm_new_reserved?(memoryLimit: number, live: number): number;
  /** Since 0146 (D2304); older modules lack it. */
  vm_new_with?(memoryLimit: number, live: number, flags: number): number;
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
  /** Since 0117 (D2260); older modules lack them and write uncompressed snapshots. */
  djs_base_ptr?(): number;
  djs_base_len?(): number;
  djs_base_inject_ptr?(len: number): number;
  djs_base_inject?(): number;
}

/** An instance of the module, before a VM is created in it (`Engine.restore`). */
export interface Instance {
  readonly module: WebAssembly.Module;
  readonly exports: Exports;
}

/** A new instance of `module`, checked to be djs-wasm. */
export function instance(module: WebAssembly.Module): Instance {
  return { module, exports: instantiate(module) };
}

/** The module's build hash, from an instance (`buildHash` as a number). */
export function instanceBuild(i: Instance): number {
  return i.exports.djs_build_hash();
}

/** The base snapshot of the instance's module, as a snapshot reference (D2260). */
export function instanceBase(i: Instance): Reference | undefined {
  return baseReference(i.module, i.exports);
}

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

/** The module trapped (an engine bug); the instance is unusable. */
export class EngineTrap extends Error {}

/** One djs VM in its own wasm instance. */
export class Engine {
  private readonly x: Exports;
  private readonly module: WebAssembly.Module;
  readonly vm: number;
  private readonly atomCache: (string | undefined)[] = [];

  /**
   * A new VM; throws if the module is not djs-wasm or `memoryLimit` is too small. With
   * `live`, the heap starts with the room a heap of that size gets (its next collection
   * threshold): growing later would move it and leave the old block behind as a hole in
   * wasm memory.
   */
  static create(module: WebAssembly.Module, memoryLimit: number | undefined, live = 0, frozenBuiltins = false): Engine {
    const x = instantiate(module);
    // The module's base, decoded once by its first instance, not by each (D2252), and
    // kept for later instances before the VM takes this instance's bytes as its arena
    // (D2060).
    provideBase(module, x);
    rememberBase(module, x);
    let vm: number;
    if (frozenBuiltins) {
      // Frozen builtins (D2304): an option at creation.
      if (!x.vm_new_with) throw new Error("The djs-wasm module is too old for frozenBuiltins (no vm_new_with)");
      vm = x.vm_new_with(memoryLimit ?? 0, live, VM_FROZEN_BUILTINS);
    } else vm = x.vm_new_reserved ? x.vm_new_reserved(memoryLimit ?? 0, live) : x.vm_new(memoryLimit ?? 0);
    if (vm === 0) throw new Error(`Cannot create a djs VM: ${outText(x) || "memory limit too small"}`);
    return new Engine(module, x, vm);
  }

  /** A VM restored from `image()` of the same build, in `into` (default: a new instance). */
  static restore(
    module: WebAssembly.Module,
    image: Uint8Array,
    memoryLimit: number | undefined,
    into: Instance = instance(module),
  ): Engine {
    const x = into.exports;
    // The image's buffer becomes the arena: stage it with the arena's capacity (D1942).
    const at = x.djs_image_staging ? x.djs_image_staging(image.length, memoryLimit ?? 0) : x.djs_staging(image.length);
    new Uint8Array(x.memory.buffer).set(image, at);
    const vm = x.vm_from_bytes(image.length, memoryLimit ?? 0);
    if (vm === 0) throw new Error(outText(x));
    return new Engine(module, x, vm);
  }

  private constructor(module: WebAssembly.Module, x: Exports, vm: number) {
    this.module = module;
    this.x = x;
    this.vm = vm;
  }

  /** The build hash as a number (`djs_build_hash`). */
  get build(): number {
    return this.x.djs_build_hash();
  }

  /** The module's base snapshot as a snapshot reference (D2260); undefined for old modules. */
  baseReference(): Reference | undefined {
    return baseReference(this.module, this.x);
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

  /**
   * Collects garbage; false if the heap could not reserve the to-space (it is unchanged
   * then). Raw values are invalid afterwards; handles stay valid.
   */
  collect(): boolean {
    return this.trap(() => this.x.vm_collect(this.vm)) === 0;
  }

  /**
   * Collects garbage and makes everything live old (D2041): the collections while scripts
   * run leave it alone. False as for `collect`.
   */
  seal(): boolean {
    return this.trap(() => this.x.vm_seal(this.vm)) === 0;
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
