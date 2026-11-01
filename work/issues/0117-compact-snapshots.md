# 0117: Compact snapshot images (snapshot direction A, D019)

status: done
agent: snapshot
branch: issue/0117-compact-snapshots
depends-on: 0088, 0089
guide: docs/guide/snapshots.md, docs/guide/host.md
touches: crates/djs-wasm/ (an export for the reference descriptor, if needed), js/src/sandbox.ts, js/pi/ (snapshot()/resumeVm), crates/djs-core/src/vm/ (image header: reference descriptor, additive), docs/host.md, scripts/snapshot-research/ (re-measure)

## Goal
Direction A from 0088 (work/issues/0088-snapshot-design-research.md), approved by the owner
(D019): snapshots are collected images compressed with zstd against the recomputable
reference image (the base image, or the post-prelude image for pi) used as a dictionary,
with a header that names the reference (engine build hash + which reference image +
its hash) and restore pinned to the engine build that wrote it. 0088 measured a waiting
session at 943 KB raw → 22–27 KB this way, with ~0.6 ms extra on restore.

## Acceptance criteria
- [x] A snapshot container format (versioned header: magic, format version, engine build
      hash, reference kind + reference hash, compression, payload) documented in
      docs/host.md; the Sandbox's `bytes()/fromBytes()` and the pi adapter's
      `snapshot()/resumeVm()` produce and consume it; old raw images still load.
- [x] Compression in the host (node's built-in zstd with a dictionary; check what the
      browser path can use, and fall back to uncompressed where zstd isn't available) or in
      wasm if that turns out better; measured both ways.
- [x] Refusing to restore an image from a different engine build or reference, with a clear
      error (the host keeps old engine builds if it needs old snapshots: document the
      engine-registry idea from 0088 as a follow-up).
- [x] Sizes and restore times re-measured with scripts/snapshot-research (wasm, not native
      dev builds), and a round-trip test in the gate's node tests.
- [x] The guide's snapshots.md and host.md updated (D2102 rules).

## Log

- **Container** (js/src/snapshot.ts, D2260): a 40-byte header (magic `DJSN`, format 1, app,
  compression, engine build hash, reference kind + FNV-1a 64 + length, image length,
  metadata and payload lengths), JSON metadata, then the collected image raw or as one
  zstd frame (content checksum) with the reference as a raw-content dictionary. Readers
  check container, app, build, reference, checksum, then the image's own verification;
  every refusal is a `SnapshotError` with a plain message (the Sandbox turns it into a
  `misuse` SandboxError). Old `DJSS`/`DJPI` v1 snapshots still load.
- **References**: the module's base snapshot through two new exports, `djs_base_ptr` /
  `djs_base_len` (additive; ABI version stays 1; hosts treat them as optional, like
  `djs_image_staging`); for the pi adapter, the prelude image the session started from
  (D2262). To make the prelude image reproducible, the adapter runs the host globals and
  the prelude with the clock at 0 in UTC (`Session.fixedClock`); the image was the same
  across two compilations (test) and two processes (`cmp` of `prelude.img`). `resumeVm`
  finds the reference among the process's prelude images by hash, or recomputes it from
  `options.prelude` + `options.data`, refusing data that make another image.
- **Sandbox**: `bytes(opts?)` collects first (D2263), writes the container against the base
  image; `fromBytes` reads both formats. `SandboxOptions.snapshotCodec`,
  `BytesOptions.compression`. pi: `VmHooks.snapshotCodec`, `ResumeOptions` (`prelude`,
  `data`).
- **Compression, host vs wasm** (D2261): node's `zlib.zstdCompressSync` with `dictionary`,
  found via `process.getBuiltinModule` (so the package still loads in browsers); browsers
  have no zstd with dictionaries (`CompressionStream` has gzip/deflate, no dictionary), so
  they write uncompressed containers unless a codec is plugged in. In wasm: zstd-rs 0.1
  (pure Rust, no_std, dictionary support) built as a scratch cdylib
  (work/scratch/0117/zwasm): level 3 sizes within 1% of node's (3% at level 19), faster on
  a waiting session's small delta (0.95 vs 1.26 ms), 1.6–1.8x slower on megabytes,
  decompression 0.5–1.9x node's time, frames interoperable both ways, +141 KB module (57
  KB gzip; decoder only 54 KB / 24 KB gzip). Not built in (D020 budget, 0.1 crate).
