// What a trapped djs-wasm instance says about the trap (D2543).

const textDecoder = new TextDecoder();

/** The exports `trapText` reads. */
export interface TrapExports {
  memory: WebAssembly.Memory;
  djs_panic_ptr(): number;
  djs_panic_len(): number;
  /** The shadow stack pointer (djs-wasm exports it for this). */
  __stack_pointer: WebAssembly.Global;
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
  const sp = (x.__stack_pointer.value as number) >>> 0;
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
/** `vm_compile_file`'s result for such a compile (`u32::MAX` as an i32). */
const COMPILE_NEED_STACK = -1;
/** `vm_run`'s status for an instruction that needs Unicode tables the module leaves out (D3350). */
const NEED_TABLES = 7;
/** `vm_compile_file`'s result for a compile that needs them (`u32::MAX - 1` as an i32). */
const COMPILE_NEED_TABLES = -2;
/** The larger stack: the parser's deepest accepted nesting needs at most ~115 KiB (D2540). */
const BIG_STACK = 128 * 1024;

/** The exports `elasticStack` wraps and uses. */
export interface StackExports {
  memory: WebAssembly.Memory;
  __stack_pointer: WebAssembly.Global;
  /** Modules with the cold Unicode tables only (D3350). */
  djs_tables_ptr?(family: number, len: number): number;
  djs_tables_set?(): number;
  vm_tables_need(vm: number): number;
  djs_elastic_stack(on: number): void;
  djs_stack(len: number): number;
  vm_run(vm: number, fuel: number): number;
  vm_fuel_used(vm: number): number;
  vm_compile_file(vm: number, goal: number, nameLen: number, paramsLen: number, srcLen: number): number;
}

/**
 * The exports of a djs-wasm instance with an elastic stack (D3280): the module links a small
 * shadow stack, enough for any script short of deep nesting. A compile that nests deeper (in
 * `vm_compile_file`, or at run time: `eval`, `Function`, `new RegExp`, a lazy function's first
 * call) stops before it has any effect; the wrappers then move the stack pointer to a larger
 * block for one more call and back. What compiles and its errors are the same as with a large
 * stack. The shipped module also leaves its Unicode property and normalization tables out
 * of its memory (D3350): a compile or instruction that needs them (a regular expression
 * with `\p{..}`, `normalize` or `localeCompare` of non-Latin text) stops the same way, and
 * the wrappers hand the instance the module's section of that family (`djs_props`,
 * `djs_norm`; `module` is required for such a module) and call again. Call it again after the instance's memory was reset (a pooled
 * instance). Replaces the functions in `x` (an object of `djsExports`, which is the caller's
 * own): copying its ~70 properties into a new object cost a sandbox more than the rest of its
 * JavaScript setup.
 */
export function elasticStack<X extends StackExports>(x: X, module?: WebAssembly.Module): X {
  const { __stack_pointer: sp, djs_stack: stack, vm_fuel_used: fuelUsed } = x;
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
  const tables = (vm: number) => {
    const need = x.vm_tables_need(vm);
    for (const [family, name] of TABLE_SECTIONS) {
      if (!(need & family)) continue;
      const bytes = module && tableSection(module, name);
      const at = bytes && x.djs_tables_ptr ? x.djs_tables_ptr(family, bytes.length) : 0;
      if (!bytes || at === 0) throw new Error(`djs-wasm: the program needs the module's Unicode tables (its ${name} section), which this host cannot hand over`);
      new Uint8Array(x.memory.buffer, at, bytes.length).set(bytes);
      if (x.djs_tables_set?.() !== 1) throw new Error(`djs-wasm refused the ${name} section`);
      tableLoads++;
    }
  };
  const y = x;
  const run = x.vm_run;
  // The fuel of the last run's parts that stopped for the stack or the tables, which
  // `vm_fuel_used` adds.
  let carry = 0;
  let carryVm = 0;
  y.vm_run = (vm, fuel) => {
    carry = 0;
    carryVm = vm;
    let s = run(vm, fuel);
    // Fuel 0: the run goes on with what the stopped part had left.
    for (;;) {
      if (s === NEED_TABLES) {
        carry += fuelUsed(vm);
        tables(vm);
        s = run(vm, 0);
      } else if (s === NEED_STACK) {
        s = big(() => {
          let t: number;
          do {
            carry += fuelUsed(vm);
            t = run(vm, 0);
          } while (t === NEED_STACK);
          return t;
        });
      } else {
        return s;
      }
    }
  };
  y.vm_fuel_used = (vm) => fuelUsed(vm) + (vm === carryVm ? carry : 0);
  const compile = x.vm_compile_file;
  y.vm_compile_file = (...a) => {
    let h = compile(...a);
    if (h === COMPILE_NEED_TABLES) {
      tables(a[0]);
      h = compile(...a);
    }
    return h === COMPILE_NEED_STACK ? big(() => compile(...a)) : h;
  };
  wrapped.add(y);
  return y;
}

/** The table families (`vm_tables_need`'s bits) and their sections. */
const TABLE_SECTIONS = [
  [1, "djs_props"],
  [2, "djs_norm"],
] as const;

/** A table section of the module, once per module and name. */
const tableSections = new WeakMap<WebAssembly.Module, Map<string, Uint8Array | undefined>>();
function tableSection(module: WebAssembly.Module, name: string): Uint8Array | undefined {
  let m = tableSections.get(module);
  if (!m) tableSections.set(module, (m = new Map()));
  if (!m.has(name)) {
    const [section] = WebAssembly.Module.customSections(module, name);
    m.set(name, section && new Uint8Array(section));
  }
  return m.get(name);
}

/** How often an instance was handed a family of tables in this process (for tests). */
export let tableLoads = 0;

const wrapped = new WeakSet<object>();

/** How often a call moved to the larger stack in this process (for tests). */
export let stackMoves = 0;
