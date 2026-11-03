# 0165: Tool results stored once: sliced strings over result buffers

status: in-progress
agent: heap
branch: issue/0165-result-buffers
depends-on: 0152, 0158, 0162 item 3 (D2551)
guide: docs/guide/strings.md (or values/heap), snapshots.md
touches: crates/djs-core/src/heap/ (a result-buffer cell kind, sliced string representation), crates/djs-core/src/vm/ (host result path), crates/djs-core/src/builtins/ + json (parse into slices), image/session writers (buffers in snapshots), js/src/ (host side, if buffers are referenced instead of stored), docs/decisions.md; coordinate: speed (0162, JSON stringify output, host results), bloat (0151, heap/ templates and atoms), strings (0163, string hashing)

## Goal
docs/lessons-from-pico.md lesson 3: pico stores a tool result once, as a buffer tagged with
the call id and a hash; strings derived from it are slices over that buffer, and snapshots
list the buffer instead of storing it (a 5 MB log costs 0.44 MB in an image instead of
5.44 MB). djs's peak memory on pico's log and parallel-call cases is 2.2–2.5x pico's.

Design and build the djs version:
- a result-buffer cell (host text kept once, immutable), slices (substring/split/JSON
  string values) referencing it without copying, with the GC keeping a buffer alive while
  any slice is (and a policy to copy out small slices of huge dead buffers);
- JSON.parse of a tool result producing slices for string values where profitable;
- snapshots: buffers written once; optionally referenced by (call id, hash) so a host that
  journals results doesn't store them twice (a container option; D024: no compat code);
- determinism and canonical_hash unaffected by the representation;
- measure peak memory (mem-perturb) on pico's log / parallel-call cases, the workloads and
  the pi bench, image sizes, and instructions (no workload slower than +1% without a trade).
Start with the design note in the issue log (sent to the coordinator) before building.

## Log
### Design note (sent to the coordinator before building)

**Where djs stands now.** pico's 2.2–2.5x (log-5mb 41.4 vs 18.7 MB, pmap-200 26.8 vs
15.5) predates D2551 (large ASCII results straight into a heap string) and 0158. pico's
scripts are not in the checkout (work/scratch/0152/w is gone), so I rebuilt the cases from
their descriptions (work/scratch/0165/cases.ts: Sandbox, async host functions, current
main):

| case | what | peak | arena |
| --- | --- | ---: | ---: |
| log-1mb | read 1 MB log, `split("\n")`, filter `" ERROR "` | 5.1 MiB | 2.6 |
| log-5mb | the same, 5 MB, 57k lines | 20.1 MiB (pico 18.7) | 12.0 |
| pmap-200 | 200 parallel fetches of `{url, status, headers, body: ~47 KB}` | 11.0 MiB (pico 15.5) | 9.2 |
| records | 20k records, filter + map | 10.8 MiB (pico 10.8) | 6.4 |

A snapshot taken while log-5mb waits on a second call (holding `log`, `lines`, `errs`):
11.7 MiB raw, 1.75 MiB zstd. So in memory, the gap is gone except where a script cuts a
big string into pieces: log-5mb holds the 5 MB text once and then **every line again as
its own copy** (57k × ~108 B = 6.1 MB). Text results and wire results are already stored
once (a text result as one reserved `Str8`; a wire result transcoded once, staging
reused). What remains is (1) copies made by substring/split/JSON.parse, and (2)
snapshots storing every tool result in full.

**1. Sliced strings (the memory part).** A new cell kind `Slice` (24 B: `LEN` u32 at 8 and
`HASH` u32 at 12 like every string kind, `PARENT: Ref` at 16, `OFF: u32` at 20): the units
`[OFF, OFF+LEN)` of a flat `Str8` parent. Not a `Rope` flag: the layout table traces
`RIGHT` as a reference, so the offset needs its own kind.
- **Only one-byte parents**, for now: the width invariant (a `Str16` always holds a unit
  above 0xFF, so equal strings have equal width and compare by `memcmp`) would otherwise
  need a scan per slice. Logs, files and JSON text are one-byte. Append buffers are fine
  as parents (bytes below their `LEN` never change).
