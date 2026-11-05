// Runs the smoke test in headless Chromium (D3108): serves a directory over HTTP, opens a
// page that runs js/pkg-test/smoke.js and waits for its result.
//   node browser.js DIR PAGE      e.g. `. index.html` (native modules), `vite-dist index.html`
// Needs the `playwright` package and its Chromium (scripts/npm-test.sh installs both).
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize, resolve } from "node:path";
import { chromium } from "playwright";

const [dir, page] = process.argv.slice(2);
const root = resolve(dir);
const types = { ".html": "text/html", ".js": "text/javascript", ".mjs": "text/javascript", ".wasm": "application/wasm", ".json": "application/json" };
const server = createServer(async (req, res) => {
  const path = normalize(join(root, decodeURIComponent(new URL(req.url, "http://x").pathname)));
  if (!path.startsWith(root)) return void res.writeHead(403).end();
  try {
    const body = await readFile(path);
    res.writeHead(200, { "content-type": types[extname(path)] ?? "application/octet-stream" }).end(body);
  } catch {
    res.writeHead(404).end();
  }
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const url = `http://127.0.0.1:${server.address().port}/${page}`;
const browser = await chromium.launch();
let failed = true;
try {
  const tab = await browser.newPage();
  const errors = [];
  tab.on("pageerror", (e) => errors.push(String(e)));
  tab.on("console", (m) => m.type() === "error" && errors.push(m.text()));
  await tab.goto(url);
  const result = await tab
    .waitForFunction(() => window.smokeResult, null, { timeout: 60_000 })
    .then((h) => h.jsonValue())
    .catch((e) => ({ ok: false, error: `${e.message}\n${errors.join("\n")}` }));
  if (result.ok) {
    for (const line of result.lines) console.log(`ok - ${line}`);
    console.log(`smoke (chromium, ${dir}/${page}): passed`);
    failed = false;
  } else {
    console.error(`smoke (chromium, ${dir}/${page}): ${result.error}`);
  }
} finally {
  await browser.close();
  server.close();
}
process.exit(failed ? 1 : 0);
