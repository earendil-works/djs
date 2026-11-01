# 0133: A heap dump analyzer

status: done
agent: snapshot
branch: issue/0133-heap-dump-analyzer
depends-on: 0117
guide: docs/guide/snapshots.md or gc.md (a short "inspecting a heap" section, D2102 rules)
touches: crates/djs-cli/ (a subcommand) or a new small tool crate, js/ (only a helper to unwrap snapshot containers if done in node), scripts/ (wrapper), TESTING.md / docs/host.md (usage)

## Goal
Owner request: a small tool to see what is in a heap dump, i.e. a raw heap image, a
legacy DJSS/DJPI snapshot, or a 0117 snapshot container (zstd against a reference image).
Today only the REPL's `.heap` prints cells per kind and objects per class for the live VM.

## Acceptance criteria
- [x] Input: raw image, legacy snapshot formats, and the 0117 container (decompress with
      the reference image: the base image from the module, or pi's prelude image, which
      the tool can rebuild or be given as a file). A tool crate may use the zstd crate.
- [x] Output (text by default, `--json` for scripts):
  - summary: total size, live vs garbage (after a collection), counts and bytes per cell
    kind and per object class, old region (OLD_END) vs young;
  - **diff against the reference image**: which cells are new or changed relative to the
    base/prelude image, grouped by kind/class with bytes. This is what explains a
    snapshot's size;
  - top-N largest cells, and largest retained sizes (dominator tree or a simple
    approximation) with a path from the roots (e.g. `globalThis.cache[3].body`);
  - strings: count, bytes, top duplicates, the largest strings (truncated);
  - shapes: count, dictionary vs shared, the most common property-name lists;
  - functions/bytecode: templates and bytecode bytes, closures;
  - pending jobs, promises, suspended frames (what a waiting session holds).
- [x] Reuses djs-core's own cell layout and walker (heap/walk.rs etc.), not a second
      parser, so it can't drift; a test runs it over a few known images (empty Sandbox,
      a pi waiting session from scripts/snapshot-research cases) and checks key numbers.
- [x] Usage documented; one example output in the issue's Result.

## Log

- Early merge (at the coordinator's request, for bloat's 0135): `djs heap` in
  crates/djs-cli/src/heap/ and the Rust snapshot container reader/writer in
  crates/djs-cli/src/container.rs (zstd crate, D2264), `heap::analyze_image` for the
  REPL, scripts/heap/reference.ts (a module's base, pi's prelude image for start data;
  `preludeImage` exported from js/pi/vm.ts), wasm.ts writes containers and start.json,
  docs/cli.md. bloat owns the REPL (`.snapshot`/`.restore`/`.heap`); I don't touch repl/.
  Still to do: the pi waiting-session test, the guide section, the re-measurement.
- Diff accuracy: positional pairing mismatched shapes (an inserted transition shifts a
  sorted transition list), giving 124 "new" + 116 "gone" shapes for an unchanged tree;
  pairing a transition list's children by the key they add fixed it (ask: 78 new cells,
  21 changed, 1 gone). Immutable cells pair only with equal content (strings ignoring
  the lazily computed hash). Added the first cell out of line and the bytes moved cells
  rewrite (references to moved cells), which is what a moved cell costs zstd.
- Test of a pi waiting session (heap/tests.rs): scripts/snapshot-research's Rust copy of
  the adapter (`#[path]` to its pi.rs), pi's prelude evaluated from
  tests/pi/prelude-source.ts by djs itself (TypeScript), the prelude sealed as the
  reference, ask.js until it waits; checks the activation (codemode.js:3, its locals),
  the diff (>90% in place, 1–16 KB new, the largest new subtree under `~pending`), and
  the same through a container against the prelude image (refused without the reference).
- **The snapshot-research harness had a corpus bug** the analyzer found: its search tool
  stepped through djs-core's source files with stride 7; while djs-core had 147 files
  (a multiple of 7), tool-heavy's 60 hits were 20 files three times, and `djs heap`
  showed "321 contents stored more than once, 226.6 KB in the extra copies". That is why
  tool-heavy looked like 45 KB (max) on main this morning against 0117's 121 KB. Fixed
  (stride coprime with the file count) in wasm.ts and snaplab.