- **Producers**, through the choke points that exist: `builtins::string::substring`
  (slice, substring, substr, split's pieces, trim, regexp match and capture strings via
  `substring_value`), JSON.parse string values without escapes (`json/parse.rs`), and in
  step 2 the wire decoder. Rule: result ≥ 32 units and parent ≥ 256 units, else a copy as
  today. A slice of a slice points at the root parent.
- **Readers** need almost nothing: string kinds are encapsulated in heap/string.rs
  (24 sites) and walk.rs (12). `str_view`/`str_view_flat` return a view into the parent
  in place (as they do for append ropes), `str_chunks`/`char_code_at` handle it,
  `flatten` copies once and turns the slice into a flattened rope (same 24 B, in place),
  `str_eq`/hash/atoms work on views. The interpreter's flat fast path stays a two-kind
  check; slices take the existing slow path.
- **Retention** (V8's sliced-string problem: a 30-char slice pins a 5 MB log; for pi this
  also means snapshots carrying the whole text). In a **full** collection, the marker
  treats `PARENT` as weak: it records slices and, after the transitive mark, sums the
  live slice bytes of each parent nothing else reached. A parent kept less than 1/4 by its
  slices (or below 4 KiB) is not kept: its slices are flagged; right after the collection
  (the arena consistent) each flagged slice gets a flat copy and becomes a flattened rope
  over it, in place; the parent dies at the next collection. Otherwise the parent is
  marked as usual. Minor collections keep parents (strong). Deterministic: a function of
  the heap at deterministic collection points.
- **canonical_hash**: the canonical copier writes a slice as a flat string, so the hash
  does not depend on whether, or since which collection, a string is a slice
  (`canonical_hash_ignores_collection_points` must keep passing). Base images have no
  slices. Session images store slices as they are.
- **verify/walk** (images): `PARENT` must be a `Str8` (or an append buffer),
  `OFF + LEN <= len(PARENT)`.

Expected: log-5mb arena 12.0 → ~7.2 MiB (57k slices = 1.4 MB instead of 6.1), peak
20.1 → ~13 MiB; the waiting snapshot 11.7 → ~7 MiB raw. JSON.parse of a text result
keeps its strings in the text. Cost: ~2–3 KB gzip (the size ceiling has 3.8 KB left);
string-heavy workloads must stay within +1% (slices only exist above the thresholds).

**2. Result buffers.** A result buffer is a `Str8` with a `STR_RESULT` flag and the call
id in 8 bytes of spare capacity after the data (readers ignore the capacity), immutable.
- Text results: D2551's reserved string becomes the result buffer (the call id is
  recorded when the result is resolved): no new copy.
- Wire results (objects): new export `vm_result_reserve(vm, len)` (like
  `vm_string_reserve`) for the encoded value; the host writes the wire bytes there and
  `vm_decode_wire_buffer(vm, h)` decodes from it, with ASCII strings of ≥ 32 bytes as
  slices of it. Memory is about the same as now (one copy, plus the framing and the small
  strings, which stay in the buffer while any slice lives; retention as in 1); it is the
  unit that step 3 leaves out of snapshots. Non-ASCII strings are copied as now.
  Overlaps with speed's D2551/0162 item 3 (strings inside objects): this would replace
  that extension, so the two need sequencing.

