# 0304: Remove continuation dumps

status: in-progress
agent: psnap
branch: issue/0304-remove-dumps
depends-on: 0301, 0302, 0303
touches: everything dump: crates/djs-core/src/vm/dump*.rs, call_site.rs, compiler/smap*.rs and the dump cfgs in the compiler/interp/heap/builtins, djs-wasm dump exports, js/src (dump-runtime, sandbox dump paths), djs-cli/djs-difftest/djs-docs dump parts, scripts (dump-*), tests/dump, tests/smap, gate legs, docs/guide (dumps.md, continuations.md, suspension.md's persistence part), docs/continuation-dumps.md, TESTING.md, ARCH.md, docs/decisions.md
guide: docs/guide/SUMMARY.md, dumps.md, continuations.md, suspension.md, snapshots.md, js-api.md, cli.md, host.md, pi.md, goals.md

## Goal
With snapshots the persistence format (0301-0303), delete continuation dumps in one issue:
the dumper (~26.6k lines with tests), suspension maps (~2.7k lines plus ~310 recorder calls
in the parser and ~1k lines of dump-only compiler code), the `$djs` hooks and dump runtime,
the dump ABI, the JS paths, the CLI/REPL/difftest parts, the scripts, tests/dump and
tests/smap, the gate legs, `djs_optsize` if nothing else needs it, and the guide pages.
Rules that existed for dumps lapse (origin tracking, builtin state re-enacted, the refusal
taxonomy, statement state in named temporaries as a *requirement*: keep the lowering itself).
No compatibility code (owner: earlier releases need not load).

## Acceptance criteria
- [ ] Nothing of dumps left (grep for dump, smap, $djs, continuation in code, tests, scripts
      and docs; what stays has a reason in the log).
- [ ] One guide page on persistence (snapshots), written for how djs is now; the guide's
      pages that described dumps rewritten or removed; ARCH.md and TESTING.md follow.
- [ ] Size: the shipped module's gz size measured before and after; the ceiling lowered to
      the new size.
- [ ] Decisions recorded (what was removed, superseding the dump decisions).

## Log

## Follow-ups

## Result
