// Continues the sleep case's dumps in djs through the Sandbox (0216): main's dump as it is,
// the minimal one with a shim for positional `$djs.k` after its header. Short deadline: the
// dumps' `due` has passed, so the resumed sleep answers at once.
//   DJS_WASM=target/djs-dump.wasm node work/research/0216/check-sandbox.ts A.js B.js
import { readFileSync } from "node:fs";
import { SCRIPT, options, resume } from "../../../examples/sleep-dump/sleep-dump.ts";

const shim =
  "{ const k0 = $djs.k, gp = [Object.getPrototypeOf(function* () {}), Object.getPrototypeOf(async function* () {})]; $djs.k = (K, f, at, p) => k0(K, { f, at }, p ?? (gp.includes(Object.getPrototypeOf(K)) ? f.prototype : undefined)); }\n";
for (const [i, f] of process.argv.slice(2).entries()) {
  let dump = readFileSync(f, "utf8");
  if (i > 0) dump = dump.replace(/^("use strict";\n)/m, `$1${shim}`);
  const lines: string[] = [];
  const opts = { ...options((...a) => lines.push(a.join(" "))), sources: (n: string) => (n === "sleep.js" ? SCRIPT : undefined) };
  const r = await resume(dump, opts);
  console.log(f, JSON.stringify(lines.map((l) => l.replace(/\d+ ms:/, "N ms:"))), JSON.stringify(r));
}
