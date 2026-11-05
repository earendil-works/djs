#!/usr/bin/env bash
# Builds the npm package @earendil-works/djs (js/, D2842, D3105) and packs it:
#   scripts/npm-pack.sh [--modules MAIN SLIM DUMP] [--out DIR]
# 1. the three wasm modules: the shipped, slim and dump builds of scripts/build-wasm.sh (release,
#    build-std, as they ship), or the given files (--modules: the gate packs its own builds);
# 2. the JavaScript and its declarations: tsc over js/src (js/tsconfig.build.json), with
#    TypeScript from js/package-lock.json (npm ci, once);
# 3. the package directory DIR/package (default target/npm/package): dist/, wasm/, js/README.md,
#    the license files and js/package.json without its scripts and devDependencies;
# 4. `npm pack` into DIR. The last line of the output is the tarball's path.
# The package is tested by scripts/npm-test.sh; .github/workflows/publish.yml publishes it.
set -euo pipefail
cd "$(dirname "$0")/.."
out=target/npm
modules=()
while [ $# -gt 0 ]; do
  case "$1" in
    --modules) modules=("$2" "$3" "$4"); shift 4 ;;
    --out) out=$2; shift 2 ;;
    *) echo "usage: scripts/npm-pack.sh [--modules MAIN SLIM DUMP] [--out DIR]" >&2; exit 2 ;;
  esac
done
mkdir -p "$out"
out=$(cd "$out" && pwd)

if [ ${#modules[@]} -eq 0 ]; then
  scripts/build-wasm.sh >&2
  scripts/build-wasm.sh --slim >&2
  scripts/build-wasm.sh --dump >&2
  modules=(target/wasm32-unknown-unknown/wasm/djs_wasm.wasm target/djs-slim.wasm target/djs-dump.wasm)
fi

# TypeScript as js/package-lock.json pins it; installed again only when that changed.
want=$(node -p 'require("./js/package-lock.json").packages["node_modules/typescript"].version')
have=$(node -p 'try { require("./js/node_modules/typescript/package.json").version } catch { "" }')
if [ "$want" != "$have" ]; then
  (cd js && npm ci --prefer-offline --no-audit --no-fund --loglevel=error) >&2
fi

pkg=$out/package
rm -rf "$pkg"
mkdir -p "$pkg/wasm"
js/node_modules/.bin/tsc -p js/tsconfig.build.json --outDir "$pkg/dist"
# The dump runtime is JavaScript generated from Rust, with declarations beside it.
cp js/src/dump-runtime.js js/src/dump-runtime.d.ts "$pkg/dist/"
# tsc keeps the sources' `./x.ts` specifiers in the declarations; the package has `./x.js`
# (D3105).
node -e '
  const fs = require("fs"), path = require("path");
  const dir = process.argv[1];
  for (const f of fs.readdirSync(dir)) {
    if (!f.endsWith(".d.ts")) continue;
    const p = path.join(dir, f);
    fs.writeFileSync(p, fs.readFileSync(p, "utf8").replace(/(from |import\()"(\.\/[^"]+)\.ts"/g, "$1\"$2.js\""));
  }
' "$pkg/dist"
cp "${modules[0]}" "$pkg/wasm/djs.wasm"
cp "${modules[1]}" "$pkg/wasm/djs-slim.wasm"
cp "${modules[2]}" "$pkg/wasm/djs-dump.wasm"
cp js/README.md LICENSE-MIT LICENSE-APACHE "$pkg/"
node -e '
  const fs = require("fs");
  const p = JSON.parse(fs.readFileSync("js/package.json", "utf8"));
  delete p.scripts;
  delete p.devDependencies;
  fs.writeFileSync(process.argv[1], JSON.stringify(p, null, 2) + "\n");
' "$pkg/package.json"

# Every relative specifier of the built code names a file of the package.
node -e '
  const fs = require("fs"), path = require("path");
  const dir = process.argv[1], missing = [];
  for (const f of fs.readdirSync(dir)) {
    if (!/\.(js|d\.ts)$/.test(f)) continue;
    const code = fs.readFileSync(path.join(dir, f), "utf8");
    // Imports, and the assets bundlers copy: `new URL("…", import.meta.url)`.
    const specs = /(?:\bfrom\s*|\bimport\s*\(\s*)"(\.{1,2}\/[^"]+)"|new URL\(\s*"(\.{1,2}\/[^"]+)",\s*import\.meta\.url/g;
    for (const m of code.matchAll(specs)) {
      const spec = m[1] ?? m[2];
      if (!fs.existsSync(path.join(dir, spec))) missing.push(`${f}: ${spec}`);
    }
  }
  if (missing.length) { console.error("npm-pack: relative specifiers that name no file:\n  " + missing.join("\n  ")); process.exit(1); }
' "$pkg/dist"

(cd "$pkg" && npm pack --pack-destination "$out" --loglevel=warn --json) > "$out/pack.json"
node -e '
  const [r] = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"));
  const kb = (n) => (n / 1024).toFixed(1).padStart(8) + " KiB";
  for (const f of r.files) console.error(kb(f.size) + "  " + f.path);
  console.error(`${r.entryCount} files, ${kb(r.unpackedSize).trim()} unpacked, ${kb(r.size).trim()} packed`);
  console.log(require("path").join(process.argv[2], r.filename));
' "$out/pack.json" "$out"
