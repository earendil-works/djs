// The docs site's live examples at `debugger;` pauses and the snapshots page's restore box
// (crates/djs-docs/assets/live.js, snapview.js; D3761), on a stub page (stub-dom.ts) with
// the docs module: pause, the Snapshot tab in JavaScript terms, a record opened, Copy,
// Continue through every pause, and restoring a copied snapshot, garbage, and one this
// setup refuses. Needs the docs module (djs-wasm's `docs`, as the gate's dev module).

import test from "node:test";
import assert from "node:assert/strict";
import { Sandbox, loadModule } from "../src/sandbox.ts";
import { docsModule } from "./docs-module.ts";
import { StubElement, stubDocument } from "./stub-dom.ts";

const FETCH_PAGE = `async function fetchPage(n) {
  const page = { n, items: [n * 10, n * 10 + 1] };
  debugger;            // a tool call would wait here
  return page;
}
const pages = [];
for (let n = 1; n <= 3; n++) {
  pages.push(await fetchPage(n));
  console.log("got page", n, "of 3");
}
const items = pages.flatMap((p) => p.items);
console.log(items.join(" "));
return items;`;

const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

/** A live block's markup as djs-docs writes it (render.rs live_block), the parts live.js uses. */
function block(n: number, code: string, restore: boolean): string {
  const tab = (id: string, label: string, sel: boolean) =>
    `<button role="tab" data-tab="${id}" aria-selected="${sel}"${sel ? "" : " hidden"}>${label}</button>`;
  const pane = (tag: string, id: string, hidden: boolean) => `<${tag} class="live-pane" data-pane="${id}"${hidden ? " hidden" : ""}></${tag}>`;
  return (
    `<div class="live${restore ? " live-restoring" : ""}" id="live-${n}">` +
    `<div class="live-bar"><span class="live-note">output from the build</span><a class="live-info" hidden>i</a>` +
    `<button class="live-reset" hidden>Reset</button><button class="live-run" hidden>Run</button></div>` +
    (restore
      ? `<div class="live-restore" hidden><textarea class="live-restore-in"></textarea><button class="live-load">Load</button><span class="live-restore-status"></span></div>`
      : "") +
    `<div class="live-editor"><pre class="code"><code class="lang-js">${esc(code)}</code></pre></div>` +
    `<div class="live-tabs">${tab("out", "Output", true)}${tab("bytecode", "Bytecode", false)}${tab("memory", "Memory", false)}${tab("snapshot", "Snapshot", false)}` +
    `<span class="live-status"></span></div>` +
    `${pane("pre", "out", false)}${pane("pre", "bytecode", true)}${pane("div", "memory", true)}${pane("div", "snapshot", true)}</div>`
  );
}

const settle = () => new Promise((r) => setTimeout(r, 20));

async function until(f: () => boolean, what: string) {
  for (let k = 0; k < 400; k++) {
    if (f()) return;
    await settle();
  }
  assert.fail(`timed out waiting for ${what}`);
}

