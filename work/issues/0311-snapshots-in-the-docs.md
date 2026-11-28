# 0311: Snapshots in the docs: see what suspends, copy it, restore it

status: in-progress
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
- [ ] The tabs, panel, copy/download/link, restore box, page rewrite as above; djs-docs check
      clean; the CLI round trip verified with a snapshot the docs module wrote.

## Log

## Follow-ups

## Result
