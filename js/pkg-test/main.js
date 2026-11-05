// Runs the smoke test in node, Bun or Deno: `node main.js` in a directory where the
// package is installed (scripts/npm-test.sh). Exits non-zero at the first failure.
import { smoke } from "./smoke.js";

for (const line of await smoke()) console.log(`ok - ${line}`);
console.log("smoke: passed");
