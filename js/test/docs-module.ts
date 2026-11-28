// Whether the tests run on the docs module (djs-wasm's `docs` feature, as the gate's dev
// module): only it has `describeSnapshot` and `callSite` (D3751). On the shipped module
// (scripts/npm-test.sh js-tests) their tests check that the methods refuse.

import { Sandbox, SandboxError } from "../src/sandbox.ts";

let cached: Promise<boolean> | undefined;

export function docsModule(): Promise<boolean> {
  cached ??= (async () => {
    const sb = await Sandbox.create();
    try {
      sb.callSite(0);
      return true;
    } catch (e) {
      if (e instanceof SandboxError && e.kind === "misuse") return false;
      throw e;
    } finally {
      sb.dispose();
    }
  })();
  return cached;
}
