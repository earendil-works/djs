/**
 * pi codemode's VM contract (pi-private packages/codemode/src/runtime/vm.ts, `startVm`) on
 * djs instead of quickjs-wasi. pi's launchers (worker thread, in-thread, remote) and its
 * host side (execution.ts) stay as they are; only the module that drives one VM changes.
 * docs/host.md "pi codemode" compares the two.
 *
 * Contract (as pi's vm.ts):
 * - `post` receives every message for the host, synchronously from inside VM callbacks; the
 *   caller must not re-enter the VM from it.
 * - `deliver` hands the VM a tool result and runs the script until it waits again; only
 *   while the VM is idle.
 * - `shouldInterrupt` is polled between fuel slices (every `fuelPerPoll` units of fuel, which
 *   also covers promise jobs: djs charges fuel for every instruction wherever it runs).
 *   Returning true abandons the VM's call (`vm_abort`, which the script cannot catch) and
 *   the VM reports `crash`. Not while the host globals and pi's prelude run: a start that
 *   reuses a prelude image skips them, and a budget must mean the same either way.
 * - The VM never decides on its own when it is finished.
 *
 * Deterministic mode (`data.deterministic`): djs reads no clock and no entropy itself
 * (ARCH "Nondeterminism"), so the host sets them: `Date` reads `startedAt`, then the `now` of
 * the result being processed (never backwards); `Math.random` is seeded from `seed`; the time
 * zone is UTC; results apply in `seq` order. The VM deletes `WeakRef` and
 * `FinalizationRegistry` (`vm_set_weak_refs`, D1680), as pi's quickjs VM leaves them out, so
 * its GC is not observable.
 *
 * Beyond pi's contract: `snapshot()` returns the whole session (the VM image plus the
 * adapter's state) while the VM waits for results, and `resumeVm` continues it in a new
 * instance, possibly in another process: the script resumes where it waited, with no replay.
 */
import {
  Engine,
  STATUS_DONE,
  STATUS_HOST_CALL,
  STATUS_HOST_OUT_OF_MEMORY,
  STATUS_IDLE,
  STATUS_OUT_OF_FUEL,
  STATUS_THREW,
} from "./engine.ts";
import type { HostToWorkerMessage, VmStartData, WorkerToHostMessage } from "./protocol.ts";

export type { HostToWorkerMessage, VmStartData, WorkerToHostMessage } from "./protocol.ts";

export interface VmSession {
  deliver(message: HostToWorkerMessage): void;
  /** Releases the VM's wasm instance. The session must not be used afterwards. */
  dispose(): void;
  /**
   * The session as bytes, for `resumeVm`: only while the VM waits (not from inside `post`).
   * Results the host has not delivered yet must be delivered to the resumed session.
   */
  snapshot(): Uint8Array;
}

export interface VmHooks {
  /** The compiled djs-wasm module (`loadModule()` of `@djs/host`). */
  wasm: WebAssembly.Module | object;
  /** Interrupt polls this VM may use, if the launcher counts them; part of the fingerprint. */
  interruptBudget?: number;
  /**
   * Fuel between two `shouldInterrupt` polls. Default {@link FUEL_PER_POLL}. Part of the
   * fingerprint, since it decides where a budget runs out.
   */
  fuelPerPoll?: number;
  /**
   * Start from a VM image taken after the prelude when an earlier start had the same prelude,
   * tools, globals, store and heap limit (default true; see `imageCache`).
   */
  reuseImages?: boolean;
  /** Accepted for pi's interface; djs keeps frames in its heap, not on the host stack. */
  maxStackSize?: number;
  post(message: WorkerToHostMessage): void;
  shouldInterrupt(): boolean;
}

export interface StartVmOptions extends VmHooks {
  data: VmStartData;
  /** pi's `PRELUDE_SOURCE` (prelude-source.ts). */
  prelude: string;
}

/**
 * Fuel per poll. With 10,000, a tight loop polls about as often per second as pi's quickjs VM
 * (about 33,000 polls/s), so pi's interrupt budgets mean about the same CPU time; code that
 * allocates or calls builtins polls far more often than on QuickJS, whose poll counter does
 * not count that work (docs/host.md "pi codemode").
 */
