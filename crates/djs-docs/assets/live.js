// Live examples: every ```js live block of the guide is an editor whose code runs in
// djs-wasm in the page (D2103). A `debugger;` in the code pauses the run there (D3050):
// the page writes it out as a snapshot (D3720), shows it in the Snapshot tab (D3761,
// snapview.js), and Continue resumes the run from it in a fresh sandbox. A ```js live
// restore block also loads a snapshot pasted as base64 or given in the page's link
// (`#restore=...`). The editor is a textarea with transparent text over the
// highlighted `pre` the page already has (render.rs live_block): the textarea does the
// editing (caret, selection, undo, input methods, screen readers) and highlight.js
// repaints the `pre` under it on every input. Without JavaScript (or from file://) the
// page shows the code and the output and bytecode generated at build time.
import { esc, tokens } from "./highlight.js";
import { fromBase64, panelHtml, toBase64Url, wirePanel, words } from "./snapview.js";

const FUEL_SLICE = 50;
// Stops a runaway loop (`while (true)` typed halfway) in well under a second.
const MAX_FUEL = 20_000_000;
// The parameters of a codemode body, as the build-time output and listings use them.
const PARAMS = "tools\nconsole";
const INDENT = "  ";
const AUTORUN_MS = 600;
const MAC = /Mac|iPhone|iPad/.test(navigator.platform);
const RUN_KEYS = MAC ? "⌘↵" : "Ctrl+↵";

// The sandbox package and the wasm module load on first use: a page that is only read
// fetches neither.
let libPromise;
function lib() {
  // (`djsLiveImports`: the same four from elsewhere, for the node tests' stub page.)
  libPromise ??= (
    globalThis.djsLiveImports?.() ??
    Promise.all([
      import("./djs/sandbox.js"),
      import("./djs/layout.generated.js"),
      WebAssembly.compileStreaming(fetch(new URL("./djs/djs_wasm.wasm", import.meta.url))),
      import("./djs/abi.js"),
    ])
  ).then(([{ Sandbox }, { LAYOUT }, module, { djsExports }]) => {
    // The instance's operations by name (the module has one export, `djs`) serve the
    // Bytecode tab.
    disasmExports = djsExports(module);
    return {
      Sandbox,
      module,
      kindNames: Object.fromEntries(Object.entries(LAYOUT.cells).map(([n, c]) => [c.kind, n])),
      classNames: Object.fromEntries(Object.entries(LAYOUT.objectClasses).map(([n, c]) => [c, n])),
    };
  });
  return libPromise;
}

// The Bytecode tab: djs-wasm built with the `docs` feature (scripts/build-wasm.sh --docs)
// exports `djs_disasm`, the listing `djs disasm` prints, so an edited example shows its
// own bytecode. With the shipping module the tab keeps the build-time listing.
let disasmExports;
function disasm(module, src, ts) {
  const x = disasmExports;
  if (!x?.djs_disasm) return null;
  const enc = new TextEncoder();
  const params = enc.encode(PARAMS);
  const body = enc.encode(src);
  const at = x.djs_staging(params.length + body.length);
  const mem = new Uint8Array(x.memory.buffer);
  mem.set(params, at);
  mem.set(body, at + params.length);
  // Goal bits as for vm_compile_file: 1 a codemode body, 2 TypeScript.
  x.djs_disasm(ts ? 3 : 1, params.length, body.length);
  return new TextDecoder().decode(new Uint8Array(x.memory.buffer, x.djs_out_ptr(), x.djs_out_len()));
}

/**
 * Walks the arena cell by cell: the heap is walkable because every header has a size.
 * Cells below `baseEnd` (the fresh VM's length) are the base realm every VM starts as a
 * copy of; cells after it are what the program allocated (D2340).
 */