- **Measurements** (scripts/snapshot-research/wasm.ts, wasm profile, node 25, 144 idle
  points; reports in work/scratch/0117/sr2 and sr3, load ~10, so times are rough). First
  measured on main before 0101 (Cheney collections): waiting sessions 42–48 KB. After the
  rebase onto 0101 (mark-compact, D2040), the same sessions are 2–9 KB:

  | case | image (collected) | container | `snapshot()` | `resumeVm()` | uncompressed resume |
  | --- | ---: | ---: | ---: | ---: | ---: |
  | ask | 546 KB | 2.0 KB | 1.9 ms | 0.7–1.0 ms | 0.6–0.8 ms |
  | agent-loop | 558 KB | 6.9 KB | 2.2–3.0 ms | 1.0–1.2 ms | 0.8–1.0 ms |
  | sequential | 560 KB | 7.3 KB | 2.1–2.8 ms | 0.8–1.1 ms | 0.7–0.9 ms |
  | tool-heavy | 748 KB | 63 KB | 2.5–2.8 ms | 0.9–1.1 ms | 0.6–0.75 ms |
  | records | 1791 KB | 325 KB | 5–7.6 ms | 2.3–3.3 ms | 1.3–1.9 ms |
  | big-log | 2225 KB | 403 KB | 5.3–7.7 ms | 2.0–2.6 ms | 0.7–1.0 ms |
  | Sandbox, new / 1000 records | 395 / 560 KB | 33 / 48 KB | 1.7–2.5 ms | 0.7–1.2 ms | 0.5–0.95 ms |

  Why: sliding compaction keeps allocation order, so a waiting session's image has 99.9%
  of its 8-byte words at the prelude image's offsets (12% under Cheney), and zstd encodes
  them as one match. Level sweep on `ask` (prelude dictionary): L1 126 KB, L3 1.8 KB (1.2
  ms, 0.09 ms to decompress), L6/L9 1.6 KB, L19 1.5 KB (40 ms); base dictionary L3 66 KB;
  no dictionary 134 KB. On records (1.8 MB): L3 324 KB in 4–6 ms, L6 301 KB, L19 265 KB in
  300–400 ms. Window covering both: no effect; long-distance matching: worse. The Sandbox
  stays at 33 KB: its console grows the root shape's transition RefArray (40 bytes at
  offset 42192 of the base) and the atom table's entries (15.5 KB at 2912, exactly full in
  the base); their old cells become garbage near the start, and the compaction shifts
  everything after them (8.6% of words in place). Giving the base's atom entries 256 free
  slots alone did not help (29 KB), so I reverted that (it was outside `touches:`).
- Tests: js/test/snapshot.test.ts (7 tests: Sandbox round trip compressed and not, no
  codec, another build / reference / damage / truncation refused, the DJSS format; pi
  round trip against the prelude image, recompute in a fresh module with the right and
  wrong inputs, base reference without prelude images, the DJPI format, another build,
  the app check). sandbox.test.ts's damaged-image check now uses an uncompressed container.
  All js tests pass with the `wasm` build and the gate's debug build; pi's suite 221/226,
  the same five failures as before.
- Guide pages updated: docs/guide/snapshots.md ("In short", the raw-images trade-off,
  "Images in practice", a new "Compact snapshots" section with two trade-offs, "Where to
  look", decisions) and docs/guide/host.md (the Sandbox example, the adapter's snapshot
  bullet, decisions); `djs-docs check` passes. docs/host.md: "Snapshots" (the layout, the
  checks, compression, numbers), the ABI table, the Sandbox bullet, the pi section, Gaps.
- Outside `touches:`: README.md (one code comment), scripts/snapshot-research/src/pi.rs
  (the prelude's fixed clock, to stay a copy of the adapter).

## Follow-ups

- **Engine registry** (0088): restoring snapshots of older builds needs the host to keep
  those builds (wasm + adapter, keyed by build hash) and a revocation list; the container
  already names the build. Not built.
- **Sandbox snapshots at a few KB**: a new Sandbox's first definitions grow cells that sit
  near the start of the base image (the root shape's transition list, the atom table's
  entries), and the compaction shifts everything after their old places, so its snapshots
  are 33 KB where a pi session's are 2–9 KB. Slack in every growable cell of the base, or a
  Sandbox-level reference (the image right after its console is set up, recomputable from
  the module alone, like pi's prelude image), would line them up again.
- **Browser codec**: the docs site's live examples write uncompressed snapshots. A separate
  zstd-rs wasm module behind `snapshotCodec` would give browsers compact snapshots
  without growing djs-wasm.
- If a future collector reorders cells (or the base image layout is changed by a copying
  collection of a live VM), snapshot sizes jump from a few KB to tens of KB: the
  snapshot-research wasm.ts run shows it at once.

## Result

Snapshots are compact containers (D2260–D2263): the collected image, zstd-compressed
(node's built-in zstd, level 3) against a reference image both sides can produce (the
module's base snapshot, exported as `djs_base_ptr`/`djs_base_len`, or pi's prelude image,
now reproducible), with a header naming the engine build and the reference. The Sandbox
(`bytes()`/`fromBytes()`) and the pi adapter (`snapshot()`/`resumeVm()`) use it; old
`DJSS`/`DJPI` snapshots still load; another build or reference is refused with a message
naming both hashes. With mark-compact collections a pi session waiting on a tool is
546 KB → 2–9 KB (60–270x), one holding megabytes of tool data compresses to about its
data (63–403 KB); a new Sandbox 395 KB → 33 KB. Restore costs 0.1–0.3 ms more. Hosts
without zstd (browsers) write uncompressed containers or plug in a codec; zstd in wasm was
measured and left out (module size). js/test/snapshot.test.ts runs in the gate's node
tests. Known gaps: no engine registry; Sandbox snapshots don't line up with the base image
as well as pi's do with the prelude image (follow-up).
