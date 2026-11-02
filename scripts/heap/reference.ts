// Reference images for `djs heap` (D2264): what a snapshot container is compressed
// against, when `djs heap` cannot rebuild it itself.
//
//   node scripts/heap/reference.ts base [--wasm MODULE] [-o base.img]
//       the base image of a djs-wasm module (djs heap uses its own build's base when the
//       hashes match; this is for a module of another build)
//   node scripts/heap/reference.ts pi-prelude --data START.json [--prelude FILE] [--wasm MODULE] [-o prelude.img]
//       pi's prelude image for a session's start data (tools, globals, store,
//       memoryLimitBytes, deterministic), with tests/pi's copy of pi's prelude unless
//       --prelude names a file holding the prelude source
//
// MODULE defaults to $DJS_WASM, else the `wasm` profile build.

import { readFileSync, writeFileSync } from "node:fs";
import { defaultWasmPath, loadModule } from "../../js/src/sandbox.ts";
import { baseReference } from "../../js/src/snapshot.ts";
import { instance } from "../../js/pi/engine.ts";
import { preludeImage } from "../../js/pi/vm.ts";
import type { VmStartData } from "../../js/pi/protocol.ts";
import { PRELUDE_SOURCE } from "../../tests/pi/prelude-source.ts";

const args = process.argv.slice(2);
const opt = (name: string) => {
  const i = args.indexOf(name);
  return i === -1 ? undefined : args[i + 1];
};
const what = args[0];
const module = await loadModule(opt("--wasm") ?? defaultWasmPath());
let image: Uint8Array;
let out: string;
if (what === "base") {
  const x = instance(module);
  image = baseReference(module, x.exports).bytes;
  out = opt("-o") ?? "base.img";
} else if (what === "pi-prelude") {
  const data = opt("--data");
  if (data === undefined) throw new Error("pi-prelude needs --data START.json");
  const start = JSON.parse(readFileSync(data, "utf8")) as VmStartData;
  const prelude = opt("--prelude") === undefined ? PRELUDE_SOURCE : readFileSync(opt("--prelude")!, "utf8");
  image = preludeImage(module, prelude, start);
  out = opt("-o") ?? "prelude.img";
} else {
  console.error("usage: node scripts/heap/reference.ts base|pi-prelude [--data START.json] [--prelude FILE] [--wasm MODULE] [-o FILE]");
  process.exit(2);
}
writeFileSync(out, image);
console.log(`${out}: ${image.length} bytes`);