function walk(sb, { kindNames, classNames }, baseEnd = Infinity) {
  const heap = sb.heapView();
  const len = heap.length;
  const dv = new DataView(heap.buffer, heap.byteOffset, len);
  const kinds = new Map();
  const classes = new Map();
  let cells = 0;
  const own = { cells: 0, bytes: 0, kinds: new Map() };
  for (let off = 0; off + 8 <= len; ) {
    const kind = dv.getUint8(off);
    const size = dv.getUint32(off + 4, true);
    if (size === 0) break;
    const name = kindNames[kind] ?? `kind ${kind}`;
    const k = kinds.get(name) ?? { n: 0, bytes: 0 };
    k.n++;
    k.bytes += size;
    kinds.set(name, k);
    if (name === "Object") {
      const cls = classNames[dv.getUint32(off + 8, true)] ?? "?";
      classes.set(cls, (classes.get(cls) ?? 0) + 1);
    }
    if (off >= baseEnd) {
      own.cells++;
      own.bytes += size;
      const o = own.kinds.get(name) ?? { n: 0, bytes: 0 };
      o.n++;
      o.bytes += size;
      own.kinds.set(name, o);
    }
    cells++;
    off += size;
  }
  return { len, cells, kinds, classes, own };
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

/** The Memory tab at a pause: the arena, and what of it the snapshot keeps. */
function memoryAtPause(before, after, i) {
  return (
    heapHtml(before, after, false) +
    `<p>The snapshot of this pause keeps none of the arena's bytes: it holds the run's data as ${i.records.length} records ` +
    `(${words(i.sizes.find((s) => s.part === "data")?.bytes ?? 0)} with their strings), names the builtins and the host's values instead of storing them, ` +
    `and stores the script's text for a loading djs to compile again: ${words(i.bytes)} in all. The sandbox and its arena are gone once the snapshot is written.</p>`
  );
}

function heapHtml(before, after, gc = true) {
  const own = after.own;
  const rows = [...own.kinds.entries()].sort((a, b) => b[1].bytes - a[1].bytes);
  const max = rows[0]?.[1].bytes ?? 1;
  let h = `<p>This program: <b>${own.cells}</b> cells, <b>${kb(own.bytes)}</b>, allocated after the base realm.</p>`;
  if (rows.length > 0) {
    h += `<table class="bars">`;
    for (const [name, k] of rows.slice(0, 12)) {
      h += `<tr><td class="mono">${name}</td><td class="num">${k.n}</td><td class="bar"><span style="width:${(100 * k.bytes) / max}%"></span></td><td class="num">${kb(k.bytes)}</td></tr>`;
    }
    h += `</table>`;
  }
  h += `<p class="muted">The base realm before it (the builtins, their code and names, which every VM starts with as a copy of the base image): ${before.cells} cells, ${kb(before.len)}. Arena in total: ${after.cells} cells, ${kb(after.len)}. Counted by walking the arena cell by cell, with the layout the engine exports.${gc ? ` Garbage stays until the next collection, which slides the program's live cells together: <button class="live-gc">Collect now</button>` : ""}</p>`;
  return h;
}

// Uncaught errors. A thrown Error's stack is not readable from the host once the run is
// over, so the code runs inside a try whose catch hands the stack out in a plain object.
// The `try {` shares line 1 with the code, so only line 1's columns move (by WRAP_COLS).
const WRAP_HEAD = "try {";
const WRAP_COLS = WRAP_HEAD.length;
const wrap = (src) =>
  `${WRAP_HEAD}${src}\n} catch (e) { throw e instanceof Error && typeof e.stack === "string" ? { __live: String(e), stack: e.stack } : e; }`;

/** Where in `src` (a UTF-16 index) line `line`, column `col` (both from 1) is. */
function offsetOf(src, line, col) {
  let at = 0;
  for (let l = 1; l < line; l++) {
    const nl = src.indexOf("\n", at);
    if (nl < 0) return src.length;
    at = nl + 1;
  }
  return Math.min(at + col - 1, src.length);
}

/** The line, with a caret under the column: the way compilers point at an error. */
function frame(src, line, col) {
  const text = src.split("\n")[line - 1];
  if (text === undefined) return "";
  const n = String(line);
  return `\n\n ${n}  | ${text}\n ${" ".repeat(n.length)}  | ${text.slice(0, col - 1).replace(/[^\t]/g, " ")}^`;
}

/** A failed run as `{ text, line, col }` (line 0: no position). */
function describe(e, src, wrapped) {
  if (e?.kind === "syntax") {
    const m = /^(.*) at (\d+):(\d+)$/s.exec(e.message);
    if (!m) return { text: e.message, line: 0, col: 0 };
    const [line, col] = [+m[2], +m[3]];
    return { text: `${m[1]}\n    at line ${line}, column ${col}${frame(src, line, col)}`, line, col };
  }
  if (e?.kind === "fuel") {
    return {
      text: `Stopped: ${e.message}. A live example may use ${MAX_FUEL.toLocaleString("en")} units; is there an endless loop?`,
      line: 0,
      col: 0,
    };
  }
  if (e?.kind === "threw") {
    const v = e.value;
    if (wrapped && v && typeof v === "object" && typeof v.__live === "string") {
      let first = null;
      const stack = String(v.stack)
        .replace(/:(\d+):(\d+)\)/g, (all, l, c) => (l === "1" ? `:1:${Math.max(1, c - WRAP_COLS)})` : all))
        .trimEnd();
      const m = /:(\d+):(\d+)\)/.exec(stack);
      if (m) first = [+m[1], +m[2]];
      return {
        text: `Uncaught ${v.__live}${stack ? "\n" + stack : ""}${first ? frame(src, ...first) : ""}`,
        line: first?.[0] ?? 0,
        col: first?.[1] ?? 0,
      };
    }
    return { text: `Uncaught ${e.message}`, line: 0, col: 0 };
  }
  return { text: String(e?.message ?? e), line: 0, col: 0 };
}

