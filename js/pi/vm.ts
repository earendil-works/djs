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
 * Beyond pi's contract, in the `next` mode (prelude-next, D2650): `dumpSession()` writes a
 * waiting session as a continuation dump plus the adapter's state, and `resumeFromDump`
 * continues it in a new instance, possibly in another process or a later djs: the script
 * resumes where it waited, with no replay. Heap images are not a persistence format
 * (D2700); the adapter uses them only in-process, for its prelude images.
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
import { plainDump, scriptHash } from "../src/sandbox.ts";

export type { HostToWorkerMessage, VmStartData, WorkerToHostMessage } from "./protocol.ts";

export interface VmSession {
  deliver(message: HostToWorkerMessage): void;
  /**
   * `next` mode: the waiting session as a continuation dump (D2650), for `resumeFromDump`:
   * only while the VM waits (not from inside `post`). Results the host has not delivered
   * yet must be delivered to the resumed session. The session may go on either way.
   */
  dumpSession(): SessionDump;
  /**
   * `next` mode: the tool and global calls without a result yet, as pi's `call` messages
   * posted them. A session resumed from a dump does not post them again: the host decides
   * what to do with each (deliver a recorded result, issue it again, reject it).
   */
  openCalls(): Extract<WorkerToHostMessage, { type: "call" }>[];
  /** Releases the VM's wasm instance. The session must not be used afterwards. */
  dispose(): void;
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
  /**
   * Frozen builtins (djs's `frozenBuiltins` VM option, D2304): the builtins behave as frozen,
   * and assigning over an inherited builtin property defines an own one (as after a
   * lockdown). Default false; always on in the `next` mode (D2652). Part of the prelude
   * image's key.
   */
  frozenBuiltins?: boolean;
  /**
   * The prelude's shape: "pi" (default) for pi's `PRELUDE_SOURCE`, "next" for
   * tests/pi/prelude-next.ts (D2565, D2650): tools as engine async host calls, the body's
   * promise, `finished`, the output counters and the store in the adapter, so a waiting
   * session persists as a continuation dump (`dumpSession`, `resumeFromDump`).
   */
  shape?: "pi" | "next";
  /**
   * `next` mode, for tests (D2651): at every idle point the session dumps itself and goes on
   * in a new instance of the module, resumed from the dump, as a restarted host would.
   */
  dumpEveryIdle?: boolean;
  post(message: WorkerToHostMessage): void;
  shouldInterrupt(): boolean;
}

