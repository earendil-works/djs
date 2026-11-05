# Writing the guide

The guide explains djs to a human reader: what a user of djs sees and can do, and the
architecture that was chosen, how it works and, where it helps, why it beats the obvious
alternative. It is not a second ARCH.md: ARCH.md, docs/*.md and the decision log stay the
detailed reference for people changing the code, and the guide says what it needs in its
own words.

## The rules

`djs-docs check` enforces these on the pages in SUMMARY.md (code blocks are exempt; inline
code is held to the word rules):

- **Self-contained.** A reader of the site has the guide and nothing else. Never mention or
  link ARCH.md, TESTING.md, docs/*.md (the decision log included), builtins/README.md or
  work/, or issue numbers (`0088`): write what the page needs, or what was found.
- **No decisions.** No decision IDs (`D1050`) and no links to the decision log. Explain the
  reason in the page's own words.
- **No history.** Describe djs as it is: no "used to", "was replaced", format versions,
  earlier designs or migration stories. `check` catches the obvious phrases ("no longer",
  "used to be", "originally", "was replaced", "format 2", ...); the rest is on you.
- **pi on one page.** pi, the agent djs was built for, is named only on `pi.md`: how pi's
  codemode uses djs and how another agent host could. Everywhere else, describe hosts in
  general ("a host", "an agent's codemode"). The check matches the whole word `pi` (and
  paths such as `js/pi/`), not `π` or `pipeline`.
- **Goals by name, and only goals.md's.** Every goal a page refers to or relies on is a
  section of goals.md, which says what it means for a user, why djs chose it, and what it
  costs or rules out. Refer to one by its name and link its section
  (`[persistence](goals.md#persistence)`), never by number ("G4"). An aim that is not on
  goals.md is not called a goal elsewhere: say it in plain words. A new goal goes onto
  goals.md first. `check` fails on a link to a section that does not exist, on any page.
- **Source as source.** Pages may point at code, as links to repository files
  (`[`crates/djs-core/src/value.rs`](../../crates/djs-core/src/value.rs)`). The site turns
  them into links to the published source at the commit it was built from, marked as
  source links. Every mention of a repository file or directory is such a link: inline
  code that names a tracked path (`crates/djs-core/src/heap/gc.rs`, `js/src/`,
  `scripts/gate.sh:40` with its line) or ends one uniquely (`heap/cells.rs`) becomes a
  link by itself, as do full paths in plain text and paths inside commands or globs
  (`node scripts/gate.py`, `builtins/*.ts` links the directory); directories go to
  GitHub's tree view, files to the blob view. A mention of a path that no longer exists,
  a partial path that fits several files (`lib.rs`: write the path), and a link whose
  path text names another file than it links are errors. A `gen:excerpt` region starts
  with a link to the quoted lines.

(While the rewrite of the guide is under way, the pages it has not reached yet are listed in
`NOT_YET_REWRITTEN` in `crates/djs-docs/src/render.rs`: they are exempt from the pi and
history rules until it does.)

`SUMMARY.md` lists the pages in reading order (mdBook's format). A `- [Title]()` entry
is a chapter not written yet.

## Tooling

```sh
cargo run -q -p djs-docs -- check             # what the gate runs
cargo run -q -p djs-docs -- regen             # rewrite generated regions and gen/*.svg
cargo run -q -p djs-docs -- regen --examples  # also accept changed example output
cargo run -q -p djs-docs -- build             # the site: target/docs-site/
```

`build` renders every page, copies djs-wasm (the docs build from
`scripts/build-wasm.sh --docs`, with the dumper and the `djs_disasm` export that gives
edited examples their own bytecode listing; else the shipping build from
`scripts/build-wasm.sh`, whose examples can't pause at `debugger;`; or `--wasm FILE`) and the host package with its types stripped by node, for the live
examples. Open it over HTTP (`python3 -m http.server -d target/docs-site`); from
`file://` the pages work but the examples show only their build-time output.

The site has a search box (`/` focuses it): `build` writes `search.json`, one entry per
section of every page, which the browser loads on first use.

The site's look is Elanor, Earendil's design system: its tokens, fonts, icons and
logo are vendored in `crates/djs-docs/assets/elanor/` by `scripts/elanor-update.sh`
(which needs an Earendil npm login; nothing else does), and `assets/style.css` uses only
its tokens.

`.github/workflows/docs.yml` publishes the site to GitHub Pages on every push to main:
it builds djs-wasm, runs `build --out _site` with node 24, and deploys that directory.

## Generated regions

Content between `<!-- gen:NAME ARGS -->` and `<!-- /gen -->` is generated and checked
in, so the Markdown reads the same on GitHub. Never edit it by hand.

| region | content |
| --- | --- |
| `value-tags` | the value tag table (`EXPORTED_TAGS`) |
| `value-bits` | `gen/value-bits.svg`: the NaN-boxing diagram |
| `cell Str8 Object ...` | `gen/cell-KIND.svg`: cell layout diagrams from `LAYOUTS` |
| `cell-all` | every cell kind, with headings |
| `ops push_i8 add ...` | an opcode table from `OPS`, descriptions from the `///` in ops.rs |
| `ops-all` | every opcode |
| `intrinsics-all` | every intrinsic, descriptions from builtins/intrinsics.d.ts |
| `excerpt PATH "ANCHOR" "START" N` | N lines of a repository file: from the first line containing START after the line containing ANCHOR (quoted code that cannot drift) |
| `status conformance\|speed\|size\|inventory` | status numbers: counted from the repository, or read from `tests/scores/status.tsv` (`scripts/status.py`); also used in README.md |
| `disasm` | the bytecode of the closest `js` or `ts` block above |
| `run` | the output of the closest `js` or `ts` block above, as this build prints it |
| `dump [N]` | the continuation dump the closest `js` or `ts` block above writes at its Nth `debugger;` pause (default 1), as the live examples' Dump tab writes it |

Examples (`js` blocks, and `ts` blocks, which compile as TypeScript) are codemode async
function bodies: `await` and `return` work at the top level and `console.log` prints. `run` executes them in a VM from the base
snapshot, suspended and restored from its heap image every 50 fuel units. `run` and
`dump` regions are only rewritten by `regen --examples`: a changed output fails the gate,
because it is a behavior change that someone should look at. Since the dump format still
changes, prose around a `dump` region describes what the dump does, not the exact text.

A block marked ```` ```js live ```` (or ```` ```ts live ````) becomes a live example on the site: the code is
an editor, highlighted as it is typed in the same colors as static code, and runs in
djs-wasm in the browser, with its bytecode, a walk over the VM's heap, and its fuel slices
and snapshot. Put its `gen:run` region right after it; the site shows that output inside
the widget until the reader runs the code, and without JavaScript.

How the editor behaves: Run or Ctrl/Cmd-Enter runs the code, and it also runs by itself
600 ms after the last edit. Tab and Shift-Tab indent and dedent (two spaces, also a
selection of lines), Enter keeps the line's indentation, Escape leaves the editor so that
Tab moves on through the page, and Reset (shown once the code differs) brings back the
original code; undo works throughout. A SyntaxError shows its line and column, an
uncaught error its stack, both with the source line and a caret, and the place is
underlined in the code. Runs stop after 20 million units of fuel. The highlighter in the
browser, `crates/djs-docs/assets/highlight.js`, is a port of
[highlight.rs](../../crates/djs-docs/src/highlight.rs); a test keeps the two in step, so
a change to one needs the same change in the other.

## Checks

`check` also renders every page, so it catches: a link to a page or repository file that
does not exist (relative links are resolved from `docs/guide/`; links to
`../decisions.md` go to the site's copy of the log), a decision ID in prose that
is not in the log, an unknown opcode, cell kind or generator, a missing diagram, a
diagram in `gen/` that nothing generates.

## How a page is written

- Start with what the reader wants to know, then an `> **In short.** ...` paragraph: the
  whole page in three sentences.
- Pages about using djs (the language, the JavaScript interface, dumps, limits, the CLI)
  say what a user sees and can do, with examples. Pages about how it works go from the
  problem to the design. Show real code, real bytecode, real dumps and real layouts
  (generated), not paraphrases.
- Give a design choice its cost where it helps: `> **Trade-off.** X over Y. Why ... The
  cost: ...`.
- End with "Where to look" (source links).
- Plain, direct sentences, in the present tense.
- Diagrams: generated from data where the data exists; otherwise hand-drawn SVG in
  `img/`, drawn with the classes of the generated ones (`text`, `.small`, `.mono`,
  `.box`, `.host`, `.engine`, `.arena`, `.cell`, `.arrow`, ...: `RULES` in
  crates/djs-docs/src/svg.rs). Give a new file any `<style></style><defs></defs>` block:
  `regen` replaces it with the shared stylesheet in Elanor's colors, light and dark.