/**
 * The error mark for line `line`, column `col` (from 1): from there to the end of the
 * line, as `[from, to]` indices into `src` (`from === to` marks a point).
 */
function markAt(src, line, col) {
  const from = offsetOf(src, line, col);
  let to = src.indexOf("\n", from);
  if (to < 0) to = src.length;
  while (to > from && /\s/.test(src[to - 1])) to--;
  return [from, to];
}

/** The highlighted HTML of `src`, with the `[from, to]` of `mark` (or null) underlined. */
function paint(src, mark) {
  const [from, to] = mark ?? [-1, -1];
  let html = "";
  let at = 0;
  for (const [role, s] of tokens(src)) {
    const end = at + s.length;
    let inner = esc(s);
    if (from < to && from < end && to > at) {
      const [a, b] = [Math.max(from, at) - at, Math.min(to, end) - at];
      inner = `${esc(s.slice(0, a))}<span class="live-mark">${esc(s.slice(a, b))}</span>${esc(s.slice(b))}`;
    } else if (from === to && from >= at && from < end) {
      // A point (the end of a line): a marker that takes no room.
      const k = from - at;
      inner = `${esc(s.slice(0, k))}<span class="live-mark-at"></span>${esc(s.slice(k))}`;
    }
    html += role ? `<span class="tok-${role}">${inner}</span>` : inner;
    at = end;
  }
  if (from === to && from >= src.length) html += `<span class="live-mark-at"></span>`;
  // A pre drops a final newline; the textarea shows the empty line after it.
  if (src === "" || src.endsWith("\n")) html += " ";
  return html;
}

// ---- Editing ----------------------------------------------------------------------

/** Replaces the selection with `text`, through the browser's editing so undo works. */
function insert(ta, text) {
  const ok = text === "" ? document.execCommand("delete") : document.execCommand("insertText", false, text);
  if (!ok) {
    ta.setRangeText(text, ta.selectionStart, ta.selectionEnd, "end");
    ta.dispatchEvent(new InputEvent("input", { inputType: "insertText", data: text }));
  }
}

/** Indents (or with `out`, dedents) the lines the selection touches. */
function indentLines(ta, out) {
  const v = ta.value;
  const [s, e] = [ta.selectionStart, ta.selectionEnd];
  const from = v.lastIndexOf("\n", s - 1) + 1;
  // A selection that ends at the start of a line leaves that line alone.
  let to = v.indexOf("\n", e > s && v[e - 1] === "\n" ? e - 1 : e);
  if (to < 0) to = v.length;
  const lines = v.slice(from, to).split("\n");
  const changed = lines.map((l) => (out ? l.replace(/^( {1,2}|\t)/, "") : l === "" ? l : INDENT + l));
  const text = changed.join("\n");
  if (text === v.slice(from, to)) return;
  const firstDelta = changed[0].length - lines[0].length;
  ta.setSelectionRange(from, to);
  insert(ta, text);
  const ns = Math.max(from, s + firstDelta);
  ta.setSelectionRange(ns, s === e ? ns : e + text.length - (to - from));
}

const PAIRS = { "{": "}", "[": "]", "(": ")" };

