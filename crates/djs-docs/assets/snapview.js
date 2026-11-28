// The Snapshot tab of the live examples and the snapshots page's restore box (D3761): a
// snapshot as `Sandbox.inspectSnapshot` reads it, in three layers. First the run in
// JavaScript terms (each open call, the functions stopped where, their variables as
// JavaScript values, what each waits for, the result still pending), each part with a
// plain note on what it is and why the snapshot holds it; then how djs stores it (records
// with named fields and sizes); then the bytes. The same object reached twice is one
// `data-rec`: hovering one lights up all. Plain functions from the inspection to HTML,
// and one function that wires a rendered panel's events: node tests run them on a stub
// document (js/test/docs-snapview.test.ts).

/** HTML-escapes `s`. */
export function esc(s) {
  return String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
}

/** A byte count in plain words. */
export function words(n) {
  if (n < 1000) return `${n} bytes`;
  return `${(n / 1024).toFixed(1)} KB`;
}

/** `bytes` as base64 (plain, as `base64 -d` reads it). */
export function toBase64(bytes) {
  let s = "";
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
}

/** Base64 (plain or URL-safe, whitespace ignored) as bytes; throws on anything else. */
export function fromBase64(text) {
  const s = text.replace(/\s+/g, "").replace(/-/g, "+").replace(/_/g, "/");
  if (!/^[A-Za-z0-9+/]*={0,2}$/.test(s)) throw new Error("not base64");
  const bin = atob(s + "=".repeat((4 - (s.length % 4)) % 4));
  return Uint8Array.from(bin, (c) => c.charCodeAt(0));
}