export const FUEL_PER_POLL = 10_000;

/** Bumped whenever this module changes what a deterministic script observes. */
const DETERMINISTIC_VERSION = 1;

const SNAPSHOT_MAGIC = 0x49504a44; // "DJPI"
const SNAPSHOT_VERSION = 1;
const BRIDGE_FN = 1;
const SCRIPT_PREFIX = "return (async (tools, console) => {";

/**
 * Globals quickjs-wasi's host layer gives pi's scripts that djs does not have (`queueMicrotask`
 * is native since 0059), defined before
 * pi's prelude so its lockdown freezes them too. `performance.now()` counts from the VM's
 * start on the VM clock (deterministic mode: from `startedAt`, as pi's quickjs VM does).
 */
const HOST_GLOBALS = `
const define = (name, value) =>
  Object.defineProperty(globalThis, name, { value, writable: true, enumerable: false, configurable: true });
const now = Date.now;
let origin = 0;
const performance = {};
Object.defineProperty(performance, "now", { value: { now() { return now() - origin; } }.now, writable: true, configurable: true });
define("performance", performance);
const ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const invalidCharacter = (message) => {
  const error = new Error(message);
  Object.defineProperty(error, "name", { value: "InvalidCharacterError", writable: true, configurable: true });
  return error;
};
define("btoa", { btoa(data) {
  if (arguments.length === 0) throw new TypeError("btoa: 1 argument required");
  const s = String(data);
  let out = "";
  for (let i = 0; i < s.length; i += 3) {
    const a = s.charCodeAt(i), b = s.charCodeAt(i + 1), c = s.charCodeAt(i + 2);
    if (a > 255 || b > 255 || c > 255) throw invalidCharacter("btoa: the string contains characters outside of Latin1");
    const n = (a << 16) | ((b || 0) << 8) | (c || 0);
    out += ALPHABET[n >> 18] + ALPHABET[(n >> 12) & 63] +
      (i + 1 < s.length ? ALPHABET[(n >> 6) & 63] : "=") + (i + 2 < s.length ? ALPHABET[n & 63] : "=");
  }
  return out;
} }.btoa);
define("atob", { atob(data) {
  if (arguments.length === 0) throw new TypeError("atob: 1 argument required");
  let s = "";
  for (const ch of String(data)) if (ch !== " " && ch !== "\\t" && ch !== "\\n" && ch !== "\\f" && ch !== "\\r") s += ch;
  if (s.length % 4 === 0 && s.endsWith("==")) s = s.slice(0, -2);
  else if (s.length % 4 === 0 && s.endsWith("=")) s = s.slice(0, -1);
  if (s.length % 4 === 1) throw invalidCharacter("atob: the string is not correctly encoded");
  let out = "", bits = 0, count = 0;
  for (let i = 0; i < s.length; i++) {
    const v = ALPHABET.indexOf(s[i]);
    if (v === -1) throw invalidCharacter("atob: the string is not correctly encoded");
    bits = (bits << 6) | v;
    count += 6;
    if (count >= 8) {
      count -= 8;
      out += String.fromCharCode((bits >> count) & 255);
    }
  }
  return out;
} }.atob);
// The host sets the origin per run, after a VM image with the prelude may have been reused.
return (time) => { origin = time; };
`;

