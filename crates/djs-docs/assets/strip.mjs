// Strips the host package's TypeScript for the browser (node >= 22.13):
//   node strip.mjs <js/src> <out-dir>
// Imports of `./x.ts` become `./x.js`. Phase 2 could use djs's own type eraser instead.
import { stripTypeScriptTypes } from "node:module";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const [src, out] = process.argv.slice(2);
for (const f of readdirSync(src)) {
  if (!f.endsWith(".ts")) continue;
  let code = stripTypeScriptTypes(readFileSync(join(src, f), "utf8"), { mode: "strip" });
  code = code.replace(/from "(\.\/[^"]+)\.ts"/g, 'from "$1.js"');
  writeFileSync(join(out, f.replace(/\.ts$/, ".js")), code);
}
