# 0311: Snapshots in the docs: see what suspends, copy it, restore it

status: done
agent: pview2
branch: issue/0311-snapshots-in-the-docs
depends-on: 0310
touches: crates/djs-docs (assets/live.js, CSS, the snapshots page's restore box), docs/guide/snapshots.md, docs/guide/suspension.md (links), docs/guide/cli.md (the round trip), docs/decisions.md
guide: docs/guide/snapshots.md, docs/guide/cli.md, docs/guide/suspension.md

## Goal
Make the docs teach how snapshots work and how to use them, by doing:
- **Live example tabs** become Output | Bytecode | Memory | Snapshot. Memory is today's Heap
  (the live arena; at a pause, walk the sandbox before it is disposed, with one line that
  contrasts the arena with what the snapshot keeps). Snapshot (only for examples with
  `debugger;`) shows the 0310 tree for the current pause, interactive: clicking a frame marks
  where it waits in the editor, clicking a reference opens the record (its named fields, size,
  held by), section sizes as bars. The Output transcript's "snapshot N" lines open it. The
  Suspend tab goes; its fuel-slice fact moves to the status line, linking suspension.md.
- **Copy / download / link** at each pause: the snapshot as plain base64 (Copy), a
  `.snapshot` file (Download), and a link to the snapshots page with the snapshot in the URL
  fragment (never sent to a server).
- **Restore** on the snapshots guide page: a box to paste base64 (and the link target):
  decode, `loadSnapshot` in the docs' fixed setup (console, debugger), show the tree, put the
  snapshot's script into an editor, Continue. A snapshot from another build loads or is
  refused with the real reason, shown as such (the "Later builds" table, live).
- **The snapshots page** leads with one example and the same steps as host code next to the
  buttons: `run.snapshot()` -> bytes -> base64 -> stored -> `sandbox.loadSnapshot(bytes)` ->
  settle the open call -> `advance()`; and the CLI round trip (`base64 -d > pause.snapshot`,
  `djs describe`, `djs resume --debugger`), checked to work with the docs' setup.
- A browser-level test of the new live.js parts (a stub-DOM node harness like 0306's at least).

**Owner, on what the view shows:** much more human friendly, so a reader can interactively
understand what is in the snapshot. Lead in JavaScript terms (functions stopped where, with
their variables as JS values; what each waits for; the result still pending), explain each
part in one plain sentence (what it is and why it is in the snapshot), show identity by
highlighting the same object reached twice, and keep engine detail in layers one can open:
the JS view first, then how djs stores it (records, named fields, sizes), then bytes. Sizes
in plain words. The JSON carries per node a human label, a one-line explanation and the JS
rendering, then the record and byte details.

## Acceptance criteria
- [x] The tabs, panel, copy/download/link, restore box, page rewrite as above; djs-docs check
      clean; the CLI round trip verified with a snapshot the docs module wrote.

## Log
- Built on 0310's JSON (`view`, records' `view`/`known`/`note`/`js`, `sizes`, `notes`).
- crates/djs-docs/assets/snapview.js (new): the panel as HTML from the inspection, and
  its events (notes, identity on hover, record cards, hex with the record marked,
  Copy/Download/Link). live.js: tabs Output/Bytecode/Memory/Snapshot, the Suspend tab's
  fact in the status line (fuel slices, linking suspension.md#fuel), Memory walked at each
  pause before the sandbox goes plus a line on what the snapshot keeps, the transcript's
  "snapshot N" links open the Snapshot tab, the restore box (`js live restore`, pasted
  base64 or `#restore=` in the link) loading in the docs' setup or showing the refusal.
  render.rs: the tabs, the Snapshot pane (the text description without JavaScript), the
  restore markup; site.rs writes snapview.js.
- snapshots.md leads with the example and its Snapshot tab, then "The same steps as host
  code" (begin, snapshot, base64, dispose, loadSnapshot, settle, advance) and the CLI
  round trip, then the description, and a "Restoring a snapshot here" section with a
  `Promise.all` example (two pauses at once). "Later builds" points at the restore box.
  suspension.md links to the Snapshot tab; cli.md says how site snapshots go to the CLI
  and back.
- The CLI round trip, checked: a snapshot the docs module wrote (node, Sandbox with
  console and debugger), as base64, `base64 -d > pause.snapshot`, `djs describe`, `djs
  resume --debugger next.snapshot pause.snapshot` (prints "got page 1 of 3", pauses at
  3:3), `djs resume next.snapshot` to the end. A `djs run` snapshot in the restore box is
  refused: "this sandbox has no host value or function later, print, tools".
- Checked in headless Chrome (a small CDP driver in work/scratch/0311): pause, tab,
  notes, hover identity (the Map reached from three frames lights up together), record
  card, bytes, Continue x3, restore by link, garbage, refusal.
- Test: js/test/docs-live.test.ts on js/test/stub-dom.ts (live.js and snapview.js with
  the docs module; skipped on the shipped module). render.rs test for the tabs and the
  restore box.
- Outside `touches:`: js/test (the new test and the stub DOM), and one hook in live.js
  (`globalThis.djsLiveImports`) for it.

## Follow-ups
- The panel's layout on a phone width is unchecked (scripts/docs-overflow.sh).
- Copy and Link use `navigator.clipboard`, which needs a secure context (https or
  localhost): on a plain-http mirror they fail with the browser's reason in the status.

## Result
Live examples with `debugger;` have a Snapshot tab: the run in JavaScript terms with notes,
identity on hover, record cards, the storage layer and the bytes, plus Copy (base64),
Download (.snapshot) and Link (`snapshots.html#restore=...`). Memory shows the arena at
each pause against what the snapshot keeps; the Suspend tab's fact is in the status line.
The snapshots page leads with the example, the host-code steps and the CLI round trip, and
has a restore box that loads a pasted or linked snapshot, or shows why this djs refuses
it. D3761.