/** FNV-1a over UTF-16 code units, as 8 hex digits: identifies the prelude and the host globals in the fingerprint. */
function fnv1a(text: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < text.length; index++) {
    hash ^= text.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

/** The adapter's own globals change what scripts observe, so they are part of the fingerprint. */
const HOST_GLOBALS_HASH = fnv1a(HOST_GLOBALS);

/** The deterministic VM's clock: virtual time, moved by delivered results. */
interface VirtualClock {
  startedAt: number;
  now: number;
}

/** What the adapter keeps outside the VM; all of it goes into a snapshot. */
interface SessionState {
  /** Handles to the prelude's api object and its functions. */
  api: number;
  settle: number;
  stalled: number;
  issued: number;
  nextSeq: number;
  current: number;
  early: [number, HostToWorkerMessage][];
  clock: VirtualClock | null;
  memoryLimit: number | undefined;
}

/** An exception that escaped a VM call (the prelude catches the script's own). */
class VmThrew extends Error {}

function crashMessage(error: unknown): WorkerToHostMessage {
  return {
    type: "crash",
    message: error instanceof VmThrew ? error.message : error instanceof Error ? `${error.name}: ${error.message}` : String(error),
  };
}

/** A djs compile error (`SyntaxError: message at line:col`) as pi's script error JSON. */
function describeCompileError(text: string): string {
  const match = /^(\w+): ([\s\S]*?)(?: at (\d+):(\d+))?$/.exec(text);
  if (!match) return JSON.stringify({ name: "SyntaxError", message: text, stack: `SyntaxError: ${text}` });
  const [, name, message, line, col] = match;
  const head = `${name}: ${message}`;
  if (line === undefined) return JSON.stringify({ name, message, stack: head });
  // The script's first line shares its line with the wrapper.
  const column = line === "1" ? Math.max(1, Number(col) - SCRIPT_PREFIX.length) : Number(col);
  return JSON.stringify({ name, message, stack: `${head}\n    at codemode.js:${line}:${column}` });
}

/** One step of xorshift64*'s state (the multiplication only shapes the output). */
function xorshift(x: bigint): bigint {
  const mask = (1n << 64n) - 1n;
  x ^= x >> 12n;
  x = (x ^ (x << 25n)) & mask;
  return x ^ (x >> 27n);
}

/** Reads a 64-bit seed for `Math.random` from the host's entropy. */
function entropy(): [number, number] {
  const words = new Uint32Array(2);
  globalThis.crypto.getRandomValues(words);
  return [words[0], words[1] | 1];
}

class Session implements VmSession {
  private disposed = false;
  private readonly early: Map<number, HostToWorkerMessage>;
  private readonly fuel: number;
  private readonly engine: Engine;
  private readonly hooks: VmHooks;
  private readonly state: SessionState;
  private zoneOffset = 0;
  private zoneCheckedAt = -Infinity;
  /**
   * Whether fuel slices poll `shouldInterrupt`. Off while the host globals and pi's prelude
   * run (trusted, bounded code): a start that reuses a prelude image skips that work, so
   * polling it would make the budget a cold start has left differ from a warm one (D1660).
   */
  polling = true;

  constructor(engine: Engine, hooks: VmHooks, state: SessionState) {
    this.engine = engine;
    this.hooks = hooks;
    this.state = state;
    this.early = new Map(state.early);
    this.fuel = hooks.fuelPerPoll ?? FUEL_PER_POLL;
  }

  deliver(message: HostToWorkerMessage): void {
    if (this.disposed) return;
    const { state } = this;
    try {
      const clock = state.clock;
      if (!clock) {
        this.apply(message);
        return;
      }
      const { seq } = message;
      if (seq === undefined || !Number.isInteger(seq) || seq < state.nextSeq || this.early.has(seq)) {
        throw new Error(`Result ${message.id} has sequence number ${seq}, expected ${state.nextSeq} or later, once`);
      }
      this.early.set(seq, message);
      for (let next = this.early.get(state.nextSeq); next !== undefined; next = this.early.get(state.nextSeq)) {
        this.early.delete(state.nextSeq);
        // Never backwards: a resumed host's clock may be behind the one that started the run.
        if (next.now !== undefined && Number.isFinite(next.now)) clock.now = Math.max(clock.now, next.now);
        state.current = state.nextSeq;
        this.apply(next);
        this.hooks.post({ type: "idle", seq: state.nextSeq, issued: state.issued });
        state.nextSeq++;
      }
    } catch (error) {
      this.hooks.post(crashMessage(error));
    }
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.engine.dispose();
  }

  snapshot(): Uint8Array {
    if (this.disposed) throw new Error("The session is disposed");
    const header = new TextEncoder().encode(JSON.stringify({ ...this.state, early: [...this.early] }));
    const image = this.engine.image();
    const out = new Uint8Array(12 + header.length + image.length);
    const view = new DataView(out.buffer);
    view.setUint32(0, SNAPSHOT_MAGIC, true);
    view.setUint32(4, SNAPSHOT_VERSION, true);
    view.setUint32(8, header.length, true);
    out.set(header, 12);
    out.set(image, 12 + header.length);
    return out;
  }

  /** Settles call `message.id` in the VM and runs the script until it waits again. */
  private apply(message: HostToWorkerMessage): void {
    const { engine, state } = this;
    const args = [
      engine.value(message.id),
      engine.value(message.ok),
      engine.value(message.payload),
      engine.value(message.properties),
    ];
    try {
      engine.free(this.invoke(state.settle, state.api, args));
    } finally {
      for (const h of args) engine.free(h);
    }
    this.drain();
  }

  /** Fails a script that waits on nothing that can ever resume it (pi's `stalled`). */
  drain(): void {
    this.engine.free(this.invoke(this.state.stalled, this.state.api, []));
  }

  /**
   * Calls `f` and runs the VM, its promise jobs included, until it is idle again. Returns
   * the result's handle (the caller frees it). Throws VmThrew for an uncaught exception.
   */
  invoke(f: number, thisArg: number, args: number[]): number {
    const { engine } = this;
    engine.call(f, thisArg, args);
    for (;;) {
      this.setClock();
      const status = engine.run(this.fuel);
      switch (status) {
        case STATUS_OUT_OF_FUEL:
          if (this.polling && this.hooks.shouldInterrupt()) {
            engine.abort();
            throw new Error("Execution interrupted");
          }
          continue;
        case STATUS_HOST_CALL: {
          const { callId, fnId, args: callArgs } = engine.hostCall();
          try {
            if (fnId !== BRIDGE_FN) throw new Error(`Unknown host function ${fnId}`);
            this.bridge(callArgs);
          } catch (error) {
            engine.abort();
            throw error;
          }
          engine.resolveHostCall(callId, 0);
          continue;
        }
        case STATUS_DONE:
          return engine.result();
        case STATUS_THREW: {
          const h = engine.result();
          const text = engine.display(h);
          engine.free(h);
          throw new VmThrew(text);
        }
        case STATUS_IDLE:
          return 0;
        case STATUS_HOST_OUT_OF_MEMORY:
          // Not a script error: the engine abandoned the call; pi reports a crash (D1690).
          throw new Error("out of host memory");
        default:
          throw new Error(`Unexpected VM status ${status}`);
      }
    }
  }

  private setClock(): void {
    const clock = this.state.clock;
    if (clock) this.engine.setClock(clock.now, 0);
    else {
      const now = Date.now();
      // The offset only changes with DST; reading it every slice would cost more than the slice.
      if (now - this.zoneCheckedAt > 1000) {
        this.zoneOffset = -new Date(now).getTimezoneOffset();
        this.zoneCheckedAt = now;
      }
      this.engine.setClock(now, this.zoneOffset);
    }
  }

  /** The prelude's host bridge, called with primitives only. */
  private bridge([kind, a, b, c]: unknown[]): void {
    const { post } = this.hooks;
    const state = this.state;
    switch (kind) {
      case "call":
      case "global":
        state.issued++;
        post({
          type: "call",
          id: Number(a),
          target: kind === "call" ? "tool" : "global",
          name: String(b),
          args: c === undefined ? undefined : String(c),
        });
        break;
      case "output":
        post({
          type: "output",
          item:
            a === "image"
              ? { type: "image", data: String(b), mimeType: String(c) }
              : a === "console"
                ? { type: "text", text: String(b), console: true }
                : { type: "text", text: String(b) },
        });
        break;
      case "done": {
        // The host stops the VM on `done`, so the `idle` of this drain never arrives: report its numbers here.
        const during = state.clock ? { during: { seq: state.current, issued: state.issued } } : {};
        if (a === true) {
          post({
            type: "done",
            ok: true,
            value: b === undefined ? undefined : String(b),
            writes: String(c),
            ...during,
          });
        } else {
          post({ type: "done", ok: false, error: String(b), ...during });
        }
        break;
      }
    }
  }
}

/** Calls an async-body closure and returns its fulfilled value's handle. */
function evaluate(session: Session, engine: Engine, f: number, args: number[] = []): number {
  const p = session.invoke(f, 0, args);
  const { state, result } = engine.promiseState(p);
  engine.free(p);
  if (state === 2) return result;
  const text = state === 3 ? engine.display(result) : "the expression did not settle";
  engine.free(result);
  throw new VmThrew(text);
}

/** Handles to what the prelude set up, valid in the VM and in images of it. */
interface Prepared {
  api: number;
  settle: number;
  stalled: number;
  run: number;
  setOrigin: number;
}

/**
 * VM images taken right after the prelude, per module, by everything the prelude depends on
 * (the prelude, tools, globals, store, heap limit and deterministic mode): a start with the same inputs restores
 * the image instead of running the prelude again. Most of a start is the prelude's
 * lockdown, so this makes repeated starts about 10x faster (docs/host.md "pi codemode").
 */
const imageCache = new WeakMap<object, Map<string, { image: Uint8Array; prepared: Prepared }>>();
const IMAGE_CACHE_ENTRIES = 16;
const MAX_CACHE_KEY = 64 * 1024;

/** Evaluates the host globals and pi's prelude in a fresh VM. */
function prepare(session: Session, engine: Engine, prelude: string, data: VmStartData): Prepared {
  session.polling = false;
  try {
    return prepareUnpolled(session, engine, prelude, data);
  } finally {
    session.polling = true;
  }
}

function prepareUnpolled(session: Session, engine: Engine, prelude: string, data: VmStartData): Prepared {
  // Handles live as long as the VM, which the caller disposes as a whole.
  const bridge = engine.hostFunction(BRIDGE_FN, "bridge");
  const getter = compileOrThrow(engine, "codemode-prelude.js", ["o", "k"], "return o[k]");
  // Deterministic runs have no WeakRef or FinalizationRegistry, as pi's quickjs VM leaves
  // them out: the VM deletes the globals before the prelude's lockdown (D1680).
  if (data.deterministic !== undefined) engine.setWeakRefs(2);
  const host = compileOrThrow(engine, "codemode-prelude.js", [], HOST_GLOBALS);
  const setOrigin = evaluate(session, engine, host);
  engine.free(host);
  const preludeFn = compileOrThrow(engine, "codemode-prelude.js", [], `return ${prelude}`);
  const factory = evaluate(session, engine, preludeFn);
  const args = [
    bridge,
    engine.value(JSON.stringify(data.tools)),
    engine.value(JSON.stringify(data.globals)),
    engine.value(JSON.stringify(data.store)),
  ];
  const api = session.invoke(factory, 0, args);
  for (const h of [preludeFn, factory, ...args]) engine.free(h);
  const member = (name: string): number => {
    const key = engine.value(name);
    const f = evaluate(session, engine, getter, [api, key]);
    engine.free(key);
    return f;
  };
  const prepared = { api, settle: member("settle"), stalled: member("stalled"), run: member("run"), setOrigin };
  engine.free(getter);
  return prepared;
}

/** Like pi's `startVm`: rejects only when the VM cannot be created at all. */
export async function startVm(options: StartVmOptions): Promise<VmSession> {
  const { data, post, prelude } = options;
  const module = options.wasm as WebAssembly.Module;
  const deterministic = data.deterministic;
  const key = JSON.stringify([
    prelude,
    data.tools,
    data.globals,
    data.store,
    data.memoryLimitBytes ?? null,
    data.deterministic !== undefined,
  ]);
  let cache = imageCache.get(module);
  if (!cache) imageCache.set(module, (cache = new Map()));
  const cached = options.reuseImages === false || key.length > MAX_CACHE_KEY ? undefined : cache.get(key);
  const engine = cached
    ? Engine.restore(module, cached.image, data.memoryLimitBytes)
    : Engine.create(module, data.memoryLimitBytes);
  const state: SessionState = {
    api: 0,
    settle: 0,
    stalled: 0,
    issued: 0,
    nextSeq: 1,
    current: 0,
    early: [],
    clock: deterministic ? { startedAt: deterministic.startedAt, now: deterministic.startedAt } : null,
    memoryLimit: data.memoryLimitBytes,
  };
  const session = new Session(engine, options, state);
  try {
    let prepared = cached?.prepared;
    if (!prepared) {
      prepared = prepare(session, engine, prelude, data);
      if (options.reuseImages !== false && key.length <= MAX_CACHE_KEY) {
        if (cache.size >= IMAGE_CACHE_ENTRIES) cache.delete(cache.keys().next().value!);
        cache.set(key, { image: engine.image(), prepared });
      }
    }
    Object.assign(state, { api: prepared.api, settle: prepared.settle, stalled: prepared.stalled });
    if (deterministic) {
      // djs's Math.random is QuickJS's xorshift64*. pi's quickjs VM seeds it with seed + 1 (its
      // first clock read) and draws once while creating the context, so starting djs one step
      // after seed + 1 gives the same numbers on both engines.
      const seed = xorshift(BigInt(deterministic.seed) + 1n);
      engine.setRandomSeed(Number(seed & 0xffffffffn), Number(seed >> 32n));
    } else engine.setRandomSeed(...entropy());
    const origin = engine.value(state.clock ? state.clock.startedAt : Date.now());
    engine.free(session.invoke(prepared.setOrigin, 0, [origin]));
    engine.free(origin);
    if (state.clock) {
      const budget = options.interruptBudget ?? "none";
      const fuel = options.fuelPerPoll ?? FUEL_PER_POLL;
      post({
        type: "started",
        fingerprint: `djs ${engine.buildHash}, prelude ${fnv1a(prelude)}, host ${HOST_GLOBALS_HASH}, deterministic ${DETERMINISTIC_VERSION}, fuel per poll ${fuel}, heap ${data.memoryLimitBytes ?? "none"}, budget ${budget}`,
      });
    }

    // The prefix shares the first line with the script so reported line numbers
    // match the script as written.
    const compiled = engine.compile(1, "codemode.js", [], `${SCRIPT_PREFIX}${data.code}\n})`);
    if ("error" in compiled) {
      post({ type: "done", ok: false, error: describeCompileError(compiled.error) });
      return session;
    }
    const fn = evaluate(session, engine, compiled.handle);
    engine.free(compiled.handle);
    engine.free(session.invoke(prepared.run, state.api, [fn]));
    engine.free(fn);
    session.drain();
    if (state.clock) post({ type: "idle", seq: 0, issued: state.issued });
  } catch (error) {
    post(crashMessage(error));
  }
  return session;
}

/** Continues a session from `VmSession.snapshot()` in a new instance of the same djs build. */
export function resumeVm(snapshot: Uint8Array, hooks: VmHooks): VmSession {
  const view = new DataView(snapshot.buffer, snapshot.byteOffset, snapshot.byteLength);
  if (snapshot.length < 12 || view.getUint32(0, true) !== SNAPSHOT_MAGIC || view.getUint32(4, true) !== SNAPSHOT_VERSION) {
    throw new Error("Not a pi codemode session snapshot");
  }
  const headerLength = view.getUint32(8, true);
  const state = JSON.parse(new TextDecoder().decode(snapshot.subarray(12, 12 + headerLength))) as SessionState;
  const engine = Engine.restore(hooks.wasm as WebAssembly.Module, snapshot.subarray(12 + headerLength), state.memoryLimit);
  return new Session(engine, hooks, state);
}

function compileOrThrow(engine: Engine, file: string, params: string[], src: string): number {
  const compiled = engine.compile(1, file, params, src);
  if ("error" in compiled) throw new Error(`${file}: ${compiled.error}`);
  return compiled.handle;
}
