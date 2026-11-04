# 0212: Exporter cleanup after format 2: dead branches, chain-test allowances

status: done
agent: pjd
branch: issue/0212-dump-dead-code
depends-on: 0205, 0209
guide: -
touches: vm/dump.rs, vm/dump_flight.rs (format-2-only branches), dump chain test

## Goal
0205 and 0209 follow-ups: delete the exporter's format-2-only branches in dump.rs and
dump_flight.rs (and anything else only format 2 reached: helpers, fields, options), and drop
the chain test's `runtime_bytes` allowance if D2951 made it unnecessary. No behaviour change;
record freed bytes (size ceilings) and lower them with `--update`. pjs (dump_cont.rs,
dump_plain.rs callback parts) and pjr (dump_origin.rs, hosts; dump_cont.rs assign/decl_list)
work nearby: one small merge, rebase right before it.

## Log
- `Dumper::pl` is a `plain::Ir`, no longer an `Option`: every `pl.is_some()`/`is_none()`
  branch is gone with its format-2 side. Deleted, as only format 2 reached them:
  `read_patches`/`builtin_patches`/`same_property` and the builtin patch lines (frozen
  builtins, which the dumper requires, returned early), `$dump.builtin(...)`,
  `$dump.host(...)`, `$dump.pending(...)`, `$dump.private(...)` patches and values
  (plain dumps refuse them, D2878), `$dump.iteratorHelper(...)`, the `$dump.using(res, …)`
  line, the write-only `ns` flag, `Builtin::cell`; `rt()` is `"$djs."`. The `$dump.*`
  lines the plain writer still reads (`resume`, `handle`, `cell`, `TDZ`, `using`, `bN`)
  stay: they are its input (D2860), not format 2.
- Chain test: the `runtime_bytes` allowance is gone (D2951 writes no runtime code);
  every link is within 512 bytes of a dump of the run there without it.
- No behaviour change: every plain dump of the corpus (difftest --plain-out) is
  identical before and after, apart from wall-clock digits.
- Size: the shipped modules carry no exporter (the size and slim builds are byte-
  identical in raw size, 1,033,002 and 905,887), so nothing is freed there and the
  ceilings stay; the change only shrinks the dump-feature builds.
- tests/smap/summary.txt: see the proposal below (not built).
- Live docs fix (coordinator, branch issue/0212-docs-live-fix): the site's sandbox.js
  imports ./dump-runtime.js, which crates/djs-docs/assets/strip.mjs never copied (it took
  only `*.ts`). It now copies js/src's `.js` files as they are, and fails `djs-docs build`
  when a relative import of the copied package names no file in the output (checked:
  without dump-runtime.js it reports `sandbox.js: ./dump-runtime.js`).

## Follow-ups
- Proposal (needs the coordinator's OK): tests/smap/summary.txt is derived output, the
  totals over every pinned .smap (sites by kind, role sequences, paths, gaps). Every
  branch that adds or changes a corpus case with an await changes its totals, so two
  such branches always conflict there. Make it regenerated like the other generated
  files: `tests/smap/summary.txt merge=djs-generated` in .gitattributes, and in
  scripts/regen.sh a summary-only rewrite (a new `DJS_UPDATE_SMAP_SUMMARY=1` mode of
  `pinned_maps` that writes summary.txt but still checks the per-case pins, so a real
  map change is not hidden). Alternative: drop the file and print the summary from the
  test on demand; nothing but the test reads it. tests/scores/dump-size.tsv conflicts
  the same way since it is per case (adjacent rows); a "keep main's, re-apply this
  branch's changed rows" driver would fix that, but it needs the measurement, so it is a
  separate question.

## Result
Exporter format-2 branches and what only they reached deleted (dump.rs, dump_flight.rs,
dump_statics.rs); the chain test's runtime allowance dropped. Plain dumps unchanged over
the corpus; shipped module sizes unchanged (they carry no exporter), ceilings kept.
