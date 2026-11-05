// The package's main entry point (`@earendil-works/djs`): the Sandbox API with the
// shipped module, wasm/djs.wasm, as its default; `$DJS_WASM` overrides it (D3106).
// Each entry point loads only its own module, so a bundler copies only the modules an
// application imports.

import { Sandbox as Base, loadModule as load } from "./sandbox.ts";
import { envVar } from "./env.ts";

export * from "./sandbox.ts";

/** This entry point's module: `$DJS_WASM`, else the package's djs.wasm. */
export function defaultWasmPath(): string | URL {
  return envVar("DJS_WASM") ?? new URL("../wasm/djs.wasm", import.meta.url);
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
