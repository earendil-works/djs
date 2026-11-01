# 0113: Docs follow-ups: stale reference docs, status numbers, search, live disassembly

status: open
agent: -
branch: issue/0113-docs-followups
depends-on: 0103
guide: docs/guide/ (status numbers, search), ARCH.md, docs/compiler.md
touches: ARCH.md, docs/*.md, docs/guide/, crates/djs-docs/, crates/djs-wasm/ (a docs-only feature export for disassembly)

## Goal
0103's follow-ups:
1. Stale reference docs: ARCH "Interpreter" still names fast_step/fast_more (D1930
   replaced them with run_hot); docs/compiler.md 7.1 gives the frame header as 5 words (it
   is 6, prev_hp); ARCH's table says vm_new is "a few µs" (D1990 measured 39–40 µs). Do a
   pass for similar drift.
2. Generated status numbers (test262/difftest counts from the ratchet files, perf vs
   quickjs-wasi from the bench scores, wasm size) as gen regions, used by README.md and
   the guide, so prose numbers stop going stale.
3. Search on the site (client-side, small).
4. Live disassembly for edited examples (a `djs_disasm` export behind a docs-only djs-wasm
   feature, so the shipping module doesn't grow).

## Acceptance criteria
- [ ] Each item merged separately; `djs-docs check` green; the shipping wasm unchanged in
      size.

## Log

## Follow-ups

## Result
