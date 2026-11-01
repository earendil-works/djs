// ts-blank-space over files: `node blank.mjs <out-dir> <root> <file...>` writes
// <out-dir>/<file relative to root> for each file it erases and prints a line for the
// others: `invalid <file>` (TypeScript reports a syntax error: the conformance tests
// have many on purpose) or `unsupported <file>` (syntax ts-blank-space cannot erase:
// enums, namespaces with values, parameter properties, `<T>x`, ...).
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";

const require = createRequire(path.join(process.env.STRIP_NODE_DIR, "package.json"));
const ts = require("typescript");
const { blankSourceFile } = await import(require.resolve("ts-blank-space"));
const options = {
  languageVersion: ts.ScriptTarget.ESNext,
  impliedNodeFormat: ts.ModuleKind.ESNext,
};

const [outDir, root, ...files] = process.argv.slice(2);
for (const file of files) {
  const rel = path.relative(root, file);
  const src = fs.readFileSync(file, "utf8");
  const sf = ts.createSourceFile("input.ts", src, options, false, ts.ScriptKind.TS);
  if (sf.parseDiagnostics.length > 0) {
    console.log(`invalid ${rel}`);
    continue;
  }
  let unsupported = false;
  const out = blankSourceFile(sf, () => { unsupported = true; });
  if (unsupported) {
    console.log(`unsupported ${rel}`);
    continue;
  }
  const dest = path.join(outDir, rel);
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.writeFileSync(dest, out);
}
