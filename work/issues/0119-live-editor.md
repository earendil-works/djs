# 0119: Live examples as a real live editor with syntax highlighting

status: done
agent: docs
branch: issue/0119-live-editor
depends-on: 0114, 0115
guide: all pages with live examples (the widget only; no prose changes needed)
touches: crates/djs-docs/ (the live-example widget: assets/live.js, its markup in render.rs/site.rs, its CSS within the Elanor style), docs/guide/README.md (how live examples behave), js/src/sandbox.ts (only if the widget needs a small public API)

## Goal
Owner's feedback: the "runs in djs-wasm in your browser" examples are a view + Edit toggle
today, and "the current experience is shit". Make every live example a **live editor**:
the code is directly editable in place (no separate view/edit mode), with **syntax
highlighting while typing**, and running it is immediate and obvious (Run button plus a
keyboard shortcut, optionally re-run on edit with a debounce), with the output, bytecode,
heap and suspend tabs updating for the edited code (0113's live disassembly).

## Acceptance criteria
- [x] An editor component with live highlighting that matches the site's Elanor code colors
      (the same roles/tokens as the static highlighter, D2200): choose between a small
      vendored editor (for example CodeMirror 6 or similar, vendored like the Elanor
      assets: no CDN at runtime, pinned, license noted) and a lightweight textarea-overlay
      highlighter (for example our Rust highlighter compiled into the docs-only wasm, or a
      small JS tokenizer with the same roles). Decide by quality and weight; record it.
- [x] Good editing basics: tab/indent handling, keyboard shortcut to run
      (Ctrl/Cmd-Enter), reset to the original code, line wrapping or scroll consistent with
      the site's no-overflow rule (scripts/docs-overflow.sh stays clean), works in light and
      dark, accessible (focus, labels), works without JavaScript by showing the static
      highlighted code and its build-time output as today.
- [x] Error output readable (SyntaxErrors with position, uncaught errors with stack).
- [x] Checked in headless Chrome (screenshots before/after in work/scratch/0119), and
      `djs-docs check`/build green; page weight noted (what the editor adds per page and
      once per site).

## Log
- Decided (D2103): textarea with transparent text over the highlighted `pre`, highlighting
  by assets/highlight.js, a port of highlight.rs's JS/TS rules. CodeMirror 6 rejected for
  weight (~150-200 KB min) and a second color mapping; Rust highlighter in the docs wasm
  rejected (highlight.rs is in the std tool crate; the wasm loads only on first use).
- highlight.rs test `js_port_agrees` runs highlight.js in node over all 24 JS/TS blocks of
  the guide plus corner cases and compares the HTML byte for byte (checked that it fails
  when a keyword is dropped from the JS list).
- render.rs: new markup (editor container, ARIA tabs/tabpanels with ids, role=status,
  Run/Reset hidden until JS; tabs other than Output hidden without JS, since they cannot
  switch). site.rs writes highlight.js and loads live.js only on pages with live examples;
  live.js imports the sandbox package and compiles djs-wasm on first use (focus, hover on
  Run, run) instead of at page load on every page.
- live.js: editor (Tab/Shift-Tab block indent, Enter auto-indent and bracket split,
  Backspace in indentation, Ctrl/Cmd-Enter run, Esc to the result tabs, Reset via
  insertText so undo works), auto-run 600 ms after the last edit, superseded runs are
  dropped, maxFuel 20M (an endless loop stops in ~150 ms). Errors: SyntaxError with line,
  column, source line and caret; uncaught Error with its stack (the code runs as
  `try {<code>\n} catch ...` that passes the stack out; line-1 columns corrected; on a
  SyntaxError the unwrapped code is compiled for the true position), underlined in the
  editor. The Heap tab's "Collect now" used a disposed sandbox before; it keeps the last
  run's sandbox now.
- Checked in headless Chrome via a small CDP driver (work/scratch/0119/cdp.mjs, t2.mjs:
  20 keyboard/run/error/overflow checks, all ok; t3.mjs: every live example on all 10
  pages gives its build-time output when run, except builtins' printed stack naming
  `sandbox.js` instead of `example.js`, as before). scripts/docs-overflow.sh: no overflow
  at 1440/1000/390. Screenshots: work/scratch/0119/before-*.png and after-*.png (light,
  dark, edited, threw, syntax error, heap tab, 390 px, no JS).
- Page weight (raw / gzip -9): editor once per site: live.js 19.3 KB / 7.3 KB (was
  7.8 / 3.1), highlight.js 3.8 / 1.5 KB, style.css +3.1 / +0.7 KB: about +18 KB / +6 KB.
  Per page: +0.2-0.9 KB HTML for ids and ARIA (the hidden textarea copy of the code is
  gone). Saved: pages without live examples no longer load live.js and the sandbox package
  (~95 KB / ~26 KB on every page before), and pages with examples load the package and
  the 1.5 MB (545 KB gzip) wasm only on first use.
- Guide pages: docs/guide/README.md describes the editor; no chapter prose changed.

## Follow-ups
- djs-wasm names the sandbox's source `sandbox.js` (crates/djs-wasm/src/lib.rs) while the
  build-time run uses `example.js`, so a printed stack differs after running (builtins
  page). Cosmetic; could become an option of the compile call.

## Result
Every live example is an editor: highlighted while typing in the Elanor code colors
(assets/highlight.js, kept equal to highlight.rs by a test), Run button and Ctrl/Cmd-Enter
plus auto-run after a 600 ms pause, Tab/Shift-Tab/Enter/Backspace indentation handling,
Esc to leave, Reset (undoable), errors with position, stack, source line and caret and an
underline in the code, all four tabs updating for the edited code. No-JS pages show the
highlighted code and build-time output. Long lines wrap, so no horizontal overflow.
Decision D2103; behavior documented in docs/guide/README.md. Known gaps: no line numbers
(wrapped lines would need per-line layout); stacks name the file `sandbox.js`.