function onKey(ev, ex) {
  const ta = ex.ta;
  if (ev.isComposing) return;
  const v = ta.value;
  const [s, e] = [ta.selectionStart, ta.selectionEnd];
  if (ev.key === "Enter" && (ev.metaKey || ev.ctrlKey)) {
    ev.preventDefault();
    if (ex.cont) ex.cont();
    else run(ex);
  } else if (ev.key === "Escape") {
    // Tab indents in the editor; Escape gives the keyboard back to the page, on the
    // result tabs right after the editor, so that Tab goes on from there.
    ex.el.querySelector('[role="tab"][aria-selected="true"]').focus();
  } else if (ev.key === "Tab" && !ev.altKey && !ev.ctrlKey && !ev.metaKey) {
    ev.preventDefault();
    if (ev.shiftKey || v.slice(s, e).includes("\n")) indentLines(ta, ev.shiftKey);
    else insert(ta, INDENT);
  } else if (ev.key === "Enter" && !ev.altKey) {
    // The new line starts at the indentation of this one, one step deeper after an
    // opening bracket; between a pair of brackets the closing one gets its own line.
    ev.preventDefault();
    const from = v.lastIndexOf("\n", s - 1) + 1;
    const indent = /^[ \t]*/.exec(v.slice(from, s))[0];
    const open = v.slice(from, s).trimEnd().at(-1);
    if (PAIRS[open]) {
      if (v[e] === PAIRS[open]) {
        insert(ta, `\n${indent}${INDENT}\n${indent}`);
        const caret = s + 1 + indent.length + INDENT.length;
        ta.setSelectionRange(caret, caret);
      } else insert(ta, `\n${indent}${INDENT}`);
    } else insert(ta, `\n${indent}`);
  } else if (ev.key === "Backspace" && s === e && s > 0 && !ev.altKey && !ev.metaKey && !ev.ctrlKey) {
    // In the indentation, Backspace removes one step of it.
    const from = v.lastIndexOf("\n", s - 1) + 1;
    const before = v.slice(from, s);
    if (before.length >= 2 && /^ +$/.test(before)) {
      ev.preventDefault();
      const n = before.length % 2 || 2;
      ta.setSelectionRange(s - n, s);
      insert(ta, "");
    }
  }
}

// ---- Running ----------------------------------------------------------------------

/** The code has a `debugger` statement (the keyword, not in a string or a comment). */
const hasDebugger = (src) => tokens(src).some(([role, s]) => role === "keyword" && s === "debugger");

/** Bytes as KiB with one decimal, or bytes. */
const size = (n) => (n < 1024 ? `${n} B` : `${(n / 1024).toFixed(1)} KiB`);

/** The gzip size of `bytes` (CompressionStream), or null where the browser has none. */
async function gzipSize(bytes) {
  if (typeof CompressionStream === "undefined") return null;
  const stream = new Blob([bytes]).stream().pipeThrough(new CompressionStream("gzip"));
  return (await new Response(stream).arrayBuffer()).byteLength;
}

/**
 * A run with `debugger;` pauses (D3050) as part of the normal Run: a host-driven run in
 * which `debugger;` is an open call of the async host function `debugger`, so the run is
 * idle at the statement and the page writes it out as a snapshot there (D3720). Run turns
 * into Continue, which loads that snapshot into a fresh sandbox, settles the pause and
 * runs on to the next pause or the end. The output of all of them is one transcript, with
 * a line at each pause; its "snapshot N" links open that snapshot in the Snapshot tab
 * (D3761). A run that cannot be snapshotted (a value the snapshot refuses) says why and
 * continues in the same sandbox. `from`: a snapshot to start from instead of the code
 * (the restore box), already checked to be one.
 *
 * Each pause line says where the run stopped (`Sandbox.callSite`, D3054): a line and
 * column of the example, in every part of the run (a loaded run is in the same script).
 */
