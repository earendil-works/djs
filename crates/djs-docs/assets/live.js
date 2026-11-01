// Live examples: every ```js live block of the docs runs in djs-wasm in the page.
// Without JavaScript (or from file://) the page still shows the output and bytecode
// generated at build time.
import { Sandbox } from "./djs/sandbox.js";
import { LAYOUT } from "./djs/layout.generated.js";

const KIND_NAMES = Object.fromEntries(Object.entries(LAYOUT.cells).map(([n, c]) => [c.kind, n]));
const CLASS_NAMES = Object.fromEntries(Object.entries(LAYOUT.objectClasses).map(([n, c]) => [c, n]));
const FUEL_SLICE = 50;

let modulePromise;
function module() {
  modulePromise ??= WebAssembly.compileStreaming(fetch(new URL("./djs/djs_wasm.wasm", import.meta.url)));
  return modulePromise;
}

// The Bytecode tab: djs-wasm built with the `docs` feature (scripts/build-wasm.sh --docs)
// exports `djs_disasm`, the listing `djs disasm` prints, so an edited example shows its
// own bytecode. With the shipping module the tab keeps the build-time listing.
let disasmExports;
async function disasm(src) {
  const mod = await module();
  if (!WebAssembly.Module.exports(mod).some((e) => e.name === "djs_disasm")) return null;
  disasmExports ??= new WebAssembly.Instance(mod, {}).exports;
  const x = disasmExports;
  const enc = new TextEncoder();
  // The parameters of a codemode body, as the guide's build-time listings use them.
  const params = enc.encode("tools\nconsole");
  const body = enc.encode(src);
  const at = x.djs_staging(params.length + body.length);
  const mem = new Uint8Array(x.memory.buffer);
  mem.set(params, at);
  mem.set(body, at + params.length);
  x.djs_disasm(1, params.length, body.length);
  return new TextDecoder().decode(new Uint8Array(x.memory.buffer, x.djs_out_ptr(), x.djs_out_len()));
}

/** Walks the arena cell by cell: the heap is walkable because every header has a size. */
function walk(sb) {
  const heap = sb.heapView();
  const len = heap.length;
  const dv = new DataView(heap.buffer, heap.byteOffset, len);
  const kinds = new Map();
  const classes = new Map();
  let cells = 0;
  for (let off = 0; off + 8 <= len; ) {
    const kind = dv.getUint8(off);
    const size = dv.getUint32(off + 4, true);
    if (size === 0) break;
    const name = KIND_NAMES[kind] ?? `kind ${kind}`;
    const k = kinds.get(name) ?? { n: 0, bytes: 0 };
    k.n++;
    k.bytes += size;
    kinds.set(name, k);
    if (name === "Object") {
      const cls = CLASS_NAMES[dv.getUint32(off + 8, true)] ?? "?";
      classes.set(cls, (classes.get(cls) ?? 0) + 1);
    }
    cells++;
    off += size;
  }
  return { len, cells, kinds, classes };
}

function show(v) {
  if (typeof v === "string") return v;
  if (typeof v === "bigint") return `${v}n`;
  if (typeof v === "number") return Object.is(v, -0) ? "-0" : String(v);
  try {
    return JSON.stringify(v);
  } catch {
    return String(v);
  }
}

function kb(n) {
  return n < 10240 ? `${n} B` : `${(n / 1024).toFixed(1)} KiB`;
}

function heapHtml(before, after) {
  const rows = [...after.kinds.entries()].sort((a, b) => b[1].bytes - a[1].bytes);
  const max = rows[0]?.[1].bytes ?? 1;
  let h = `<p>Arena: <b>${kb(after.len)}</b> in <b>${after.cells}</b> cells. The base snapshot it started from: ${kb(before.len)} in ${before.cells} cells.</p><table class="bars">`;
  for (const [name, k] of rows.slice(0, 12)) {
    const b = before.kinds.get(name);
    const delta = k.n - (b?.n ?? 0);
    h += `<tr><td class="mono">${name}</td><td class="num">${k.n}</td><td class="num delta">${delta > 0 ? "+" + delta : ""}</td><td class="bar"><span style="width:${(100 * k.bytes) / max}%"></span></td><td class="num">${kb(k.bytes)}</td></tr>`;
  }
  h += `</table><p class="muted">Counted by walking the arena from offset 0, cell by cell, with the layout the engine exports. Garbage stays until the next collection: <button class="live-gc">Collect now</button></p>`;
  return h;
}