test("live examples pause into snapshots; the restore box loads them", async (t) => {
  if (!(await docsModule())) {
    t.skip("needs the docs module");
    return;
  }
  const module = await loadModule();
  const doc = stubDocument(block(1, FETCH_PAGE, false) + block(2, "debugger;\nconsole.log('restored');", true));
  const copied: string[] = [];
  const g = globalThis as Record<string, unknown>;
  Object.assign(g, {
    document: doc,
    location: { hash: "", href: "http://localhost/snapshots.html" },
    // The live examples load these four from the site (live.js lib()).
    djsLiveImports: () =>
      Promise.all([import("../src/sandbox.ts"), import("../src/layout.generated.ts"), Promise.resolve(module), import("../src/abi.ts")]),
  });
  Object.defineProperty(g, "navigator", { value: { platform: "test", clipboard: { writeText: async (s: string) => void copied.push(s) } }, configurable: true });
  await import("../../crates/djs-docs/assets/live.js");

  const ex = doc.querySelector("#live-1")!;
  const status = () => ex.querySelector(".live-status")!.textContent;
  const out = () => ex.querySelector('[data-pane="out"]')!.textContent;
  const snap = ex.querySelector('[data-pane="snapshot"]')!;
  ex.querySelector(".live-run")!.click();
  await until(() => status().startsWith("paused"), "the first pause");
  assert.equal(status(), "paused at line 3:3 (pause 1)");
  assert.match(out(), /paused at line 3:3: the run is now snapshot 1/);
  assert.equal(ex.querySelector('[data-tab="snapshot"]')!.hidden, false);

  // The Snapshot tab: the run in JavaScript terms, explained, with its layers.
  const text = snap.textContent;
  assert.match(text, /fetchPage\(n = 1\) is stopped at debugger; \(line 3\)/);
  assert.match(text, /the body is waiting for fetchPage, at await fetchPage\(n\) \(line 8\), halfway through pages\.push\(…\)/);
  assert.match(text, /the run's result is a promise that is still pending/);
  assert.match(text, /page = \{ n: 1, items: \[10, 11\] \}/);
  assert.match(text, /How djs stores it: 21 records/);
  const why = snap.querySelector(".sv-why")!;
  assert.equal(why.nextElementSibling!.hidden, true);
  why.click();
  assert.equal(why.nextElementSibling!.hidden, false);
  // A frame's title marks where it waits in the editor.
  snap.querySelectorAll(".sv-at")[1].click();
  assert.ok(ex.querySelector(".live-editor .live-mark"), "a mark in the editor");
  // A value opens its record: what it is, its fields as stored, what holds it; its bytes.
  const page = snap.querySelector('.sv-binds .sv-ref[data-rec="14"]')!;
  page.dispatchEvent(Object.assign(new (await import("./stub-dom.ts")).StubEvent("mouseover"), {}));
  assert.equal(snap.querySelectorAll(".sv-ref.is-same").length, snap.querySelectorAll('.sv-ref[data-rec="14"]').length);
  page.click();
  const card = snap.querySelector(".sv-card")!.textContent;
  assert.match(card, /#14 page/);
  assert.match(card, /Stored as Object\.Ordinary, 16 bytes at \d+\.\.\d+ of the snapshot; held by #11/);
  snap.querySelector(".sv-bytes-of")!.click();
  assert.equal(snap.querySelector(".sv-layer-bytes")!.open, true);
  assert.equal(snap.querySelectorAll(".sv-hex mark").length, 2 * 16);
  // Copy: the snapshot as base64.
  snap.querySelector('[data-act="copy"]')!.click();
  await until(() => copied.length > 0, "the copy");
  const first = copied[0];
  assert.match(first, /^REpTTkFQ/);

  // Continue through the pauses to the end.
  for (let k = 2; k <= 3; k++) {
    ex.querySelector(".live-run")!.click();
    await until(() => status() === `paused at line 3:3 (pause ${k})`, `pause ${k}`);
  }
  ex.querySelector(".live-run")!.click();
  await until(() => status().startsWith("ran to the end"), "the end");
  assert.equal(status(), "ran to the end, continued 3× from a snapshot");
  assert.match(out(), /got page 3 of 3\n10 11 20 21 30 31$/);

  // The restore box: the copied snapshot comes back, script and all, and goes on.
  const rx = doc.querySelector("#live-2")!;
  const box = rx.querySelector(".live-restore")!;
  assert.equal(box.hidden, false);
  const say = () => box.querySelector(".live-restore-status")!.textContent;
  const load = async (text: string, done: () => boolean) => {
    box.querySelector(".live-restore-in")!.value = text;
    box.querySelector(".live-load")!.click();
    await until(done, `loading ${text.slice(0, 20)}`);
  };
  await load("not base64!", () => say() !== "");
  assert.equal(say(), "That is not base64.");
  await load("aGVsbG8=", () => say().startsWith("not a snapshot"));
  await load(first, () => rx.querySelector(".live-status")!.textContent.startsWith("paused"));
  assert.match(say(), /^Loaded: 2\.6 KB/);
  assert.equal((rx.querySelector("textarea.live-src") as StubElement).value, FETCH_PAGE);
  assert.match(rx.querySelector('[data-pane="out"]')!.textContent, /restored from the snapshot you gave/);
  rx.querySelector(".live-run")!.click();
  await until(() => /got page 1 of 3/.test(rx.querySelector('[data-pane="out"]')!.textContent), "the restored run going on");

  // A snapshot of another setup: refused with the engine's reason, its contents shown.
  const other = await Sandbox.create({ module, debugger: true, asyncFunctions: { fetch: () => 1 } });
  const run = other.begin("await fetch(); debugger;", {}, { filename: "other.js" });
  const bytes = run.snapshot();
  other.dispose();
  await load(Buffer.from(bytes).toString("base64"), () => say().startsWith("This djs refuses it"));
  assert.match(say(), /no host value or function fetch/);
  assert.match(rx.querySelector('[data-pane="snapshot"]')!.textContent, /other\.js, waiting for the host to answer 1 call/);
  assert.equal(rx.querySelector(".live-run")!.classList.contains("is-paused"), false);
});
