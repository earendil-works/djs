# 0220: Rewrite the guide: behavior and architecture, no history, a JS interface page, live dumps

status: open
agent: -
branch: issue/0220-docs-rewrite
depends-on: -
guide: all of docs/guide
touches: docs/guide/, crates/djs-docs/ (check rules, live examples, build), docs/guide/README.md, AGENTS.md's guide section; the `debugger` hook (engine + js/src, design approved by the coordinator first)

## Goal (owner)
Largely redo the guide (docs/guide, published as the docs site):

- **Focus on behavior and architecture:** what a user of djs sees and can do, and the
  architecture that was chosen and how it works.
- **No history.** No "used to", "was replaced", format versions, earlier designs, migration
  stories, issue numbers. Describe djs as it is.
- **No decision log.** Don't mention the decision log, don't cite decision IDs (D1234), don't
  cross-reference decisions. Remove the decision-log page and search entries from the site, and
  the `gen:decisions` region from use in guide pages. The repository keeps docs/decisions.md for
  developers; the guide doesn't point at it.
- **Do explain why** where it helps: why this design works better than the obvious alternative
  (e.g. one arena vs a native heap; continuations as plain JavaScript vs heap images; no AST;
  fuel instead of preemption), in the page's own words.
- **The interactive interpreter shows continuation dumps.** The site's live examples (the
  in-browser editor running djs-wasm) can suspend a running example and show the dump it
  produces (and resume it). The CLI REPL already has `.dump`; document it on the right page.
- **`debugger` as a forced suspend for the demo.** In the demo, a `debugger;` statement
  suspends the run at that point so the reader can see the dump. Design how (it must be an
  opt-in host behavior, invisible to normal runs and cheap or absent in the shipped module, whose
  size ceiling has almost no room: ~200 B). Send the design to the coordinator before building.
- **Find missing pages and add them.** At least: a page on the JavaScript interface (the host
  package: Sandbox, run/tasks, host functions and bindings, suspend/resume and dumps, errors,
  limits; with examples), a page on continuation dumps for users (what they contain, how to
  resume, what refuses), the CLI and REPL, the TypeScript support, the supported language subset
  and builtins, limits and resource control. Propose the list.
- **Pi only on one page:** a single page explains how pi's codemode uses djs (or how another agent
  could). Everywhere else, describe hosts generically.

Keep the guide self-contained (no links to ARCH.md, docs/*.md, work/). Update
docs/guide/README.md's writing rules and the `djs-docs check` rules to match (forbid decision IDs
and decision-log links in guide pages; keep the source-link rules). Update AGENTS.md's "Keeping
the guide current" section accordingly (no "the ID only as a citation").

## Constraints
- Generated regions and examples: keep using them where they stay true (opcodes, cells, example
  outputs), so pages track the code. Dump examples should be generated from the current exporter,
  not pasted: the format is still changing (pjs, 0207: seed only when used, no "use strict",
  32-bit source hashes).
- Other agents: pjs (dump exporter, hosts' seed/source handling), pjr (writer performance), pjz
  (fuzzer). Touch engine or js/src code only for the `debugger` hook and the live dump demo, after
  the coordinator approves the design.
- Batch merges (at most one every 1-2 hours; e.g. by chapter group), running `djs-docs check`
  and the docs build locally first.

## Log

## Follow-ups

## Result
