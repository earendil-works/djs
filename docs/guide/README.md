# Writing the guide

The guide explains djs to a human reader: how it works and why it is built that way. It
is not a second ARCH.md. ARCH.md, docs/*.md and the decision log stay the detailed
reference for people changing the code; the guide tells the story and links to them.

`SUMMARY.md` lists the pages in reading order (mdBook's format). A `- [Title]()` entry
is a chapter not written yet.

## Tooling (D2100)

```sh
cargo run -q -p djs-docs -- check             # what the gate runs
cargo run -q -p djs-docs -- regen             # rewrite generated regions and gen/*.svg
cargo run -q -p djs-docs -- regen --examples  # also accept changed example output
cargo run -q -p djs-docs -- build             # the site: target/docs-site/
```

`build` renders every page plus the decision log, copies djs-wasm (the `wasm` profile
build from `scripts/build-wasm.sh`, or `--wasm FILE`) and the host package with its
types stripped by node, for the live examples. Open it over HTTP
(`python3 -m http.server -d target/docs-site`); from `file://` the pages work but the
examples show only their build-time output.

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
| `decisions D1050 D1051 ...` | a list of decisions with their titles |
| `excerpt PATH "ANCHOR" "START" N` | N lines of a repository file: from the first line containing START after the line containing ANCHOR (quoted code that cannot drift) |
| `status conformance\|speed\|size\|inventory` | status numbers (D2101): counted from the repository, or read from `tests/scores/status.tsv` (`scripts/status.py`); also used in README.md |
| `disasm` | the bytecode of the closest `js` block above |
| `run` | the output of the closest `js` block above, as this build prints it |

Examples (`js` blocks) are codemode async function bodies: `await` and `return` work at
the top level and `console.log` prints. `run` executes them in a VM from the base
snapshot, suspended and restored from its heap image every 50 fuel units. A `run` region
is only rewritten by `regen --examples`: a changed output fails the gate, because it is a
behavior change that someone should look at.

A block marked ```` ```js live ```` becomes a live example on the site: edit and run it in
djs-wasm in the browser, with its bytecode, a walk over the VM's heap, and its fuel slices
and snapshot. Put its `gen:run` region right after it; the site shows that output inside
the widget.

## Checks

`check` also renders every page, so it catches: a link to a page or repository file
that does not exist (relative links are resolved from `docs/guide/`; links to
`../decisions.md` go to the site's copy of the log), a decision ID in prose (D1050) that
is not in the log, an unknown opcode, cell kind or generator, a missing diagram, a
diagram in `gen/` that nothing generates.

## How a page is written

- Start with what the reader wants to know, then an `> **In short.** ...` paragraph: the
  whole page in three sentences.
- Explain from the problem to the design. Show real code, real bytecode and real layouts
  (generated), not paraphrases.
- Give every design choice its cost: `> **Trade-off.** X over Y. Why ... The cost: ...`.
  These are what the guide is for.
- End with "Where to look" (files) and the page's decisions (`gen:decisions`).
- Plain, direct sentences. Cite decisions by ID instead of retelling their history.
- Diagrams: generated from data where the data exists; otherwise hand-drawn SVG in
  `img/` with the same embedded stylesheet as the generated ones (light and dark).