/** URL-safe base64, for a link's fragment. */
export function toBase64Url(bytes) {
  return toBase64(bytes).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

const BREADTH = 8;
const STRING = 48;

function quote(s) {
  const clipped = s.length > STRING ? `${s.slice(0, STRING)}…` : s;
  const q = JSON.stringify(clipped);
  return s.length > STRING ? `${q} (${s.length} chars)` : q;
}

const isIdent = (k) => /^[A-Za-z_$][\w$]*$/.test(k);

/**
 * Value `v` of inspection `i` as HTML in JavaScript terms, records `depth` levels deep.
 * Every record is a `span.sv-ref[data-rec]`: the page lights up all spans of one record
 * together and opens it on a click.
 */
export function value(i, v, depth = 3, open = []) {
  if (!v) return "";
  switch (v.t) {
    case "undefined":
    case "null":
      return `<span class="sv-prim">${v.t}</span>`;
    case "hole":
      return `<span class="sv-prim">&lt;empty&gt;</span>`;
    case "uninitialized":
      return `<span class="sv-prim" title="${esc(i.notes.uninitialized)}">&lt;not initialized&gt;</span>`;
    case "bool":
    case "int":
    case "number":
      return `<span class="sv-prim">${esc(String(v.v))}</span>`;
    case "string":
      return `<span class="sv-str">${esc(quote(v.v))}</span>`;
    case "builtin":
      return `<span class="sv-named" title="${esc(i.notes.builtin)}">${esc(v.v)}</span>`;
    case "host": {
      const h = i.host.find((x) => x.name === v.v);
      return `<span class="sv-named" title="${esc(i.notes.host)}">${esc(v.v)}${h?.kind === "function" ? "()" : ""}</span>`;
    }
    case "function":
      return `<span class="sv-named" title="${esc(i.notes.function)}">the code of ${esc(i.functions[v.v]?.label ?? "?")}</span>`;
    case "script":
      return `<span class="sv-named" title="${esc(i.notes.script)}">the script ${esc(i.scripts[v.v]?.name ?? "?")}</span>`;
    case "record":
      return record(i, v.v, depth, open);
    default:
      return esc(JSON.stringify(v));
  }
}

function ref(i, id, inner) {
  const r = i.records[id];
  return `<span class="sv-ref" data-rec="${id}" title="${esc(`${r.label}: ${r.note}`)}">${inner}</span>`;
}

function record(i, id, depth, open) {
  const r = i.records[id];
  if (!r) return `#${id}`;
  const w = r.view;
  if (w.shape === "box") return value(i, w.value, depth, open);
  if (open.includes(id)) return ref(i, id, "[Circular]");
  const d = depth - 1;
  const inner = [...open, id];
  const items = (vs, more) => {
    const parts = vs.slice(0, BREADTH).map((x) => value(i, x, d, inner));
    const rest = vs.length - Math.min(vs.length, BREADTH) + (more ?? 0);
    if (rest > 0) parts.push(`<span class="sv-more">… ${rest} more</span>`);
    return parts;
  };
  const props = (ps) =>
    (ps ?? []).slice(0, BREADTH).map((p) => {
      const k = isIdent(p.key) ? esc(p.key) : esc(JSON.stringify(p.key));
      return p.get || p.set ? `${k}: <span class="sv-prim">[Getter/Setter]</span>` : `${k}: ${value(i, p.value, d, inner)}`;
    });
  switch (w.shape) {
    case "promise":
      if (w.state === "pending") return ref(i, id, "Promise { &lt;pending&gt; }");
      return ref(i, id, `Promise { ${w.state === "fulfilled" ? "" : `&lt;${esc(w.state)}&gt; `}${depth > 0 ? value(i, w.value, d, inner) : "…"} }`);
    case "array": {
      if (depth <= 0) return ref(i, id, "[…]");
      const parts = items(w.items, w.more);
      parts.push(...props(w.props));
      return ref(i, id, `[${parts.join(", ")}]`);
    }
    case "map":
    case "set": {
      const name = w.shape === "map" ? "Map" : "Set";
      if (depth <= 0) return ref(i, id, `${name}(${w.size}) {…}`);
      const parts = w.entries.slice(0, BREADTH).map((e) => (w.shape === "map" ? `${value(i, e[0], d, inner)} =&gt; ${value(i, e[1], d, inner)}` : value(i, e, d, inner)));
      if (w.size > BREADTH) parts.push(`<span class="sv-more">… ${w.size - BREADTH} more</span>`);
      return ref(i, id, `${name}(${w.size}) {${parts.length ? ` ${parts.join(", ")} ` : ""}}`);
    }
    case "object": {
      if (r.type.startsWith("Patch.") || r.type.startsWith("Object.Record.") || r.type.startsWith("Object.IteratorHelper")) return ref(i, id, esc(r.js));
      if (depth <= 0) return ref(i, id, "{…}");
      const parts = [];
      (w.elements ?? []).forEach((e, k) => e.t !== "hole" && parts.length < BREADTH && parts.push(`${k}: ${value(i, e, d, inner)}`));
      parts.push(...props(w.props));
      const total = (w.elements ?? []).filter((e) => e.t !== "hole").length + (w.props ?? []).length;
      if (total > parts.length) parts.push(`<span class="sv-more">… ${total - parts.length} more</span>`);
      const cls = w.class && w.class !== "Error" ? `${esc(w.class)} ` : "";
      if (w.class === "Error" || w.class === "HostFunction") return ref(i, id, esc(r.js));
      return ref(i, id, `${cls}{${parts.length ? ` ${parts.join(", ")} ` : ""}}`);
    }
    default:
      // Functions, activations, lists and a builtin's records: their short form.
      return ref(i, id, esc(r.js));
  }
}

/** The line `text` with a caret under column `col` (from 1). */
function sourceLine(line, col, text) {
  const n = String(line).padStart(4);
  const pad = text.slice(0, col - 1).replace(/[^\t]/g, " ");
  return `<pre class="sv-src"><span class="sv-ln">${n} |</span> ${esc(text)}\n<span class="sv-ln">${" ".repeat(4)} |</span> ${esc(pad)}<span class="sv-caret">^</span></pre>`;
}

/** Escaped text with `code` spans as code. */
const prose = (t) => esc(t).replace(/`([^`]+)`/g, "<code>$1</code>");

/** A sentence with its note behind a "why" toggle. */
function noted(note, body) {
  return `${body} <button class="sv-why" type="button" aria-expanded="false" title="What is this?">?</button><span class="sv-note" hidden>${esc(note)}</span>`;
}

function binding(i, b) {
  const boxed = b.box === null || b.box === undefined ? "" : ` <span class="sv-box" title="${esc(i.notes.box)}">shared</span>`;
  return `<li><code class="sv-name">${esc(b.name)}</code> = ${value(i, b.value)}${boxed}</li>`;
}

function node(i, n) {
  const at = n.source ? ` data-line="${n.source.line}" data-col="${n.source.col}"` : "";
  const title = n.source ? `<button class="sv-at" type="button"${at} title="Show where it waits in the code">${prose(n.title)}</button>` : prose(n.title);
  let h = `<li class="sv-node sv-${n.kind}"${n.record === null ? "" : ` data-node-rec="${n.record}"`}>`;
  h += `<div class="sv-title">${noted(n.note, title)}</div>`;
  if (n.source) h += sourceLine(n.source.line, n.source.col, n.source.text);
  const binds = n.bindings.filter((b) => !b.param);
  if (binds.length) h += `<ul class="sv-binds">${binds.map((b) => binding(i, b)).join("")}</ul>`;
  if (n.captured.length) h += `<div class="sv-sub">from outside (captured):</div><ul class="sv-binds">${n.captured.map((b) => binding(i, b)).join("")}</ul>`;
  if (n.uninitialized.length) h += `<div class="sv-sub" title="${esc(i.notes.uninitialized)}">not reached yet: ${n.uninitialized.map((u) => `<code>${esc(u)}</code>`).join(", ")}</div>`;
  if (n.doing && !n.title.endsWith(n.doing)) h += `<div class="sv-doing">${prose(n.doing)}</div>`;
  if (n.detail.length) h += `<details class="sv-store"><summary>how djs stores it</summary><ul>${n.detail.map((d) => `<li>${linkIds(esc(d))}</li>`).join("")}</ul></details>`;
  h += `</li>`;
  // A chain goes on below, one link after another; where several things wait on one
  // promise, each gets a branch.
  if (n.children.length === 1) h += node(i, n.children[0]);
  else if (n.children.length > 1) h += `<li class="sv-fork"><span>${n.children.length} things wait for it:</span><ol class="sv-chain">${n.children.map((c) => node(i, c)).join("")}</ol></li>`;
  return h;
}

/** `#14` in storage-layer text as a link to record 14. */
const linkIds = (html) => html.replace(/#(\d+)/g, `<a href="#" class="sv-id" data-rec="$1">#$1</a>`);

/** The parts of the size as a bar and in words. */
function sizes(i) {
  const total = i.bytes || 1;
  const bar = i.sizes
    .filter((s) => s.bytes > 0)
    .map((s) => `<span class="sv-size-${s.part}" style="inline-size:${((100 * s.bytes) / total).toFixed(2)}%" title="${esc(`${words(s.bytes)} ${s.note}`)}"></span>`)
    .join("");
  const legend = i.sizes
    .filter((s) => s.bytes > 0)
    .map((s) => `<li><span class="sv-swatch sv-size-${s.part}"></span>${esc(words(s.bytes))} ${esc(s.note)}</li>`)
    .join("");
  return `<div class="sv-sizes"><div class="sv-bar" role="img" aria-label="${esc(i.sizeText)}">${bar}</div><ul class="sv-legend">${legend}</ul></div>`;
}

/** What a loading djs must have: named, not stored. */
function needs(i) {
  const fns = (ids) => {
    const names = ids.map((f) => `<code>${esc(i.functions[f].label)}</code>`);
    return names.length > 1 ? `${names.slice(0, -1).join(", ")} and ${names.at(-1)}` : names.join("");
  };
  let h = `<ul class="sv-needs">`;
  if (i.needs.bytecode.length) h += `<li>${noted("A stopped function's frame only means the same in the same code: a loading djs compiles the script again and checks each of these by its bytecode hash.", `the same code for ${fns(i.needs.bytecode)}, which ${i.needs.bytecode.length === 1 ? "is" : "are"} stopped halfway`)}</li>`;
  if (i.needs.interface.length) h += `<li>${noted("A function held only as a value may change its code: a later djs needs the same parameters and captured variables.", `the same parameters and captured variables for ${fns(i.needs.interface)}`)}</li>`;
  if (i.needs.builtins.length) h += `<li>${noted(i.notes.builtin, `builtins every djs has: ${i.needs.builtins.map((b) => `<code>${esc(b)}</code>`).join(", ")}`)}</li>`;
  if (i.needs.host.length) h += `<li>${noted(i.notes.host, `from the host: ${i.host.map((x) => `<code>${esc(x.name)}${x.kind === "function" ? "()" : ""}</code>`).join(", ")}`)}</li>`;
  return `${h}</ul>`;
}

/** A field as stored, records as links. */
function raw(i, f) {
  if (f === null || f === undefined) return "";
  if (Array.isArray(f)) return `[${f.map((x) => raw(i, x)).join(", ")}]`;
  if (typeof f !== "object") return esc(String(f));
  if ("t" in f) {
    switch (f.t) {
      case "record":
        return `<a href="#" class="sv-id" data-rec="${f.v}">#${f.v}</a>`;
      case "string":
        return esc(quote(f.v));
      case "undefined":
      case "null":
      case "hole":
      case "uninitialized":
        return esc(f.t);
      case "function":
        return `code of ${esc(i.functions[f.v]?.label ?? "?")}`;
      default:
        return esc(String(f.v));
    }
  }
  if ("key" in f) return `${raw(i, f.key)} = ${f.get ? `get ${raw(i, f.get)}, set ${raw(i, f.set)}` : raw(i, f.value)}`;
  return `{${Object.entries(f).map(([k, x]) => `${esc(k)}: ${raw(i, x)}`).join(", ")}}`;
}

/** Record `id` in full: what it is, then its fields as stored and what holds it. */
export function recordCard(i, id) {
  const r = i.records[id];
  if (!r) return "";
  const fields = Object.entries(r.fields)
    .filter(([, f]) => !(Array.isArray(f) && f.length === 0) && !(f && f.t === "undefined"))
    .map(([k, f]) => `<tr><th>${esc(k)}</th><td>${raw(i, f)}</td></tr>`)
    .join("");
  const held = (r.heldBy ?? []).map((h) => (h === "vm" ? "the VM record" : `<a href="#" class="sv-id" data-rec="${h}">#${h}</a>`)).join(", ");
  return (
    `<div class="sv-card" data-card="${id}"><p><b>#${id}</b> ${esc(r.known || r.label)}: ${value(i, { t: "record", v: id }, 3)}</p>` +
    `<p class="sv-note-text">${esc(r.note)}</p>` +
    `<p class="muted">Stored as <code>${esc(r.type)}</code>, ${r.end - r.start} bytes at ${r.start}..${r.end} of the snapshot${held ? `; held by ${held}` : ""}. <a href="#" class="sv-bytes-of" data-rec="${id}">show its bytes</a></p>` +
    `<table class="sv-fields">${fields}</table></div>`
  );
}

/** Every record as djs stores it: id, kind, what the view calls it, size. */
function recordList(i) {
  const rows = i.records
    .map((r, id) => `<tr data-rec="${id}"><td><a href="#" class="sv-id" data-rec="${id}">#${id}</a></td><td><code>${esc(r.type)}</code></td><td>${esc(r.known)}</td><td class="num">${r.end - r.start}</td></tr>`)
    .join("");
  return `<table class="sv-records"><thead><tr><th>record</th><th>kind</th><th>what it is</th><th>bytes</th></tr></thead><tbody>${rows}</tbody></table>`;
}

const HEX_MAX = 64 * 1024;

/** The bytes as hex, the sections labelled, record `sel`'s bytes marked. */
export function hexDump(i, bytes, sel = -1) {
  const r = i.records[sel];
  const [from, to] = r ? [r.start, r.end] : [-1, -1];
  const n = Math.min(bytes.length, HEX_MAX);
  const starts = new Map(i.sections.map((s) => [s.start, s.name]));
  let h = "";
  for (let row = 0; row < n; row += 16) {
    for (let k = row; k < row + 16 && k < n; k++) {
      const name = starts.get(k);
      if (name) h += `<span class="sv-hex-sec">— ${esc(name)} (${words(i.sections.find((s) => s.start === k).end - k)}) at ${k}</span>\n`;
    }
    let hex = "";
    let txt = "";
    for (let k = row; k < row + 16; k++) {
      if (k >= n) {
        hex += "   ";
        continue;
      }
      const b = bytes[k];
      const cell = b.toString(16).padStart(2, "0");
      const ch = b >= 0x20 && b < 0x7f ? String.fromCharCode(b) : "·";
      const on = k >= from && k < to;
      hex += on ? `<mark>${cell}</mark> ` : `${cell} `;
      txt += on ? `<mark>${esc(ch)}</mark>` : esc(ch);
    }
    h += `<span class="sv-ln">${row.toString(16).padStart(6, "0")}</span>  ${hex} ${txt}\n`;
  }
  if (bytes.length > n) h += `… ${bytes.length - n} more bytes\n`;
  return h;
}

/**
 * The whole panel for snapshot `i` (an inspection) of `bytes`. `actions`: the Copy,
 * Download and Link buttons (the live examples and the restore box have them).
 */
export function panelHtml(i, { actions = true } = {}) {
  let h = `<div class="sv">`;
  h += `<p class="sv-summary">${esc(i.summary)}</p>`;
  if (actions) {
    h +=
      `<div class="sv-actions">` +
      `<button type="button" data-act="copy" title="The bytes of run.snapshot(), as base64">Copy as base64</button>` +
      `<button type="button" data-act="download" title="The bytes of run.snapshot(), as a file for djs describe and djs resume">Download .snapshot</button>` +
      `<button type="button" data-act="link" title="The snapshots page with this snapshot in the link's fragment, which is never sent to a server">Link to restore it</button>` +
      `<span class="sv-act-status" role="status"></span></div>`;
  }
  h += `<ol class="sv-chain">${i.view.calls.map((n) => node(i, n)).join("")}</ol>`;
  if (i.view.others.length) h += `<p class="sv-sub">Also suspended, not waiting for the host:</p><ol class="sv-chain">${i.view.others.map((n) => node(i, n)).join("")}</ol>`;
  h += `<p class="sv-h">Named, not stored: a loading djs must have them</p>${needs(i)}`;
  h += `<p class="sv-h">Its size: ${esc(words(i.bytes))}</p>${sizes(i)}`;
  h += `<div class="sv-card-slot"></div>`;
  h += `<details class="sv-layer sv-layer-records"><summary>How djs stores it: ${i.records.length} records, each a kind with named fields</summary>${recordList(i)}</details>`;
  h += `<details class="sv-layer sv-layer-bytes"><summary>The bytes</summary><pre class="sv-hex"></pre></details>`;
  return `${h}</div>`;
}

/**
 * Wires a rendered panel (`root`, the element holding `panelHtml`): the notes' toggles,
 * identity on hover, records opened on a click, the hex view, the actions. `ctx`:
 * `{ i, bytes, mark(line, col), copy(text), download(bytes, name), link(bytes) }`.
 */
export function wirePanel(root, ctx) {
  const { i, bytes } = ctx;
  const hex = root.querySelector(".sv-hex");
  const bytesLayer = root.querySelector(".sv-layer-bytes");
  let sel = -1;
  const showHex = () => {
    if (hex && bytesLayer?.open) hex.innerHTML = hexDump(i, bytes, sel);
  };
  bytesLayer?.addEventListener("toggle", showHex);
  const open = (id) => {
    sel = id;
    const slot = root.querySelector(".sv-card-slot");
    slot.innerHTML = recordCard(i, id);
    for (const tr of root.querySelectorAll(".sv-records tr.is-selected")) tr.classList.remove("is-selected");
    root.querySelector(`.sv-records tr[data-rec="${id}"]`)?.classList.add("is-selected");
    showHex();
    slot.scrollIntoView?.({ block: "nearest" });
  };
  root.addEventListener("click", (ev) => {
    const t = ev.target;
    const why = t.closest?.(".sv-why");
    if (why) {
      const note = why.nextElementSibling;
      note.hidden = !note.hidden;
      why.setAttribute("aria-expanded", String(!note.hidden));
      return;
    }
    const at = t.closest?.(".sv-at");
    if (at) {
      ctx.mark?.(Number(at.dataset.line), Number(at.dataset.col));
      return;
    }
    const bytesOf = t.closest?.(".sv-bytes-of");
    if (bytesOf) {
      ev.preventDefault();
      sel = Number(bytesOf.dataset.rec);
      if (bytesLayer) bytesLayer.open = true;
      showHex();
      hex?.querySelector("mark")?.scrollIntoView?.({ block: "center" });
      return;
    }
    const act = t.closest?.("[data-act]");
    if (act) {
      const status = root.querySelector(".sv-act-status");
      const say = (s) => status && (status.textContent = s);
      if (act.dataset.act === "copy") ctx.copy?.(toBase64(bytes)).then(() => say(`copied ${words(Math.ceil(bytes.length / 3) * 4)} of base64`), (e) => say(`could not copy: ${e.message}`));
      if (act.dataset.act === "download") ctx.download?.(bytes, "pause.snapshot");
      if (act.dataset.act === "link") ctx.link?.(bytes)?.then?.(() => say("link copied"), () => {});
      return;
    }
    const r = t.closest?.("[data-rec]");
    if (r) {
      ev.preventDefault();
      open(Number(r.dataset.rec));
    }
  });
  // One object reached twice is one record: hovering one shows all.
  const light = (id, on) => {
    for (const e of root.querySelectorAll(`.sv-ref[data-rec="${id}"]`)) e.classList.toggle("is-same", on);
  };
  root.addEventListener("mouseover", (ev) => {
    const r = ev.target.closest?.(".sv-ref");
    if (r) light(r.dataset.rec, true);
  });
  root.addEventListener("mouseout", (ev) => {
    const r = ev.target.closest?.(".sv-ref");
    if (r) light(r.dataset.rec, false);
  });
  return { open };
}