- **Re-measured** (scripts/snapshot-research/wasm.ts, wasm profile, node 25, load ~30;
  work/scratch/0133/sr-final/wasm-report.md), against 0117's numbers (after mark-compact):

  | case | 0117 image / container | now image / container | why (from `djs heap`) |
  | --- | ---: | ---: | --- |
  | ask | 546 / 2.0 KB | 489 / 2.1 KB | image: the base lost 62 KB (internal names, D2253; kind order, D2251); the session's own part unchanged: 78 new cells, 4.7 KB |
  | agent-loop | 558 / 6.9 | 501 / 7.0 | as ask |
  | sequential | 560 / 7.3 | 503 / 7.4 | as ask; 21 KB new (summaries kept) |
  | tool-heavy median / max | 748 / 62.7, 944 / 120.7 | 671 / 57.9, 854 / 110.9 | 369 KB new, nearly all 60 file texts (Str8 307 KB, Str16 36 KB); the corpus (this repo's sources) changed |
  | records | 1791 / 324.5 | 1740 / 334.8 | 1245 KB new: the 1500 records, prose from docs that grew |
  | big-log | 2225 / 403.1 | 2168 / 403.9 | 1684 KB new: the log |
  | Sandbox, new | 395 / 33.0 | 333 / 15.7 | the first cell out of line moved from 3 KB (atom entries, BFS base) to 68 KB (the global object's PropTable, kind-ordered base); 80% of the image still moves, costing 43 KB of rewritten references |
  | Sandbox, 1000 records | 560 / 48.0 | 498 / 31.2 | as new, plus the records |

  Every pi session keeps 99% of the prelude image in place except the last ~9 KB: pi's
  `pending` Map entries array (96 bytes) grows with the first calls, and what follows it
  slides (223 cells, 1.8 KB of rewritten references).
- Guide: docs/guide/snapshots.md got "Inspecting a heap" (what the tool reads, how the
  diff matches, an example, a trade-off) and current sizes in "Compact snapshots"; `djs-docs
  check` passes. docs/cli.md documents `djs heap`; docs/host.md has the re-measured table
  and the explanation. TESTING.md: nothing changed.
- Outside `touches:`: js/pi/vm.ts (one export, `preludeImage`), docs/cli.md (the CLI's
  own doc), scripts/snapshot-research (the stride; containers and start.json in wasm.ts).

## Follow-ups

- **Sandbox snapshots at 1–2 KB:** a VM's first definitions grow cells that the base image
  holds exactly full: the global object's property table (68 KB in), the atom table's
  entries (30 KB) and the root shape's transitions. The grown copies go to the young end
  and the collection slides everything after the old places, so a new Sandbox's image
  lines up with the base for only 20%. Placing those growable singletons last in the
  base image (heap/compact.rs `order_for_image`), or leaving them slack, should give
  Sandbox and REPL snapshots the 99% in place that pi sessions have against their sealed
  prelude image. Passed to bloat for 0135.
- **pi sessions:** the `pending` Map's entries array grows with the first tool calls and
  shifts the last ~9 KB of the prelude image (223 cells); a few slots of slack in it (pi's prelude,
  or the adapter's seal) would remove the last moved cells.
- An automated cross test of crates/djs-cli/src/container.rs against js/src/snapshot.ts
  in the gate (checked by hand both ways today).
- The engine registry (0117) is still not built; `djs heap` reads another build's images
  only when the layout matches (it warns that builtin names may be off).

## Result

`djs heap [--json] [--top N] [--only SECTIONS] [--diff FILE|base|none] [--reference FILE]
<dump>` (crates/djs-cli/src/heap/, D2264) analyzes raw images, DJSS/DJPI snapshots and
DJSN containers; crates/djs-cli/src/container.rs reads and writes containers in Rust
(zstd crate); `heap::analyze_image` serves the REPL (bloat, 0135); scripts/heap/reference.ts
writes a module's base or pi's prelude image for start data. Sections: summary (live,
garbage, old/young), kinds and classes, the diff against the reference (in place / moved /
changed / new / gone, by kind, the largest new subtrees with paths, the first cell out of
line, the references moved cells rewrite), the largest cells and retained sizes (dominator
tree), strings, shapes, functions and bytecode, and the session (host calls, promises,
suspended activations with their locals). Everything reads through djs-core's verified
`Heap`, walker and layout table. Tests: 4 container tests, 5 analyzer tests (a sandbox,
data with paths/strings/shapes, a VM waiting on an async host call, a container, a pi
session waiting on a tool against its prelude image). Usage in docs/cli.md and the guide.

Example: the ask case waiting on its second tool call (a 2.1 KB container):

```text
$ djs heap --reference prelude.img --only diff,session --top 5 ask-1.djsn
== diff against pi's prelude image
  against pi's prelude image (484.3 KB): 98.9% of the image's 8-byte words are the reference's at the same offset; cells line up for the first 484.2 KB (99.0%)
  the first cell out of line is the reference's Object/Promise of 0.1 KB at <interp><result>: what follows sits elsewhere
  cells                             count     KB  differing KB
  unchanged, in place                8415  443.9
  unchanged, moved                      0    0.0           0.0
  changed                              21   40.3           0.5
  new                                  78    4.7           4.7
  gone (the reference's, not here)      1    0.1
  (the differing KB of moved cells are references to cells that moved)

  largest new subtrees (new cells and the new cells only they keep alive):
  KB   cells             kind  path
  3.2     47  Object/Ordinary  handle[1]~pending<entries>[5]
  0.3      1         U32Array  handle[4]<code><ic>
  0.3      1         U32Array  globalThis.tools<handler>.get<code><ic>
  0.1      4            Shape  <root_shape><transitions>{question}
  0.1      1       ValueArray  <jobs>

== session: what the VM holds while it waits
  run state Done; 0 value stack slots in use; 5 handles
  0 queued jobs, 0 host results waiting in the inbox, 0 pending host calls, 0 unhandled rejections
  promises: 3 pending (2 reactions waiting), 0 fulfilled, 0 rejected; 0 weak entries

  suspended activations:
  kind                  state     function                at  slots  path
  async function  at an await  <anonymous>  codemode.js:5:13     14  handle[1]~pending<entries>[5].resolve<data><reactions>[1]
  <anonymous> at codemode.js:5:13 holds: plan = Ordinary, answer = Ordinary, out = undefined
```

Current snapshot sizes are in the Log's table (and docs/host.md): waiting pi sessions
2.1–7.4 KB, sessions holding tool data 58–404 KB (their data), a new Sandbox 15.7 KB.

Known gaps: the diff's structural matching can pair the wrong sibling where two are
equally plausible (shown as changed instead of new); retained sizes treat ephemeron values
as owned by their table; paths for prelude-internal cells use field names and
`%intrinsicN` (the realm's slots have no names in the image).
