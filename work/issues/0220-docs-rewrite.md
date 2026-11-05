# 0220: Rewrite the guide: behavior and architecture, no history, a JS interface page, live dumps

status: in-progress
agent: guide
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

### Plan (approved by the coordinator)
TOC: Part "Using djs": introduction, goals, language (new), typescript (new), js-api (new),
dumps (new), limits (new), determinism, cli (new), pi (new, the only page naming pi). Part
"How it works": arena, values, heap, compiler, interpreter, suspension, gc, snapshots (images
and the base), continuations (new, how dumps are written), builtins, host ("the wasm
boundary"), testing. Reference: opcodes, cells, intrinsics, status. Batches: 1 tooling +
`debugger`; 2 Using djs; 3 How it works.

### Batch 1 (tooling, `debugger`)
- `debugger;` as a pause (D3050): `CompileOptions::debugger_awaits` (dump builds only,
  cfg-gated field), bit 6 of `bits()`, `vm_compile`'s `| 16`; the parser emits
  `await debugger()` with the syntax of that statement, so the exporter needs no case of its
  own (dump_cont/dump_plain untouched; one wrapper in dump_tests.rs for the tests). Tests:
  djs-core `vm::dump_debugger_tests` (a pause in a loop dumped and resumed at each of 3
  pauses, every load re-dumped; a nested async function; no-ops), js/test/debugger.test.ts
  (task pauses, suspend/resume chain with re-dumps of equal size, misuse without dumper).
- Size: shipped 1,033,436 B before and after, slim 906,125 → 906,126 B; the only differing
  bytes are djs-core's source-hash constant (an `i32.const`, one LEB byte longer in slim),
  which any djs-core edit changes. No code difference.
- Sandbox option `debugger` (identity "debugger", global `debugger`, needs `vm_dump`).
- djs-docs: decision log off the site (page, nav, search, `a.decision`), `gen:decisions`
  gone, decision IDs/decision-log links are errors, generated op/intrinsic docs drop their
  citations; pi rule (whole word, prose + inline code, not on pi.md) and history phrases
  (D3051), with a temporary `NOT_YET_REWRITTEN` list exempting the pages not rewritten
  yet; `gen:status` names pi only on README.md/pi.md. `gen:dump [N]` (native, matching the
  browser's dump but for the seed). Live examples: a Dump tab; code with `debugger;` runs as
  a task with the `debugger` option, frozen builtins, dumps at the pause, Resume loads the
  dump in a fresh sandbox (checked in headless Chrome: 3 pauses and the end, a pause in a
  `Promise.all` of async callbacks). Docs module = `--features docs,dump`.
- Pages: decision IDs and `gen:decisions` stripped mechanically from every page (some
  lines are ragged until their rewrite); suspension.md has the first live `debugger`
  example with a `gen:dump`. README.md (writing rules) and AGENTS.md's guide section
  rewritten.

### Batch 2 (Using djs)
- SUMMARY.md: parts "Using djs" (introduction, goals, language, typescript, js-api, dumps,
  limits, determinism, cli, pi), "How it works", "Reference"; the chapters of batch 3 keep
  their titles until rewritten.
- New pages: language.md, typescript.md, js-api.md, dumps.md, limits.md, cli.md (with
  dbg's `debugger;` support: `--debugger OUT`, `.c`/`.continue`, `.abort`, pauses in
  `--frozen`), pi.md (the adapter, prelude images, 10,000 fuel per poll, the next mode,
  pi's tests via `gen:status conformance`/`speed`, wiring another agent host).
  Rewritten: introduction.md, goals.md (owner: every goal the guide relies on, each with
  what it means, why, the cost; five ranked + isolation, determinism, wasm32 first,
  QuickJS's behavior; D3052), determinism.md (pi removed). host.md lost "The pi adapter"
  (now pi.md); suspension.md's demo text follows the new live UI. These three, and the
  other "How it works" pages, are batch 3.
- Tooling: `ts` blocks (static and live) run as TypeScript; `check` fails on links to a
  section a page lacks (any page; D3052); status.rs "an agent-like script"; live examples
  pause at `debugger;` in their normal Run (owner's design, D3053): Continue resumes from
  the dump in a fresh sandbox, one transcript with pause lines, the dump in a `<details>`
  under the output, refusals shown and continued in place, an info tip linking dumps.md.
  Checked in headless Chrome (work/scratch/0220/steps4.mjs, steps5.mjs, shot.mjs):
  3 pauses + end on suspension.html and dumps.html, a WeakMap refusal, the tip.
- NOT_YET_REWRITTEN now lists only the batch-3 pages.

### Exact pause positions (D3054)
- `Vm::call_site(call_id)` (djs-core vm/call_site.rs, `dump` only): file/line/col of the
  innermost activation waiting on an open call (breadth first through reactions and
  combinators, like the CLI's `waiting_on`). wasm `vm_call_site` (dump builds),
  js `Sandbox.callSite(id)`. Test: `dump_debugger_tests::call_sites_of_pauses` (loop +
  `Promise.all` pauses, through re-dumps), js/test/debugger.test.ts (two tests).
- Decided: positions are where the code runs: example.js for the first part, `dump.js`
  (the dump that part continued from) after a resume; mapping back to the original would
  need the dumper's normalization. live.js: pause lines "paused at line 7:3 of dump 1"
  with links that show any of the run's dumps (marked at the statement); box summary
  "Dump N of this run". Checked in headless Chrome (steps7.mjs) on suspension.html,
  dumps.html. Size: shipped 1,033,538 B and slim 906,238 B before and after (5 bytes:
  source hash). Guide: dumps.md, js-api.md, README.md.

## Follow-ups

## Result
