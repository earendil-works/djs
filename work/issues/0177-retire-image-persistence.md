# 0177: Retire heap images as persistence; decide the packaging

status: open
agent: -
branch: issue/0177-retire-images
depends-on: 0176
touches: crates/djs-cli/src/container.rs, crates/djs-cli/src/heap/, crates/djs-cli/src/repl/, crates/djs-core/src/vm/session.rs and image loading of untrusted images, crates/djs-wasm, js/src/snapshot.ts, js/src/sandbox.ts, js/pi/vm.ts, js/test/, scripts/size, docs/guide/ (snapshots.md, suspension.md, goals.md, host.md), README.md
guide: snapshots.md, suspension.md, goals.md, host.md, introduction.md

## Goal
Once dumps cover the harvested corpus, remove images as a persistence format (D2560) and
decide how wasm hosts get the dumper (D2564).

## Acceptance criteria
- [ ] Removed: snapshot containers (Rust and TS), session images, reference compression and
      zstd codec plumbing, `.snapshot`, `Sandbox.bytes`/`fromBytes`, pi's `snapshot`/`resumeVm`,
      the verification of untrusted images where nothing loads them any more. Kept: the base
      image, pi's prelude image cache, `Vm::image`/`from_bytes` for the stress and determinism
      checks and the module transport, `djs heap` on raw images.
- [ ] Sizes of both packagings measured after the removal; the packaging decided and recorded
      (a decision superseding or confirming D2564's lean); the size ceiling updated accordingly.
- [ ] Guide: the snapshots chapter rewritten around continuation dumps (images as the
      in-process mechanism), suspension.md and goals.md restated (persistence at idle points,
      suspend anywhere inside a process), host.md's protocol.
- [ ] Decisions superseded by D2560 marked in a new entry listing what was removed.

## Log

## Follow-ups

## Result
