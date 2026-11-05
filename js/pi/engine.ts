// A thin handle-level client of djs-wasm for the pi codemode adapter (vm.ts). The Sandbox
// (src/sandbox.ts) runs whole function bodies; pi's VM contract instead keeps functions
// from its prelude (`settle`, `run`, `stalled`) alive across many calls, so this works on
// heap handles directly. Handles are GC roots and survive collections and images; raw
// values do not (docs/host.md "The ABI").

import { IN_PLACE_MIN, encode, stringInPlace } from "../src/wire.ts";
import { readValue } from "../src/reader.ts";
import { trapText } from "../src/trap.ts";
import { provideBase, rememberBase } from "../src/base.ts";
import { scriptFor as dumpRuntimeFor } from "../src/dump-runtime.js";

export const STATUS_DONE = 0;
export const STATUS_THREW = 1;
export const STATUS_OUT_OF_FUEL = 2;
export const STATUS_HOST_CALL = 3;
export const STATUS_IDLE = 4;
/** The host could not provide memory below the cap (D1690); the call is abandoned. */
export const STATUS_HOST_OUT_OF_MEMORY = 5;
const ABI_VERSION = 5;
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
  __stack_pointer?: WebAssembly.Global;
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
  vm_new_async_host_function(vm: number, fnId: number, nameLen: number): number;
  vm_new_host_calls(vm: number): number;
  vm_host_call_info(vm: number, callId: number): number;
  /** Since 0176 (D2650): `next` mode. */
  vm_outcome?(vm: number): number;
  /** With the `dump` feature (D2352): continuation dumps. */
  vm_dump?(vm: number, len: number): number;
  /** Since 0204 (D2890): loading plain dumps. */
  vm_open_call?(vm: number, parked: number, fnId: number): number;
  vm_set_next_call_id?(vm: number, id: number): void;
  vm_global(vm: number): number;
  vm_new_object(vm: number): number;
  /** Since 0207 (D2903): the plain dump runtime, `$djs`'s helpers. */
  vm_dump_runtime?(vm: number, len: number): number;
  vm_define(vm: number, obj: number, nameLen: number, value: number): number;
  vm_decode_wire(vm: number, len: number): number;
  /** Since 0162 (D2551); older modules lack it. */
  vm_string_reserve?(vm: number, len: number): number;
  vm_handle_bits(vm: number, h: number): void;
  vm_handle_free(vm: number, h: number): void;
  vm_display(vm: number, h: number): number;
  vm_call(vm: number, f: number, thisArg: number, argc: number): number;
  vm_run(vm: number, fuel: number): number;
  /** Since 0176 (D2653). */
  vm_fuel_used?(vm: number): number;
  vm_result(vm: number): number;
  vm_host_call(vm: number): number;
  vm_promise_state(vm: number, h: number): number;
  vm_resolve_host_call(vm: number, callId: number, ok: number, value: number): number;
  vm_abort(vm: number): void;
  vm_set_now(vm: number, ms: number): void;
  vm_set_time_zone_offset(vm: number, minutes: number): void;
  vm_set_random_seed(vm: number, lo: number, hi: number): void;
  vm_set_math_random_seed(vm: number, lo: number, hi: number): void;
  vm_set_weak_refs(vm: number, mode: number): number;
  /** The base snapshot (D2260); the inject exports only with the base coded (D2252). */
  djs_base_ptr(): number;
  djs_base_len(): number;
  djs_base_inject_ptr?(len: number): number;
  djs_base_inject?(): number;
  djs_base_coded_ptr?(len: number): number;
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
  /** The instance trapped: it is not called again. */
  private trapped = false;

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

  /** A VM restored from `image()` (a prelude image of this process) in a new instance. */
  static restore(module: WebAssembly.Module, image: Uint8Array, memoryLimit: number | undefined): Engine {
    const x = instantiate(module);
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

  /** Seeds `Math.random` and keys the VM's string hash from the seed (D2433). */
  setRandomSeed(lo: number, hi: number): void {
    this.x.vm_set_random_seed(this.vm, lo >>> 0, hi >>> 0);
  }

  /** Seeds `Math.random` alone; the hash key stays. */
  setMathRandomSeed(lo: number, hi: number): void {
    this.x.vm_set_math_random_seed(this.vm, lo >>> 0, hi >>> 0);
  }

  /** A handle to a copy of a host value (sent through the wire format); 0 for undefined. */
  value(v: unknown): number {
    if (v === undefined) return 0;
    const bytes = encode(v);
    if (typeof v === "string" && v.length >= IN_PLACE_MIN) {
      const h = this.trap(() => stringInPlace(this.x, this.vm, v, bytes));
      if (h !== 0) return h;
    }
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

  /** An async host function (D1374): a call returns a promise and adds a pending call. */
  asyncHostFunction(fnId: number, name: string): number {
    const len = this.stageText(name);
    const h = this.trap(() => this.x.vm_new_async_host_function(this.vm, fnId, len));
    if (h === 0) throw new Error("Out of memory creating a host function");
    return h;
  }

  /** The async host calls made since the last check. */
  newHostCalls(): number[] {
    return this.trap(() => outWords(this.x, this.x.vm_new_host_calls(this.vm)));
  }

  /** A pending host call's function id and arguments as host values. */
  hostCallInfo(callId: number): { fnId: number; args: unknown[] } {
    return this.trap(() => {
      const argc = this.x.vm_host_call_info(this.vm, callId);
      return this.readCall(argc);
    });
  }

  /** Settles host call `callId`: returns or throws (sync), fulfills or rejects (async). */
  settleHostCall(callId: number, ok: boolean, value: number): void {
    if (this.trap(() => this.x.vm_resolve_host_call(this.vm, callId, ok ? 1 : 0, value)) !== 0) {
      throw new Error(`No pending host call ${callId}`);
    }
  }

  /**
   * The entry call's outcome while no frame is active, also at IDLE (`vm_outcome`): the
   * return value (or exception) of a function called while async host calls are pending.
   */
  outcome(): { handle: number; threw: boolean } {
    const x = this.x;
    if (!x.vm_outcome) throw new Error("The djs-wasm module is too old for the next mode (no vm_outcome)");
    return this.trap(() => {
      const handle = x.vm_outcome!(this.vm);
      return { handle, threw: outWords(x, 1)[0] === 1 };
    });
  }

  /**
   * A dump of the idle VM (`vm_dump`, a plain Script, D2830; the VM needs frozen builtins,
   * D2836): `host` are the globals the host defines (never written; their values are
   * host roots by the global's name), `fnNames` the host functions' identities by id - 1,
   * `roots` other values the host names (`$djs.host(name)`, D2971), `handles` the values to
   * name for the host, `states` the host's state of open calls as JSON text, by call id
   * (D2770). Throws with the reasons when the VM cannot be dumped.
   */
  dump(host: string[], fnNames: string[], handles: [string, number][], states: [number, string][] = [], roots: [string, number][] = []): string {
    const x = this.x;
    if (!x.vm_dump) throw new Error("The djs-wasm module has no dumper (build it with the `dump` feature)");
    const text = [
      host.join(","),
      "",
      fnNames.join("\t"),
      handles.map(([n, h]) => `${h}:${n}`).join("\t"),
      roots.map(([n, h]) => `${h}:${n}`).join("\t"),
      states.map(([id, json]) => `${id}:${textEncoder.encode(json).length}`).join("\t"),
      states.map(([, json]) => json).join(""),
    ].join("\n");
    let r = this.trap(() => x.vm_dump!(this.vm, this.stageText(text)));
    if (r === 2) {
      provideBase(this.module, x);
      r = this.trap(() => x.vm_dump!(this.vm, this.stageText(text)));
    }
    if (r !== 0) throw new Error(r === 1 ? `Cannot dump the session:\n${outText(x)}` : "No base image to dump against");
    return outText(x);
  }

  /**
   * Compiles `src` named `file` in stack traces: goal 0 a Script, 1 an async function body
   * with `params`, 2 a TypeScript Script. Returns the closure's handle, or the compile error's text
   * (`SyntaxError: message at line:col`).
   */
  compile(goal: 0 | 1 | 2, file: string, params: string[], src: string): { handle: number } | { error: string } {
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

  /** A new object holding the plain dump runtime's helpers that dump `text` names (D2903): `$djs` without the hooks. */
  dumpRuntime(text: string): number {
    const f = this.x.vm_dump_runtime;
    if (!f) throw new Error("The djs-wasm module cannot load plain dumps (no vm_dump_runtime)");
    const bytes = textEncoder.encode(dumpRuntimeFor(text));
    this.stage(bytes);
    const h = this.trap(() => f(this.vm, bytes.length));
    if (h === 0) throw new Error("Out of memory creating the dump runtime");
    return h;
  }

  /** A new ordinary object's handle. */
  newObject(): number {
    const h = this.trap(() => this.x.vm_new_object(this.vm));
    if (h === 0) throw new Error("Out of memory creating an object");
    return h;
  }

  /**
   * Defines property `name` of `obj` (0: the global object) as `value` (handles): writable,
   * configurable, not enumerable.
   */
  define(obj: number, name: string, value: number): void {
    const x = this.x;
    const g = obj === 0 ? this.trap(() => x.vm_global(this.vm)) : obj;
    const r = this.trap(() => x.vm_define(this.vm, g, this.stageText(name), value));
    if (obj === 0) this.free(g);
    if (r !== 0) throw new Error(`Cannot define ${name}`);
  }

  /**
   * A plain dump's `$djs.resume(id, name, args, state)`, the parked call `parked` (D2890):
   * opens the dump's call `id` of host function `fnId` again under that id and returns its
   * promise from `parked`. The reason when it cannot (the host throws it at the call).
   */
  openCall(parked: number, fnId: number): string | undefined {
    const x = this.x;
    if (!x.vm_open_call) throw new Error("The djs-wasm module cannot load plain dumps (no vm_open_call)");
    return this.trap(() => x.vm_open_call!(this.vm, parked, fnId)) === 0 ? undefined : outText(x);
  }

  /** Sets the id the next host call gets (after a plain dump's load, D2897). */
  setNextCallId(id: number): void {
    if (this.x.vm_set_next_call_id) this.trap(() => this.x.vm_set_next_call_id!(this.vm, id));
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

  /** The fuel the last `run` used; without `vm_fuel_used` (older modules), all of it. */
  fuelUsed(fuel: number): number {
    return this.x.vm_fuel_used ? this.x.vm_fuel_used(this.vm) : fuel;
  }

  /** The DONE/THREW value's handle (the caller frees it). */
  result(): number {
    return this.x.vm_result(this.vm);
  }

  /** The parked host call: its id, function id and arguments as host values. */
  hostCall(): { callId: number; fnId: number; args: unknown[] } {
    return this.trap(() => {
      const argc = this.x.vm_host_call(this.vm);
      const w = outWords(this.x, 1);
      return { callId: w[0], ...this.readCall(argc) };
    });
  }

  /** Reads `[call_id, fn_id, this, args...]` from the out buffer. */
  private readCall(argc: number): { fnId: number; args: unknown[] } {
    const w = outWords(this.x, 4 + 2 * argc);
    const arena = this.arena();
    const args: unknown[] = [];
    for (let i = 0; i < argc; i++) {
      args.push(readValue(arena, [w[4 + 2 * i], w[5 + 2 * i]], { atomCache: this.atomCache }));
    }
    return { fnId: w[1], args };
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
    // A trapped instance is not called again (a stack overflow would trap once more).
    if (!this.trapped) this.x.vm_free(this.vm);
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
        this.trapped = true;
        throw new EngineTrap(`djs-wasm trapped: ${trapText(this.x, e)}`);
      }
      throw e;
    }
  }
}

function instantiate(module: WebAssembly.Module): Exports {
  const x = new WebAssembly.Instance(module, {}).exports as unknown as Exports;
  if (typeof x.djs_abi_version !== "function" || x.djs_abi_version() !== ABI_VERSION) {
    throw new Error(`The module is not djs-wasm of ABI version ${ABI_VERSION}`);
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