async function runPausing(ex, l, id, from = null) {
  const { el } = ex;
  const pane = (n) => el.querySelector(`[data-pane="${n}"]`);
  const setStatus = (t) => (el.querySelector(".live-status").textContent = t);
  const { Sandbox, module } = l;
  const src = ex.ta.value;
  const lines = [];
  const out = (...a) => lines.push(esc(a.map(show).join(" ")));
  // `html` is markup: escaped text and snapshot links.
  const marker = (html) => lines.push(`<span class="live-pause">${html}</span>`);
  const showOut = () => (pane("out").innerHTML = lines.join("\n") || `<span class="muted">(no output)</span>`);
  // The snapshots of this run, in order, shown by the transcript's links.
  const snaps = [];
  const showSnap = (k) => showSnapshot(ex, snaps[k], `Snapshot ${k + 1} of this run`);
  pane("out").onclick = (ev) => {
    const a = ev.target.closest?.("a.live-snap-link");
    if (!a) return;
    ev.preventDefault();
    showSnap(Number(a.dataset.snap));
    select(ex, "snapshot");
  };
  const snapLink = (k) => `<a href="#" class="live-snap-link" data-snap="${k}">snapshot ${k + 1}</a>`;
  const pausesOf = (run) => run.calls.filter((c) => c.fn === "debugger");
  /** Where the run waits at its open pauses, as text: lines and columns of the example. */
  const where = (sb, run) => {
    const sites = [];
    for (const c of pausesOf(run)) {
      const at = sb.callSite(c.id);
      if (at && !sites.some((s) => s.line === at.line && s.column === at.column)) sites.push(at);
    }
    if (sites.length === 0) return "debugger;";
    return `line${sites.length > 1 ? "s" : ""} ${sites.map((s) => `${s.line}:${s.column}`).join(", ")}`;
  };
  /** Settles the run's pauses and runs on to the next idle point or the end. */
  const goOn = async (run) => {
    for (const c of pausesOf(run)) run.settle(c.id, { ok: true, value: undefined });
    await run.advance();
  };
  const opts = sandboxOptions(ex, module, out);
  let pauses = 0;
  let resumes = 0;
  const done = () => {
    ex.cont = null;
    setRunLabel(ex, false);
  };
  // `run` is idle or ended in sandbox `sb`; `before`: the fresh sandbox's arena, for the
  // Memory tab; `last`: the index of the snapshot this part continued from, -1 for none.
  const step = async (sb, run, before, last) => {
    if (id !== ex.runs) return sb.dispose();
    if (run.state !== "idle" || pausesOf(run).length === 0) {
      pane("memory").innerHTML = heapHtml(before, walk(sb, l, before.len), false);
      sb.dispose();
      const failure = run.state === "threw" ? describe(run.error, src, false) : null;
      if (failure) lines.push(`<span class="live-error">${esc(failure.text)}</span>`);
      showOut();
      setStatus(failure ? "threw" : resumes ? `ran to the end, continued ${resumes}× from a snapshot` : "ran to the end");
      if (last >= 0) showSnap(last);
      done();
      return;
    }
    pauses++;
    const at = where(sb, run);
    let snap;
    let inspection;
    try {
      snap = run.snapshot();
      inspection = sb.inspectSnapshot(snap);
    } catch (e) {
      // Not snapshottable here: say why, and let Continue go on in this sandbox.
      marker(`⏸ paused at ${esc(at)}, but the run cannot be snapshotted here:`);
      lines.push(`<span class="live-error">${esc(String(e?.message ?? e).replace(/^cannot snapshot the run:\s*/, "").trimEnd())}</span>`);
      showOut();
      pane("snapshot").innerHTML = `<p class="muted">No snapshot at this pause: the run holds something a snapshot cannot (see the Output tab).</p>`;
      setStatus(`paused at ${at} (pause ${pauses}), not snapshottable`);
      ex.cont = async () => {
        ex.cont = null;
        lines.push(`<span class="live-pause">▶ continued in the same sandbox</span>`);
        showOut();
        setRunLabel(ex, false);
        await goOn(run).catch(() => {});
        step(sb, run, before, last);
      };
      setRunLabel(ex, true);
      return;
    }
    // The arena at the pause, before the sandbox goes.
    pane("memory").innerHTML = memoryAtPause(before, walk(sb, l, before.len), inspection);
    sb.dispose();
    const gz = await gzipSize(snap);
    if (id !== ex.runs) return;
    const n = snaps.push({ bytes: snap, inspection, gz }) - 1;
    marker(`⏸ paused at ${esc(at)}: the run is now ${snapLink(n)} (${esc(size(snap.length))}), and its sandbox is gone`);
    showOut();
    showSnap(n);
    setStatus(`paused at ${at} (pause ${pauses})`);
    ex.cont = async () => {
      ex.cont = null;
      setRunLabel(ex, false);
      resumes++;
      lines[lines.length - 1] = `<span class="live-pause">⏸ paused at ${esc(at)}: continued from ${snapLink(n)} (${esc(size(snap.length))}) in a fresh sandbox</span>`;
      showOut();
      setStatus("continuing…");
      let sb2;
      try {
        sb2 = new Sandbox(opts);
        const before2 = walk(sb2, l);
        const loaded = sb2.loadSnapshot(snap);
        await goOn(loaded).catch(() => {});
        await step(sb2, loaded, before2, n);
      } catch (e) {
        sb2?.dispose();
        setStatus(`could not continue: ${e.message}`);
        done();
      }
    };
    setRunLabel(ex, true);
  };
  showOut();
  const sb = new Sandbox(opts);
  const before = walk(sb, l);
  let run;
  try {
    if (from) {
      run = sb.loadSnapshot(from);
      marker(`▶ restored from the snapshot you gave (${esc(size(from.length))}), in a fresh sandbox`);
    } else run = sb.begin(src, {}, { filename: "example.js" });
  } catch (e) {
    sb.dispose();
    throw e;
  }
  await step(sb, run, before, -1);
}

/** The options of the live examples' sandboxes: the docs' fixed setup (console, debugger). */
function sandboxOptions(ex, module, out) {
  return {
    module,
    typescript: ex.ts,
    console: { log: out, info: out, warn: out, error: out },
    fuelSlice: FUEL_SLICE,
    maxFuel: MAX_FUEL,
    now: 0,
    // The pause: the run is idle while this call is open, so it can be snapshotted.
    debugger: true,
  };
}

