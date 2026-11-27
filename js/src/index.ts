// The package's entry point (`@earendil-works/djs`): the Sandbox API with the one module
// djs ships, wasm/djs.wasm, as its default; `$DJS_WASM` overrides it.
// The module's literal `new URL("../wasm/djs.wasm", import.meta.url)` is here, not in
// sandbox.ts, so that bundlers copy it as an asset of the package and running from the
// repository's sources does not need it (D3106, D3602).

import { Sandbox as Base, loadModule as load } from "./sandbox.ts";
import { envVar } from "./env.ts";

export * from "./sandbox.ts";

/** The package's module: `$DJS_WASM`, else its djs.wasm. */
export function defaultWasmPath(): string | URL {
  return envVar("DJS_WASM") ?? new URL("../wasm/djs.wasm", import.meta.url);
}

/** Compiles djs-wasm once per location and caches it; default: the package's module. */
export function loadModule(location: string | URL = defaultWasmPath()): Promise<WebAssembly.Module> {
  return load(location);
}

/** The Sandbox, with the package's module as the default. */
export class Sandbox extends Base {
  static override defaultModule(): string | URL {
    return defaultWasmPath();
  }
}
