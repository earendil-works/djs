# 0176: pi's future shape: prelude-next, adapter mode, dumps

status: in-progress
agent: example
branch: issue/0176-pi-future-shape
depends-on: 0175
touches: tests/pi/prelude-next.ts (new), js/pi/, js/test/pi-vm.test.ts, docs/host.md
guide: -

## Goal
Build the shape pi should move to (D2565) in djs's own adapter, and persist pi sessions as
continuation dumps. pi itself is not changed.

## Acceptance criteria
- [ ] tests/pi/prelude-next.ts: pi's prelude with tools as engine async host functions, the
      body's promise as a host handle, `finished` and the store's writes in the adapter; the
      differences listed at its top. tests/pi/prelude-source.ts stays verbatim.
- [ ] An adapter mode that uses it, with `dumpSession()` / `resumeFromDump()`; the prelude is
      rerun on resume (deterministic, D2262) and the dump names its globals as host globals.
- [ ] pi's suite (`node js/pi/pi-tests.ts`) passes in both modes; in the new mode also with
      `--dump-every-idle`, each wait resumed in a new module instance.
- [ ] pi-vm.test.ts covers dump and resume through the adapter, with the open-call policies
      (deliver recorded, re-issue, reject).
- [ ] docs/host.md describes the new shape as what pi should adopt.

## Log

## Follow-ups

## Result
