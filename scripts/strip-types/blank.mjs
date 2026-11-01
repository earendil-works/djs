// ts-blank-space over files: `node blank.mjs <out-dir> <root> <file...>` writes
// <out-dir>/<file relative to root> for each file it accepts and prints
// `unsupported <file>` for the others (syntax it cannot erase: enums, namespaces with
// values, parameter properties, `<T>x`, ...).
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";

const require = createRequire(path.join(process.env.STRIP_NODE_DIR, "package.json"));
const tsBlankSpace = (await import(require.resolve("ts-blank-space"))).default;

const [outDir, root, ...files] = process.argv.slice(2);
for (const file of files) {
  const src = fs.readFileSync(file, "utf8");
  let unsupported = false;
  const out = tsBlankSpace(src, () => { unsupported = true; });
  if (unsupported) {
    console.log(`unsupported ${path.relative(root, file)}`);
    continue;
  }
  const dest = path.join(outDir, path.relative(root, file));
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.writeFileSync(dest, out);
}
