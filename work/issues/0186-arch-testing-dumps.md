# 0186: ARCH.md and TESTING.md for continuation dumps (images in-process only)

status: open
agent: -
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