/** `resumeFromDump`'s options: the session's prelude and start data, which run again. */
export interface ResumeOptions extends VmHooks {
  prelude: string;
  data: VmStartData;
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

/**
 * Bumped whenever this module changes what a deterministic script observes. 2: every start
 * collects after the prelude (D1890), which moves later collection points, and with them
 * the out-of-memory points of allocations that are not retried (D1700). 3: the fuel between
 * polls counts across sync host calls (D2653), so scripts that make them poll at other points.
 */
const DETERMINISTIC_VERSION = 3;

const BRIDGE_FN = 1;
/** `next` mode: the async host function `call(kind, name, argsJson)` of tools and globals. */
const CALL_FN = 2;
/**
 * The identities of the host functions by id - 1 (D2970): a dump writes a held one as
 * `$djs.host("call")` and an open call as `$djs.resume(id, "call", ...)` (D2971); the
 * adapter answers `$djs.host` with the functions it gave the prelude.
 */
export const FN_NAMES = ["bridge", "call"];
/** A plain dump's hooks `$djs.resume` and `$djs.seed` (D2890). */
const HOOK_RESUME = 0x7fff_fff0;
const HOOK_SEED = 0x7fff_fff1;
/** `$djs.source(name, hash, params, 1)` (D2882): the codemode function the dump continues. */
const HOOK_SOURCE = 0x7fff_fff2;
/** `$djs.host(name)` (D2971): `bridge`, `call`, or one of the host globals by name. */
const HOOK_HOST = 0x7fff_fff3;

/** pi's limits (prelude-source.ts), which the adapter enforces in `next` mode. */
const MAX_STORE_VALUE_CHARS = 256 * 1024;
const MAX_STORE_TOTAL_CHARS = 1024 * 1024;
const MAX_OUTPUT_CHARS = 16 * 1024 * 1024;
const MAX_OUTPUT_ITEMS = 100_000;
const STALLED = JSON.stringify({
  name: "Error",
  message: "The script is waiting on a promise that can never settle: no tool call is pending, and timers do not exist here.",
});

/**
 * A waiting `next` session as a continuation dump plus the adapter's state (D2650), JSON
 * serializable. `resumeFromDump` continues it from the session's prelude and start data.
 */
export interface SessionDump {
  /** The VM as a dump (a plain Script, D2830): the script's data, activations and open calls. */
  dump: string;
  /**
   * The adapter's state, without VM handles and without its call table: the dump carries
   * each open call's pi id as the call's state (`$djs.resume(4, "call", [...], {"id":2})`,
   * D2770), and a resumed session rebuilds the table from it.
   */
  state: Omit<SessionState, "api" | "settle" | "stalled" | "next"> & { next: Omit<NextState, "handles" | "ids"> };
}

/** An open tool or global call's state in a dump: its id in pi's protocol. */
interface CallState {
  id: number;
}
const SCRIPT_PREFIX = "return (async (tools, console) => {";

/**
 * Globals quickjs-wasi's host layer gives pi's scripts that djs does not have (`queueMicrotask`
 * is native since 0059, `atob`/`btoa` since 0084), defined before
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
// Frozen, as pi's lockdown leaves it, also where frozen builtins replace the lockdown.
Object.freeze(performance.now);
Object.freeze(performance);
define("performance", performance);
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

/** `next` mode's handles: the prelude's functions and the body's promise. */
interface NextHandles {
  api: number;
  /** The host functions the prelude got, by their identities (`$djs.host`, D2971). */
  bridge: number;
  call: number;
  decode: number;
  failure: number;
  encode: number;
  describe: number;
  /** The body's promise; 0 before the script starts or when `run` threw. */
  body: number;
}

/** What the adapter keeps in `next` mode, which pi's prelude keeps itself (D2565). */
interface NextState {
  handles: NextHandles;
  finished: boolean;
  /** The store: key to JSON text, and the keys written (undefined: deleted), in order. */
  stored: [string, string][];
  writes: [string, string | null][];
  outputChars: number;
  outputItems: number;
  /** pi's call ids by the engine's host call ids, and the next pi id. */
  ids: [number, number][];
  nextId: number;
  /** `performance.now()`'s origin, set again on resume. */
  origin: number;
}

/** What the adapter keeps outside the VM; a dump carries it beside the VM's dump. */
interface SessionState {
  /** `next` mode (D2650); absent in pi's. */
  next?: NextState;
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
  /** The VM; `next` mode with `dumpEveryIdle` moves the session to new instances. */
  engine: Engine;
  private readonly hooks: VmHooks;
  readonly state: SessionState;
  private zoneOffset = 0;
  private zoneCheckedAt = -Infinity;
  /**
   * Whether fuel slices poll `shouldInterrupt`. Off while the host globals and pi's prelude
   * run (trusted, bounded code): a start that reuses a prelude image skips that work, so
   * polling it would make the budget a cold start has left differ from a warm one (D1660).
   */
  polling = true;
  /**
   * While pi's prelude runs: the clock reads 0 in UTC, so the prelude image is a function of
   * its inputs alone (D2262), and a session resumed from a dump reruns the same prelude. Nothing in the host
   * globals or the prelude reads the time; each run sets the real clock before its first
   * slice.
   */
  fixedClock = false;
  /** The prelude image the session started from. */
  prelude: PreludeImage | undefined;
  /** `next` mode: what moving to a new instance needs (`dumpEveryIdle`). */
  resume: { prelude: string; data: VmStartData } | undefined;
  /** The prelude's shape (`VmHooks.shape`). */
  readonly shape: "pi" | "next";
  /** While a plain dump loads: its hooks are served (`loadPlain`). */
  private loading = false;
  /** While a dump loads: the codemode function `$djs.source` returns, and its hash. */
  private source: { handle: number; hash: string } | undefined;
  /** `$djs.seed`'s next call id and the highest reopened one, for the counter (D2897). */
  private plainIds: [number | undefined, number] = [undefined, 0];
  /** While a plain dump loads: `$djs.host`'s values by name, as handles (D2971). */
  private hostValues = new Map<string, number>();

  constructor(engine: Engine, hooks: VmHooks, state: SessionState) {
    this.engine = engine;
    this.hooks = hooks;
    this.state = state;
    this.early = new Map(state.early);
    this.fuel = hooks.fuelPerPoll ?? FUEL_PER_POLL;
    this.shape = hooks.shape ?? "pi";
  }

