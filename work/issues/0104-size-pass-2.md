# 0104: wasm size pass 2

status: in-progress
agent: num
branch: issue/0104-size-pass-2
depends-on: 0099
touches: crates/ (code-size changes anywhere, coordinated: heap is in heap/ (0098), perf in bgen/natives/hot.rs (0100), infra in js/pi + cli + error builtins (0102)), Cargo.toml profiles only with a recorded decision

## Goal
Since 0057 (code 709 KB, module 1,241 KB raw / 428 KB gzip) the module grew to ~1,515 KB raw /
531 KB gzip (0095 README): RegExp \p (+27 KB gz), normalization (+20 KB gz), natives (+80 KB
code), @fast (+24 KB), shapes (+12 KB code), JSON native (+10.5 KB), and more. D016: size
counts. Do a second measured pass with the 0052/0043 tools (scripts/wasm-size.py,
wasm-funcs.py, wasm-dwarf-lines.py, native-sizes.sh).

## Acceptance criteria
- [ ] Attribution of the growth since 0057 by area, and the current top 30 functions.
- [ ] Reductions that cost no measurable speed (wasm instructions on the perf workloads):
      duplicated code between @fast entries, dense.rs and natives, monomorphization over
      string widths, inlining of cold paths, table encodings. Coordinate with the areas'
      owners through the coordinator before touching their files.
- [ ] Before/after: raw, gzip, brotli, code and data; no behavior change.

## Log

## Follow-ups

## Result
