// The ABI of djs-wasm's instances (D3320): the module exports one function, `djs(op, a, b,
// c, d, e, f, t)`, which runs operation `op` (numbered by `OPS`, crates/djs-wasm/src/lib.rs
// `ops!`; a test there compares the two tables) with integer arguments `a` to `f` and the
// float `t`. One export instead of one per operation: V8 makes a function object for every
// export of every instance, and 75 of them were a quarter of `new WebAssembly.Instance`.
// `djsExports` turns an instance into the functions by name that the hosts call.

/** The operations of `djs`, by number. */
export const OPS = [
  "djs_abi_version", "djs_build_hash", "djs_base_ptr", "djs_base_inject_ptr", "djs_base_inject",
  "djs_base_coded_ptr", "djs_base_len", "djs_staging", "djs_out_ptr", "djs_out_len",
  "djs_panic_ptr", "djs_panic_len", "djs_debug_panic", "vm_new", "vm_frozen_builtins",
  "vm_new_compiled", "vm_free", "vm_arena_ptr", "vm_arena_len", "vm_collect", "vm_compile_file",
  "vm_set_dump_source_refs", "vm_call_site", "djs_disasm", "vm_global", "vm_new_object",
  "vm_dump_runtime", "vm_new_host_function", "vm_new_async_host_function", "vm_define", "vm_dump",
  "vm_decode_wire", "vm_string_reserve", "vm_new_error", "vm_handle_bits", "vm_handle_free",
  "vm_display", "vm_call", "vm_run", "vm_fuel_used", "vm_set_now", "vm_canonical_hash",
  "vm_set_weak_refs", "vm_set_random_seed", "vm_set_time_zone_offset", "vm_abort", "vm_result",
  "vm_outcome", "vm_host_call", "vm_host_call_info", "vm_new_host_calls",
  "vm_take_unhandled_rejections", "vm_promise_state", "vm_resolve_host_call",
  "vm_resolve_host_call_at", "vm_open_call", "vm_restore_math_random", "vm_set_next_call_id",
  "djs_elastic_stack", "djs_stack", "djs_alloc_stats", "djs_has", "djs_tables_ptr",
  "djs_tables_set", "vm_tables_need", "vm_snapshot", "vm_load_snapshot", "djs_snapshot_needs",
  "djs_snapshot_describe",
] as const;

/** An operation's name. */
export type Op = (typeof OPS)[number];

/** The operations whose argument at this position is the float `t` (the rest are integers). */
const FLOAT_ARG: Partial<Record<Op, number>> = { vm_set_now: 1, vm_resolve_host_call_at: 4 };

type Djs = (op: number, a?: number, b?: number, c?: number, d?: number, e?: number, f?: number, t?: number) => number;

/** The operations a module's build has (`djs_has`): name, number, position of a float argument. */
const available = new WeakMap<WebAssembly.Module, [Op, number, number | undefined][]>();

/**
 * The exports of an instance of djs-wasm `module` (a new one unless given) as one function
 * per operation the build has, by its name, plus `memory` and `__stack_pointer`. The functions
 * need no `this`. Throws for a module without the one export (another ABI).
 */
export function djsExports(module: WebAssembly.Module, instance = new WebAssembly.Instance(module, {})): Record<string, unknown> {
  const e = instance.exports as { djs?: Djs; memory: WebAssembly.Memory; __stack_pointer?: WebAssembly.Global };
  const djs = e.djs;
  if (typeof djs !== "function") throw new Error("not a djs-wasm module of this ABI (it has no `djs` export)");
  let ops = available.get(module);
  if (ops === undefined) {
    const has = OPS.indexOf("djs_has");
    ops = OPS.flatMap((name, op) => (djs(has, op) === 1 ? [[name, op, FLOAT_ARG[name]] as [Op, number, number | undefined]] : []));
    available.set(module, ops);
  }
  const x: Record<string, unknown> = { memory: e.memory, __stack_pointer: e.__stack_pointer };
  for (const [name, op, float] of ops) {
    x[name] =
      float === 1
        ? (a: number, t: number) => djs(op, a, 0, 0, 0, 0, 0, t)
        : float === 4
          ? (a: number, b: number, c: number, d: number, t: number) => djs(op, a, b, c, d, 0, 0, t)
          : (a?: number, b?: number, c?: number, d?: number, e?: number, f?: number) => djs(op, a, b, c, d, e, f);
  }
  return x;
}
