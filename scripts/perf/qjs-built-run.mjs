// qjs-built-run.mjs <file.js>...: the same bodies in Bellard's QuickJS as the engine
// comparison builds it (js/bench/engines, vendor/engines/build/quickjs.wasm; QJS_FLAVOR=ng
// for QuickJS-NG), once each: slope.py --qjs2 counts its instructions.
import { readFileSync } from "node:fs";
import { quickjsBuilt } from "../../js/bench/engines/engines.ts";
const e = quickjsBuilt(process.env.QJS_FLAVOR === "ng" ? "ng" : "bellard");
await e.load();
for (const name of process.argv.slice(2)) {
  const vm = await e.create();
  const src = e.program(readFileSync(name, "utf8"));
  const t = performance.now();
  const r = await vm.run(src, JSON.stringify({ ALL_TOOLS: "[]" }));
  const dt = performance.now() - t;
  if (String(r).startsWith("ERROR")) throw new Error(String(r));
  vm.dispose();
  console.log(`${name}\t${dt.toFixed(1)} ms`);
}
