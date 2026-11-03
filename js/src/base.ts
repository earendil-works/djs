// The module's base snapshot (D1321) on the host side: the module ships it coded in its
// custom section `djs_base` (D2252, D2541), every instance needs it decoded before its
// first VM, and decoding takes a few ms. So the host hands each fresh instance the coded
// section, or the bytes an earlier instance of the same module decoded (checked by the
// instance), and keeps those bytes once per module.

/** The exports of an instance that `provideBase` and `rememberBase` use. */
export interface BaseExports {
  memory: WebAssembly.Memory;
  djs_base_ptr(): number;
  djs_base_len(): number;
  djs_base_inject_ptr?(len: number): number;
  djs_base_inject?(): number;
  djs_base_coded_ptr?(len: number): number;
}

/** The decoded base of each module, as an earlier instance of it decoded it. */
const bases = new WeakMap<WebAssembly.Module, Uint8Array>();
const codedSections = new WeakMap<WebAssembly.Module, Uint8Array>();

/**
 * Hands a fresh instance of `module` its coded base, the module's custom section
 * `djs_base` (D2541): the module keeps it out of its linear memory, and an instance
 * decodes it when it needs the base and has none injected. `provideBase` and
 * `rememberBase` call it; a host that drives the exports itself calls it right after
 * instantiating, before anything creates a VM or reads the base. Returns whether the
 * instance took the bytes (false if it has its base, or the module has no such section).
 */
export function provideCodedBase(module: WebAssembly.Module, exports: BaseExports): boolean {
  if (exports.djs_base_coded_ptr === undefined) return false;
  let coded = codedSections.get(module);
  if (coded === undefined) {
    const [section] = WebAssembly.Module.customSections(module, "djs_base");
    if (section === undefined) return false;
    coded = new Uint8Array(section);
    codedSections.set(module, coded);
  }
  const at = exports.djs_base_coded_ptr(coded.length);
  if (at === 0) return false;
  new Uint8Array(exports.memory.buffer, at, coded.length).set(coded);
  return true;
}

/**
 * Gives a fresh instance of `module` the base snapshot an earlier instance of it decoded
 * (D2252): the module ships the base coded, and an instance decodes it at its first use
 * (a few ms) unless the host provides it. Call right after instantiating, before anything
 * that creates a VM or reads the base; and `rememberBase` once a VM exists, so that later
 * instances get it. The instance checks the bytes (length and hash). Returns whether the
 * instance took them; if not (refused, or no instance remembered the base yet), it gets
 * the coded base to decode its own (`provideCodedBase`).
 */
export function provideBase(module: WebAssembly.Module, exports: BaseExports): boolean {
  const bytes = bases.get(module);
  if (bytes !== undefined && exports.djs_base_inject_ptr !== undefined && exports.djs_base_inject !== undefined) {
    const at = exports.djs_base_inject_ptr(bytes.length);
    if (at !== 0) {
      new Uint8Array(exports.memory.buffer, at, bytes.length).set(bytes);
      if (exports.djs_base_inject() === 1) return true;
    }
  }
  // The instance decodes its own, from the coded base.
  provideCodedBase(module, exports);
  return false;
}

/**
 * Keeps the base of `module` (from this instance, once per module) for `provideBase`. Call
 * before the instance's first `vm_new`, which takes the bytes as its arena (D2060).
 */
export function rememberBase(module: WebAssembly.Module, exports: BaseExports): void {
  if (bases.has(module)) return;
  provideCodedBase(module, exports);
  // The pointer first: an instance decodes its base at the first use (D2252), which may
  // grow (and detach) its memory buffer.
  const at = exports.djs_base_ptr();
  if (at === 0) throw new Error("djs-wasm has no base snapshot (no djs_base section?)");
  bases.set(module, new Uint8Array(exports.memory.buffer, at, exports.djs_base_len()).slice());
}
