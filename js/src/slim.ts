// The package's `./slim` entry point: the Sandbox API with the slim module,
// wasm/djs-slim.wasm (without typed arrays, BigInt, the iterator helpers and the other
// families scripts rarely use, and without the Unicode tables), as its default (D3106).
// Each entry point loads only its own module, so a bundler copies only the modules an
// application imports.

import { Sandbox as Base, loadModule as load } from "./sandbox.ts";

export * from "./sandbox.ts";

/** This entry point's module: the package's djs-slim.wasm. */
export function defaultWasmPath(): string | URL {
  return new URL("../wasm/djs-slim.wasm", import.meta.url);
}

/** Compiles djs-wasm once per location and caches it; default: this entry point's module. */
export function loadModule(location: string | URL = defaultWasmPath()): Promise<WebAssembly.Module> {
  return load(location);
}

/** The Sandbox, with this entry point's module as the default. */
export class Sandbox extends Base {
  static override defaultModule(): string | URL {
    return defaultWasmPath();
  }
}