/** The Snapshot tab shows snapshot `s` ({ bytes, inspection, gz }), headed `head`. */
function showSnapshot(ex, s, head) {
  const pane = ex.el.querySelector('[data-pane="snapshot"]');
  const gz = s.gz === null || s.gz === undefined ? "" : ` (${size(s.gz)} with gzip)`;
  pane.innerHTML = `<p class="sv-head">${esc(head)}${esc(gz)}</p>${panelHtml(s.inspection)}`;
  wirePanel(pane, {
    i: s.inspection,
    bytes: s.bytes,
    // A frame's title marks where it waits in the editor.
    mark: (line, col) => {
      ex.mark = markAt(ex.ta.value, line, col);
      repaint(ex);
      ex.el.querySelector(".live-editor").scrollIntoView?.({ block: "nearest" });
    },
    copy: (text) => navigator.clipboard.writeText(text),
    download: (bytes, name) => {
      const a = document.createElement("a");
      a.href = URL.createObjectURL(new Blob([bytes], { type: "application/octet-stream" }));
      a.download = name;
      a.click();
      setTimeout(() => URL.revokeObjectURL(a.href), 1000);
    },
    link: (bytes) => {
      const url = restoreUrl(bytes);
      const status = pane.querySelector(".sv-act-status");
      status.innerHTML = `<a href="${esc(url)}">the link</a> (${esc(words(url.length))}) is copied`;
      return navigator.clipboard.writeText(url);
    },
  });
}

/** The snapshots page with `bytes` in the fragment (never sent to a server). */
function restoreUrl(bytes) {
  return new URL(`snapshots.html#restore=${toBase64Url(bytes)}`, location.href).href;
}

/** The Run button: "Continue" while a run is paused at `debugger;`. */
function setRunLabel(ex, paused) {
  const b = ex.el.querySelector(".live-run");
  b.innerHTML = `${paused ? "Continue" : "Run"} <kbd>${RUN_KEYS}</kbd>`;
  b.classList.toggle("is-paused", paused);
}

/** Runs the example (from snapshot `from` instead, the restore box's). */
async function run(ex, from = null) {
  const { el, ta } = ex;
  clearTimeout(ex.timer);
  const id = ++ex.runs;
  const src = ta.value;
  const pane = (n) => el.querySelector(`[data-pane="${n}"]`);
  const setStatus = (t) => (el.querySelector(".live-status").textContent = t);
  setStatus("running…");
  el.classList.add("is-running");
  try {
    const l = await lib();
    const { Sandbox, module } = l;
    if (ex.cont) {
      ex.cont = null;
      setRunLabel(ex, false);
    }
    if (from || hasDebugger(src)) {
      pane("bytecode").textContent = disasm(module, src, ex.ts) ?? ex.listing;
      showTab(ex, "snapshot", true);
      if (!from) select(ex, "out");
      await runPausing(ex, l, id, from);
      return;
    }
    showTab(ex, "snapshot", false);
    let lines = [];
    const out = (...a) => lines.push(esc(a.map(show).join(" ")));
    let slices = 0;
    const make = () =>
      new Sandbox({
        module,
        typescript: ex.ts,
        console: { log: out, info: out, warn: out, error: out },
        fuelSlice: FUEL_SLICE,
        maxFuel: MAX_FUEL,
        // Called before every fuel slice: count how often the VM stopped and resumed.
        now: () => (slices++, 0),
      });
    let sb = make();
    const before = walk(sb, l);
    let failure = null;
    const t0 = performance.now();
    try {
      await sb.runAsync(wrap(src));
    } catch (e) {
      failure = describe(e, src, true);
      if (e?.kind === "syntax") {
        // Compiling failed, so nothing ran: compile the code as written, for the
        // error's true position (and in case the wrapper was the problem).
        sb.dispose();
        sb = make();
        lines = [];
        slices = 0;
        failure = null;
        try {
          await sb.runAsync(src);
        } catch (e2) {
          failure = describe(e2, src, false);
        }
      }
    }
    const ms = performance.now() - t0;
    if (id !== ex.runs) {
      // Edited again while this ran: the newer run fills the panes.
      sb.dispose();
      return;
    }
    if (failure) lines.push(`<span class="live-error">${esc(failure.text)}</span>`);
    pane("out").innerHTML = lines.join("\n") || `<span class="muted">(no output)</span>`;
    ex.mark = failure?.line ? markAt(src, failure.line, failure.col) : null;
    repaint(ex);
    const listing = disasm(module, src, ex.ts);
    pane("bytecode").textContent =
      listing ?? (src === ex.original ? ex.listing : `(The original example's bytecode, from the build.)\n\n${ex.listing}`);
    const memory = () => {
      pane("memory").innerHTML = heapHtml(before, walk(keep, l, before.len));
      pane("memory").querySelector(".live-gc").onclick = () => {
        keep.collect();
        memory();
      };
    };
    const keep = sb;
    memory();
    ex.sandbox?.dispose();
    ex.sandbox = sb;
    // The run stopped and resumed between fuel slices: the fact the Suspend tab had.
    const status = el.querySelector(".live-status");
    if (failure) status.textContent = failure.text.startsWith("SyntaxError") ? "did not compile" : "threw";
    else {
      status.innerHTML =
        `ran in ${ms.toFixed(1)} ms, in <a href="suspension.html#fuel" title="The VM returned control to this page between two instructions ` +
        `${Math.max(0, slices - 1)} times and resumed where it stopped; nothing of the run was on the Rust stack in between">${slices} fuel slices</a>`;
    }
  } catch (e) {
    if (id === ex.runs) setStatus(`could not run: ${e.message}`);
  } finally {
    if (id === ex.runs) el.classList.remove("is-running");
  }
}

