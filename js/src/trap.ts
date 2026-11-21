// What a trapped djs-wasm instance says about the trap (D2543).

const textDecoder = new TextDecoder();

/** The exports `trapText` reads. */
export interface TrapExports {
  memory: WebAssembly.Memory;
  djs_panic_ptr(): number;
  djs_panic_len(): number;
  /** The shadow stack pointer (djs-wasm exports it for this). */
  __stack_pointer?: WebAssembly.Global;
}

/**
 * The text of a wasm trap `e` of the instance with exports `x`: for a stack overflow, a
 * message that says so; else the engine's panic message if it recorded one, else the
 * runtime's message. The shadow stack lies at the bottom of memory and grows down: a
 * frame that does not fit takes the stack pointer below 0, where it wraps around past the
 * end of memory, and the first store there traps as "memory access out of bounds".
 */
export function trapText(x: TrapExports, e: Error): string {
  // First: with the stack pointer wrapped, calling any export would trap again.
  const sp = x.__stack_pointer ? (x.__stack_pointer.value as number) >>> 0 : 0;
  if (sp > x.memory.buffer.byteLength) {
    return `stack overflow: the engine's wasm stack is exhausted (${e.message})`;
  }
  const msg = textDecoder.decode(new Uint8Array(x.memory.buffer, x.djs_panic_ptr(), x.djs_panic_len()));
  if (msg) return msg;
  if (/unreachable/.test(e.message)) {
    return `engine panic (an engine bug; the release module has no panic messages, a debug or native build has them): ${e.message}`;
  }
  return e.message;
}

// ---- The elastic stack (D3280) ------------------------------------------------------------

/** `vm_run`'s status for an instruction that compiles something deeper than the stack allows. */
const NEED_STACK = 6;
/** `vm_compile*`'s result for such a compile (`u32::MAX` as an i32). */
const COMPILE_NEED_STACK = -1;
/** The larger stack: the parser's deepest accepted nesting needs at most ~115 KiB (D2540). */
const BIG_STACK = 128 * 1024;

/** The exports `elasticStack` wraps and uses. */
export interface StackExports {
  __stack_pointer?: WebAssembly.Global;
  djs_elastic_stack?(on: number): void;
  djs_stack?(len: number): number;
  vm_run(vm: number, fuel: number): number;
  vm_fuel_used?(vm: number): number;
  vm_compile?(vm: number, goal: number, paramsLen: number, srcLen: number): number;
  vm_compile_file?(vm: number, goal: number, nameLen: number, paramsLen: number, srcLen: number): number;
  vm_compile_cached?(vm: number, goal: number, nameLen: number, paramsLen: number, srcLen: number, blobLen: number): number;
}

/**
 * The exports of a djs-wasm instance with an elastic stack (D3280): the module links a small
 * shadow stack, enough for any script short of deep nesting. A compile that nests deeper (in
 * `vm_compile*`, or at run time: `eval`, `Function`, `new RegExp`, a lazy function's first
 * call) stops before it has any effect; the wrappers then move the stack pointer to a larger
 * block for one more call and back. What compiles and its errors are the same as with a large
 * stack. Modules without the exports are returned as they are. Call it again after the
 * instance's memory was reset (a pooled instance). Replaces the functions in `x` (an object
 * of `djsExports`, which is the caller's own): copying its ~70 properties into a new object
 * cost a sandbox more than the rest of its JavaScript setup.
 */
export function elasticStack<X extends StackExports>(x: X): X {
  const { __stack_pointer: sp, djs_stack: stack, vm_fuel_used: fuelUsed } = x;
  if (!sp || !stack || !fuelUsed || !x.djs_elastic_stack) return x;
  x.djs_elastic_stack(1);
  // Already wrapped (a pooled instance): the static was reset with the memory, the wrappers
  // are still in place.
  if (wrapped.has(x)) return x;
  const big = <T>(f: () => T): T => {
    stackMoves++;
    const small = sp.value as number;
    sp.value = stack(BIG_STACK);
    // A trap leaves the pointer where it is: the instance is dead, `trapText` reads it.
    const r = f();
    sp.value = small;
    stack(0);
    return r;
  };
  const y = x;
  const run = x.vm_run;
  // The fuel of the last run's parts that stopped for the stack, which `vm_fuel_used` adds.
  let carry = 0;
  let carryVm = 0;
  y.vm_run = (vm, fuel) => {
    carry = 0;
    carryVm = vm;
    const st = run(vm, fuel);
    if (st !== NEED_STACK) return st;
    // Fuel 0: the run goes on with what this one had left.
    return big(() => {
      let s: number;
      do {
        carry += fuelUsed(vm);
        s = run(vm, 0);
      } while (s === NEED_STACK);
      return s;
    });
  };
  y.vm_fuel_used = (vm) => fuelUsed(vm) + (vm === carryVm ? carry : 0);
  for (const name of ["vm_compile", "vm_compile_file", "vm_compile_cached"] as const) {
    const f = x[name] as ((...a: number[]) => number) | undefined;
    if (!f) continue;
    (y as Record<string, unknown>)[name] = (...a: number[]) => {
      const h = f(...a);
      return h === COMPILE_NEED_STACK ? big(() => f(...a)) : h;
    };
  }
  wrapped.add(y);
  return y;
}

const wrapped = new WeakSet<object>();

/** How often a call moved to the larger stack in this process (for tests). */
export let stackMoves = 0;
