# 0150: Deduplicate base strings; analyzer shows the session's own part; atom entries diff

status: done
agent: strings
branch: issue/0150-base-strings
depends-on: 0133, 0147, 0148
guide: docs/guide/snapshots.md ("Inspecting a heap"), arena.md if base content numbers change
touches: crates/djs-core/src/vm/ (prepare_base_image), crates/djs-core/src/heap/compact.rs (order_for_image) or a new pass next to it, crates/djs-cli/src/heap/ (analyzer), docs/cli.md; slim (0149) is research-only in scratch

## Owner
Looking at an almost empty REPL session with `djs heap /tmp/what.snap` (1.3 KB container,
324.8 KB expanded): "quite a few strings in there… why do we have that many duplicated
strings in the base? That seems silly."

## Tasks
1. **Deduplicate strings in the base image** at image build: 97 contents stored more than
   once, 2.9 KB in extra copies (e.g. "get [Symbol.species]" x7, "TypeError" x3). Find the
   sources (function `name` strings per function, atom-table strings vs string values,
   constant pools per template), then merge equal flat strings into one cell (strings are
   immutable primitives; sharing is unobservable), preferring the atom table's copy. Keep
   the kind-grouped order, the growable-cells-last order (0136) and the immutable prefix
   (0147); canonical_hash/determinism checks stay green. Measure base raw size, cells,
   module gzip (ratchet), vm_new.
2. **Analyzer: show the session's own part by default.** In `djs heap`, the strings,
   largest, shapes and functions sections currently describe the whole expanded image
   (base + session), which looks like the session holds 900+ strings. Default to the
   session region (new + changed vs the reference), with the base totals as one summary
   line, and `--all` for today's whole-image view. Keep JSON in step; update docs/cli.md
   and the guide's "Inspecting a heap".
3. **Atom entries diff**: in that snapshot `<atoms><entries>` (19.5 KB, in the writable
   tail) shows 19,536 of 19,536 bytes differing for a session that added a handful of
   atoms. Find out why (whole array rewritten/reallocated? entries shifted? analyzer
   pairing it with the wrong reference cell?) and fix whichever it is, so an almost empty
   session changes only what it added. Report the raw session-image size before/after.

## Log
- 2026-11-09 (strings): baseline on main: base 325,008 B / 5,852 cells, module 458,952 B
  gzip, vm_new 3.81 µs, fresh REPL raw session image 34,568 B (/tmp/what.snap: 34,896 B).
- Duplicate sources (`djs heap` with paths per copy): builtins' `name` strings per function
  (`get [Symbol.species]` x7, `get byteLength` x3, `map` next to the atom), constant-pool
  strings (`enumerable`, `lastIndex`, `true`/`false`), Error ctor + prototype `name`, all
  next to the atom table's copy. Fixed in `order_for_image` (D2430): equal strings sort
  together, atom copy first, others dropped and repointed. Base 322,168 B / 5,750 cells,
  0 duplicates; module 458,640 B gzip.
- `<atoms><entries>` cause: the base shrinks the entries to their length, 1,460 of 1,952
  slots are holes from D2253; the array is in the tail, so the first new atom copies it to
  a 25% bigger cell (19.5 KB) and the analyzer counted any resized cell as differing
  everywhere. Fix (coordinator approved going outside touches into heap/atom.rs, cells.rs,
  walk.rs): two-level atom table (D2431), base entries+index in the prefix, VM's own in a
  small tail list (room 32) + index; analyzer counts only differing words of resized cells.
  Hole compaction declined (atom ids untyped in too many places; see D2431, follow-up).
- Perf: first version (base index first, dyn eq) cost props +2.2% native: interning a
  session atom missed through the base index re-hashing prefix strings. Now VM's index
  first, BASE_COUNT field, generic probe: native props +0.6%, push-pop +0.8%, json +0.2%,
  json-date −4.6%; wasm within noise (22 workloads, wmeasure n=3/5).
- Analyzer (D2432): session view by default for largest/strings/shapes/functions with a
  reference-totals line, `--all`; summary shows the session image size; JSON `scope`,
  `reference_part`, `session_image_bytes`, `base_end`, `extra_copies`.
- Checks: djs-core lib (incl. D2067 order independence, armed barrier, seal), djs-prelude
  (prefix references only itself, registry current: no registry update needed), djs-cli,
  e2e plain, difftest whole corpus plain + codemode/codemode-harvest with stress and
  determinism (barrier armed), test262 Symbol/defineProperty/identifiers.
- Guide: snapshots.md (tail, diet list, atom table paragraph, "Inspecting a heap"),
  arena.md, gc.md, heap.md; docs/cli.md.

## Follow-ups
- Compact the base's 1,460 atom holes by renumbering atom ids at base build (11.7 KB raw
  per VM in the prefix): needs a complete inventory of where atom ids are stored
  (property tables, shape descs, template names/debug names, bytecode immediates,
  global_decls in constant pools, ICs) or a typed marker for the constant-pool ones.
- `Vm::new_host_function` allocates its `name` string instead of using the atom's string
  (a sandbox's console adds 5 duplicate strings per session).
- The global object's property index (2 KB) differs in 600 bytes after a sandbox defines
  `console`: rehash positions; a session that adds globals rewrites much of it.

## Result
- **Base strings (D2430):** equal flat strings are one cell in the base (atom copy kept).
- **Two-level atom table (D2431):** the base's atoms and index live in the builtin prefix;
  a VM's own atoms go to a small list (room 32) and index in the tail. Layout: AtomTable
  `BASE_ENTRIES`, `BASE_INDEX`, `BASE_COUNT`.
- **`djs heap` (D2432):** largest/strings/shapes/functions show the session's own part
  (new + changed vs the reference) with one reference-totals line; `--all` for the whole
  image; resized cells count only their differing words; summary shows the session image
  size.
- **Numbers (main → branch):** base 325,008 → 322,720 B, 5,852 → 5,752 cells (tail
  27,160 → 8,016 B); module gzip 458,952 → 459,013 B (dedup alone −312, the atom split
  +373; ceiling unchanged); vm_new 3.81 → 3.76 µs; fresh REPL raw session image 34,568 →
  11,472 B (what.snap was 34,896 B), container 1,177 → 1,096 B; `<atoms><entries>` of an
  almost empty session differs in 48 of 272 bytes (was 19,536 of 19,536).
- Registry: unchanged (`registry -- check` passes).