/** Shows the result tab `name`. */
function select(ex, name) {
  ex.select?.(ex.el.querySelector(`[role="tab"][data-tab="${name}"]`));
}

/** Shows or hides the tab `name` (the Snapshot tab: only for runs that pause). */
function showTab(ex, name, on) {
  const t = ex.el.querySelector(`[role="tab"][data-tab="${name}"]`);
  if (!t) return;
  t.hidden = !on;
  if (!on && t.getAttribute("aria-selected") === "true") select(ex, "out");
}

// ---- Setup ------------------------------------------------------------------------

function repaint(ex) {
  ex.code.innerHTML = paint(ex.ta.value, ex.mark);
}

function setup(el, n) {
  const pre = el.querySelector(".live-editor pre");
  const code = pre.querySelector("code");
  const ex = {
    el,
    code,
    // The Markdown block's final newline would be an empty last line in the editor.
    original: code.textContent.replace(/\n$/, ""),
    listing: el.querySelector('[data-pane="bytecode"]').textContent,
    // A ```ts live block runs as TypeScript.
    ts: code.classList.contains("lang-ts"),
    mark: null,
    runs: 0,
    timer: 0,
    sandbox: null,
    // While a run is paused at `debugger;`: what Continue does.
    cont: null,
  };
  const hint = document.createElement("span");
  hint.className = "live-keys";
  hint.id = `${el.id}-keys`;
  hint.innerHTML = `<kbd>${RUN_KEYS}</kbd> run · <kbd>Tab</kbd> indent · <kbd>Esc</kbd> leave`;
  const note = el.querySelector(".live-note");
  note.textContent = "editable · runs in djs-wasm in your browser";
  note.after(hint);

  const ta = (ex.ta = document.createElement("textarea"));
  ta.className = "live-src";
  ta.value = ex.original;
  ta.spellcheck = false;
  for (const [k, v] of Object.entries({ autocapitalize: "off", autocomplete: "off", autocorrect: "off", wrap: "soft" })) ta.setAttribute(k, v);
  ta.setAttribute("aria-label", `Example ${n}: ${ex.ts ? "TypeScript" : "JavaScript"}, editable`);
  ta.setAttribute("aria-describedby", hint.id);
  pre.setAttribute("aria-hidden", "true");
  pre.after(ta);
  repaint(ex);
  el.classList.add("is-live");

  el.querySelector(".live-info").hidden = false;
  const runBtn = el.querySelector(".live-run");
  runBtn.innerHTML = `Run <kbd>${RUN_KEYS}</kbd>`;
  runBtn.setAttribute("aria-keyshortcuts", MAC ? "Meta+Enter" : "Control+Enter");
  runBtn.hidden = false;
  runBtn.onclick = () => (ex.cont ? ex.cont() : run(ex));
  const reset = el.querySelector(".live-reset");
  reset.onclick = () => {
    // Through the editing commands, so that undo brings the edits back.
    ta.focus();
    ta.select();
    insert(ta, ex.original);
  };

  const restore = el.querySelector(".live-restore");
  if (restore) setupRestore(ex, restore);

  ta.addEventListener("keydown", (ev) => onKey(ev, ex));
  ta.addEventListener("input", () => {
    ex.mark = null;
    repaint(ex);
    reset.hidden = ta.value === ex.original;
    el.querySelector(".live-status").textContent = "edited";
    clearTimeout(ex.timer);
    ex.timer = setTimeout(() => run(ex), AUTORUN_MS);
  });
  // The textarea never scrolls: the pre under it sizes the editor.
  ta.addEventListener("scroll", () => (ta.scrollTop = ta.scrollLeft = 0));
  // Start loading djs-wasm when the reader is about to use it.
  const warm = () => lib().catch(() => {});
  ta.addEventListener("focus", warm, { once: true });
  runBtn.addEventListener("pointerenter", warm, { once: true });

  // Tabs: arrow keys move between them (the ARIA tab pattern).
  const tabs = [...el.querySelectorAll('[role="tab"]')];
  const select = (b) => {
    for (const t of tabs) {
      t.setAttribute("aria-selected", String(t === b));
      t.tabIndex = t === b ? 0 : -1;
    }
    for (const p of el.querySelectorAll(".live-pane")) p.hidden = p.dataset.pane !== b.dataset.tab;
  };
  ex.select = select;
  for (const [i, b] of tabs.entries()) {
    b.hidden = false;
    b.onclick = () => select(b);
    b.onkeydown = (ev) => {
      const j = { ArrowRight: i + 1, ArrowLeft: i - 1, Home: 0, End: tabs.length - 1 }[ev.key];
      if (j === undefined) return;
      ev.preventDefault();
      const t = tabs[(j + tabs.length) % tabs.length];
      select(t);
      t.focus();
    };
  }
  return ex;
}

