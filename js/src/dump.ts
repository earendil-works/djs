// The package's `./dump` entry point: the Sandbox API with the dump module,
// wasm/djs-dump.wasm (the shipped module plus the dumper), as its default: a sandbox that dumps
// itself, or the `dumpModule` of a sandbox of the main entry point (D3106).
// Each entry point loads only its own module, so a bundler copies only the modules an
// application imports.

import { Sandbox as Base, loadModule as load } from "./sandbox.ts";

export * from "./sandbox.ts";

/** This entry point's module: the package's djs-dump.wasm. */
export function defaultWasmPath(): string | URL {
  return new URL("../wasm/djs-dump.wasm", import.meta.url);
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
