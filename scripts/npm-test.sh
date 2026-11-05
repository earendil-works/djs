#!/usr/bin/env bash
# Tests a packed @earendil-works/djs (scripts/npm-pack.sh's tarball) as a user gets it (D3108):
# installs it into a clean directory and runs, per argument,
#   node | bun | deno   js/pkg-test/smoke.js, which imports the package by name and uses every
#                       entry point, in that runtime;
#   js-tests            js/test's tests and the sleep-dump example on the package (node >= 22.18:
#                       they are TypeScript), the main module replaced by the dump module and
#                       DJS_WASM_NODUMP naming the main one, as the gate runs them on its builds;
#                       the tests of internal modules (base, trap, wire) and of pi's adapter stay
#                       out: the package does not export them;
#   browser             smoke.js in headless Chromium, the package as native modules (an import
#                       map), bundled by Vite and by webpack (installs playwright, its Chromium,
#                       vite and webpack, so it needs the network; on Linux, `npx playwright
#                       install-deps chromium` first).
#   scripts/npm-test.sh TARBALL [--dir DIR] [node|bun|deno|js-tests|browser]...   (default: node)
set -euo pipefail
cd "$(dirname "$0")/.."
repo=$PWD
tarball=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
shift
dir=
if [ "${1:-}" = "--dir" ]; then dir=$2; shift 2; fi
[ $# -gt 0 ] || set -- node
if [ -z "$dir" ]; then
  dir=$(mktemp -d)
  trap 'rm -rf "$dir"' EXIT
fi
mkdir -p "$dir"
cd "$dir"
echo '{ "name": "djs-npm-test", "private": true, "type": "module" }' > package.json
npm install --no-audit --no-fund --no-save --prefer-offline --loglevel=error "$tarball"
cp "$repo"/js/pkg-test/*.js "$repo"/js/pkg-test/index.html .
pkg=$dir/node_modules/@earendil-works/djs

for what in "$@"; do
  echo "--- $what"
  case "$what" in
    node) node main.js ;;
    bun) bun main.js ;;
    # Deno reads the package's files (its modules) and nothing else; no env access, so
    # `$DJS_WASM` stays unread without a prompt.
    deno) deno run --allow-read=. main.js ;;
    js-tests)
      rm -rf js examples && mkdir -p js/test examples
      cp -R "$repo/examples/sleep-dump" examples/
      rm -rf examples/sleep-dump/out
      for f in "$repo"/js/test/*.test.ts; do
        case "$(basename "$f")" in base.test.ts | trap.test.ts | wire.test.ts | pi-vm.test.ts) continue ;; esac
        cp "$f" js/test/
      done
      # The sources' relative imports become the package's entry points.
      node -e '
        const fs = require("fs");
        for (const f of process.argv.slice(1)) {
          const s = fs.readFileSync(f, "utf8")
            .replace(/"(?:\.\.\/)+(?:js\/)?src\/sandbox\.ts"/g, "\"@earendil-works/djs\"")
            .replace(/"(?:\.\.\/)+(?:js\/)?src\/dump-runtime\.js"/g, "\"@earendil-works/djs/dump-runtime\"");
          const left = /"(?:\.\.\/)+(?:js\/)?src\/[^"]*"/.exec(s);
          if (left) { console.error(`${f}: imports ${left[0]}, which the package does not export`); process.exit(1); }
          fs.writeFileSync(f, s);
        }
      ' js/test/*.ts examples/sleep-dump/*.ts
      DJS_WASM=$pkg/wasm/djs-dump.wasm DJS_WASM_NODUMP=$pkg/wasm/djs.wasm node --test js/test/*.test.ts
      ;;
    browser)
      npm install --no-audit --no-fund --no-save --loglevel=error playwright@1 vite@8 webpack@5 webpack-cli@6 "$tarball"
      npx playwright install chromium # once per playwright version
      node browser.js . index.html
      mkdir -p vite && cp "$repo/js/pkg-test/vite/index.html" vite/
      npx vite build vite --outDir ../vite-dist --emptyOutDir --logLevel warn
      node browser.js vite-dist index.html
      mkdir -p webpack-dist && cp "$repo/js/pkg-test/webpack.html" webpack-dist/index.html
      npx webpack --mode production --entry ./page.js --output-path webpack-dist --no-stats
      node browser.js webpack-dist index.html
      ;;
    *) echo "npm-test: unknown test $what" >&2; exit 2 ;;
  esac
done