/**
 * The restore box of a ```js live restore block: base64 pasted (or from the page's
 * `#restore=` fragment) becomes the snapshot's bytes; Load shows what it holds in the
 * Snapshot tab, puts its script into the editor and loads it into a fresh sandbox of the
 * docs' setup, from where Continue goes on. A snapshot this djs refuses (another build,
 * other host values) says so with the engine's reason; its contents still show.
 */
function setupRestore(ex, box) {
  box.hidden = false;
  const input = box.querySelector(".live-restore-in");
  const status = box.querySelector(".live-restore-status");
  const say = (html) => (status.innerHTML = html);
  box.querySelector(".live-load").onclick = async () => {
    let bytes;
    try {
      bytes = fromBase64(input.value);
      if (bytes.length === 0) throw new Error("empty");
    } catch {
      say(`<span class="live-error">That is not base64.</span>`);
      return;
    }
    const l = await lib();
    const id = ++ex.runs;
    let inspection;
    const probe = new l.Sandbox(sandboxOptions(ex, l.module, () => {}));
    try {
      inspection = probe.inspectSnapshot(bytes);
    } catch (e) {
      say(`<span class="live-error">${esc(e.message)}</span>`);
      return;
    } finally {
      probe.dispose();
    }
    const s = { bytes, inspection, gz: await gzipSize(bytes) };
    // A run of this example from before is over: its pause and transcript go.
    ex.cont = null;
    setRunLabel(ex, false);
    ex.el.querySelector('[data-pane="out"]').innerHTML = `<span class="muted">(not run: the snapshot is not loaded)</span>`;
    ex.el.querySelector(".live-status").textContent = "a snapshot, not loaded";
    showTab(ex, "snapshot", true);
    showSnapshot(ex, s, "The snapshot you gave");
    select(ex, "snapshot");
    const source = inspection.scripts[0]?.source;
    if (source === undefined) {
      say(`<span class="live-error">It names its script by a hash only, so this page cannot show or compile it.</span>`);
      return;
    }
    // Its script into the editor: a frame's title marks where it waits there.
    ex.ta.value = source;
    ex.mark = null;
    repaint(ex);
    ex.el.querySelector(".live-reset").hidden = source === ex.original;
    // Loading it for real is the test: this djs and the docs' setup load it, or refuse it
    // with the reason.
    const check = new l.Sandbox(sandboxOptions(ex, l.module, () => {}));
    try {
      check.loadSnapshot(bytes);
    } catch (e) {
      say(
        `<span class="live-error">This djs refuses it: ${esc(e.message)}</span> ` +
          `(<a href="#later-builds">what a later build loads</a>, <a href="#host-values">host values</a>)`,
      );
      return;
    } finally {
      check.dispose();
    }
    say(`Loaded: ${esc(words(bytes.length))}. Continue runs it on from where it waits.`);
    if (id === ex.runs) await run(ex, bytes);
  };
  // A link to this page with a snapshot in its fragment fills the box and loads it.
  const m = /^#restore=([A-Za-z0-9_-]+)$/.exec(location.hash);
  if (m) {
    input.value = m[1];
    box.scrollIntoView?.({ block: "start" });
    box.querySelector(".live-load").click();
  }
}

const examples = [...document.querySelectorAll(".live")].map((el, i) => setup(el, i + 1));

// `#run` in the URL runs every example on load (for screenshots and links).
if (location.hash === "#run") for (const ex of examples) run(ex);
