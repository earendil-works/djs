# 0150: Deduplicate base strings; analyzer shows the session's own part; atom entries diff

status: open
agent: -
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

## Follow-ups

## Result
