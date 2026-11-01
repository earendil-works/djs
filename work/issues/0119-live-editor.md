# 0119: Live examples as a real live editor with syntax highlighting

status: in-progress
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
- [ ] An editor component with live highlighting that matches the site's Elanor code colors
      (the same roles/tokens as the static highlighter, D2200): choose between a small
      vendored editor (for example CodeMirror 6 or similar, vendored like the Elanor
      assets: no CDN at runtime, pinned, license noted) and a lightweight textarea-overlay
      highlighter (for example our Rust highlighter compiled into the docs-only wasm, or a
      small JS tokenizer with the same roles). Decide by quality and weight; record it.
- [ ] Good editing basics: tab/indent handling, keyboard shortcut to run
      (Ctrl/Cmd-Enter), reset to the original code, line wrapping or scroll consistent with
      the site's no-overflow rule (scripts/docs-overflow.sh stays clean), works in light and
      dark, accessible (focus, labels), works without JavaScript by showing the static
      highlighted code and its build-time output as today.
- [ ] Error output readable (SyntaxErrors with position, uncaught errors with stack).
- [ ] Checked in headless Chrome (screenshots before/after in work/scratch/0119), and
      `djs-docs check`/build green; page weight noted (what the editor adds per page and
      once per site).

## Log

## Follow-ups

## Result
