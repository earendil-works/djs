# 0177: Retire heap images as persistence; decide the packaging

status: done
agent: example
branch: issue/0177-retire-images
depends-on: 0176
touches: crates/djs-cli/src/container.rs, crates/djs-cli/src/heap/, crates/djs-cli/src/repl/, crates/djs-core/src/vm/session.rs and image loading of untrusted images, crates/djs-wasm, js/src/snapshot.ts, js/src/sandbox.ts, js/pi/vm.ts, js/test/, scripts/size, docs/guide/ (snapshots.md, suspension.md, goals.md, host.md), README.md
guide: snapshots.md, suspension.md, goals.md, host.md, introduction.md

## Goal
Once dumps cover the harvested corpus, remove images as a persistence format (D2560) and
decide how wasm hosts get the dumper (D2564).

## Acceptance criteria
- [x] Removed: snapshot containers (Rust and TS), session images, reference compression and
      zstd codec plumbing, `.snapshot`, `Sandbox.bytes`/`fromBytes`, pi's `snapshot`/`resumeVm`,
      the verification of untrusted images where nothing loads them any more. Kept: the base
      image, pi's prelude image cache, `Vm::image`/`from_bytes` for the stress and determinism
      checks and the module transport, `djs heap` on raw images.
- [x] Sizes of both packagings measured after the removal; the packaging decided and recorded
      (a decision superseding or confirming D2564's lean); the size ceiling updated accordingly.
- [x] Guide: the snapshots chapter rewritten around continuation dumps (images as the
      in-process mechanism), suspension.md and goals.md restated (persistence at idle points,
      suspend anywhere inside a process), host.md's protocol.
- [x] Decisions superseded by D2560 marked in a new entry listing what was removed.

## Log

- Removed (D2700): session images (vm/session.rs, `Vm::session_bytes`/`from_session`),
  the CLI container (container.rs, the zstd dependency), the REPL's `.snapshot` and image
  `.restore` (new `.image FILE` writes a raw image for `djs heap`), `djs heap` containers
  and `--reference` (raw images only; the prefix diff computed locally), js/src/snapshot.ts
  (containers, codec, deps checks; js/src/base.ts keeps the base injection),
  `Sandbox.bytes`/`fromBytes`/`snapshotCodec`, pi's `snapshot`/`resumeVm`/`preludeImage`,
  scripts/heap/reference.ts, scripts/determinism/pi_resume.ts, snapshot-research's
  wasm.ts/upgrade.ts, the docs site's snapshot pane.
- djs-wasm's `vm_from_bytes` loads trusted (`Vm::from_bytes_trusted`): the shipped module
  drops the verifier (1,038,666 -> 1,029,026 B; 444.0 -> 440.5 KB gzip). `Vm::from_bytes`
  still verifies for tools and stress modes.
- Kept the dump-module instance between dumps (coordinator, via portable): one per
  Sandbox, `vm_replace_image` / `Vm::replace_image` keeps the VM (and D2626's name table
  once it lands) (D2701). Re-measured with portable's dump-costs (adapted: no containers):
  corpus p50 dump 2.8 ms, resume 0.9 ms in B, level with A.
- Packaging decided: B (D2704); ceilings lowered (441,522; slim 376,679).
- Sandbox maxFuel counts fuel across sync host calls (`vm_fuel_used`, D2702).
- Hosts store dumps zstd level 3 (D2703, docs/host.md "Storing dumps").
- Tests: snapshot.test.ts removed; sandbox.test.ts's journal test replays in a fresh
  sandbox given the same first run; the slices test survives a dump; pi-vm's two snapshot
  tests removed (dump-every-idle covers resume equivalence); REPL test for `.image`.
- Guide: snapshots.md rewritten ("Images and continuation dumps"), goals.md (persistence),
  suspension.md ("Persisting a waiting run"), host.md (protocol, storage, packaging, pi's
  next mode), introduction.md, arena.md, determinism.md; docs/host.md, docs/cli.md,
  README.md, continuation-dumps.md, functions-as-code.md.

## Follow-ups
- ARCH.md and TESTING.md still describe snapshots/containers in places (coordinator's
  documents).
- After 0184 (D2626) lands: re-measure B's dumps with the kept VM (the name table then
  survives between dumps).
- scripts/snapshot-research's Rust crates and docs/{goldilocks,portable}-snapshots.md are
  historical research describing containers.

## Result

Heap images are an in-process mechanism only (base, prelude images, stress/determinism,
dump-module transport); persistence is continuation dumps, stored zstd-compressed by the
host. Packaging B ships: djs.wasm without the dumper (440.5 KB gzip), djs-dump.wasm
beside it (548.9 KB gzip) for hosts that persist, its instance kept between dumps.
Decisions D2700-D2704.