**3. Snapshots.** With 1 and 2, a session image already stores each tool result once
(buffer + 24 B slices) instead of text + pieces. Optional external mode (a container
option, no compat code): the session image writes a live result buffer's header but not
its payload, plus a table `(call id, cell offset, len, FNV-1a 64)`; restoring asks the
host for those payloads (`Sandbox.fromBytes(bytes, { results: (callId) => bytes })`)
and refuses a mismatching hash. The Sandbox would keep the bytes it wrote per call id (or
its journal's) to supply them. log-wait: ~7 MiB raw / ~1.4 zstd → ~2 MiB raw / ~0.3 zstd.
pi journals results as host values (D1730), so pi either keeps the encoded bytes or
re-encodes deterministically; that is pi-side work.

**Order and checkpoints.** (1) slices + producers + retention + canonical/verify: measure
log cases, all workloads (mem-perturb), instructions, size, then merge; (2) result buffers
and wire slices, after speed's 0162 lands its host-result change; (3) the external
snapshot mode. Each step is a merge of its own. Touches heap/cells.rs, string.rs, walk.rs,
compactor.rs, gc.rs (canonical copy), builtins/string.rs, json/parse.rs, later wire/,
vm/session.rs, djs-wasm, js/src/{sandbox,snapshot}.ts.

**Questions.** Are the thresholds (32 units, 256-unit parents, the 1/4 retention rule) the
right defaults to start with? Is one-byte parents only acceptable? Should step 2 wait for
speed? Is the external mode (3) wanted now or later, given pi would need to keep result
bytes?

### Step 1: slices (D2542), merged on its own
- Coordinator: go with the defaults as named constants, one-byte parents only; follow
  0163's hashing rules (rebased onto it: `set_hash_key` clears slices too, hashes go over
  views); step 2 after step 1 (speed2 no longer extends D2551 to strings in objects); step
  3 later, as a follow-up.
- Built: the `Slice` kind (cells.rs 22), readers in heap/string.rs, `contiguous` where a
  view is enough, `flatten` turning a slice into a flattened rope, producers (substring,
  fast split, JSON.parse), the collector's weak parent rule with copy-out and a second
  pass, the canonical copy writing slices flat, verify, the JS reader, layout regenerated.
- Measured: work/scratch/0165/cases.ts (log-1mb/5mb, log-wait, snippets, parse, pmap,
  records) against main and seven constant variants; mem-perturb n=8 on all workloads
  (unchanged); wasm instructions n=5 (noise ±1.5% between runs on the same builds),
  native n=5 within ±1%; pi bench (tool-heavy warm 24 → 20 pages, times unchanged).
  Numbers in D2542.
- Size: +1,956 B gzip, slim +2,233: recorded as a trade in D2542, ceilings raised
  (tests/scores/size.tsv). A sort of the slices by parent cost +4.4 KB and was replaced.
- Shared files touched: heap/cells.rs, string.rs, walk.rs, compactor.rs, gc.rs, mod.rs
  (test module), builtins/string.rs, builtins/fast.rs, builtins/number.rs,
  interp/conv.rs, interp/run/{intrinsics,iter,json_regexp}.rs, json/parse.rs,
  json/stringify.rs (two `flatten` → `contiguous`), js/src/reader.ts.
- Guide: heap.md (Strings: the Slice kind), gc.md (slices and the strings they keep;
  canonical copy). Tests: heap/slice_tests.rs, a sandbox test (slices to the host and
  through snapshots).

## Follow-ups
- Step 3, external results in snapshots (later; needs the owner's call on pi keeping or
  re-encoding result bytes). Estimate: session image format (a flag on result buffers and
  an external table of call id, offset, length, FNV-1a 64), restore asking the host for
  payloads and checking hashes, `Sandbox.bytes({ externalResults })` /
  `fromBytes(..., { results })`, the Sandbox keeping result bytes per call id: about a
  week, +~1 KB gzip. The log-wait snapshot would go from 6.9 MiB raw / 0.98 zstd to about
  2 MiB / 0.3.
- The snapshot of "snippets" keeps a parent its slices cover 40% of (5.0 MiB against
  main's 1.9): if pi sessions show such cases, lower `SLICE_KEEP_SHARE` to 2 and look at
  the copy-out's peak (21.3 MiB on that case).

## Result
