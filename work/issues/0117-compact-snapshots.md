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
  (work/scratch/0117/zwasm): level 3 sizes within 10% of node's, compression 1.1–1.8x and
  decompression 1.5–1.8x slower, frames interoperable both ways, +141 KB module (57 KB
  gzip; decoder only 54 KB / 24 KB gzip). Not built in (D020 budget, 0.1 crate).
- **Measurements** (scripts/snapshot-research/wasm.ts, wasm profile, node 25, 144 idle
  points; full report in work/scratch/0117/sr/wasm-report.md):

  | case | image (collected) | container | `snapshot()` | `resumeVm()` | uncompressed resume |
  | --- | ---: | ---: | ---: | ---: | ---: |
  | ask | 546 KB | 42.3 KB | 1.88 ms | 0.85 ms | 0.60 ms |
  | agent-loop | 558 KB | 46.1 KB | 1.81 ms | 0.86 ms | 0.57 ms |
  | sequential | 560 KB | 46.7 KB | 1.84 ms | 0.80 ms | 0.55 ms |
  | tool-heavy | 718 KB | 96.2 KB | 2.33 ms | 0.96 ms | 0.57 ms |
  | records | 1777 KB | 335 KB | 5.05 ms | 2.37 ms | 1.29 ms |
  | big-log | 2226 KB | 445 KB | 5.68 ms | 2.22 ms | 0.75 ms |
  | Sandbox, new / 1000 records | 395 / 560 KB | 33.3 / 47.1 KB | 1.37 / 1.69 ms | 0.72 / 1.01 ms | 0.49 / 0.60 ms |

  Level sweep on `ask` (prelude dictionary): L1 132 KB, L3 40.0 KB (1.4 ms; 0.21 ms to
  decompress), L6 36.6, L9 34.4, L19 29.8 KB (106 ms); base dictionary L3 70.2 KB; no
  dictionary L3 137 KB, L19 112 KB. A window covering dictionary + image changes nothing;
  long-distance matching is 2% worse. 0088 had estimated 22–27 KB (level 19, CLI); today's
  images give 29.8 KB at level 19 with the CLI too, so the difference is the engine's
  images, not the tooling. Only 12% of a waiting session's 8-byte words sit at the same
  offset as in the prelude image (Cheney order shifts early), which is what keeps the
  compressed delta at ~40 KB when the real change is a few KB (0088's logical estimate).
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
- **Layout-stable images**: only 12% of a waiting session's words sit where they sat in
  the reference, because a collection's order shifts early. If the reference's cells kept
  their offsets (a non-moving region for the base/prelude part, as ARCH "Deltas" once
  proposed, or a collection order that keeps reference cells first and in order), the
  dictionary would match almost everything, and a waiting session could drop from ~40 KB
  toward the few KB it actually changes. Worth measuring once mark-compact (D018) lands,
  since sliding compaction keeps allocation order.
- **Browser codec**: the docs site's live examples write uncompressed snapshots. A separate
  zstd-rs wasm module behind `snapshotCodec` would give browsers compact snapshots
  without growing djs-wasm.

## Result

Snapshots are compact containers (D2260–D2263): the collected image, zstd-compressed
(node's built-in zstd, level 3) against a reference image both sides can produce (the
module's base snapshot, exported as `djs_base_ptr`/`djs_base_len`, or pi's prelude image,
now reproducible), with a header naming the engine build and the reference. The Sandbox
(`bytes()`/`fromBytes()`) and the pi adapter (`snapshot()`/`resumeVm()`) use it; old
`DJSS`/`DJPI` snapshots still load; another build or reference is refused with a message
naming both hashes. A pi session waiting on a tool: 546 KB image → 42–47 KB (13x), restore
+0.3 ms (0.8–0.9 ms); a new Sandbox 395 KB → 33 KB. Hosts without zstd (browsers) write
uncompressed containers or plug in a codec; zstd in wasm was measured and left out (size).
js/test/snapshot.test.ts runs in the gate's node tests. Known gaps: no engine registry; the
compressed size is bounded by layout shifts, not by the session's real changes (follow-up).
