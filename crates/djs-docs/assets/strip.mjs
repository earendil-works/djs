// Strips the host package's TypeScript for the browser (node >= 22.13):
//   node strip.mjs <js/src> <out-dir>
// Imports of `./x.ts` become `./x.js`; plain `.js` files (js/src/dump-runtime.js, generated)
// are copied as they are. Phase 2 could use djs's own type eraser instead. Then every
// relative import of the copied package must name a file in <out-dir>, or this fails: a
// missing one breaks the live examples only in the browser.
import { stripTypeScriptTypes } from "node:module";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, normalize } from "node:path";

const [src, out] = process.argv.slice(2);
const written = [];
for (const f of readdirSync(src)) {
  let code;
  let name = f;
  if (f.endsWith(".ts")) {
    code = stripTypeScriptTypes(readFileSync(join(src, f), "utf8"), { mode: "strip" });
    code = code.replace(/from "(\.\/[^"]+)\.ts"/g, 'from "$1.js"');
    name = f.replace(/\.ts$/, ".js");
  } else if (f.endsWith(".js")) {
    code = readFileSync(join(src, f), "utf8");
  } else {
    continue;
  }
  writeFileSync(join(out, name), code);
  written.push([name, code]);
}

// `from "./x.js"`, `import "./x.js"`, `import("./x.js")`.
const relative = /(?:\bfrom|\bimport)\s*\(?\s*(["'])(\.{1,2}\/[^"']+)\1/g;
const missing = [];
for (const [name, code] of written) {
  for (const m of code.matchAll(relative)) {
    const target = normalize(join(dirname(join(out, name)), m[2]));
    if (!existsSync(target)) missing.push(`${name}: ${m[2]}`);
  }
}
if (missing.length) {
  console.error(`strip.mjs: relative imports that name no file in ${out}:\n  ${missing.join("\n  ")}`);
  process.exit(1);
}
