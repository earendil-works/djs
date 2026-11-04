# 0186: ARCH.md and TESTING.md for continuation dumps (images in-process only)

status: done
agent: example
branch: issue/0186-arch-testing-dumps
depends-on: 0177
guide: -
touches: ARCH.md, TESTING.md (prose only; no code)

## Goal
After 0177 (D2700–D2704), ARCH.md and TESTING.md still describe snapshots, session images
and containers as persistence in ~27 places. Rewrite those parts to the current design:
persistence is continuation dumps (D2560 ff., docs/continuation-dumps.md), heap images are an
in-process mechanism (base image, pi's prelude cache, stress/determinism checks, module
transport), packaging B (D2704). Keep ARCH's structure; update goals wording ("Persistence"),
the snapshot sections, the testing sections (golden dumps, dump-every-idle legs, image
checks that remain). Cite decisions; no code changes.

## Log

## Result
Merged e6705afe (4797d7ee..e6705afe). ARCH: G4 = images in-process, dumps persist; host interface lists vm_dump/vm_load_dump/vm_replace_image; "Snapshots (G4)" became "Images and dumps (G4)" with a Superseded trail (D2260/D2066/D2400 -> D2560 -> D2700); base image ~190 KB. TESTING: stress modes include dump-at-every-idle-point; "Snapshot compatibility" renamed "Image compatibility". References updated in AGENTS.md, vm/compat.rs, djs-prelude, docs/host.md.