async function runExample(el) {
  const status = el.querySelector(".live-status");
  const pane = (n) => el.querySelector(`[data-pane="${n}"]`);
  const ta = el.querySelector(".live-src");
  const src = ta.value;
  status.textContent = "running…";
  const lines = [];
  const out = (...a) => lines.push(a.map(show).join(" "));
  let slices = 0;
  try {
    const mod = await module();
    const sb = new Sandbox({
      module: mod,
      console: { log: out, info: out, warn: out, error: out },
      fuelSlice: FUEL_SLICE,
      // Called before every fuel slice: count how often the VM stopped and resumed.
      now: () => (slices++, 0),
    });
    const before = walk(sb);
    const t0 = performance.now();
    try {
      await sb.runAsync(src);
    } catch (e) {
      lines.push(`Uncaught ${e.message}`);
    }
    const ms = performance.now() - t0;
    pane("out").textContent = lines.join("\n");
    const listing = await disasm(src);
    if (listing !== null) pane("bytecode").textContent = listing;
    const after = walk(sb);
    pane("heap").innerHTML = heapHtml(before, after);
    pane("heap").querySelector(".live-gc").onclick = () => {
      sb.collect();
      pane("heap").innerHTML = heapHtml(before, walk(sb));
    };
    const t1 = performance.now();
    const image = sb.bytes();
    const t2 = performance.now();
    const restored = await Sandbox.fromBytes(image, { module: mod, console: { log: out, info: out, warn: out, error: out } });
    const t3 = performance.now();
    const check = restored.run("return typeof console.log");
    pane("snapshot").innerHTML =
      `<p>The run took <b>${ms.toFixed(2)} ms</b> in <b>${slices}</b> fuel slices of ${FUEL_SLICE} units: ` +
      `the VM returned control to this page ${slices - 1} times between two instructions and resumed where it stopped. ` +
      `Nothing of the run was on the Rust stack in between; it is all in the arena.</p>` +
      `<p>Snapshot: <code>sb.bytes()</code> is <b>${kb(image.length)}</b> (the arena plus a small header), taken in ${(t2 - t1).toFixed(2)} ms. ` +
      `Restored into a fresh wasm instance with <code>Sandbox.fromBytes</code> in ${(t3 - t2).toFixed(2)} ms; ` +
      `the restored VM runs (<code>typeof console.log</code> → ${check}).</p>`;
    restored.dispose();
    sb.dispose();
    status.textContent = `ran in djs-wasm · ${ms.toFixed(1)} ms`;
  } catch (e) {
    status.textContent = `could not run: ${e.message}`;
  }
}

for (const el of document.querySelectorAll(".live")) {
  const tabs = el.querySelectorAll(".live-tabs button");
  for (const b of tabs) {
    b.onclick = () => {
      for (const t of tabs) t.setAttribute("aria-selected", String(t === b));
      for (const p of el.querySelectorAll(".live-pane")) p.hidden = p.dataset.pane !== b.dataset.tab;
    };
  }
  el.querySelector(".live-run").onclick = () => runExample(el);
  el.querySelector(".live-edit").onclick = (ev) => {
    const ta = el.querySelector(".live-src");
    const pre = el.querySelector("pre.code");
    ta.hidden = !ta.hidden;
    pre.hidden = !ta.hidden;
    ta.style.height = `${pre.offsetHeight || 120}px`;
    ev.target.textContent = ta.hidden ? "Edit" : "View";
    const bc = el.querySelector('[data-pane="bytecode"]');
    if (!ta.hidden && !bc.dataset.note) {
      bc.dataset.note = "1";
      module()
        .then((mod) => WebAssembly.Module.exports(mod).some((e) => e.name === "djs_disasm"))
        .catch(() => false)
        .then((live) => {
          const note = live
            ? "(The original example's bytecode. Run shows the listing of the edited code.)"
            : "(The bytecode shown is the original example's, generated at build time.)";
          bc.textContent = `${note}\n\n${bc.textContent}`;
        });
    }
  };
}

// `#run` in the URL runs every example on load (for screenshots and links).
if (location.hash === "#run") for (const el of document.querySelectorAll(".live")) runExample(el);