  deliver(message: HostToWorkerMessage): void {
    if (this.disposed) return;
    const { state } = this;
    try {
      const clock = state.clock;
      if (!clock) {
        this.apply(message);
        this.afterIdle();
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
      this.afterIdle();
    } catch (error) {
      this.hooks.post(crashMessage(error));
    }
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.engine.dispose();
  }

  /** Results that arrived early (deterministic mode), as a resumed session has them. */
  restoreEarly(early: [number, HostToWorkerMessage][]): void {
    this.early.clear();
    for (const [seq, m] of early) this.early.set(seq, m);
  }

  dumpSession(): SessionDump {
    if (this.disposed) throw new Error("The session is disposed");
    const next = this.state.next;
    if (!next) throw new Error("dumpSession needs the next mode (shape: \"next\" with prelude-next)");
    const host = this.resume ? hostGlobals(this.resume.data) : [];
    const handles: [string, number][] = next.handles.body !== 0 ? [["run", next.handles.body]] : [];
    const states = next.ids.map(([callId, id]): [number, string] => [callId, JSON.stringify({ id } satisfies CallState)]);
    const dump = this.engine.dump(host, FN_NAMES, handles, states);
    const { handles: _, ids: _i, ...rest } = next;
    const { api: _a, settle: _s, stalled: _t, next: _n, ...state } = this.state;
    return { dump, state: structuredClone({ ...state, early: [...this.early], next: rest }) };
  }

  openCalls(): Extract<WorkerToHostMessage, { type: "call" }>[] {
    const next = this.state.next;
    if (!next) throw new Error("openCalls needs the next mode");
    return next.ids.map(([callId, id]) => {
      const [kind, name, json] = this.engine.hostCallInfo(callId).args;
      return { type: "call" as const, id, target: kind === "call" ? ("tool" as const) : ("global" as const), name: String(name), args: json === undefined ? undefined : String(json) };
    });
  }

  /** Settles call `message.id` in the VM and runs the script until it waits again. */
  private apply(message: HostToWorkerMessage): void {
    if (this.state.next) {
      this.applyNext(message);
      return;
    }
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

  /** `next` mode: settles the engine's host call with the decoded result (D2650). */
  private applyNext(message: HostToWorkerMessage): void {
    const { engine } = this;
    const next = this.state.next!;
    const index = next.ids.findIndex(([, id]) => id === message.id);
    // An unknown or settled id is ignored, as pi's settle does.
    if (index < 0) return;
    const [callId] = next.ids[index];
    next.ids.splice(index, 1);
    const payload = engine.value(message.payload);
    let outcome: { handle: number; threw: boolean };
    try {
      if (!message.ok) {
        const properties = engine.value(message.properties);
        outcome = this.callFn(next.handles.failure, [payload, properties]);
        engine.free(properties);
        outcome.threw = true;
      } else if (message.payload === undefined) outcome = { handle: 0, threw: false };
      else outcome = this.callFn(next.handles.decode, [payload]);
    } finally {
      engine.free(payload);
    }
    engine.settleHostCall(callId, !outcome.threw, outcome.handle);
    engine.free(outcome.handle);
    this.drain();
  }

  /** Fails a script that waits on nothing that can ever resume it (pi's `stalled`). */
  drain(): void {
    if (this.state.next) {
      const status = this.loop();
      if (status === STATUS_THREW) {
        const h = this.engine.result();
        const text = this.engine.display(h);
        this.engine.free(h);
        throw new VmThrew(text);
      }
      this.drainNext(status);
      return;
    }
    this.engine.free(this.invoke(this.state.stalled, this.state.api, []));
  }

  /**
   * `next` mode, once the VM is idle or done: reports the body's outcome when it settled,
   * and a script that waits on nothing (no host call open) as stalled.
   */
  private drainNext(status: number): void {
    const { engine } = this;
    const next = this.state.next!;
    const body = next.handles.body;
    if (next.finished || body === 0) return;
    const { state, result } = engine.promiseState(body);
    // Not a promise: `run` threw, and reported it.
    if (state === 0) return;
    if (state === 1) {
      if (status !== STATUS_IDLE) this.done(false, STALLED);
      return;
    }
    let ok = state === 2;
    let text: unknown;
    if (ok) {
      const encoded = this.callFn(next.handles.encode, [result]);
      if (encoded.threw) {
        ok = false;
        text = this.describe(encoded.handle);
      } else text = engine.read(encoded.handle);
      engine.free(encoded.handle);
    } else text = this.describe(result);
    engine.free(result);
    this.done(ok, text === undefined ? undefined : String(text));
  }

  private describe(error: number): string {
    const d = this.callFn(this.state.next!.handles.describe, [error]);
    const text = String(this.engine.read(d.handle));
    this.engine.free(d.handle);
    return text;
  }

  /** `next` mode's `done`: once; a success carries the store's writes. */
  private done(ok: boolean, payload: string | undefined): void {
    const next = this.state.next!;
    if (next.finished) return;
    next.finished = true;
    const state = this.state;
    const during = state.clock ? { during: { seq: state.current, issued: state.issued } } : {};
    if (ok) {
      const writes = JSON.stringify(next.writes.map(([key, json]) => (json === null ? [key] : [key, json])));
      this.hooks.post({ type: "done", ok: true, value: payload, writes, ...during });
    } else this.hooks.post({ type: "done", ok: false, error: String(payload), ...during });
  }

  /**
   * `next` mode: calls `f` and runs the VM until it is idle again. Its return value or
   * exception (`threw`), also when async host calls are pending (`vm_outcome`).
   */
  callFn(f: number, args: number[]): { handle: number; threw: boolean } {
    const { engine } = this;
    engine.call(f, 0, args);
    const status = this.loop();
    if (status === STATUS_THREW) return { handle: engine.result(), threw: true };
    if (status === STATUS_DONE) return { handle: engine.result(), threw: false };
    return engine.outcome();
  }

  /**
   * After the VM went idle, outside `post`: in `next` mode with `dumpEveryIdle`, the session
   * moves to a new instance through a dump while it waits (D2651).
   */
  afterIdle(): void {
    const next = this.state.next;
    if (!next || !this.hooks.dumpEveryIdle || next.finished || next.ids.length === 0 || !this.resume) return;
    const dumped = this.dumpSession();
    const fresh = resumeSession(dumped, { ...this.hooks, ...this.resume });
    this.engine.dispose();
    this.engine = fresh.engine;
    Object.assign(this.state, fresh.state);
    this.prelude = fresh.prelude;
  }

  /**
   * Calls `f` and runs the VM, its promise jobs included, until it is idle again. Returns
   * the result's handle (the caller frees it). Throws VmThrew for an uncaught exception.
   */
  invoke(f: number, thisArg: number, args: number[]): number {
    const { engine } = this;
    engine.call(f, thisArg, args);
    const status = this.loop();
    if (status === STATUS_DONE) return engine.result();
    if (status === STATUS_THREW) {
      const h = engine.result();
      const text = engine.display(h);
      engine.free(h);
      throw new VmThrew(text);
    }
    return 0;
  }

  /** Runs the VM until it is idle or done; returns that status (THREW: the result is the exception). */
  loop(): number {
    const { engine } = this;
    // The fuel left until the next poll: counted across sync host calls, which end a
    // `vm_run` early, so a loop of them still polls (D2653).
    let left = this.fuel;
    const poll = () => {
      left = this.fuel;
      if (this.polling && this.hooks.shouldInterrupt()) {
        engine.abort();
        throw new Error("Execution interrupted");
      }
    };
    for (;;) {
      this.setClock();
      const status = engine.run(left);
      left -= engine.fuelUsed(left);
      if (this.state.next) this.postCalls();
      switch (status) {
        case STATUS_OUT_OF_FUEL:
          poll();
          continue;
        case STATUS_HOST_CALL: {
          const { callId, fnId, args: callArgs } = engine.hostCall();
          if (this.loading && (fnId === HOOK_RESUME || fnId === HOOK_SEED || fnId === HOOK_SOURCE || fnId === HOOK_HOST)) {
            this.serveHook(callId, fnId, callArgs);
            continue;
          }
          let answer = 0;
          try {
            if (fnId !== BRIDGE_FN) throw new Error(`Unknown host function ${fnId}`);
            answer = engine.value(this.bridge(callArgs));
          } catch (error) {
            engine.abort();
            throw error;
          }
          engine.resolveHostCall(callId, answer);
          engine.free(answer);
          if (left <= 0) poll();
          continue;
        }
        case STATUS_DONE:
        case STATUS_THREW:
        case STATUS_IDLE:
          return status;
        case STATUS_HOST_OUT_OF_MEMORY:
          // Not a script error: the engine abandoned the call; pi reports a crash (D1690).
          throw new Error("out of host memory");
        default:
          throw new Error(`Unexpected VM status ${status}`);
      }
    }
  }

  /**
   * Loads the plain dump `text` (D2830) into this session's VM, which ran the host globals
   * and the prelude: runs it as a Script with `$djs` = the dump runtime (D2903) and
   * {resume, seed, source, host} to its first stop that is not a sync call (the dump
   * deletes `$djs` at its end, the adapter too). Each `$djs.resume` reopens a `call` under its engine id and takes pi's id from
   * its state into the call table. Returns the run's promise (the Script's completion
   * value; the caller owns the handle). Throws when the dump does not load.
   */
  loadPlain(text: string, code?: string): number {
    const { engine } = this;
    const compiled = engine.compile(plainDump(text)!.typescript ? 2 : 0, "dump.js", [], text);
    if ("error" in compiled) throw new Error(`Cannot load the dump: ${compiled.error}`);
    // `$djs.source`'s answer (D2882): the codemode function, made again by its wrapper,
    // which runs none of the script's code.
    if (code !== undefined) {
      const wrapper = `${SCRIPT_PREFIX}${code}\n})`;
      const w = compileOrThrow(engine, "codemode.js", [], wrapper);
      const f = evaluate(this, engine, w);
      engine.free(w);
      this.source = { handle: f, hash: scriptHash(wrapper, "", 1) };
    }
    // `$djs.host`'s answers (D2971): the prelude's own `bridge` and `call`, and the host
    // globals' values, read now (the hook cannot run JS while the dump waits on it).
    const fns0 = this.state.next!.handles;
    const values = new Map<string, number>([["bridge", fns0.bridge], ["call", fns0.call]]);
    const read = compileOrThrow(engine, "dump.js", ["k"], "return globalThis[k]");
    for (const name of this.resume ? hostGlobals(this.resume.data) : []) {
      const k = engine.value(name);
      values.set(name, evaluate(this, engine, read, [k]));
      engine.free(k);
    }
    engine.free(read);
    this.hostValues = values;
    const hooks = engine.dumpRuntime(text);
    const fns: [string, number][] = [
      ["resume", engine.hostFunction(HOOK_RESUME, "resume")],
      ["seed", engine.hostFunction(HOOK_SEED, "seed")],
      ["source", engine.hostFunction(HOOK_SOURCE, "source")],
      ["host", engine.hostFunction(HOOK_HOST, "host")],
    ];
    for (const [name, h] of fns) {
      engine.define(hooks, name, h);
      engine.free(h);
    }
    engine.define(0, "$djs", hooks);
    engine.free(hooks);
    this.loading = true;
    this.plainIds = [undefined, 0];
    let status: number;
    try {
      engine.call(compiled.handle, 0, []);
      engine.free(compiled.handle);
      status = this.loop();
    } finally {
      this.loading = false;
      if (this.source) engine.free(this.source.handle);
      this.source = undefined;
      for (const [name, h] of this.hostValues) if (name !== "bridge" && name !== "call") engine.free(h);
      this.hostValues = new Map();
    }
    if (status === STATUS_THREW) {
      const h = engine.result();
      const why = engine.display(h);
      engine.free(h);
      throw new Error(`Cannot load the dump: the dump threw: ${why}`);
    }
    const run = status === STATUS_DONE ? engine.result() : engine.outcome().handle;
    const [next, last] = this.plainIds;
    if (next !== undefined) engine.setNextCallId(Math.max(next, last + 1));
    // The backstop for a dump that leaves `$djs` (D2890): an undefined property is no
    // hook, and the next dump would refuse it, so take it out by running JS.
    const drop = engine.compile(0, "dump.js", [], "delete globalThis.$djs");
    if (!("error" in drop)) this.invoke(drop.handle, 0, []);
    return run;
  }

  /** Serves `$djs.resume(id, name, args, state)`, `$djs.seed(lo, hi, next)`, `$djs.source` and `$djs.host` while a dump loads. */
  private serveHook(callId: number, fnId: number, [a, b, c, d]: unknown[]): void {
    const { engine } = this;
    let why: string | undefined;
    if (fnId === HOOK_HOST) {
      const h = typeof a === "string" ? this.hostValues.get(a) : undefined;
      if (h !== undefined) {
        engine.resolveHostCall(callId, h);
        return;
      }
      why = `host: unknown host value ${String(a)}`;
    } else if (fnId === HOOK_SOURCE) {
      if (a === "codemode.js" && this.source && b === this.source.hash) {
        engine.resolveHostCall(callId, this.source.handle);
        return;
      }
      why = `source: the dump continues the script ${String(a)} (${String(b)}), not this session's code`;
    } else if (fnId === HOOK_SEED) {
      if (typeof a === "number" && typeof b === "number") engine.setMathRandomSeed(a >>> 0, b >>> 0);
      else why = "seed: not two int32 halves";
      if (typeof c === "number") this.plainIds[0] = c;
    } else {
      const id = (d as Partial<CallState> | undefined)?.id;
      if (b !== "call" || typeof id !== "number") why = `resume: call ${String(a)} is not a tool call with a pi id`;
      else {
        why = engine.openCall(callId, CALL_FN);
        if (why === undefined) {
          this.state.next!.ids.push([a as number, id]);
          this.plainIds[1] = Math.max(this.plainIds[1], a as number);
          return;
        }
      }
    }
    if (why === undefined) engine.resolveHostCall(callId, 0);
    else {
      const h = engine.value(why);
      engine.settleHostCall(callId, false, h);
      engine.free(h);
    }
  }

  /** `next` mode: the tool and global calls the script made, as pi's `call` messages. */
  private postCalls(): void {
    const next = this.state.next!;
    for (const callId of this.engine.newHostCalls()) {
      const { fnId, args } = this.engine.hostCallInfo(callId);
      if (fnId !== CALL_FN) continue;
      const [kind, name, json] = args;
      const id = next.nextId++;
      next.ids.push([callId, id]);
      this.state.issued++;
      this.hooks.post({
        type: "call",
        id,
        target: kind === "call" ? "tool" : "global",
        name: String(name),
        args: json === undefined ? undefined : String(json),
      });
    }
  }

  private setClock(): void {
    const clock = this.state.clock;
    if (this.fixedClock) this.engine.setClock(0, 0);
    else if (clock) this.engine.setClock(clock.now, 0);
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

  /** The prelude's host bridge, called with primitives only; returns the answer (`next` mode). */
  private bridge([kind, a, b, c]: unknown[]): unknown {
    const { post } = this.hooks;
    const state = this.state;
    const next = state.next;
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
      case "output": {
        if (next) {
          if (next.finished) return undefined;
          next.outputChars += String(b).length;
          next.outputItems++;
          if (next.outputChars > MAX_OUTPUT_CHARS || next.outputItems > MAX_OUTPUT_ITEMS) {
            return `script output exceeded the limit of ${MAX_OUTPUT_CHARS} characters or ${MAX_OUTPUT_ITEMS} text(), image(), and console calls. Print a summary instead, or write large data to a file with a tool.`;
          }
        }
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
      }
      case "store":
        return next ? this.store(next, String(a), b === undefined ? undefined : String(b)) : undefined;
      case "load":
        return next ? next.stored.find(([key]) => key === String(a))?.[1] : undefined;
      case "done": {
        if (next) {
          this.done(a === true, b === undefined ? undefined : String(b));
          break;
        }
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
    return undefined;
  }

  /** `next` mode's `store(key, json)`: pi's limits; undefined, or the limit's message. */
  private store(next: NextState, key: string, json: string | undefined): string | undefined {
    const at = next.stored.findIndex(([k]) => k === key);
    const previous = at >= 0 ? key.length + next.stored[at][1].length : 0;
    const used = next.stored.reduce((n, [k, v]) => n + k.length + v.length, 0);
    const record = (value: string | null) => {
      const w = next.writes.findIndex(([k]) => k === key);
      if (w >= 0) next.writes[w][1] = value;
      else next.writes.push([key, value]);
    };
    if (json === undefined) {
      if (at >= 0) next.stored.splice(at, 1);
      record(null);
      return undefined;
    }
    if (json.length > MAX_STORE_VALUE_CHARS) {
      return `store(${JSON.stringify(key)}) value has ${json.length} characters of JSON, more than the limit of ${MAX_STORE_VALUE_CHARS}. `;
    }
    if (used - previous + key.length + json.length > MAX_STORE_TOTAL_CHARS) {
      return `store is full: stored values would exceed ${MAX_STORE_TOTAL_CHARS} characters of JSON. Delete keys with store(key, undefined). `;
    }
    if (at >= 0) next.stored[at][1] = json;
    else next.stored.push([key, json]);
    record(json);
    return undefined;
  }
}

/**
 * Whether the VM has frozen builtins (D2304): asked for, or the `next` mode, whose prelude
 * has no lockdown (D2652).
 */
function frozenOf(hooks: VmHooks): boolean {
  return hooks.frozenBuiltins === true || hooks.shape === "next";
}

/** The globals the host globals and the prelude define: a dump names them, never writes them. */
function hostGlobals(data: VmStartData): string[] {
  const names = new Set(["performance", "tools", "ALL_TOOLS", "console", "text", "image", "exit", "store", "load"]);
  for (const { name } of data.globals) names.add(name.split(".")[0]);
  return [...names];
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
  /** `next` mode: prelude-next's functions. */
  next?: Omit<NextHandles, "body">;
}

/**
 * VM images taken right after the prelude, per module, by everything the prelude depends on
 * (the prelude, tools, globals, store, heap limit and deterministic mode): a start with the same inputs restores
 * the image instead of running the prelude again. Most of a start is the prelude's
 * lockdown, so this makes repeated starts about 10x faster (docs/host.md "pi codemode").
 */
const imageCache = new WeakMap<object, Map<string, PreludeImage>>();
const IMAGE_CACHE_ENTRIES = 16;
const MAX_CACHE_KEY = 64 * 1024;
/**
 * The heap size the last cold start sealed after its prelude: the next one starts with the
 * room for it (D2043). A hint only: 0 before the first.
 */
let sealedSize = 0;

/** A prelude image: the collected VM right after the prelude, and its handles. */
interface PreludeImage {
  image: Uint8Array;
  prepared: Prepared;
}

/** Everything the prelude image depends on, besides the engine build. */
function preludeKey(prelude: string, data: VmStartData, frozenBuiltins = false): string {
  return JSON.stringify([
    prelude,
    data.tools,
    data.globals,
    data.store,
    data.memoryLimitBytes ?? null,
    data.deterministic !== undefined,
    ...(frozenBuiltins ? ["frozen builtins"] : []),
  ]);
}

/** Keeps `entry` in the module's prelude images, dropping the oldest beyond the limit. */
function cachePreludeImage(module: object, key: string, entry: PreludeImage): void {
  if (key.length > MAX_CACHE_KEY) return;
  let cache = imageCache.get(module);
  if (!cache) imageCache.set(module, (cache = new Map()));
  if (cache.size >= IMAGE_CACHE_ENTRIES) cache.delete(cache.keys().next().value!);
  cache.set(key, entry);
}

/** Evaluates the host globals and pi's prelude in a fresh VM. */
function prepare(session: Session, engine: Engine, prelude: string, data: VmStartData): Prepared {
  session.polling = false;
  session.fixedClock = true;
  try {
    return prepareUnpolled(session, engine, prelude, data);
  } finally {
    session.polling = true;
    session.fixedClock = false;
  }
}

function prepareUnpolled(session: Session, engine: Engine, prelude: string, data: VmStartData): Prepared {
  // Handles live as long as the VM, which the caller disposes as a whole.
  const shape = session.shape;
  const bridge = engine.hostFunction(BRIDGE_FN, "bridge");
  // `next` mode: tools and globals are calls of this async host function (D2650).
  const call = shape === "next" ? engine.asyncHostFunction(CALL_FN, "call") : 0;
  const getter = compileOrThrow(engine, "codemode-prelude.js", ["o", "k"], "return o[k]");
  // Deterministic runs have no WeakRef or FinalizationRegistry, as pi's quickjs VM leaves
  // them out: the VM deletes the globals before the prelude's lockdown (D1680).
  if (data.deterministic !== undefined) engine.setWeakRefs(2);
  const host = compileOrThrow(engine, "codemode-prelude.js", [], HOST_GLOBALS);
  const setOrigin = evaluate(session, engine, host);
  engine.free(host);
  const preludeFn = compileOrThrow(engine, "codemode-prelude.js", [], `return ${prelude}`);
  const factory = evaluate(session, engine, preludeFn);
  const args =
    shape === "next"
      ? [bridge, call, engine.value(JSON.stringify(data.tools)), engine.value(JSON.stringify(data.globals))]
      : [
          bridge,
          engine.value(JSON.stringify(data.tools)),
          engine.value(JSON.stringify(data.globals)),
          engine.value(JSON.stringify(data.store)),
        ];
  const api = session.invoke(factory, 0, args);
  // `next` mode keeps `bridge` and `call`: a dump's `$djs.host` gets them (D2971).
  for (const h of [preludeFn, factory, ...args]) if (shape !== "next" || (h !== bridge && h !== call)) engine.free(h);
  const member = (name: string): number => {
    const key = engine.value(name);
    const f = evaluate(session, engine, getter, [api, key]);
    engine.free(key);
    return f;
  };
  const prepared: Prepared =
    shape === "next"
      ? {
          api,
          settle: 0,
          stalled: 0,
          run: member("run"),
          setOrigin,
          next: { api, bridge, call, decode: member("decode"), failure: member("failure"), encode: member("encode"), describe: member("describe") },
        }
      : { api, settle: member("settle"), stalled: member("stalled"), run: member("run"), setOrigin };
  engine.free(getter);
  return prepared;
}

/**
 * A new session's VM: a restored prelude image, or a fresh VM. Throws only when the VM
 * cannot be created; `setUp` does the rest.
 */
function newSession(options: VmHooks, prelude: string, data: VmStartData): { session: Session; cached: PreludeImage | undefined } {
  const module = options.wasm as WebAssembly.Module;
  const deterministic = data.deterministic;
  const frozen = frozenOf(options);
  const key = preludeKey(prelude, data, frozen);
  const cached = options.reuseImages === false ? undefined : imageCache.get(module)?.get(key);
  const engine = cached
    ? Engine.restore(module, cached.image, data.memoryLimitBytes)
    : Engine.create(module, data.memoryLimitBytes, sealedSize, frozen);
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
  session.resume = { prelude, data };
  return { session, cached };
}

/**
 * Runs the host globals and the prelude (or takes them from the prelude image), seeds
 * `Math.random` and sets `performance.now()`'s origin.
 */
function setUp(session: Session, cached: PreludeImage | undefined, options: VmHooks, prelude: string, data: VmStartData, origin: number): Prepared {
  const { engine, state } = session;
  const module = options.wasm as WebAssembly.Module;
  const deterministic = data.deterministic;
  const frozen = frozenOf(options);
  const key = preludeKey(prelude, data, frozen);
  let prepared = cached?.prepared;
  session.prelude = cached;
  if (!prepared) {
    // Outside deterministic mode the hash key (D2433) is chosen once, before the
    // prelude: its image and every session restored from it keep that key, and each
    // session only reseeds Math.random (rekeying would rehash the prelude's atoms and
    // tables in every session).
    if (!deterministic) engine.setRandomSeed(...entropy());
    prepared = prepare(session, engine, prelude, data);
    // Every cold start collects after the prelude (D1890), cached or not: the prelude
    // image then holds only live cells, and cold and warm starts go on from the same heap.
    // Sealed (D2041): the prelude's state is old, so the collections while scripts run
    // only trace and move what they made.
    engine.seal();
    // The next cold start reserves for this size up front (D2043): growing the heap
    // while the prelude runs moved it, and the old block stayed behind as a hole.
    sealedSize = engine.heapSize;
    if (options.reuseImages !== false && key.length <= MAX_CACHE_KEY) {
      const entry = { image: engine.image(), prepared };
      cachePreludeImage(module, key, entry);
      session.prelude = entry;
    }
  }
  Object.assign(state, { api: prepared.api, settle: prepared.settle, stalled: prepared.stalled });
  if (prepared.next) {
    state.next = {
      handles: { ...prepared.next, body: 0 },
      finished: false,
      stored: Object.entries(data.store),
      writes: [],
      outputChars: 0,
      outputItems: 0,
      ids: [],
      nextId: 1,
      origin,
    };
  }
  if (deterministic) {
    // djs's Math.random is QuickJS's xorshift64*. pi's quickjs VM seeds it with seed + 1 (its
    // first clock read) and draws once while creating the context, so starting djs one step
    // after seed + 1 gives the same numbers on both engines.
    const seed = xorshift(BigInt(deterministic.seed) + 1n);
    // Keys the hash from the seed too (D2433): same seed, same layout, cold or warm.
    engine.setRandomSeed(Number(seed & 0xffffffffn), Number(seed >> 32n));
  } else engine.setMathRandomSeed(...entropy());
  const h = engine.value(origin);
  engine.free(session.invoke(prepared.setOrigin, 0, [h]));
  engine.free(h);
  return prepared;
}

/** Like pi's `startVm`: rejects only when the VM cannot be created at all. */
export async function startVm(options: StartVmOptions): Promise<VmSession> {
  const { data, post, prelude } = options;
  const frozen = frozenOf(options);
  const { session, cached } = newSession(options, prelude, data);
  const { engine, state } = session;
  try {
    const prepared = setUp(session, cached, options, prelude, data, state.clock ? state.clock.startedAt : Date.now());
    if (state.clock) {
      const budget = options.interruptBudget ?? "none";
      const fuel = options.fuelPerPoll ?? FUEL_PER_POLL;
      post({
        type: "started",
        fingerprint: `djs ${engine.buildHash}, prelude ${fnv1a(prelude)}, host ${HOST_GLOBALS_HASH}, deterministic ${DETERMINISTIC_VERSION}, fuel per poll ${fuel}, heap ${data.memoryLimitBytes ?? "none"}, budget ${budget}${frozen ? ", frozen builtins" : ""}`,
      });
    }

    // The prefix shares the first line with the script so reported line numbers
    // match the script as written.
    const compiled = engine.compile(1, "codemode.js", [], `${SCRIPT_PREFIX}${data.code}\n})`);
    if ("error" in compiled) {
      post({ type: "done", ok: false, error: describeCompileError(compiled.error) });
      if (state.next) state.next.finished = true;
      return session;
    }
    const fn = evaluate(session, engine, compiled.handle);
    engine.free(compiled.handle);
    if (state.next) {
      // The body's promise is the adapter's (D2650).
      state.next.handles.body = session.callFn(prepared.run, [fn]).handle;
    } else engine.free(session.invoke(prepared.run, state.api, [fn]));
    engine.free(fn);
    session.drain();
    if (state.clock) post({ type: "idle", seq: 0, issued: state.issued });
    session.afterIdle();
  } catch (error) {
    post(crashMessage(error));
  }
  return session;
}

/**
 * Continues a `next` session from `VmSession.dumpSession()` (D2650) in a new instance of a
 * djs module with the dumper (the `dump` feature): the host globals and prelude-next run
 * again (or come from the prelude image), the dump recreates the script's state and its
 * open calls, and the adapter's state is restored. `prelude` and `data` must be the
 * session's. Results the old session did not take are delivered to this one; the host
 * decides what happens to calls that were open (deliver a recorded result, issue the call
 * again, reject it). Throws when the dump does not load.
 */
export function resumeFromDump(dumped: SessionDump, options: ResumeOptions): VmSession {
  return resumeSession(dumped, options);
}

function resumeSession(dumped: SessionDump, options: VmHooks & { prelude: string; data: VmStartData }): Session {
  const hooks = { ...options, shape: "next" as const };
  const { prelude, data } = options;
  const { session, cached } = newSession(hooks, prelude, data);
  try {
    const { engine, state } = session;
    const saved = structuredClone(dumped.state);
    setUp(session, cached, hooks, prelude, data, saved.next.origin);
    const prepared = state.next!.handles;
    if (!plainDump(dumped.dump)) throw new Error('Cannot load the dump: not a djs dump (no "djs-dump"; header)');
    // The adapter's state first: the dump's code runs while it loads (D2891).
    Object.assign(state, { ...saved, api: state.api, settle: 0, stalled: 0, next: { ...saved.next, ids: [], handles: { ...prepared, body: 0 } } });
    session.restoreEarly(saved.early);
    const body = session.loadPlain(dumped.dump, data.code);
    state.next!.ids.sort((a, b) => a[1] - b[1]);
    state.next!.handles.body = body;
    return session;
  } catch (error) {
    session.dispose();
    throw error;
  }
}

function compileOrThrow(engine: Engine, file: string, params: string[], src: string): number {
  const compiled = engine.compile(1, file, params, src);
  if ("error" in compiled) throw new Error(`${file}: ${compiled.error}`);
  return compiled.handle;
}
