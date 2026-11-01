# 0103: Human-facing documentation: format proposal first, then the docs

status: in-progress (phase 2: tooling and first chapters landing; chapters in parts)
agent: docs
branch: issue/0103-docs
depends-on: -
guide: all of docs/guide
touches: phase 1: work/issues/0103-human-documentation.md (the proposal), work/scratch/0103/ (prototypes). Phase 2 (after the owner's decision): a new docs site source directory, its build tooling, generated reference content.

## Goal
Documentation that helps a **human** understand how djs works: the interpreter design,
the heap and value model, the compiler, the builtin language and native builtins,
suspension and snapshots, determinism, the host interface, and **the trade-offs** behind
each (why a position-independent arena, why a copying GC, why no AST, why the leaf rule,
why fuel, why NaN-boxing …). Today's docs (ARCH.md, docs/*.md, the ~270-entry decision
log) are written for agents working on the code; they are accurate (0095) but not a
guided explanation.

**Phase 1 (this issue's first deliverable): decide the format.** It must stay
maintainable (lives in the repo, plain-text sources, easy to keep in sync with the code,
buildable in the gate or at least checkable) and be nice to look at (good typography,
navigation, diagrams, code samples). Building our own tool is allowed if it pays off.

Questions for the proposal:
- Source format: Markdown (CommonMark + extensions), MyST, AsciiDoc, djot, or our own.
- Generator: an existing one (mdBook, Zola, a Rust-native option; check what's installed;
  don't install anything without asking), or a small custom Rust tool in the workspace.
- Diagrams: memory layouts, value encoding, frame layout, the GC, the merge of the
  pipeline. Options: SVG generated from data (the layout tables already exist: LAYOUTS,
  the ops! table), mermaid/d2/graphviz, or hand-written SVG.
- **Generated reference sections** that can't go stale: the opcode table from ops!, the
  cell layouts from LAYOUTS / layout_json, the intrinsic list, the decision index, test262
  and perf numbers from the score files.
- Live examples: djs runs in wasm, so pages could embed a djs-wasm sandbox (run snippets,
  show bytecode via the disassembler, show heap stats, suspend/snapshot/resume in the
  browser). Worth it, and at what cost?
- Structure: a guided tour (concepts → architecture → subsystems → trade-offs → host
  interface → contributing) plus reference sections; how it relates to ARCH.md and the
  decision log (link to, don't duplicate).
- Checking: links, code samples that are executed in the gate (doc-tests), and generated
  sections verified fresh.
- Publishing: static output (GitHub Pages from the repo's `origin`), offline-readable.

## Deliverables
- [x] Phase 1: 2–4 concrete format/tooling options with a small prototype page each
      (the same content, e.g. "Values and NaN-boxing" with a generated layout diagram),
      the trade-offs, and a recommendation. Send it to the coordinator; **don't build the
      docs before the owner has chosen**.
- [x] Phase 2 part 1: the tooling (djs-docs, gate, regen, AGENTS.md), introduction,
      values, reference pages.
- [ ] Phase 2, further parts: the remaining tour chapters (SUMMARY.md drafts).

## Log

- Phase 1. Read README, ARCH, the decision index, docs/*.md. Nothing is installed
  (mdbook, zola, hugo, pandoc, dot, d2, mermaid, typst, asciidoctor all absent); node 25,
  python3 and headless Chrome are. pulldown-cmark 0.13 and serde_json are in the cargo
  cache; pulldown-cmark-escape was fetched for the prototype (a crate download, nothing
  installed).
- Built one shared source (`content/`) and a prototype generator over it, the sample
  page "Values and NaN-boxing" plus two fully generated reference pages (opcodes, cell
  layouts), and live examples running the real `@djs/host` Sandbox on djs-wasm in the
  browser. Checked in headless Chrome (light and dark, served and standalone).
- The doc-test check earned its keep at once: the example output I wrote by hand was
  wrong (`-0` prints as `-0`, a BigInt as `100…0n`) and `check` failed until regen.
- Prototypes: `work/scratch/0103/` (gitignored; README.md there lists everything).
- Owner chose option A and asked for agents to keep the docs current via the
  coordinator. Phase 2 part 1: `crates/djs-docs` (check/regen/build), `docs/guide/` with
  SUMMARY.md, introduction.md and values.md, and generated reference pages for
  opcodes (with the `///` docs read from ops.rs: no wasm cost), cell layouts and
  intrinsics (docs from intrinsics.d.ts). `check` in gate.sh, `regen` in regen.sh,
  `.gitattributes` treats gen/ and ref-*.md as generated. AGENTS.md "Keeping the guide
  current" plus the coordinator's `guide:` field. D2030.
- Examples are codemode async bodies now (as in the live widget and in pi), run with a
  50-fuel suspend/restore cycle; `gen:run` output only changes with `--examples`, so a
  behavior change fails the gate instead of silently updating the page.
- `Sandbox.heapView()` and `Sandbox.collect()` (js/src/sandbox.ts, tested) replace the
  prototype's reach into private fields.
- D1980 (optional trailing fields) showed up after the rebase: the cell diagrams draw
  fields past `min_size` dashed, with an "optional" legend.

## Proposal

### What every option shares

- **Source format: CommonMark + GFM tables**, mdBook's `SUMMARY.md` for the chapter
  order. Every existing doc is Markdown, it renders on GitHub and in editors, and the
  one parser we need (pulldown-cmark) is small. Rejected: MyST, AsciiDoc, djot (no
  tooling installed, weaker GitHub and editor support, no gain for prose plus code),
  typst (PDF-first, its HTML output is experimental).
- **Generated regions, checked in**: `<!-- gen:NAME args -->` ... `<!-- /gen -->`. The
  tool rewrites the content (`regen`) and the gate fails when it is stale (`check`),
  like `layout.generated.ts` and `natives.rs` today (D1791: regenerate after a rebase).
  Because the result is checked in, the Markdown reads correctly on GitHub too.
  Generators in the prototype: `value-tags` (from `layout_json`), `value-bits` and
  `cell KIND` / `cell-all` (SVG from `LAYOUTS`), `ops LIST` / `ops-all` (from `OPS`),
  `decisions IDS` (titles from decisions.md), `disasm` (the preceding `js` block
  compiled), `run` (the preceding `js` block executed in a VM from the base snapshot,
  suspended and restored from its heap image every 50 fuel units: a doc-test that also
  exercises suspension). Planned: intrinsics, builtin list, test262/perf scores from
  `tests/scores`.
- **References are checked**: `D1050` in prose is linked to the decision and an unknown
  ID is an error; unknown opcodes and cell kinds are errors.
- **Diagrams**: SVG drawn in Rust from the engine's data, with an embedded light/dark
  stylesheet so the same file works inline on the site and as an `<img>` on GitHub.
  Conceptual pictures (pipeline, GC, suspension) as hand-written SVG. No mermaid, d2 or
  graphviz: none is installed, mermaid needs JS at view time, and none can draw a
  byte-exact cell layout.
- **Conventions for the human angle**: an "In short" box at the top of a page,
  `> **Trade-off.** ...` asides for every design choice (a styled aside on the site, a
  plain quote on GitHub), "Where to look" (files), and a generated list of the page's
  decisions. ARCH.md, docs/*.md and the decision log stay the agents' reference; the
  guide links to them and does not duplicate them.

### Option A: our own generator, `crates/djs-docs` (prototyped, built)

A std tool crate depending on djs-core, djs-prelude and pulldown-cmark. `regen` /
`check` / `build`; the HTML template, CSS and live-example JS are ours.

- Maintainability: the prototype is ~1,000 lines of Rust plus 150 lines of JS and 140 of
  CSS. It reads the layout and opcode tables directly (no JSON round trip once
  `EXPORTED_TAGS` and similar are public), runs examples in-process, and `check` takes
  0.01 s. One new dependency (pulldown-cmark, plus its escape crate) in a tool crate:
  needs a decision entry, no install anywhere.
- Looks: full control. Prototype: system fonts only (serif body, sans headings,
  monospace code; offline, no CDN), three-column layout (chapters, page, "On this
  page"), light and dark, styled asides, framed diagrams, prev/next. Screenshots in
  `work/scratch/0103/shots/`.
- Missing compared with mdBook: search (a JSON index plus ~100 lines of JS if wanted)
  and a print view.
- Publishing: `djs-docs build` writes a static directory (values.html 33 KB, wasm
  537 KB gzipped, loaded on the first Run). GitHub Pages via an Actions workflow the
  owner sets up (agents do not push). Works offline from disk except live examples,
  which then show their build-time output; a single-file bundle also works offline
  (prototype: 830 KiB).

### Option B: mdBook plus a djs preprocessor (sources prototyped, not built)

The same `content/` (already mdBook-compatible), `book.toml` in
`work/scratch/0103/B-mdbook/`. The preprocessor is option A's regen/run/svg code behind
mdBook's JSON protocol; theme overrides for the asides, diagrams and live widget.

- Gains: mdBook's theme, sidebar, search, print view, keyboard navigation; familiar to
  Rust readers.
- Costs: `cargo install mdbook` on every machine that builds the site and in the gate
  (owner's OK; a sizable dependency tree, tokio and an HTTP server for `serve` by
  default); a preprocessor tied to mdBook's protocol and version; looks like every other
  mdBook, and deeper changes mean overriding its handlebars templates. We still write
  most of option A's code: only the ~250-line HTML template and nav are saved.

### Option C: Markdown on GitHub only (prototyped: `work/scratch/0103/content/`)

The same sources and `djs-docs regen/check`, no site.

- Gains: no publishing at all, the cheapest to run.
- Costs: GitHub's typography; navigation only through links; no live examples, no
  styled asides, no inline SVG theming beyond `prefers-color-scheme`; D-numbers would
  have to be generated Markdown links. Does not meet "nice to look at" or the live
  examples, but it is what A and B degrade to, for free.

### Live examples (a layer for A or B; prototyped and working)

A `js live` block becomes a widget: Edit/Run in djs-wasm through the real `@djs/host`
Sandbox (types stripped for the browser), with tabs **Output**, **Bytecode**, **Heap**
(the arena walked cell by cell with the exported layout: counts and bytes per cell kind,
the delta against the base snapshot, and a "Collect now" button that shows the copying
GC compacting), and **Suspend & snapshot** (how many fuel slices the run took, i.e. how
often it stopped between instructions and resumed; `sb.bytes()` size; restore into a
fresh wasm instance and run there). Without JS the widget shows the build-time output
and bytecode.

- Cost: live.js ~150 lines. It reads the Sandbox's private fields for the arena walk;
  phase 2 should add a small public API (`arena()` / heap stats). Live bytecode for
  edited code needs a `djs_disasm` export: a `docs` cargo feature of djs-wasm, so the
  shipping module does not grow. The browser downloads the 537 KB (gzip) module on
  first Run.
- Worth it: yes. Suspension, snapshots and the heap model are the parts that are hardest
  to explain in prose and the easiest to show; the cost is small and isolated.

### Recommendation

**Option A with live examples**, keeping the sources GitHub-readable (C for free) and
in mdBook's SUMMARY format (B stays an exit if we ever want its search or themes). What
makes these docs worth having (generated regions, doc-tests in the VM, diagrams from the
layout data, checked decision links, the live widget) is our code in every option; the
generic part mdBook would add is a few hundred lines here, and A needs no tool installed
in the gate.

Phase 2, in parts: (1) `crates/djs-docs` with `check` in the gate and `regen` in
scripts/regen.sh, sources in `docs/guide/`, a decision entry; (2) the `docs` feature of
djs-wasm (`djs_disasm`) and a public heap-stats API in the host package; (3) the tour
chapters (the 12 in the prototype's SUMMARY.md), each with its trade-offs; (4) reference
pages (opcodes, cells, intrinsics, builtins, decision log by topic, scores); (5) search
and the Pages workflow if wanted.

Open questions for the owner:
1. A, B or C (and live examples or not)?
2. Is pulldown-cmark acceptable as a dependency of a tool crate (A and B)?
3. Publishing: GitHub Pages from `origin` through an Actions workflow (owner sets it
   up), or just the built directory?
4. The opcode reference wants the `///` docs of `ops!`. They are not in `OPS` (adding
   them to the runtime table would cost wasm size); the tool can read them from
   ops.rs or the macro can emit a separate, docs-only table. Which one?

## Result

## Follow-ups

- Live bytecode for edited examples: a `djs_disasm` export behind a docs-only feature
  of djs-wasm (the Bytecode tab now shows the build-time listing of the original).
- Search on the site (a JSON index and a little JS), if wanted.
- Publishing: a GitHub Pages workflow running `djs-docs build` (the owner sets it up;
  agents do not push).
- Generated status numbers (test262 and corpus counts, wasm size, bench scores) for an
  introduction or status page.
