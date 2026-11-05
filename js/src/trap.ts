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
